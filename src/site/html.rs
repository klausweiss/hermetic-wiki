//! HTML escaping and the document shell shared by every generated page.

use std::path::Path;

use super::paths::href;

/// Escapes text for use in element content and attribute values.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// Removes tags from an HTML fragment, keeping text (entities stay encoded).
pub fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_owned()
}

pub const STYLESHEET: &str = "_hermetic/style.css";
pub const SCRIPT: &str = "_hermetic/app.js";

/// Relative path from the file at `path` to the output root: `.`, `..`, `../..`, ...
pub fn root_path(path: &Path) -> String {
    let depth = path.components().count().saturating_sub(1);
    if depth == 0 {
        ".".to_owned()
    } else {
        vec![".."; depth].join("/")
    }
}

/// A complete HTML document for the file at `path` (relative to the output root).
///
/// The script is loaded synchronously in `<head>` so it can apply the reader's theme
/// before the first paint. It defines the `hermetic-*` elements emitted here; without
/// it they are inert and render nothing.
pub fn document(site_title: &str, path: &Path, title: &str, main: &str) -> String {
    let to = |target: &str| esc(&href(path, Path::new(target), None));
    format!(
        "<!doctype html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"hermetic-root\" content=\"{root}\">\n\
         <title>{title} · {site}</title>\n\
         <link rel=\"stylesheet\" href=\"{css}\">\n\
         <script src=\"{script}\"></script>\n\
         </head>\n\
         <body>\n\
         <header class=\"site-header\">\n\
         <a class=\"site-title\" href=\"{index}\">{site}</a>\n\
         <nav class=\"site-nav\"><a href=\"{journals}\">Journals</a><a href=\"{pages}\">All pages</a><a href=\"{tasks}\">Open tasks</a><hermetic-theme-toggle></hermetic-theme-toggle></nav>\n\
         </header>\n\
         <main>\n{main}</main>\n\
         <hermetic-sidebar></hermetic-sidebar>\n\
         <hermetic-shortcuts></hermetic-shortcuts>\n\
         </body>\n\
         </html>\n",
        root = esc(&root_path(path)),
        title = esc(title),
        site = esc(site_title),
        css = to(STYLESHEET),
        script = to(SCRIPT),
        index = to("index.html"),
        journals = to("journals.html"),
        pages = to("pages.html"),
        tasks = to("tasks.html"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping() {
        assert_eq!(
            esc(r#"<a href="x">'&'</a>"#),
            "&lt;a href=&quot;x&quot;&gt;&#39;&amp;&#39;&lt;/a&gt;"
        );
        assert_eq!(
            strip_tags("<p>a <a href=\"x\">b</a> &amp; c</p>"),
            "a b &amp; c"
        );
    }

    #[test]
    fn shell_links_are_relative() {
        let root = document("notes", Path::new("index.html"), "Journals", "x");
        assert!(root.contains(r#"href="_hermetic/style.css""#));
        assert!(root.contains(r#"<a href="pages.html">All pages</a>"#));

        let page = document("notes", Path::new("pages/a.html"), "a", "x");
        assert!(page.contains(r#"href="../_hermetic/style.css""#));
        assert!(page.contains(r#"<a href="../tasks.html">Open tasks</a>"#));

        let nested = document(
            "notes",
            Path::new("pages/Project X/Auth Flow.html"),
            "A & B",
            "x",
        );
        assert!(nested.contains(r#"<a class="site-title" href="../../index.html">notes</a>"#));
        assert!(nested.contains(r#"<a href="../../journals.html">Journals</a>"#));
        assert!(nested.contains("<title>A &amp; B · notes</title>"));
        assert!(nested.contains(r#"<meta name="hermetic-root" content="../..">"#));
        assert!(nested.contains(r#"<script src="../../_hermetic/app.js"></script>"#));
        assert!(root.contains(r#"<meta name="hermetic-root" content=".">"#));
        assert!(root.contains(r#"<script src="_hermetic/app.js"></script>"#));

        assert_eq!(page.matches("<main>").count(), 1);
        assert_eq!(page.matches("<script").count(), 1);
    }

    #[test]
    fn root_paths() {
        assert_eq!(root_path(Path::new("index.html")), ".");
        assert_eq!(root_path(Path::new("pages/a.html")), "..");
        assert_eq!(root_path(Path::new("refs/pages/a/2.html")), "../../..");
    }
}
