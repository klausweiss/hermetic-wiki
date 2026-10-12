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

/// A complete HTML document for the file at `path` (relative to the output root).
pub fn document(site_title: &str, path: &Path, title: &str, main: &str) -> String {
    let to = |target: &str| esc(&href(path, Path::new(target), None));
    format!(
        "<!doctype html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{title} · {site}</title>\n\
         <link rel=\"stylesheet\" href=\"{css}\">\n\
         </head>\n\
         <body>\n\
         <header class=\"site-header\">\n\
         <a class=\"site-title\" href=\"{index}\">{site}</a>\n\
         <nav class=\"site-nav\"><a href=\"{index}\">Journals</a><a href=\"{pages}\">All pages</a><a href=\"{tasks}\">Open tasks</a></nav>\n\
         </header>\n\
         <main>\n{main}</main>\n\
         </body>\n\
         </html>\n",
        title = esc(title),
        site = esc(site_title),
        css = to(STYLESHEET),
        index = to("index.html"),
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
        assert!(nested.contains(r#"<a href="../../index.html">Journals</a>"#));
        assert!(nested.contains("<title>A &amp; B · notes</title>"));

        assert_eq!(page.matches("<main>").count(), 1);
        assert!(!page.contains("<script"));
    }
}
