//! The maintenance API: which attachments nothing uses (see `maintenance`).

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::site::{Content, SiteFiles, encode_url_segment, url_path};

/// Upper bound on a request to move files to the trash.
pub const MAX_BODY: usize = 1024 * 1024;

/// An attachment nothing uses: its path relative to the notes and its size in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unused {
    pub path: PathBuf,
    pub size: u64,
}

/// The attachments of a build that no image or link resolves to (`referenced`) and whose
/// file name, as written or percent-encoded, appears in none of the `notes` (note file
/// texts), ordered by path. Files that cannot be read any more are left out.
pub fn unused_assets(
    files: &SiteFiles,
    referenced: &HashSet<PathBuf>,
    notes: &[String],
) -> Vec<Unused> {
    let lowered: Vec<String> = notes.iter().map(|t| t.to_lowercase()).collect();
    let mut unused: Vec<Unused> = files
        .iter()
        .filter_map(|(rel, content)| match content {
            Content::File(source) if !referenced.contains(rel) => Some((rel, source)),
            _ => None,
        })
        .filter(|(rel, _)| !named(rel, notes, &lowered))
        .filter_map(|(rel, source)| {
            let meta = fs::symlink_metadata(source).ok()?;
            meta.is_file().then(|| Unused {
                path: rel.clone(),
                size: meta.len(),
            })
        })
        .collect();
    unused.sort_by(|a, b| a.path.cmp(&b.path));
    unused
}

/// Whether a note names the file `rel`: its file name as written, or percent-encoded in
/// either case (`my%20shot.png`, `%E2%9C%93.png` or `%e2%9c%93.png`), as a whole name:
/// `d.png` is not named by `used.png` or `d.png.bak`.
fn named(rel: &Path, notes: &[String], lowered: &[String]) -> bool {
    let Some(name) = rel.file_name().map(|n| n.to_string_lossy()) else {
        return false;
    };
    if notes.iter().any(|t| contains_name(t, &name)) {
        return true;
    }
    let encoded = encode_url_segment(&name);
    encoded != name && {
        let encoded = encoded.to_lowercase();
        lowered.iter().any(|t| contains_name(t, &encoded))
    }
}

/// Whether `name` occurs in `text` with no file-name character right before or after
/// it; a `.` after it counts only when a file-name character follows (`a.png.bak`), as
/// it may otherwise end a sentence.
fn contains_name(text: &str, name: &str) -> bool {
    let part = |c: char| c.is_alphanumeric() || matches!(c, '-' | '_' | '~' | '%');
    text.match_indices(name).any(|(i, _)| {
        let before = text[..i].chars().next_back();
        let mut after = text[i + name.len()..].chars();
        let continues = match after.next() {
            Some('.') => after.next().is_some_and(part),
            next => next.is_some_and(part),
        };
        !before.is_some_and(|c| part(c) || c == '.') && !continues
    })
}

/// The texts of the note files `files` (relative to `root`); unreadable ones are skipped.
pub fn note_texts<'a>(root: &Path, files: impl Iterator<Item = &'a Path>) -> Vec<String> {
    files
        .filter_map(|rel| fs::read(root.join(rel)).ok())
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .collect()
}

/// The `GET` response: `{"assets":[{"path":…,"size":…},…]}`.
pub fn list_json(unused: &[Unused]) -> String {
    let items: Vec<String> = unused
        .iter()
        .map(|u| {
            format!(
                "{{\"path\":{},\"size\":{}}}",
                crate::json::string(&url_path(&u.path)),
                u.size
            )
        })
        .collect();
    format!("{{\"assets\":[{}]}}", items.join(","))
}

/// The `POST` response: the moved paths, the trash folder (relative to the notes), and
/// the paths left alone with the reason.
pub fn trash_json(trashed: &[String], trash: &str, skipped: &[(String, String)]) -> String {
    let s = crate::json::string;
    let trashed: Vec<String> = trashed.iter().map(|p| s(p)).collect();
    let skipped: Vec<String> = skipped
        .iter()
        .map(|(p, why)| format!("{{\"path\":{},\"reason\":{}}}", s(p), s(why)))
        .collect();
    format!(
        "{{\"trashed\":[{}],\"trash\":{},\"skipped\":[{}]}}",
        trashed.join(","),
        s(trash),
        skipped.join(",")
    )
}

/// Parses a `POST` body of the form `{"paths":["…",…]}` (whitespace allowed anywhere
/// between tokens). Anything else is `None`.
pub fn parse_paths(body: &str) -> Option<Vec<String>> {
    let mut p = Parser {
        s: body.as_bytes(),
        i: 0,
    };
    p.expect(b'{')?;
    let key = p.string()?;
    if key != "paths" {
        return None;
    }
    p.expect(b':')?;
    p.expect(b'[')?;
    let mut paths = Vec::new();
    if !p.eat(b']') {
        loop {
            paths.push(p.string()?);
            if p.eat(b']') {
                break;
            }
            p.expect(b',')?;
        }
    }
    p.expect(b'}')?;
    p.ws();
    (p.i == p.s.len()).then_some(paths)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.s.get(self.i).is_some_and(u8::is_ascii_whitespace) {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> bool {
        self.ws();
        let found = self.s.get(self.i) == Some(&c);
        if found {
            self.i += 1;
        }
        found
    }

    fn expect(&mut self, c: u8) -> Option<()> {
        self.eat(c).then_some(())
    }

    /// A JSON string, with its escapes (including `\uXXXX` surrogate pairs) decoded.
    fn string(&mut self) -> Option<String> {
        self.expect(b'"')?;
        let mut out = Vec::new();
        loop {
            let c = *self.s.get(self.i)?;
            self.i += 1;
            match c {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let e = *self.s.get(self.i)?;
                    self.i += 1;
                    let ch = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let hi = self.hex4()?;
                            if (0xD800..0xDC00).contains(&hi) {
                                if self.s.get(self.i..self.i + 2)? != b"\\u" {
                                    return None;
                                }
                                self.i += 2;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return None;
                                }
                                char::from_u32(0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00))?
                            } else {
                                char::from_u32(hi)?
                            }
                        }
                        _ => return None,
                    };
                    let mut buf = [0; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                0..0x20 => return None,
                _ => out.push(c),
            }
        }
    }

    fn hex4(&mut self) -> Option<u32> {
        let digits = std::str::from_utf8(self.s.get(self.i..self.i + 4)?).ok()?;
        self.i += 4;
        u32::from_str_radix(digits, 16).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A temporary notes folder with `files`, built as the server builds it.
    struct Notes(PathBuf);

    impl Notes {
        fn new(files: &[(&str, &str)]) -> Notes {
            use std::sync::atomic::{AtomicUsize, Ordering};
            static N: AtomicUsize = AtomicUsize::new(0);
            let root = std::env::temp_dir().join(format!(
                "hermetic-maintenance-{}-{}",
                std::process::id(),
                N.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&root);
            for (path, text) in files {
                let path = root.join(path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, text).unwrap();
            }
            Notes(fs::canonicalize(&root).unwrap())
        }

        fn unused(&self) -> Vec<String> {
            let (build, _, _) = super::super::build_site(&self.0, "t").unwrap();
            let files: Vec<&Path> = build
                .editable
                .values()
                .filter_map(|p| p.file.as_deref())
                .collect();
            let texts = note_texts(&self.0, files.into_iter());
            unused_assets(&build.files, &build.referenced, &texts)
                .iter()
                .map(|u| url_path(&u.path))
                .collect()
        }
    }

    impl Drop for Notes {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn referenced_attachments() {
        let notes = Notes::new(&[
            ("journals/2026_10_17.md", "- ![x](../assets/x.png)\n"),
            ("assets/x.png", "png"),
        ]);
        let (build, _, _) = super::super::build_site(&notes.0, "t").unwrap();
        assert!(build.referenced.contains(Path::new("assets/x.png")));
    }

    #[test]
    fn image_removed_from_its_block() {
        let notes = Notes::new(&[
            ("journals/2026_10_17.md", "- pasted, then removed\n"),
            ("assets/shot_1_0.png", "png"),
        ]);
        assert_eq!(notes.unused(), ["assets/shot_1_0.png"]);
    }

    #[test]
    fn shown_and_linked_files_are_used() {
        let notes = Notes::new(&[
            ("pages/auth.md", "- ![flow](auth-flow.png)\n"),
            ("pages/auth-flow.png", "png"),
            (
                "journals/2026_10_17.md",
                "- [spec](../assets/spec.pdf#page=2)\n",
            ),
            ("assets/spec.pdf", "pdf"),
            (
                "pages/props.md",
                "- a\n  cover:: ![c](../assets/cover.webp)\n",
            ),
            ("assets/cover.webp", "webp"),
            ("assets/z.gif", "gif"),
        ]);
        assert_eq!(notes.unused(), ["assets/z.gif"]);
    }

    #[test]
    fn named_files_are_kept() {
        let notes = Notes::new(&[
            ("pages/html.md", "- <img src=\"../assets/diagram.svg\">\n"),
            ("assets/diagram.svg", "<svg/>"),
            ("pages/ref.md", "- [r][1]\n\n[1]: ../assets/ref-style.png\n"),
            ("assets/ref-style.png", "png"),
            ("pages/enc.md", "- <img src=\"../assets/my%20shot.png\">\n"),
            ("assets/my shot.png", "png"),
            (
                "pages/lower.md",
                "- <a href=\"../assets/%e2%9c%93.png\">x</a>\n",
            ),
            ("assets/✓.png", "png"),
            ("pages/prose.md", "- see shot.png. And `x.jpg`\n"),
            ("assets/shot.png", "png"),
            ("pages/x.jpg", "jpg"),
            // Only part of a longer name: not named.
            (
                "pages/parts.md",
                "- ![u](../assets/used.png) a.png.bak b.png2 xb.png\n",
            ),
            ("assets/used.png", "png"),
            ("assets/d.png", "png"),
            ("assets/b.png", "png"),
            ("assets/a.png", "png"),
        ]);
        assert_eq!(
            notes.unused(),
            ["assets/a.png", "assets/b.png", "assets/d.png"]
        );
    }

    #[cfg(unix)]
    #[test]
    fn links_are_not_listed() {
        let notes = Notes::new(&[("assets/real.png", "png")]);
        std::os::unix::fs::symlink(
            notes.0.join("assets/real.png"),
            notes.0.join("assets/l.png"),
        )
        .unwrap();
        assert_eq!(notes.unused(), ["assets/real.png"]);
    }

    #[test]
    fn list_payload() {
        let unused = [Unused {
            path: PathBuf::from("assets/a \"b\".png"),
            size: 2048,
        }];
        assert_eq!(
            list_json(&unused),
            r#"{"assets":[{"path":"assets/a \"b\".png","size":2048}]}"#
        );
        assert_eq!(list_json(&[]), r#"{"assets":[]}"#);
    }

    #[test]
    fn parses_paths() {
        assert_eq!(
            parse_paths(r#" { "paths" : [ "assets/a.png", "a \"b\" é 😀" ] } "#),
            Some(vec!["assets/a.png".into(), "a \"b\" é 😀".into()])
        );
        assert_eq!(parse_paths(r#"{"paths":[]}"#), Some(vec![]));
        for bad in [
            "",
            "[]",
            r#"{"paths":"a"}"#,
            r#"{"path":["a"]}"#,
            r#"{"paths":["a",]}"#,
            r#"{"paths":["a"]} x"#,
            r#"{"paths":[1]}"#,
            r#"{"paths":["\ud83d"]}"#,
            "{\"paths\":[\"a\nb\"]}",
        ] {
            assert_eq!(parse_paths(bad), None, "{bad}");
        }
    }
}
