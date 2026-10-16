//! The plain text of blocks, as searched and shown in result previews.

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, TagEnd};

use crate::graph::{BlockId, Graph, Ref, RefLocation};
use crate::parse::RawRefKind;

/// Longest preview shown in results, in characters.
pub const PREVIEW_CHARS: usize = 240;
/// Longest text of one breadcrumb step, in characters.
pub const CRUMB_CHARS: usize = 40;

/// Text with references replaced: page links and tags by their names, block references
/// removed. Markdown links stay as markdown and become their link text below.
fn substitute<'r>(text: &str, refs: impl Iterator<Item = &'r Ref>) -> String {
    let mut edits: Vec<(Range<usize>, &str)> = refs
        .filter_map(|r| match r.kind {
            RawRefKind::Page | RawRefKind::Tag => Some((r.range.clone(), r.text.as_str())),
            RawRefKind::Block => Some((r.range.clone(), "")),
            RawRefKind::MdLink | RawRefKind::Attachment => None,
        })
        .collect();
    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut out = text.to_owned();
    for (range, with) in edits {
        // Spaces keep words on either side apart.
        out.replace_range(range, &format!(" {with} "));
    }
    out
}

/// Markdown reduced to its text and code, with whitespace collapsed.
fn markdown_text(markdown: &str) -> String {
    let mut out = String::new();
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_FOOTNOTES;
    for event in Parser::new_ext(markdown, options) {
        match event {
            Event::Text(t) | Event::Code(t) => out.push_str(&t),
            Event::SoftBreak
            | Event::HardBreak
            | Event::End(
                TagEnd::Paragraph | TagEnd::Item | TagEnd::TableCell | TagEnd::CodeBlock,
            ) => out.push(' '),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Everything searchable in a block: its content and property values (except `id`).
pub fn block_text(g: &Graph, block: BlockId) -> String {
    let b = g.block(block);
    let mut parts = vec![markdown_text(&substitute(
        &b.content,
        b.refs.iter().filter(|r| r.location == RefLocation::Content),
    ))];
    for (i, prop) in b.props.iter().enumerate() {
        if prop.key == "id" {
            continue;
        }
        let refs = b
            .refs
            .iter()
            .filter(move |r| r.location == RefLocation::Property(i));
        parts.push(markdown_text(&substitute(&prop.value, refs)));
    }
    parts.retain(|p| !p.is_empty());
    parts.join(" ")
}

/// `text` cut to at most `max` characters at a word boundary, with an ellipsis.
pub fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let cut: String = text.chars().take(max).collect();
    let cut = match cut.rfind(' ') {
        Some(i) if i > max / 2 => &cut[..i],
        _ => &cut,
    };
    format!("{}…", cut.trim_end())
}

/// The block's ancestors' texts, root first, each shortened, joined with ` › `.
pub fn breadcrumb(g: &Graph, block: BlockId) -> String {
    g.ancestors(block)
        .into_iter()
        .map(|a| truncate(&block_text(g, a), CRUMB_CHARS))
        .collect::<Vec<_>>()
        .join(" › ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::normalize::words;

    #[test]
    fn markdown_is_reduced_to_text() {
        assert_eq!(
            markdown_text("**bold** _it_ `code` [link](x.md) ~~gone~~"),
            "bold it code link gone"
        );
        assert_eq!(markdown_text("| a | b |\n|---|---|\n| 1 | 2 |"), "a b 1 2");
        assert_eq!(markdown_text("line one\nline two"), "line one line two");
        assert_eq!(markdown_text("```\nfn main() {}\n```"), "fn main() {}");
        assert_eq!(markdown_text("<b>raw</b> html"), "raw html");
    }

    #[test]
    fn truncation() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("one two three four five", 12), "one two…");
        assert_eq!(truncate("żółć żółć żółć", 6), "żółć…");
        assert_eq!(words(&truncate(&"x".repeat(300), 240)).len(), 1);
    }
}
