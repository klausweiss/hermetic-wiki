//! Browser editing: which site paths are editable pages, the source payload the editor
//! works from, the checks on edit requests, and safe atomic writes of note files.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write as _};
use std::net::IpAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use crate::graph::Graph;
use crate::layout::{self, ASSETS_DIR, Address, Date, JOURNALS_DIR, PAGES_DIR, ROOT_INDEX};
use crate::parse::{self, BlockKind};
use crate::site::{self, url_path};
use crate::{fnv, json};

use super::http::{self, Request};

/// Upper bound on a saved page.
pub const MAX_BODY: usize = 8 * 1024 * 1024;
/// Version of a page that has no file.
pub const MISSING: &str = "missing";
/// Request header carrying the version a save is based on.
pub const VERSION_HEADER: &str = "x-hermetic-version";
/// Request header every write must carry; it forces a CORS preflight on other origins.
pub const EDIT_HEADER: &str = "x-hermetic-edit";
/// Upper bound on an uploaded image.
pub const MAX_ASSET: usize = 32 * 1024 * 1024;
/// Request header with an uploaded image's original file name, percent-encoded.
pub const NAME_HEADER: &str = "x-hermetic-name";
/// Longest stem of an uploaded image's file name.
const MAX_STEM: usize = 64;

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
            let key = url_path(&path);
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
/// even before `index.md` exists), a journal day that has no page yet
/// (`journals/YYYY-MM-DD.html` with a valid date), or a page that has none
/// (`pages/<encoded name>.html`, e.g. one whose empty file was just removed).
pub fn resolve(map: &EditMap, path: &str) -> Option<Editable> {
    if let Some(page) = map.get(path) {
        return Some(page.clone());
    }
    // A page without a file: the home page (written to the root `index.md`), a journal
    // day, or a page at exactly the path its name maps to.
    let address = Address::from_html_path(path)?;
    let date = match address {
        Address::Journal(date) => Some(date),
        _ => None,
    };
    Some(Editable {
        name: address.name().into_owned(),
        date,
        file: None,
    })
}

/// The source file of a page relative to the notes root: its existing file, or the
/// path its name maps to.
pub fn source_path(page: &Editable) -> PathBuf {
    match &page.file {
        Some(file) => file.clone(),
        None => Address::of(&page.name, page.date).source_path(),
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
    format!("{:016x}", fnv::hash(bytes))
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

/// The extension for an image the editor may upload, recognized by its first bytes
/// alone: PNG, JPEG, GIF, or WebP. Anything else (SVG included) is `None`.
pub fn sniff_image(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("gif")
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

/// The stem for an uploaded image's file name, from its original name (percent-encoded,
/// as sent in [`NAME_HEADER`]): without folders and extension, characters other than
/// ASCII letters, digits, `-` and `_` replaced by `_`, at most [`MAX_STEM`] long, and
/// `image` when nothing is left.
pub fn asset_stem(name: Option<&str>) -> String {
    let name = name.and_then(layout::percent_decode).unwrap_or_default();
    let base = name.rsplit(['/', '\\']).next().unwrap_or_default();
    let stem = match base.rfind('.') {
        Some(dot) if dot > 0 => &base[..dot],
        _ => base,
    };
    let stem: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(MAX_STEM)
        .collect();
    if stem.chars().all(|c| c == '_') {
        "image".to_owned()
    } else {
        stem
    }
}

/// The markdown that shows the image `assets/<file_name>` in the note at `source`
/// (relative to the notes root): its destination is relative to the note's folder.
pub fn asset_markdown(source: &Path, stem: &str, file_name: &str) -> String {
    let ext = file_name.rsplit('.').next().unwrap_or_default();
    let up = source.parent().map_or(0, |p| p.components().count());
    format!(
        "![{stem}.{ext}]({}{ASSETS_DIR}/{file_name})",
        "../".repeat(up)
    )
}

/// Writes a new image to `assets/<stem>_<millis>_<n>.<ext>` in the notes `root`
/// (canonical), with the smallest `n` that names no existing file, and returns its path
/// relative to `root`. An existing file is never replaced: the image is written to a
/// hidden temporary file and then hard-linked to its name, which fails when the name is
/// taken. `assets/` is created when missing, and must not lead out of `root/assets`.
pub fn write_new_asset(
    root: &Path,
    stem: &str,
    ext: &str,
    millis: u128,
    bytes: &[u8],
) -> Result<PathBuf, Unsafe> {
    let dir = root.join(ASSETS_DIR);
    fs::create_dir_all(&dir)?;
    if fs::canonicalize(&dir)? != dir {
        return Err(Unsafe::Outside);
    }
    let tmp = dir.join(format!(
        ".{stem}.hermetic-{}-{}.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        for n in 0.. {
            let name = format!("{stem}_{millis}_{n}.{ext}");
            let path = dir.join(&name);
            match fs::hard_link(&tmp, &path) {
                Ok(()) => return Ok(name),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => {
                    // Some filesystems have no hard links: write in place, still never
                    // replacing a file, but no longer atomically.
                    eprintln!(
                        "warning: cannot link {}: {e}; writing it directly",
                        path.display()
                    );
                    match fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&path)
                    {
                        Ok(mut file) => {
                            let written = file.write_all(bytes).and_then(|()| file.sync_all());
                            if written.is_err() {
                                let _ = fs::remove_file(&path);
                            }
                            return written.map(|()| name);
                        }
                        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                        Err(e) => return Err(e),
                    }
                }
            }
        }
        unreachable!("some name is free")
    })();
    let _ = fs::remove_file(&tmp);
    Ok(PathBuf::from(ASSETS_DIR).join(result?))
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
    move_into_trash(root, path, &trash_dir(root))
}

/// A folder of the trash, relative to `root`, that does not exist yet:
/// `.trash/<timestamp>`, or `.trash/<timestamp>-<n>` when that is taken.
pub fn trash_dir(root: &Path) -> PathBuf {
    let stamp = timestamp(std::time::SystemTime::now());
    let mut dir = PathBuf::from(TRASH_DIR).join(&stamp);
    let mut n = 1;
    while fs::symlink_metadata(root.join(&dir)).is_ok() {
        n += 1;
        dir = PathBuf::from(TRASH_DIR).join(format!("{stamp}-{n}"));
    }
    dir
}

/// Moves the unused attachment `rel` (relative to the canonical notes `root`) into the
/// trash folder `dir` (see [`trash_dir`]), keeping its relative path. It must be a
/// regular file under `journals/`, `pages/`, or `assets/` with symbolic links resolved,
/// and not a note. Returns its path in the trash, relative to `root`.
pub fn trash_attachment(root: &Path, rel: &Path, dir: &Path) -> Result<PathBuf, Unsafe> {
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(Unsafe::Outside);
    }
    let path = root.join(rel);
    if fs::canonicalize(&path)? != path || !fs::symlink_metadata(&path)?.is_file() {
        return Err(Unsafe::Outside);
    }
    if !allowed_attachment(root, &path) {
        return Err(Unsafe::Outside);
    }
    move_into_trash(root, &path, dir)
}

fn allowed_attachment(root: &Path, path: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(root) else {
        return false;
    };
    let mut parts = rel.components();
    let name = rel.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    match (parts.next(), parts.next()) {
        (Some(Component::Normal(first)), Some(_)) => {
            (first == JOURNALS_DIR || first == PAGES_DIR || first == ASSETS_DIR)
                && !layout::is_markdown(name)
        }
        _ => false,
    }
}

/// Moves `path` (checked by the caller) to `dir/<its path relative to root>`, then
/// removes directories under `pages/` that this left empty. Returns the trash path
/// relative to `root`.
fn move_into_trash(root: &Path, path: &Path, dir: &Path) -> Result<PathBuf, Unsafe> {
    let rel = path.strip_prefix(root).map_err(|_| Unsafe::Outside)?;
    let trashed = dir.join(rel);
    // `.trash` must not lead out of the notes (e.g. as a symbolic link): checked before
    // anything is created in it, and again after.
    let trash = root.join(TRASH_DIR);
    if fs::symlink_metadata(&trash).is_ok() && !fs::canonicalize(&trash)?.starts_with(root) {
        return Err(Unsafe::Outside);
    }
    let dest = root.join(&trashed);
    if fs::symlink_metadata(&dest).is_ok() {
        return Err(io::Error::from(io::ErrorKind::AlreadyExists).into());
    }
    let parent = dest.parent().ok_or(Unsafe::Outside)?;
    fs::create_dir_all(parent)?;
    if !fs::canonicalize(parent)?.starts_with(root) {
        return Err(Unsafe::Outside);
    }
    ignore_trash(&trash);
    fs::rename(path, &dest)?;
    prune_empty_dirs(root, path.parent());
    Ok(trashed)
}

/// How long an empty note is kept after its file last changed, so that a page just
/// opened with "Today" or a new link stays while the author is about to write in it.
pub const EMPTY_NOTE_GRACE: Duration = Duration::from_secs(60 * 60);

/// Whether a note has no content: only blank lines, or one empty bullet (`-`).
pub fn is_empty_note(text: &str) -> bool {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    matches!((lines.next(), lines.next()), (None, _) | (Some("-"), None))
}

/// Removes the empty notes (page files under `journals/` and `pages/` and the root
/// `index.md`, see [`is_empty_note`]) that have not changed for `grace` before `now`,
/// then the directories under `pages/` this left empty. They hold nothing, so they are
/// removed rather than moved to the trash. Returns the removed paths relative to `root`.
pub fn remove_empty_notes(root: &Path, now: SystemTime, grace: Duration) -> Vec<PathBuf> {
    let mut removed = Vec::new();
    for source in layout::scan(root, &mut Vec::new()) {
        let path = root.join(&source.rel);
        // Symbolic links (the file or a directory on its way) are left alone: what they
        // point to is not the note's own file.
        if fs::canonicalize(&path).ok().as_deref() != Some(path.as_path()) {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        let old = meta
            .modified()
            .ok()
            .and_then(|m| now.duration_since(m).ok())
            .is_some_and(|age| age >= grace);
        if !meta.is_file() || !old {
            continue;
        }
        let empty = fs::read(&path)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .is_some_and(|text| is_empty_note(&text));
        if empty && remove_note(root, &path).is_ok() {
            removed.push(source.rel);
        }
    }
    removed.sort();
    removed
}

/// Deletes the note file at `path` (a checked target, see [`checked_target`]) without
/// moving it to the trash, then removes directories under `pages/` that this left empty.
pub fn remove_note(root: &Path, path: &Path) -> io::Result<()> {
    fs::remove_file(path)?;
    prune_empty_dirs(root, path.parent());
    Ok(())
}

/// Lets git ignore the trash: writes `.gitignore` into it unless one exists (whatever
/// its content, even empty, it is the author's). Best effort: a failure does not stop
/// the deletion.
fn ignore_trash(trash: &Path) {
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(trash.join(".gitignore"));
    if let Ok(mut file) = file {
        let _ = file.write_all(TRASH_GITIGNORE.as_bytes());
    }
}

const TRASH_GITIGNORE: &str = "\
# Pages and files deleted in the browser by `hermetic serve`; never emptied automatically.
# To track the trash in git, empty this file instead of deleting it.
*
";

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
        json::string(&url_path(file)),
        json::string(&version(bytes)),
        json::string(&text),
        json::string(&indent_unit(&text)),
    );
    for (i, b) in blocks.iter().enumerate() {
        let depth = b.parent.map_or(0, |p| depths[p] + 1);
        depths.push(depth);
        let anchor = parse::anchor(b.prop("id").filter(|id| parse::is_valid_block_id(id)), i);
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
            json::string(&anchor),
            json::string(&b.body),
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
    fn pages_without_files() {
        let map = EditMap::new();
        let day = resolve(&map, "journals/2026-10-13.html").unwrap();
        assert_eq!(day.name, "2026-10-13");
        assert_eq!(source_path(&day), PathBuf::from("journals/2026_10_13.md"));
        let home = resolve(&map, "index.html").unwrap();
        assert_eq!(source_path(&home), PathBuf::from("index.md"));
        let page = resolve(&map, "pages/Project X/Auth.html").unwrap();
        assert_eq!(page.name, "Project X/Auth");
        assert_eq!(source_path(&page), PathBuf::from("pages/Project X/Auth.md"));
        for path in [
            "journals/2026-13-01.html",
            "journals/2026-10-13.md",
            "journals.html",
            "tasks.html",
            "graph.html",
            "pages/index.html",
            "pages/x.md",
            "pages/ x.html",
            "pages/a//b.html",
            "pages/../x.html",
            "pages.html",
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

    #[test]
    fn image_formats() {
        assert_eq!(sniff_image(b"\x89PNG\r\n\x1a\n...."), Some("png"));
        assert_eq!(sniff_image(&[0xFF, 0xD8, 0xFF, 0xE0, 0]), Some("jpg"));
        assert_eq!(sniff_image(b"GIF89a...."), Some("gif"));
        assert_eq!(sniff_image(b"GIF87a...."), Some("gif"));
        assert_eq!(sniff_image(b"RIFF\x10\0\0\0WEBPVP8 "), Some("webp"));
        for bytes in [
            &b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"[..],
            b"<?xml version=\"1.0\"?><svg/>",
            b"<!doctype html><script>alert(1)</script>",
            b"RIFF\x10\0\0\0WAVEfmt ",
            b"\x89PN",
            b"",
        ] {
            assert_eq!(sniff_image(bytes), None, "{bytes:?}");
        }
    }

    #[test]
    fn asset_stems() {
        assert_eq!(asset_stem(None), "image");
        assert_eq!(asset_stem(Some("image.png")), "image");
        assert_eq!(
            asset_stem(Some("Screenshot%20from%202026-10-17.png")),
            "Screenshot_from_2026-10-17"
        );
        assert_eq!(asset_stem(Some("%E6%97%A5%E6%9C%AC.png")), "image");
        assert_eq!(asset_stem(Some("caf%C3%A9.jpeg")), "caf_");
        assert_eq!(asset_stem(Some("..%2F..%2Fetc%2Fpasswd")), "passwd");
        assert_eq!(asset_stem(Some("C:%5Cpics%5Cmap.v2.gif")), "map_v2");
        assert_eq!(asset_stem(Some(".hidden")), "_hidden");
        assert_eq!(asset_stem(Some("%ZZ.png")), "_ZZ");
        assert_eq!(asset_stem(Some(&"a".repeat(100))).len(), 64);
    }

    #[test]
    fn asset_destinations() {
        let md = |source: &str| asset_markdown(Path::new(source), "shot", "shot_1_0.png");
        assert_eq!(md("index.md"), "![shot.png](assets/shot_1_0.png)");
        assert_eq!(
            md("journals/2026_10_17.md"),
            "![shot.png](../assets/shot_1_0.png)"
        );
        assert_eq!(md("pages/auth.md"), "![shot.png](../assets/shot_1_0.png)");
        assert_eq!(
            md("pages/Project X/Auth.md"),
            "![shot.png](../../assets/shot_1_0.png)"
        );
    }

    #[cfg(unix)]
    #[test]
    fn new_assets() {
        let base = std::env::temp_dir().join(format!("hermetic-assets-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let root = base.join("notes");
        fs::create_dir_all(&root).unwrap();
        let root = fs::canonicalize(&root).unwrap();

        let first = write_new_asset(&root, "shot", "png", 7, b"one").unwrap();
        let second = write_new_asset(&root, "shot", "png", 7, b"two").unwrap();
        assert_eq!(first, Path::new("assets/shot_7_0.png"));
        assert_eq!(second, Path::new("assets/shot_7_1.png"));
        assert_eq!(fs::read(root.join(&first)).unwrap(), b"one");
        assert_eq!(fs::read(root.join(&second)).unwrap(), b"two");
        let names: Vec<_> = fs::read_dir(root.join("assets"))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names.len(), 2, "no temporary files left: {names:?}");

        // `assets` leading out of the notes.
        let other = base.join("other");
        fs::create_dir_all(&other).unwrap();
        fs::remove_dir_all(root.join("assets")).unwrap();
        std::os::unix::fs::symlink(&other, root.join("assets")).unwrap();
        assert!(matches!(
            write_new_asset(&root, "shot", "png", 7, b"x"),
            Err(Unsafe::Outside)
        ));
        assert_eq!(fs::read_dir(&other).unwrap().count(), 0);
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn empty_notes() {
        for text in ["", "\n", "-\n", "- \n", "  -  \r\n\n", "\n-"] {
            assert!(is_empty_note(text), "{text:?}");
        }
        for text in ["- a\n", "-\n-\n", "a\n", "- \n  - \n", "*\n", "title:: x\n"] {
            assert!(!is_empty_note(text), "{text:?}");
        }
    }

    #[test]
    fn empty_notes_removed() {
        let base = std::env::temp_dir().join(format!("hermetic-empty-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let root = base.join("notes");
        fs::create_dir_all(root.join("journals/sub")).unwrap();
        fs::create_dir_all(root.join("pages/a/b")).unwrap();
        fs::create_dir_all(root.join("pages/kept")).unwrap();
        let root = fs::canonicalize(&root).unwrap();
        let files = [
            ("journals/2026-10-01.md", "-\n"),
            ("journals/2026-10-02.md", ""),
            ("journals/notes.md", "-\n"),
            ("journals/sub/2026-10-04.md", "\n"),
            ("pages/journal.md", "-\n"),
            ("pages/a/b/c.md", "- \n"),
            ("index.md", "\n"),
            ("journals/2026-10-03.md", "- standup\n"),
            ("pages/kept/x.md", "-\n-\n"),
            ("pages/props.md", "title:: x\n"),
            ("pages/.hidden.md", "-\n"),
            ("pages/notes.txt", "-\n"),
        ];
        for (rel, text) in files {
            fs::write(root.join(rel), text).unwrap();
        }
        fs::create_dir_all(base.join("outside")).unwrap();
        fs::write(base.join("outside/linked.md"), "-\n").unwrap();
        std::os::unix::fs::symlink(
            base.join("outside/linked.md"),
            root.join("journals/2026-10-05.md"),
        )
        .unwrap();
        std::os::unix::fs::symlink(base.join("outside"), root.join("pages/linked")).unwrap();
        let grace = EMPTY_NOTE_GRACE;

        // Recently changed notes stay.
        assert!(remove_empty_notes(&root, SystemTime::now(), grace).is_empty());
        assert!(root.join("journals/2026-10-01.md").exists());

        let later = SystemTime::now() + grace + Duration::from_secs(1);
        let removed: Vec<_> = files[..7]
            .iter()
            .map(|(rel, _)| PathBuf::from(rel))
            .collect();
        let mut expected = removed.clone();
        expected.sort();
        assert_eq!(remove_empty_notes(&root, later, grace), expected);
        for rel in &removed {
            assert!(!root.join(rel).exists(), "{} removed", rel.display());
        }
        for (rel, _) in &files[7..] {
            assert!(root.join(rel).exists(), "{rel} kept");
        }
        assert!(!root.join("pages/a").exists(), "emptied directories pruned");
        assert!(root.join("pages").is_dir() && root.join("journals/sub").is_dir());
        assert!(fs::symlink_metadata(root.join("journals/2026-10-05.md")).is_ok());
        assert!(base.join("outside/linked.md").exists());
        assert!(!root.join(TRASH_DIR).exists(), "nothing goes to the trash");
        fs::remove_dir_all(&base).unwrap();
    }
}
