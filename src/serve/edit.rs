//! Browser editing: which site paths are editable pages, the source payload the editor
//! works from, the checks on edit requests, and safe atomic writes of note files.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write as _};
use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::graph::Graph;
use crate::layout::{self, Date, JOURNALS_DIR, PAGES_DIR, ROOT_INDEX};
use crate::parse::{self, BlockKind};
use crate::search::index::json_string;
use crate::site;

use super::http::{self, Request};

/// Upper bound on a saved page.
pub const MAX_BODY: usize = 8 * 1024 * 1024;
/// Version of a page that has no file.
pub const MISSING: &str = "missing";
/// Request header carrying the version a save is based on.
pub const VERSION_HEADER: &str = "x-hermetic-version";
/// Request header every write must carry; it forces a CORS preflight on other origins.
pub const EDIT_HEADER: &str = "x-hermetic-edit";

/// A page that can be edited: its name, its date for journal days, and its source file
/// relative to the notes root (`None` when it has none yet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Editable {
    pub name: String,
    pub date: Option<Date>,
    pub file: Option<PathBuf>,
}

/// Editable pages by site path (`/`-separated, not percent-encoded).
pub type EditMap = BTreeMap<String, Editable>;

/// Every page of the graph by the site path of its HTML file. Site-wide pages are not
/// pages of the graph, so they are never included; `index.html` is included only when
/// the root `index.md` makes it the page `index`.
pub fn editable_pages(g: &Graph) -> EditMap {
    g.pages()
        .map(|(id, p)| {
            let path = site::page_output_path(g, id);
            let key = path
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let page = Editable {
                name: p.name.clone(),
                date: p.date(),
                file: p.file.clone(),
            };
            (key, page)
        })
        .collect()
}

/// The editable page at a site path: a page of the build, the home page (`index.html`,
/// even before `index.md` exists), or a journal day that has no page yet
/// (`journals/YYYY-MM-DD.html` with a valid date).
pub fn resolve(map: &EditMap, path: &str) -> Option<Editable> {
    if let Some(page) = map.get(path) {
        return Some(page.clone());
    }
    // The home page can always be written, which creates the root `index.md`.
    if path == "index.html" {
        return Some(Editable {
            name: layout::ROOT_INDEX_PAGE.to_owned(),
            date: None,
            file: None,
        });
    }
    let day = path
        .strip_prefix(JOURNALS_DIR)?
        .strip_prefix('/')?
        .strip_suffix(".html")?;
    let date = Date::parse(day)?;
    Some(Editable {
        name: day.to_owned(),
        date: Some(date),
        file: None,
    })
}

/// The source file of a page relative to the notes root: its existing file, or the
/// path its name maps to.
pub fn source_path(page: &Editable) -> PathBuf {
    match (&page.file, page.date) {
        (Some(file), _) => file.clone(),
        (None, Some(date)) => layout::journal_file_path(date),
        // The page `index` is the home page, stored at the notes root.
        (None, None) if layout::is_home_name(&page.name) => PathBuf::from(ROOT_INDEX),
        (None, None) => layout::page_file_path(&page.name),
    }
}

/// Whether a request comes from this machine: a loopback peer that addressed the
/// server by a loopback name (other names indicate DNS rebinding).
pub fn same_machine(peer: IpAddr, request: &Request, port: u16) -> bool {
    let loopback = match peer {
        IpAddr::V4(ip) => ip.is_loopback(),
        IpAddr::V6(ip) => {
            ip.is_loopback() || ip.to_ipv4_mapped().is_some_and(|v4| v4.is_loopback())
        }
    };
    loopback
        && request
            .header("host")
            .is_some_and(|h| http::is_loopback_host(h, port))
}

/// Whether a write request was sent by hermetic's own editor: same origin as the
/// `Host` it addressed, with the edit header.
pub fn from_editor(request: &Request) -> bool {
    let (Some(host), Some(origin)) = (request.header("host"), request.header("origin")) else {
        return false;
    };
    origin.eq_ignore_ascii_case(&format!("http://{host}"))
        && request.header(EDIT_HEADER) == Some("1")
}

/// The version of a file's bytes (64-bit FNV-1a, 16 hex digits), or [`MISSING`].
pub fn version(bytes: Option<&[u8]>) -> String {
    let Some(bytes) = bytes else {
        return MISSING.to_owned();
    };
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Why a target file may not be written.
#[derive(Debug)]
pub enum Unsafe {
    /// Outside `journals/`, `pages/`, and the root `index.md`, or not a regular file.
    Outside,
    Io(io::Error),
}

impl From<io::Error> for Unsafe {
    fn from(e: io::Error) -> Self {
        Unsafe::Io(e)
    }
}

/// The absolute path to write for `rel` (relative to the canonical notes `root`), with
/// symbolic links resolved. It must lie under `journals/` or `pages/` of the root, or
/// be the root `index.md`; a missing file is judged by its nearest existing ancestor
/// directory, and the directories in between are created only by [`write_atomic`].
pub fn checked_target(root: &Path, rel: &Path) -> Result<PathBuf, Unsafe> {
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(Unsafe::Outside);
    }
    let path = root.join(rel);
    let resolved = match fs::symlink_metadata(&path) {
        Ok(_) => {
            let canon = fs::canonicalize(&path)?;
            if !fs::metadata(&canon)?.is_file() {
                return Err(Unsafe::Outside);
            }
            canon
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let mut ancestor = path.parent().ok_or(Unsafe::Outside)?;
            let mut rest = vec![path.file_name().ok_or(Unsafe::Outside)?];
            while fs::symlink_metadata(ancestor).is_err() {
                rest.push(ancestor.file_name().ok_or(Unsafe::Outside)?);
                ancestor = ancestor.parent().ok_or(Unsafe::Outside)?;
            }
            let mut canon = fs::canonicalize(ancestor)?;
            if !canon.is_dir() {
                return Err(Unsafe::Outside);
            }
            for part in rest.iter().rev() {
                canon.push(part);
            }
            canon
        }
        Err(e) => return Err(e.into()),
    };
    if allowed(root, &resolved) {
        Ok(resolved)
    } else {
        Err(Unsafe::Outside)
    }
}

fn allowed(root: &Path, path: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(root) else {
        return false;
    };
    let mut parts = rel.components();
    match (parts.next(), parts.next()) {
        (Some(Component::Normal(first)), Some(_)) => first == JOURNALS_DIR || first == PAGES_DIR,
        (Some(Component::Normal(first)), None) => first == ROOT_INDEX,
        _ => false,
    }
}

/// The bytes to store for an editor's text: LF converted to CRLF when the existing file
/// uses CRLF (judged by its first line ending), and a final line ending added.
pub fn file_bytes(text: &str, existing: Option<&[u8]>) -> Vec<u8> {
    let mut text = text.replace("\r\n", "\n");
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    let crlf = existing.is_some_and(|bytes| {
        bytes
            .iter()
            .position(|&b| b == b'\n')
            .is_some_and(|i| i > 0 && bytes[i - 1] == b'\r')
    });
    if crlf {
        text.replace('\n', "\r\n").into_bytes()
    } else {
        text.into_bytes()
    }
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Writes `bytes` to `path` atomically: into a hidden temporary file in the same
/// directory (which the build and the watcher ignore), synced, then renamed over the
/// target. Missing parent directories are created and checked to stay inside `root`.
/// The temporary file is removed when anything fails.
pub fn write_atomic(root: &Path, path: &Path, bytes: &[u8]) -> Result<(), Unsafe> {
    let dir = path.parent().ok_or(Unsafe::Outside)?;
    fs::create_dir_all(dir)?;
    let canon_dir = fs::canonicalize(dir)?;
    if canon_dir != dir || !allowed(root, path) {
        return Err(Unsafe::Outside);
    }
    let name = path.file_name().ok_or(Unsafe::Outside)?.to_string_lossy();
    let tmp = dir.join(format!(
        ".{name}.hermetic-{}-{}.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        file.write_all(bytes)?;
        if let Ok(meta) = fs::metadata(path) {
            file.set_permissions(meta.permissions())?;
        }
        file.sync_all()?;
        drop(file);
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    Ok(result?)
}

/// The trash, in the notes root: a dot-directory, so the build and the watcher ignore it.
pub const TRASH_DIR: &str = ".trash";

/// Moves `path` (a checked target, see [`checked_target`]) to
/// `.trash/<timestamp>/<its path relative to root>`, then removes directories under
/// `pages/` that this left empty. Returns the trash path relative to `root`.
pub fn move_to_trash(root: &Path, path: &Path) -> Result<PathBuf, Unsafe> {
    if !allowed(root, path) {
        return Err(Unsafe::Outside);
    }
    let rel = path.strip_prefix(root).map_err(|_| Unsafe::Outside)?;
    let stamp = timestamp(std::time::SystemTime::now());
    let mut trashed = PathBuf::from(TRASH_DIR).join(&stamp).join(rel);
    let mut n = 1;
    while root.join(&trashed).exists() {
        n += 1;
        trashed = PathBuf::from(TRASH_DIR)
            .join(format!("{stamp}-{n}"))
            .join(rel);
    }
    // `.trash` must not lead out of the notes (e.g. as a symbolic link): checked before
    // anything is created in it, and again after.
    let trash = root.join(TRASH_DIR);
    if fs::symlink_metadata(&trash).is_ok() && !fs::canonicalize(&trash)?.starts_with(root) {
        return Err(Unsafe::Outside);
    }
    let dest = root.join(&trashed);
    let dir = dest.parent().ok_or(Unsafe::Outside)?;
    fs::create_dir_all(dir)?;
    if !fs::canonicalize(dir)?.starts_with(root) {
        return Err(Unsafe::Outside);
    }
    fs::rename(path, &dest)?;
    prune_empty_dirs(root, path.parent());
    Ok(trashed)
}

/// Removes now-empty directories from `dir` upwards, inside `pages/` only (never
/// `pages/` itself).
fn prune_empty_dirs(root: &Path, mut dir: Option<&Path>) {
    let pages = root.join(PAGES_DIR);
    while let Some(d) = dir {
        if d == pages || !d.starts_with(&pages) || fs::remove_dir(d).is_err() {
            return;
        }
        dir = d.parent();
    }
}

/// A UTC timestamp usable in file names on every platform: `2026-10-12T14-03-05Z`.
fn timestamp(time: std::time::SystemTime) -> String {
    let secs = time
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}-{:02}-{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// The source payload: whether the page has a file, the file's path relative to the
/// notes root (where it is or would be), its version, its text (LF line
/// endings), the indentation unit for new children, and its blocks in document order.
/// `None` when the file is not UTF-8.
pub fn source_json(file: &Path, bytes: Option<&[u8]>) -> Option<String> {
    let text = match bytes {
        Some(b) => std::str::from_utf8(b).ok()?.replace("\r\n", "\n"),
        None => String::new(),
    };
    let blocks = parse::parse_blocks(&text);
    let mut depths: Vec<u32> = Vec::with_capacity(blocks.len());
    let mut out = format!(
        "{{\"exists\":{},\"file\":{},\"version\":{},\"text\":{},\"indent\":{},\"blocks\":[",
        bytes.is_some(),
        json_string(&file.to_string_lossy().replace('\\', "/")),
        json_string(&version(bytes)),
        json_string(&text),
        json_string(&indent_unit(&text)),
    );
    for (i, b) in blocks.iter().enumerate() {
        let depth = b.parent.map_or(0, |p| depths[p] + 1);
        depths.push(depth);
        let anchor = match b.prop("id").filter(|id| parse::is_valid_block_id(id)) {
            Some(id) => format!("id-{id}"),
            None => format!("b{}", i + 1),
        };
        if i > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "{{\"lines\":[{},{}],\"first\":{},\"depth\":{depth},\"bullet\":{},\"anchor\":{},\"body\":{},\"column\":{}}}",
            b.lines.start,
            b.lines.end,
            b.line - 1,
            b.kind == BlockKind::Bullet,
            json_string(&anchor),
            json_string(&b.body),
            b.column
        );
    }
    out.push_str("]}");
    Some(out)
}

/// The leading whitespace of the first indented bullet: a tab, or that many spaces.
/// Two spaces when no bullet is indented.
pub fn indent_unit(text: &str) -> String {
    for line in text.lines() {
        let rest = line.trim_start_matches([' ', '\t']);
        let ws = &line[..line.len() - rest.len()];
        if !ws.is_empty() && (rest == "-" || rest.starts_with("- ")) {
            return if ws.starts_with('\t') {
                "\t".to_owned()
            } else {
                ws.chars().take_while(|&c| c == ' ').collect()
            };
        }
    }
    "  ".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    fn req(headers: &[(&str, &str)]) -> Request {
        Request {
            method: "PUT".to_owned(),
            target: "/_hermetic/source?path=a.html".to_owned(),
            headers: headers
                .iter()
                .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
                .collect(),
        }
    }

    #[test]
    fn same_machine_matrix() {
        let local = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let mapped = IpAddr::V6(Ipv4Addr::LOCALHOST.to_ipv6_mapped());
        let v6 = IpAddr::V6(Ipv6Addr::LOCALHOST);
        let lan = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 23));
        for host in [
            "127.0.0.1:8000",
            "localhost:8000",
            "[::1]:8000",
            "LOCALHOST",
        ] {
            let r = req(&[("host", host)]);
            for peer in [local, mapped, v6] {
                assert!(same_machine(peer, &r, 8000), "{host} from {peer}");
            }
            assert!(!same_machine(lan, &r, 8000), "{host} from the network");
        }
        for host in ["evil.example:8000", "192.168.1.23:8000", "127.0.0.1:9000"] {
            assert!(
                !same_machine(local, &req(&[("host", host)]), 8000),
                "{host}"
            );
        }
        assert!(!same_machine(local, &req(&[]), 8000));
    }

    #[test]
    fn editor_requests() {
        let ok = [
            ("host", "127.0.0.1:8000"),
            ("origin", "http://127.0.0.1:8000"),
            (EDIT_HEADER, "1"),
        ];
        assert!(from_editor(&req(&ok)));
        assert!(!from_editor(&req(&ok[..2])), "without the edit header");
        assert!(!from_editor(&req(&[ok[0], ok[2]])), "without an origin");
        let other = [ok[0], ("origin", "https://evil.example"), ok[2]];
        assert!(!from_editor(&req(&other)), "another origin");
        let https = [ok[0], ("origin", "https://127.0.0.1:8000"), ok[2]];
        assert!(!from_editor(&req(&https)), "another scheme");
        let null = [ok[0], ("origin", "null"), ok[2]];
        assert!(!from_editor(&req(&null)));
    }

    #[test]
    fn timestamps() {
        let at = |s: u64| timestamp(std::time::UNIX_EPOCH + std::time::Duration::from_secs(s));
        assert_eq!(at(0), "1970-01-01T00-00-00Z");
        assert_eq!(at(1_791_806_585), "2026-10-12T12-03-05Z");
        assert_eq!(at(951_782_400), "2000-02-29T00-00-00Z");
    }

    #[test]
    fn versions() {
        assert_eq!(version(None), "missing");
        assert_eq!(version(Some(b"")), "cbf29ce484222325");
        assert_ne!(version(Some(b"- a\n")), version(Some(b"- a\r\n")));
        assert_eq!(version(Some(b"a")).len(), 16);
    }

    #[test]
    fn line_endings() {
        assert_eq!(file_bytes("- a\n- b", None), b"- a\n- b\n");
        assert_eq!(file_bytes("- a\n", Some(b"- x\r\n- y\r\n")), b"- a\r\n");
        assert_eq!(file_bytes("- a\n- b\n", Some(b"- x\n")), b"- a\n- b\n");
        assert_eq!(file_bytes("- a\r\n", Some(b"- x\n")), b"- a\n");
        assert_eq!(file_bytes("", Some(b"- x\n")), b"");
        assert_eq!(file_bytes("-", Some(b"\r\n")), b"-\r\n");
    }

    #[test]
    fn indent_units() {
        assert_eq!(indent_unit("- a\n\t- b\n"), "\t");
        assert_eq!(indent_unit("- a\n    - b\n"), "    ");
        assert_eq!(indent_unit("- a\n  more\n   - b\n"), "   ");
        assert_eq!(indent_unit("- a\n- b\n"), "  ");
        assert_eq!(indent_unit("text\n  not a bullet\n"), "  ");
    }

    #[test]
    fn journal_days_without_pages() {
        let map = EditMap::new();
        let day = resolve(&map, "journals/2026-10-13.html").unwrap();
        assert_eq!(day.name, "2026-10-13");
        assert_eq!(source_path(&day), PathBuf::from("journals/2026-10-13.md"));
        let home = resolve(&map, "index.html").unwrap();
        assert_eq!(source_path(&home), PathBuf::from("index.md"));
        for path in [
            "journals/2026-13-01.html",
            "journals/2026-10-13.md",
            "journals.html",
            "tasks.html",
            "graph.html",
            "pages/index.html",
            "pages/x.html",
            "journals/sub/2026-10-13.html",
        ] {
            assert_eq!(resolve(&map, path), None, "{path}");
        }
    }

    #[test]
    fn payload() {
        let json = source_json(
            Path::new("pages/a.md"),
            Some(b"intro\n\n- a\n  id:: x1\n\t- b\r\n  more\n- \n"),
        )
        .unwrap();
        assert!(
            json.starts_with(r#"{"exists":true,"file":"pages/a.md","version":""#),
            "{json}"
        );
        assert!(json.contains(r#""indent":"\u0009""#), "{json}");
        assert!(json.contains(r#"{"lines":[0,2],"first":0,"depth":0,"bullet":false,"anchor":"b1","body":"intro","column":0}"#), "{json}");
        assert!(json.contains(r#"{"lines":[2,4],"first":2,"depth":0,"bullet":true,"anchor":"id-x1","body":"a\u000aid:: x1","column":2}"#), "{json}");
        assert!(
            json.contains(
                r#""depth":1,"bullet":true,"anchor":"b3","body":"b\u000amore","column":6"#
            ),
            "{json}"
        );
        assert!(
            json.contains(r#""anchor":"b4","body":"","column":2"#),
            "{json}"
        );
        let none = source_json(Path::new("pages/b.md"), None).unwrap();
        assert_eq!(
            none,
            r#"{"exists":false,"file":"pages/b.md","version":"missing","text":"","indent":"  ","blocks":[]}"#
        );
        assert_eq!(source_json(Path::new("x.md"), Some(b"\xff")), None);
    }

    #[cfg(unix)]
    #[test]
    fn targets_stay_inside() {
        let base = std::env::temp_dir().join(format!("hermetic-edit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let root = base.join("notes");
        fs::create_dir_all(root.join("pages")).unwrap();
        fs::write(base.join("outside.md"), "- secret\n").unwrap();
        std::os::unix::fs::symlink(base.join("outside.md"), root.join("pages/out.md")).unwrap();
        std::os::unix::fs::symlink(&base, root.join("pages/escape")).unwrap();
        fs::write(root.join("pages/a.md"), "- a\n").unwrap();
        let root = fs::canonicalize(&root).unwrap();

        let ok = |rel: &str| checked_target(&root, Path::new(rel)).is_ok();
        assert!(ok("pages/a.md"));
        assert!(ok("pages/new/deeper/page.md"));
        assert!(ok("journals/2026-10-13.md"));
        assert!(ok("index.md"));
        assert!(!ok("pages/out.md"), "a link to a file outside");
        assert!(
            !ok("pages/escape/outside.md"),
            "a link to a directory outside"
        );
        assert!(!ok("pages/escape/new.md"), "a new file behind a link");
        assert!(!ok("assets/a.md"));
        assert!(!ok("other.md"));
        assert!(!ok("pages/../other.md"));
        assert!(!ok("pages"), "a directory");

        let target = checked_target(&root, Path::new("pages/Project X/Auth.md")).unwrap();
        write_atomic(&root, &target, b"- first note\n").unwrap();
        assert_eq!(
            fs::read(root.join("pages/Project X/Auth.md")).unwrap(),
            b"- first note\n"
        );
        let target = checked_target(&root, Path::new("pages/a.md")).unwrap();
        write_atomic(&root, &target, b"- b\n").unwrap();
        assert_eq!(fs::read(root.join("pages/a.md")).unwrap(), b"- b\n");
        let leftovers: Vec<_> = fs::read_dir(root.join("pages"))
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
        assert_eq!(fs::read(base.join("outside.md")).unwrap(), b"- secret\n");

        // A failed write (the target is a directory) leaves no temporary file behind.
        fs::create_dir_all(root.join("pages/dir.md")).unwrap();
        assert!(write_atomic(&root, &root.join("pages/dir.md"), b"x").is_err());
        assert!(
            fs::read_dir(root.join("pages"))
                .unwrap()
                .flatten()
                .all(|e| !e.file_name().to_string_lossy().ends_with(".tmp"))
        );
        fs::remove_dir_all(&base).unwrap();
    }
}
