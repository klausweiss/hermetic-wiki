//! Rendering blocks: markdown, references, properties, tasks, and block trees.

use std::fmt::Write;
use std::ops::Range;
use std::path::{Path, PathBuf};

use pulldown_cmark::{Options, Parser, html};

use super::html::{esc, strip_tags};
use super::paths::{href, output_path};
use crate::graph::{BlockId, Graph, PageId, Ref, RefLocation, RefTarget};
use crate::layout::Date;
use crate::parse::{RawRefKind, TaskState};

/// Shared state for rendering one site.
pub struct Site<'g> {
    pub g: &'g Graph,
    pub title: String,
    out: Vec<PathBuf>,
    /// File-backed journal days, ascending.
    pub journal_days: Vec<(Date, PageId)>,
    /// Number of open tasks in the graph, shown in the site navigation.
    pub open_tasks: usize,
}

/// Whether block references are expanded into their target's content.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Expand,
    /// Inside an expanded block ref: nested refs are plain links, so cycles terminate.
    Nested,
}

impl<'g> Site<'g> {
    pub fn new(g: &'g Graph, title: impl Into<String>) -> Self {
        let out = g.pages().map(|(id, _)| output_path(g, id)).collect();
        let mut journal_days: Vec<(Date, PageId)> = g
            .pages()
            .filter(|(_, p)| !p.is_implicit())
            .filter_map(|(id, p)| p.date().map(|d| (d, id)))
            .collect();
        journal_days.sort();
        Site {
            g,
            title: title.into(),
            out,
            journal_days,
            open_tasks: g.open_tasks().len(),
        }
    }

    pub fn out_path(&self, page: PageId) -> &Path {
        &self.out[page.0 as usize]
    }

    /// HTML anchor of a block: `id-<id>` or `b<n>` (1-based position in its page).
    pub fn anchor(&self, block: BlockId) -> String {
        let b = self.g.block(block);
        match &b.id {
            Some(id) => format!("id-{id}"),
            None => format!("b{}", b.index + 1),
        }
    }

    pub fn page_href(&self, from: &Path, page: PageId) -> String {
        href(from, self.out_path(page), None)
    }

    pub fn block_href(&self, from: &Path, block: BlockId) -> String {
        let page = self.g.block(block).page;
        href(from, self.out_path(page), Some(&self.anchor(block)))
    }

    /// Page title as shown in headings and link lists.
    pub fn page_title(&self, page: PageId) -> String {
        let p = self.g.page(page);
        match p.date() {
            Some(date) => format!("{} {}", p.name, date.weekday()),
            None => p.name.clone(),
        }
    }

    // -- block content ---------------------------------------------------------

    /// Renders text containing references (a block's content or a property value).
    fn render_text<'r>(
        &self,
        from: &Path,
        text: &str,
        refs: impl Iterator<Item = &'r Ref>,
        mode: Mode,
    ) -> String {
        let refs: Vec<&Ref> = refs.collect();
        // Text inside a broken markdown link is shown verbatim, so nested refs are dropped.
        let broken: Vec<Range<usize>> = refs
            .iter()
            .filter(|r| r.kind == RawRefKind::MdLink && r.target == RefTarget::Unresolved)
            .map(|r| r.range.clone())
            .collect();
        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        let mut fragments: Vec<String> = Vec::new();
        let mut placeholder = |html: String| {
            fragments.push(html);
            format!("\u{E000}{}\u{E001}", fragments.len() - 1)
        };
        for r in &refs {
            let nested_in_broken = broken
                .iter()
                .any(|b| *b != r.range && b.start <= r.range.start && r.range.end <= b.end);
            if nested_in_broken {
                continue;
            }
            let original = &text[r.range.clone()];
            match (r.kind, r.target) {
                (RawRefKind::MdLink, RefTarget::Page(p)) => {
                    let Some(dest) = r.dest.clone() else { continue };
                    let mut url = self.page_href(from, p);
                    if let Some((_, fragment)) = r.text.split_once('#') {
                        url.push('#');
                        url.push_str(fragment);
                    }
                    edits.push((dest, url));
                }
                (kind @ (RawRefKind::Page | RawRefKind::Tag), RefTarget::Page(p)) => {
                    let (class, prefix) = match kind {
                        RawRefKind::Tag => ("tag", "#"),
                        _ => ("page-ref", ""),
                    };
                    let html = format!(
                        "<a class=\"{class}\" href=\"{}\">{prefix}{}</a>",
                        esc(&self.page_href(from, p)),
                        esc(&r.text)
                    );
                    edits.push((r.range.clone(), placeholder(html)));
                }
                (RawRefKind::Block, RefTarget::Block(target)) => {
                    let link = esc(&self.block_href(from, target));
                    let html = match mode {
                        Mode::Expand => format!(
                            "<span class=\"block-ref\">{}<a class=\"block-ref-link\" href=\"{link}\" title=\"Go to block\">↗</a></span>",
                            self.content(from, target, Mode::Nested)
                        ),
                        Mode::Nested => {
                            format!(
                                "<a class=\"block-ref-link\" href=\"{link}\">{}</a>",
                                esc(original)
                            )
                        }
                    };
                    edits.push((r.range.clone(), placeholder(html)));
                }
                (_, RefTarget::Unresolved) => {
                    let html = format!(
                        "<span class=\"broken\" title=\"Broken reference\">{}</span>",
                        esc(original)
                    );
                    edits.push((r.range.clone(), placeholder(html)));
                }
                _ => {}
            }
        }
        edits.sort_by_key(|e| e.0.start);
        let mut src = String::with_capacity(text.len());
        let mut pos = 0;
        for (range, replacement) in edits {
            if range.start < pos {
                continue;
            }
            src.push_str(&text[pos..range.start]);
            src.push_str(&replacement);
            pos = range.end;
        }
        src.push_str(&text[pos..]);
        fill_placeholders(&markdown(&src), &fragments)
    }

    /// A block's content as HTML; a single paragraph is unwrapped so it flows inline.
    pub fn content(&self, from: &Path, block: BlockId, mode: Mode) -> String {
        let b = self.g.block(block);
        let refs = b.refs.iter().filter(|r| r.location == RefLocation::Content);
        unwrap_paragraph(&self.render_text(from, &b.content, refs, mode))
    }

    /// A block's content as plain text (entities kept), for breadcrumbs.
    pub fn plain(&self, from: &Path, block: BlockId) -> String {
        strip_tags(&self.content(from, block, Mode::Nested))
    }

    fn properties(&self, from: &Path, block: BlockId) -> String {
        let b = self.g.block(block);
        let mut out = String::new();
        for (i, prop) in b.props.iter().enumerate() {
            if prop.key == "id" {
                continue;
            }
            let refs = b
                .refs
                .iter()
                .filter(|r| r.location == RefLocation::Property(i));
            let value = unwrap_paragraph(&self.render_text(from, &prop.value, refs, Mode::Nested));
            out.push_str(&format!(
                "<div><dt>{}</dt><dd>{}</dd></div>",
                esc(&prop.key),
                value
            ));
        }
        if out.is_empty() {
            out
        } else {
            format!("<dl class=\"props\">{out}</dl>")
        }
    }

    /// Task marker, content, and properties of one block.
    pub fn block_body(&self, from: &Path, block: BlockId) -> String {
        let b = self.g.block(block);
        format!(
            "<div class=\"block-body\">{}<div class=\"block-content\">{}</div>{}</div>",
            b.task.map(task_badge).unwrap_or_default(),
            self.content(from, block, Mode::Expand),
            self.properties(from, block)
        )
    }

    /// Blocks and their subtrees as nested lists. Anchors are emitted only on the
    /// block's own page (`with_ids`), so they stay unique.
    pub fn tree(&self, from: &Path, blocks: &[BlockId], with_ids: bool) -> String {
        let mut out = String::new();
        self.push_tree(&mut out, from, blocks, with_ids);
        out
    }

    fn push_tree(&self, out: &mut String, from: &Path, blocks: &[BlockId], with_ids: bool) {
        if blocks.is_empty() {
            return;
        }
        out.push_str("<ul class=\"blocks\">");
        for &block in blocks {
            let b = self.g.block(block);
            out.push_str("<li class=\"block");
            if let Some(state) = b.task {
                let _ = write!(out, " task-{}", state.css_name());
            }
            out.push('"');
            if with_ids {
                let _ = write!(out, " id=\"{}\"", esc(&self.anchor(block)));
            }
            out.push('>');
            out.push_str(&self.block_body(from, block));
            self.push_tree(out, from, &b.children, with_ids);
            out.push_str("</li>");
        }
        out.push_str("</ul>");
    }
}

pub fn task_badge(state: TaskState) -> String {
    format!(
        "<span class=\"task-marker task-{}\">{}</span>",
        state.css_name(),
        state.marker()
    )
}

/// Replaces each `\u{E000}<i>\u{E001}` placeholder in `html` with `fragments[i]`.
fn fill_placeholders(html: &str, fragments: &[String]) -> String {
    if fragments.is_empty() {
        return html.to_owned();
    }
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find('\u{E000}') {
        out.push_str(&rest[..start]);
        let after = &rest[start + '\u{E000}'.len_utf8()..];
        let fragment = after.split_once('\u{E001}').and_then(|(index, tail)| {
            let f = fragments.get(index.parse::<usize>().ok()?)?;
            Some((f, tail))
        });
        match fragment {
            Some((f, tail)) => {
                out.push_str(f);
                rest = tail;
            }
            None => {
                out.push('\u{E000}');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn markdown(src: &str) -> String {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(src, options));
    out
}

/// `<p>x</p>` → `x` when the HTML is exactly one paragraph.
fn unwrap_paragraph(html: &str) -> String {
    let t = html.trim_end();
    if let Some(inner) = t.strip_prefix("<p>").and_then(|r| r.strip_suffix("</p>"))
        && !inner.contains("<p>")
    {
        return inner.to_owned();
    }
    t.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphs() {
        assert_eq!(unwrap_paragraph("<p>a <em>b</em></p>\n"), "a <em>b</em>");
        assert_eq!(
            unwrap_paragraph("<p>a</p>\n<p>b</p>\n"),
            "<p>a</p>\n<p>b</p>"
        );
        assert_eq!(
            unwrap_paragraph("<ul>\n<li>x</li>\n</ul>\n"),
            "<ul>\n<li>x</li>\n</ul>"
        );
    }

    #[test]
    fn markdown_extensions() {
        let html = markdown(
            "| a | b |\n|---|---|\n| 1 | 2 |\n\n~~gone~~ [^1]\n\n[^1]: note\n\n- [x] done",
        );
        assert!(html.contains("<table>"));
        assert!(html.contains("<del>gone</del>"));
        assert!(html.contains("footnote"));
        assert!(html.contains("type=\"checkbox\""));
    }
}
