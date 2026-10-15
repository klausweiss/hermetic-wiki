//! Output file paths and the relative hrefs between them.

use std::path::{Component, Path, PathBuf};

use crate::graph::{Graph, PageId, PageKind};
use crate::layout;

/// Output path (relative to the output root) of a page's HTML file.
pub fn output_path(g: &Graph, page: PageId) -> PathBuf {
    let p = g.page(page);
    match (&p.file, p.kind) {
        // Journal days are always at `journals/YYYY-MM-DD.html`, whether the file is named
        // `YYYY-MM-DD.md` or `YYYY_MM_DD.md`, so a day's address does not depend on it.
        (_, PageKind::Journal(date)) => layout::journal_html_path(date),
        (Some(file), _) => md_to_html(file),
        // Without `index.md`, the page `index` is the default home page.
        (None, PageKind::Regular) if layout::is_home_name(&p.name) => PathBuf::from("index.html"),
        (None, PageKind::Regular) => md_to_html(&layout::page_file_path(&p.name)),
    }
}

/// A relative path with `/` separators on every platform, not percent-encoded: how
/// output files are named in the search index, the edit API and live-reload events.
pub fn url_path(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Replaces the trailing markdown extension with `.html`. Names may contain other dots, so this is a
/// string operation rather than an extension swap.
fn md_to_html(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    let stem = s.strip_suffix(layout::EXTENSION).unwrap_or(&s);
    PathBuf::from(format!("{stem}.html"))
}

/// Output path of the `n`-th (n >= 2) continuation file of a page's linked references.
pub fn refs_continuation_path(page_path: &Path, n: usize) -> PathBuf {
    let s = page_path.to_string_lossy();
    let stem = s.strip_suffix(".html").unwrap_or(&s);
    PathBuf::from("refs").join(stem).join(format!("{n}.html"))
}

/// Output path of the `n`-th (n >= 2) continuation file of the open-tasks list.
pub fn tasks_continuation_path(n: usize) -> PathBuf {
    PathBuf::from("tasks").join(format!("{n}.html"))
}

fn segments(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect()
}

/// Percent-encodes every byte that is not an RFC 3986 unreserved character.
pub fn encode(segment: &str) -> String {
    let mut out = String::with_capacity(segment.len());
    for b in segment.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Relative href from the file at `from` to the file at `to` (both relative to the
/// output root), with an optional fragment. Links within the same file are `#fragment`.
pub fn href(from: &Path, to: &Path, fragment: Option<&str>) -> String {
    let fragment = fragment.map(|f| format!("#{}", encode(f)));
    if from == to
        && let Some(fragment) = &fragment
    {
        return fragment.clone();
    }
    let from = segments(from);
    let to = segments(to);
    let from_dir = &from[..from.len().saturating_sub(1)];
    let common = from_dir.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<String> = vec!["..".to_owned(); from_dir.len() - common];
    parts.extend(to[common..].iter().map(|s| encode(s)));
    let mut out = parts.join("/");
    if let Some(fragment) = fragment {
        out.push_str(&fragment);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md_paths() {
        assert_eq!(
            md_to_html(Path::new("pages/C#%3A a.md")),
            PathBuf::from("pages/C#%3A a.html")
        );
        assert_eq!(
            md_to_html(Path::new("pages/v1.2.md")),
            PathBuf::from("pages/v1.2.html")
        );
        assert_eq!(
            md_to_html(Path::new("pages/etc%2E.md")),
            PathBuf::from("pages/etc%2E.html")
        );
        assert_eq!(
            refs_continuation_path(Path::new("pages/Project X/Auth Flow.html"), 2),
            PathBuf::from("refs/pages/Project X/Auth Flow/2.html")
        );
        assert_eq!(tasks_continuation_path(3), PathBuf::from("tasks/3.html"));
    }

    #[test]
    fn hrefs() {
        let p = Path::new;
        assert_eq!(
            href(p("pages/a.html"), p("pages/C#%3A a.html"), None),
            "C%23%253A%20a.html"
        );
        assert_eq!(
            href(
                p("journals/2026-10-12.html"),
                p("pages/auth.html"),
                Some("b3")
            ),
            "../pages/auth.html#b3"
        );
        assert_eq!(
            href(p("index.html"), p("pages/Project X/Auth Flow.html"), None),
            "pages/Project%20X/Auth%20Flow.html"
        );
        assert_eq!(
            href(p("pages/Project X/Auth Flow.html"), p("index.html"), None),
            "../../index.html"
        );
        assert_eq!(
            href(
                p("pages/Project X/Auth Flow.html"),
                p("pages/Project X.html"),
                None
            ),
            "../Project%20X.html"
        );
        assert_eq!(
            href(p("refs/pages/a/2.html"), p("pages/a.html"), None),
            "../../../pages/a.html"
        );
        assert_eq!(
            href(p("pages/a.html"), p("pages/a.html"), Some("id-x y")),
            "#id-x%20y"
        );
        assert_eq!(href(p("pages/a.html"), p("pages/a.html"), None), "a.html");
        assert_eq!(
            href(p("pages/ż.html"), p("pages/ż.html"), None),
            "%C5%BC.html"
        );
    }
}
