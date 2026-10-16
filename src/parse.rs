//! Parsing a page's text into a tree of blocks, and finding references inside blocks.
//!
//! Two hand-written passes: a line scanner that builds the block tree (bullets,
//! indentation, fences, prose runs, properties, task markers), and an inline scanner that
//! finds `[[links]]`, `#tags`, `((block refs))` and `[markdown](links.md)`.

use std::ops::Range;

/// Width a tab advances to (the next multiple of this many columns).
const TAB_WIDTH: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    /// Started by a `- ` bullet.
    Bullet,
    /// A run of text outside any bullet; always top-level.
    Prose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskState {
    Todo,
    Doing,
    Done,
}

impl TaskState {
    pub fn is_open(self) -> bool {
        matches!(self, TaskState::Todo | TaskState::Doing)
    }

    pub fn marker(self) -> &'static str {
        match self {
            TaskState::Todo => "TODO",
            TaskState::Doing => "DOING",
            TaskState::Done => "DONE",
        }
    }

    /// Lowercase name used in CSS classes.
    pub fn css_name(self) -> &'static str {
        match self {
            TaskState::Todo => "todo",
            TaskState::Doing => "doing",
            TaskState::Done => "done",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawRefKind {
    /// `[[name]]`
    Page,
    /// `#name` or `#[[name]]`
    Tag,
    /// `((id))`
    Block,
    /// `[text](path.md)`; `text` holds the raw destination.
    MdLink,
    /// `![alt](path)` or `[text](path)` to a file in the notes that is not a note;
    /// `text` holds the raw destination.
    Attachment,
}

/// A reference as written, before resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawRef {
    pub kind: RawRefKind,
    /// Page name, block id, or markdown link destination.
    pub text: String,
    /// Byte range of the whole reference syntax within the scanned text.
    pub range: Range<usize>,
    /// For markdown links and attachments: byte range of the destination token
    /// (including any `<>`).
    pub dest: Option<Range<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    /// Lowercased key.
    pub key: String,
    pub value: String,
    /// References inside `value`; ranges are relative to `value`.
    pub refs: Vec<RawRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedBlock {
    pub kind: BlockKind,
    /// Index of the parent block in the returned list.
    pub parent: Option<usize>,
    /// 0-based line range of the source text owned by this block. The ranges of all
    /// blocks of a file partition its lines.
    pub lines: Range<u32>,
    /// 1-based line number of the block's first non-blank line.
    pub line: u32,
    /// Exact source text owned by this block (see `lines`).
    pub source: String,
    /// Display content: de-indented, without property lines and task marker.
    pub content: String,
    pub props: Vec<Property>,
    pub task: Option<TaskState>,
    /// References inside `content`; ranges are relative to `content`.
    pub refs: Vec<RawRef>,
    /// The block as an editor shows it: its content lines de-indented to `column`,
    /// without the bullet marker, including property lines and the task marker, and
    /// without trailing blank lines.
    pub body: String,
    /// Column where the block's content starts (after `- ` for bullets).
    pub column: u32,
}

impl ParsedBlock {
    pub fn prop(&self, key: &str) -> Option<&str> {
        self.props
            .iter()
            .find(|p| p.key == key)
            .map(|p| p.value.as_str())
    }
}

/// Whether a block consists only of property lines, so that, as a page's first block, its
/// properties are the page's properties.
pub fn is_properties_only(content: &str, task: Option<TaskState>, props: &[Property]) -> bool {
    content.is_empty() && task.is_none() && !props.is_empty()
}

/// The names in an `alias` property value: comma-separated, each written bare or as a
/// single page link (`[[name]]`), trimmed, empty parts skipped.
pub fn alias_names(value: &str) -> Vec<&str> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.strip_prefix("[[")
                .and_then(|p| p.strip_suffix("]]"))
                .filter(|name| !name.contains("[[") && !name.contains("]]"))
                .map_or(part, str::trim)
        })
        .filter(|name| !name.is_empty())
        .collect()
}

/// Whether `id` is acceptable as a block id: non-empty, without whitespace or parentheses.
pub fn is_valid_block_id(id: &str) -> bool {
    !id.is_empty()
        && !id
            .chars()
            .any(|c| c.is_whitespace() || c == '(' || c == ')')
}

/// HTML anchor of a block: `id-<id>` for a valid block id, else `b<n>` (1-based position
/// among its page's blocks). The editor relies on the rendered page using the same rule.
pub fn anchor(id: Option<&str>, index: usize) -> String {
    match id {
        Some(id) => format!("id-{id}"),
        None => format!("b{}", index + 1),
    }
}

// ---------------------------------------------------------------------------
// Line scanner
// ---------------------------------------------------------------------------

/// Indentation column (tabs expanded) and the rest of the line.
fn indentation(line: &str) -> (usize, &str) {
    let mut col = 0;
    for (i, c) in line.char_indices() {
        match c {
            ' ' => col += 1,
            '\t' => col += TAB_WIDTH - col % TAB_WIDTH,
            _ => return (col, &line[i..]),
        }
    }
    (col, "")
}

/// Removes up to `width` columns of leading whitespace.
fn deindent(line: &str, width: usize) -> &str {
    let mut col = 0;
    for (i, c) in line.char_indices() {
        if col >= width {
            return &line[i..];
        }
        match c {
            ' ' => col += 1,
            '\t' => col += TAB_WIDTH - col % TAB_WIDTH,
            _ => return &line[i..],
        }
    }
    ""
}

fn bullet_rest(rest: &str) -> Option<&str> {
    if rest == "-" {
        Some("")
    } else {
        rest.strip_prefix("- ")
    }
}

/// An open code fence: its character and run length.
#[derive(Clone, Copy)]
struct Fence(u8, usize);

fn fence_opener(line: &str) -> Option<Fence> {
    let t = line.trim_start();
    let c = *t.as_bytes().first()?;
    if c != b'`' && c != b'~' {
        return None;
    }
    let n = t.bytes().take_while(|&b| b == c).count();
    (n >= 3).then_some(Fence(c, n))
}

fn closes_fence(line: &str, fence: Fence) -> bool {
    let t = line.trim_start();
    let n = t.bytes().take_while(|&b| b == fence.0).count();
    n >= fence.1 && t[n..].trim().is_empty()
}

struct Builder {
    kind: BlockKind,
    parent: Option<usize>,
    start_line: usize,
    first_line: usize,
    content_col: usize,
    /// De-indented content lines and whether each is part of a code fence.
    content: Vec<(String, bool)>,
}

/// Parses a file's text (LF line endings) into blocks in document order.
pub fn parse_blocks(text: &str) -> Vec<ParsedBlock> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut line_starts = Vec::with_capacity(lines.len() + 1);
    let mut offset = 0;
    for line in &lines {
        line_starts.push(offset);
        offset += line.len() + 1;
    }
    line_starts.push(text.len());

    let mut builders: Vec<Builder> = Vec::new();
    // Indentation stack of open bullets: (column, builder index).
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut fence: Option<Fence> = None;
    let mut prev_blank = false;

    for (i, line) in lines.iter().enumerate() {
        if let Some(open) = fence {
            let current = builders.last_mut().expect("fences only open inside blocks");
            current
                .content
                .push((deindent(line, current.content_col).to_owned(), true));
            if closes_fence(line, open) {
                fence = None;
            }
            prev_blank = false;
            continue;
        }

        if line.trim().is_empty() {
            if let Some(current) = builders.last_mut() {
                current
                    .content
                    .push((deindent(line, current.content_col).to_owned(), false));
            }
            prev_blank = true;
            continue;
        }

        let (col, rest) = indentation(line);
        let content_line;
        if let Some(after) = bullet_rest(rest) {
            while stack.last().is_some_and(|&(c, _)| c >= col) {
                stack.pop();
            }
            let parent = stack.last().map(|&(_, idx)| idx);
            stack.push((col, builders.len()));
            builders.push(Builder {
                kind: BlockKind::Bullet,
                parent,
                start_line: i,
                first_line: i,
                content_col: col + 2,
                content: Vec::new(),
            });
            content_line = after;
        } else {
            let starts_prose = match builders.last() {
                None => true,
                Some(b) if b.kind == BlockKind::Prose => prev_blank && col == 0,
                Some(_) => col == 0,
            };
            if starts_prose {
                stack.clear();
                builders.push(Builder {
                    kind: BlockKind::Prose,
                    parent: None,
                    start_line: i,
                    first_line: i,
                    content_col: 0,
                    content: Vec::new(),
                });
            }
            let current = builders.last().expect("a block is open");
            content_line = deindent(line, current.content_col);
        }

        fence = fence_opener(content_line);
        builders
            .last_mut()
            .expect("a block is open")
            .content
            .push((content_line.to_owned(), fence.is_some()));
        prev_blank = false;
    }

    // The first block also owns any leading blank lines, so blocks partition the file.
    if let Some(first) = builders.first_mut() {
        first.start_line = 0;
    }
    let ends: Vec<usize> = builders
        .iter()
        .skip(1)
        .map(|b| b.start_line)
        .chain(std::iter::once(lines.len()))
        .collect();

    builders
        .into_iter()
        .zip(ends)
        .map(|(b, end)| {
            let source =
                text[line_starts[b.start_line]..line_starts[end].min(text.len())].to_owned();
            finish(b, end, source)
        })
        .collect()
}

fn property_line(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once("::")?;
    if key.is_empty()
        || !key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return None;
    }
    if !(value.is_empty() || value.starts_with([' ', '\t'])) {
        return None;
    }
    Some((key.to_ascii_lowercase(), value.trim().to_owned()))
}

fn finish(b: Builder, end: usize, source: String) -> ParsedBlock {
    let mut lines = b.content;
    while lines.last().is_some_and(|(l, _)| l.trim().is_empty()) {
        lines.pop();
    }
    let body = lines
        .iter()
        .map(|(l, _)| l.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    // Properties either form the block's first lines or directly follow its first line.
    let is_prop = |(l, fenced): &(String, bool)| !fenced && property_line(l).is_some();
    // An empty block has no first line to follow.
    let prop_start = if lines.first().is_some_and(is_prop) {
        0
    } else {
        1.min(lines.len())
    };
    let prop_end = prop_start
        + lines
            .iter()
            .skip(prop_start)
            .take_while(|l| is_prop(l))
            .count();
    let props = lines[prop_start..prop_end]
        .iter()
        .map(|(l, _)| {
            let (key, value) = property_line(l).expect("checked above");
            let refs = scan_refs(&value);
            Property { key, value, refs }
        })
        .collect();
    let display: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(i, _)| !(prop_start..prop_end).contains(i))
        .map(|(_, (l, _))| l.as_str())
        .collect();
    let display = display.join("\n");

    let (task, content) = split_task(&display);
    let refs = scan_refs(&content);

    ParsedBlock {
        kind: b.kind,
        parent: b.parent,
        lines: b.start_line as u32..end as u32,
        line: b.first_line as u32 + 1,
        source,
        content,
        props,
        task,
        refs,
        body,
        column: b.content_col as u32,
    }
}

fn split_task(display: &str) -> (Option<TaskState>, String) {
    for state in [TaskState::Todo, TaskState::Doing, TaskState::Done] {
        let Some(rest) = display.strip_prefix(state.marker()) else {
            continue;
        };
        if rest.is_empty() {
            return (Some(state), String::new());
        }
        if let Some(rest) = rest.strip_prefix('\n') {
            return (Some(state), rest.to_owned());
        }
        if rest.starts_with(' ') {
            return (Some(state), rest.trim_start_matches(' ').to_owned());
        }
    }
    (None, display.to_owned())
}

// ---------------------------------------------------------------------------
// Inline scanner
// ---------------------------------------------------------------------------

/// Finds references in `text`, skipping fenced code, inline code and escaped characters.
/// Ranges in the result are byte ranges within `text`.
pub fn scan_refs(text: &str) -> Vec<RawRef> {
    let mut out = Vec::new();
    // Split into runs of non-fenced lines and scan each run.
    let mut fence: Option<Fence> = None;
    let mut chunk_start: Option<usize> = None;
    let mut offset = 0;
    for line in text.split('\n') {
        let line_end = offset + line.len();
        if let Some(open) = fence {
            if closes_fence(line, open) {
                fence = None;
            }
        } else if let Some(open) = fence_opener(line) {
            if let Some(start) = chunk_start.take() {
                scan_chunk(
                    &text[start..offset.saturating_sub(1).max(start)],
                    start,
                    &mut out,
                );
            }
            fence = Some(open);
        } else if chunk_start.is_none() {
            chunk_start = Some(offset);
        }
        offset = line_end + 1;
    }
    if let Some(start) = chunk_start {
        scan_chunk(&text[start..], start, &mut out);
    }
    out
}

fn scan_chunk(s: &str, base: usize, out: &mut Vec<RawRef>) {
    let b = s.as_bytes();
    let mut i = 0;
    // A markdown link destination to jump over once the link text has been scanned.
    let mut skip: Option<(usize, usize)> = None;
    while i < b.len() {
        if let Some((from, to)) = skip
            && i >= from
        {
            i = to;
            skip = None;
            continue;
        }
        let next = b.get(i + 1).copied();
        match b[i] {
            b'\\' if next.is_some_and(|c| c.is_ascii_punctuation()) => i += 2,
            b'`' => i = skip_code_span(b, i),
            b'[' if next == Some(b'[') => match wikilink(s, i) {
                Some((name, end)) => {
                    out.push(raw(RawRefKind::Page, name, base + i..base + end));
                    i = end;
                }
                None => i += 1,
            },
            b'(' if next == Some(b'(') => match block_ref(s, i) {
                Some((id, end)) => {
                    out.push(raw(RawRefKind::Block, id, base + i..base + end));
                    i = end;
                }
                None => i += 1,
            },
            b'#' if i == 0 || s[..i].ends_with(char::is_whitespace) => match tag(s, i) {
                Some((name, end)) => {
                    out.push(raw(RawRefKind::Tag, name, base + i..base + end));
                    i = end;
                }
                None => i += 1,
            },
            b'!' if next == Some(b'[') && b.get(i + 2) != Some(&b'[') => {
                // Images reference nothing but their file; their alt text is skipped.
                match inline_link(s, i + 1) {
                    Some(link) => {
                        if is_attachment_link(&link.dest) {
                            out.push(link_ref(RawRefKind::Attachment, &link, base, i));
                        }
                        i = link.end;
                    }
                    None => i += 1,
                }
            }
            b'[' => match inline_link(s, i) {
                Some(link) => {
                    if is_note_link(&link.dest) {
                        out.push(link_ref(RawRefKind::MdLink, &link, base, i));
                    } else if is_attachment_link(&link.dest) {
                        out.push(link_ref(RawRefKind::Attachment, &link, base, i));
                    }
                    // Keep scanning the link text for other references.
                    skip = Some((link.text_end, link.end));
                    i += 1;
                }
                None => i += 1,
            },
            _ => i += 1,
        }
    }
}

fn raw(kind: RawRefKind, text: &str, range: Range<usize>) -> RawRef {
    RawRef {
        kind,
        text: text.to_owned(),
        range,
        dest: None,
    }
}

/// A reference for the inline link starting at `i` (its `[`, or the `!` of an image).
fn link_ref(kind: RawRefKind, link: &InlineLink, base: usize, i: usize) -> RawRef {
    let mut r = raw(kind, &link.dest, base + i..base + link.end);
    r.dest = Some(base + link.dest_range.start..base + link.dest_range.end);
    r
}

/// Returns the index after an inline code span starting at `i`, or after the literal
/// backtick run if it is never closed.
fn skip_code_span(b: &[u8], i: usize) -> usize {
    let run = b[i..].iter().take_while(|&&c| c == b'`').count();
    let mut j = i + run;
    while j < b.len() {
        if b[j] == b'`' {
            let n = b[j..].iter().take_while(|&&c| c == b'`').count();
            if n == run {
                return j + n;
            }
            j += n;
        } else {
            j += 1;
        }
    }
    i + run
}

/// `[[name]]` at `i`: the name and the index after `]]`. Must close on the same line and
/// must not contain another `[[`.
fn wikilink(s: &str, i: usize) -> Option<(&str, usize)> {
    let inner_start = i + 2;
    let line_end = s[inner_start..]
        .find('\n')
        .map_or(s.len(), |n| inner_start + n);
    let close = s[inner_start..line_end].find("]]")? + inner_start;
    let inner = &s[inner_start..close];
    if inner.contains("[[") || inner.trim().is_empty() {
        return None;
    }
    Some((inner, close + 2))
}

fn block_ref(s: &str, i: usize) -> Option<(&str, usize)> {
    let inner_start = i + 2;
    let line_end = s[inner_start..]
        .find('\n')
        .map_or(s.len(), |n| inner_start + n);
    let close = s[inner_start..line_end].find("))")? + inner_start;
    let id = &s[inner_start..close];
    is_valid_block_id(id).then_some((id, close + 2))
}

const TAG_TRAILING: &[char] = &['.', ',', ';', ':', '!', '?', ')'];

fn tag(s: &str, i: usize) -> Option<(&str, usize)> {
    let after = &s[i + 1..];
    if after.starts_with("[[") {
        return wikilink(s, i + 1);
    }
    let first = after.chars().next()?;
    if first.is_whitespace() || first == '#' {
        return None;
    }
    let word_len = after.find(char::is_whitespace).unwrap_or(after.len());
    let name = after[..word_len].trim_end_matches(TAG_TRAILING);
    if name.is_empty() {
        return None;
    }
    Some((name, i + 1 + name.len()))
}

struct InlineLink {
    /// Index of the `]` closing the link text.
    text_end: usize,
    dest: String,
    /// Range of the destination token, including any `<>`.
    dest_range: Range<usize>,
    /// Index after the closing `)`.
    end: usize,
}

/// Parses an inline link `[text](dest "title")` starting at the `[` at `i`.
fn inline_link(s: &str, i: usize) -> Option<InlineLink> {
    let b = s.as_bytes();
    let mut j = i + 1;
    let mut depth = 1;
    while j < b.len() {
        match b[j] {
            b'\\' if b.get(j + 1).is_some_and(|c| c.is_ascii_punctuation()) => j += 2,
            b'`' => j = skip_code_span(b, j),
            b'[' => {
                depth += 1;
                j += 1;
            }
            b']' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
                j += 1;
            }
            _ => j += 1,
        }
    }
    if j >= b.len() || b.get(j + 1) != Some(&b'(') {
        return None;
    }
    let text_end = j;
    let mut k = skip_ws(b, j + 2);
    let dest;
    let dest_start = k;
    if b.get(k) == Some(&b'<') {
        let start = k + 1;
        let close = s[start..].find(['>', '\n', '<'])? + start;
        if b[close] != b'>' {
            return None;
        }
        dest = &s[start..close];
        k = close + 1;
    } else {
        let start = k;
        let mut parens = 0usize;
        while k < b.len() {
            match b[k] {
                c if c.is_ascii_whitespace() || c.is_ascii_control() => break,
                b'\\' if b.get(k + 1).is_some_and(|c| c.is_ascii_punctuation()) => k += 1,
                b'(' => parens += 1,
                b')' if parens == 0 => break,
                b')' => parens -= 1,
                _ => {}
            }
            k += 1;
        }
        dest = &s[start..k];
    }
    let dest_range = dest_start..k;
    k = skip_ws(b, k);
    if let Some(&q) = b.get(k)
        && matches!(q, b'"' | b'\'' | b'(')
    {
        let closer = if q == b'(' { b')' } else { q };
        k += 1;
        while k < b.len() && b[k] != closer {
            k += if b[k] == b'\\' { 2 } else { 1 };
        }
        k = skip_ws(b, k + 1);
    }
    if b.get(k) != Some(&b')') {
        return None;
    }
    Some(InlineLink {
        text_end,
        dest: unescape(dest),
        dest_range,
        end: k + 1,
    })
}

fn skip_ws(b: &[u8], mut k: usize) -> usize {
    while k < b.len() && b[k].is_ascii_whitespace() {
        k += 1;
    }
    k
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\'
            && let Some(&n) = chars.peek()
            && n.is_ascii_punctuation()
        {
            out.push(n);
            chars.next();
            continue;
        }
        out.push(c);
    }
    out
}

/// Whether a markdown link destination points at a note: a relative or root-relative
/// path to a `.md` file, without a URL scheme.
fn is_note_link(dest: &str) -> bool {
    let path = dest.split('#').next().unwrap_or_default();
    !path.is_empty() && !has_scheme(path) && path.ends_with(".md")
}

/// Whether a link or image destination points at a file in the notes other than a note:
/// a relative or root-relative path without a URL scheme, not protocol-relative.
fn is_attachment_link(dest: &str) -> bool {
    let path = dest.split('#').next().unwrap_or_default();
    !path.is_empty() && !has_scheme(path) && !path.starts_with("//") && !path.ends_with(".md")
}

fn has_scheme(s: &str) -> bool {
    let Some((scheme, _)) = s.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contents(blocks: &[ParsedBlock]) -> Vec<&str> {
        blocks.iter().map(|b| b.content.as_str()).collect()
    }

    fn parents(blocks: &[ParsedBlock]) -> Vec<Option<usize>> {
        blocks.iter().map(|b| b.parent).collect()
    }

    fn reassembled(blocks: &[ParsedBlock]) -> String {
        blocks.iter().map(|b| b.source.as_str()).collect()
    }

    #[test]
    fn single_line_blocks() {
        let blocks = parse_blocks("- first\n- second\n");
        assert_eq!(contents(&blocks), ["first", "second"]);
        assert_eq!(parents(&blocks), [None, None]);
    }

    #[test]
    fn empty_blocks() {
        for text in ["-\n", "- \n", "- a\n-\n  - b\n", "-"] {
            let blocks = parse_blocks(text);
            assert_eq!(reassembled(&blocks), text);
            assert!(
                blocks
                    .iter()
                    .any(|b| b.content.is_empty() && b.body.is_empty())
            );
        }
        assert_eq!(
            parents(&parse_blocks("- a\n-\n  - b\n")),
            [None, None, Some(1)]
        );
    }

    #[test]
    fn bodies_and_columns() {
        let blocks = parse_blocks("- TODO a\n  id:: x\n\n  more\n\n\t- b\n\t  c\nprose\n");
        assert_eq!(blocks[0].body, "TODO a\nid:: x\n\nmore");
        assert_eq!(blocks[0].column, 2);
        assert_eq!(blocks[1].body, "b\nc");
        assert_eq!(blocks[1].column, 6);
        assert_eq!(blocks[2].body, "prose");
        assert_eq!(blocks[2].column, 0);
    }

    #[test]
    fn multi_line_block() {
        let blocks = parse_blocks("- first line\n  more text\n");
        assert_eq!(contents(&blocks), ["first line\nmore text"]);
    }

    #[test]
    fn children_and_siblings() {
        let blocks = parse_blocks("- a\n  - b\n    - c\n  - d\n- e\n");
        assert_eq!(contents(&blocks), ["a", "b", "c", "d", "e"]);
        assert_eq!(parents(&blocks), [None, Some(0), Some(1), Some(0), None]);
    }

    #[test]
    fn tabs_and_spaces_produce_the_same_tree() {
        let tabs = parse_blocks("- a\n\t- b\n\t\t- c\n\t  continued\n- d\n");
        let spaces = parse_blocks("- a\n  - b\n    - c\n      continued\n- d\n");
        assert_eq!(contents(&tabs), contents(&spaces));
        assert_eq!(parents(&tabs), parents(&spaces));
        assert_eq!(contents(&tabs)[2], "c\ncontinued");
    }

    #[test]
    fn irregular_indentation() {
        let blocks = parse_blocks("- a\n      - b\n  - c\n");
        assert_eq!(parents(&blocks), [None, Some(0), Some(0)]);
    }

    #[test]
    fn bullet_inside_code_fence() {
        let text = "- code:\n  ```\n  - not a block\n  [[not a link]]\n  ```\n- after\n";
        let blocks = parse_blocks(text);
        assert_eq!(blocks.len(), 2);
        assert_eq!(
            blocks[0].content,
            "code:\n```\n- not a block\n[[not a link]]\n```"
        );
        assert!(blocks[0].refs.is_empty());
        assert_eq!(blocks[1].content, "after");
    }

    #[test]
    fn plain_prose_page() {
        let text = "First paragraph\nstill first.\n\nSecond paragraph.\n";
        let blocks = parse_blocks(text);
        assert_eq!(
            contents(&blocks),
            ["First paragraph\nstill first.", "Second paragraph."]
        );
        assert!(blocks.iter().all(|b| b.kind == BlockKind::Prose));
        assert_eq!(reassembled(&blocks), text);
    }

    #[test]
    fn preamble_before_bullets() {
        let blocks = parse_blocks("Intro line\n- a\n");
        assert_eq!(contents(&blocks), ["Intro line", "a"]);
        assert_eq!(blocks[0].kind, BlockKind::Prose);
        assert_eq!(blocks[1].kind, BlockKind::Bullet);
    }

    #[test]
    fn unindented_text_after_bullet_is_a_new_top_level_block() {
        let blocks = parse_blocks("- a\n  - b\nloose text\n  - c\n");
        assert_eq!(contents(&blocks), ["a", "b", "loose text", "c"]);
        assert_eq!(parents(&blocks), [None, Some(0), None, None]);
    }

    #[test]
    fn sources_partition_the_file() {
        let texts = [
            "\n\n- a\n\n  - b\n    cont\n\nprose\n\n```\n- x\n```\n- c",
            "- a\r-ish\n- b\n",
            "- ```\n  unclosed\n- not a block",
        ];
        for text in texts {
            assert_eq!(reassembled(&parse_blocks(text)), text, "{text:?}");
        }
        // Whitespace-only files contain no text, so they have no blocks.
        assert!(parse_blocks("").is_empty());
        assert!(parse_blocks("   \n\n").is_empty());
    }

    #[test]
    fn line_ranges() {
        let blocks = parse_blocks("\n- a\n  cont\n- b\n");
        assert_eq!(blocks[0].lines, 0..3);
        assert_eq!(blocks[0].line, 2);
        assert_eq!(blocks[1].lines, 3..5);
        assert_eq!(blocks[1].line, 4);
    }

    #[test]
    fn property_after_first_line() {
        let blocks = parse_blocks("- decided to use JWT\n  status:: accepted\n  Status:: dup\n");
        assert_eq!(blocks[0].content, "decided to use JWT");
        assert_eq!(blocks[0].prop("status"), Some("accepted"));
        assert_eq!(blocks[0].props.len(), 2);
    }

    #[test]
    fn properties_as_first_lines() {
        let blocks = parse_blocks("title:: Auth\ntags:: [[security]]\n\n- a\n");
        assert_eq!(blocks[0].content, "");
        assert_eq!(blocks[0].prop("title"), Some("Auth"));
        assert_eq!(blocks[0].props[1].refs[0].text, "security");
    }

    #[test]
    fn alias_names_split() {
        assert_eq!(
            alias_names("a, [[b c]] ,, [[ d ]],e"),
            ["a", "b c", "d", "e"]
        );
        assert_eq!(alias_names("[[a]] [[b]]"), ["[[a]] [[b]]"]);
        assert_eq!(alias_names(" , [[]]"), Vec::<&str>::new());
    }

    #[test]
    fn properties_only() {
        let blocks = parse_blocks("- alias:: a\n- TODO\n  x:: y\n- b\n  x:: y\n");
        let only = |b: &ParsedBlock| is_properties_only(&b.content, b.task, &b.props);
        assert_eq!(
            blocks.iter().map(only).collect::<Vec<_>>(),
            [true, false, false]
        );
    }

    #[test]
    fn property_like_line_later_is_content() {
        let blocks = parse_blocks("- first\n  plain\n  note:: x\n");
        assert!(blocks[0].props.is_empty());
        assert_eq!(blocks[0].content, "first\nplain\nnote:: x");
    }

    #[test]
    fn not_properties() {
        for text in ["- a\n  key::value", "- a\n  two words:: x", "- a\n  ::x"] {
            assert!(parse_blocks(text)[0].props.is_empty(), "{text:?}");
        }
    }

    #[test]
    fn block_ids() {
        assert!(is_valid_block_id("auth-ttl-bug"));
        assert!(!is_valid_block_id("two words"));
        assert!(!is_valid_block_id(""));
        assert!(!is_valid_block_id("a(b)"));
        let blocks = parse_blocks("- x\n  id:: auth-ttl-bug\n");
        assert_eq!(blocks[0].prop("id"), Some("auth-ttl-bug"));
    }

    #[test]
    fn tasks() {
        let blocks = parse_blocks(
            "- TODO rotate signing keys\n- DOING x\n- DONE\n- todo later\n- TODOS for today\n",
        );
        assert_eq!(blocks[0].task, Some(TaskState::Todo));
        assert_eq!(blocks[0].content, "rotate signing keys");
        assert_eq!(blocks[1].task, Some(TaskState::Doing));
        assert_eq!(blocks[2].task, Some(TaskState::Done));
        assert_eq!(blocks[2].content, "");
        assert_eq!(blocks[3].task, None);
        assert_eq!(blocks[4].task, None);
        assert!(
            TaskState::Todo.is_open() && TaskState::Doing.is_open() && !TaskState::Done.is_open()
        );
    }

    #[test]
    fn task_with_property_and_ref() {
        let blocks = parse_blocks("- TODO fix [[auth]]\n  id:: t1\n");
        let b = &blocks[0];
        assert_eq!(b.task, Some(TaskState::Todo));
        assert_eq!(b.content, "fix [[auth]]");
        assert_eq!(&b.content[b.refs[0].range.clone()], "[[auth]]");
    }

    fn refs(text: &str) -> Vec<(RawRefKind, String)> {
        scan_refs(text)
            .into_iter()
            .map(|r| (r.kind, r.text))
            .collect()
    }

    fn one(kind: RawRefKind, text: &str) -> Vec<(RawRefKind, String)> {
        vec![(kind, text.to_owned())]
    }

    #[test]
    fn page_links() {
        assert_eq!(
            refs("met with X about [[auth]]"),
            one(RawRefKind::Page, "auth")
        );
        assert!(refs("`[[auth]]`").is_empty());
        assert!(refs("``a `[[auth]]` b``").is_empty());
        assert!(refs(r"\[[auth]]").is_empty());
        assert!(refs("[[auth\n]]").is_empty());
        assert!(refs("[[ ]]").is_empty());
        assert_eq!(refs("[[a [[b]]]]"), one(RawRefKind::Page, "b"));
        assert_eq!(refs("unclosed ` then [[x]]"), one(RawRefKind::Page, "x"));
    }

    #[test]
    fn tags() {
        assert_eq!(
            refs("fixed the bug #backend."),
            one(RawRefKind::Tag, "backend")
        );
        assert_eq!(refs("#start of line"), one(RawRefKind::Tag, "start"));
        assert_eq!(
            refs("x #a, #b; (#c)"),
            vec![(RawRefKind::Tag, "a".into()), (RawRefKind::Tag, "b".into())]
        );
        assert_eq!(refs("#[[Project X]]"), one(RawRefKind::Tag, "Project X"));
        assert!(refs("# Heading").is_empty());
        assert!(refs("## Heading").is_empty());
        assert!(refs("https://example.com/a#section").is_empty());
        assert!(refs("[x](#anchor)").is_empty());
        assert!(refs("#...").is_empty());
        assert_eq!(refs("line\n#second"), one(RawRefKind::Tag, "second"));
    }

    #[test]
    fn block_refs() {
        assert_eq!(
            refs("see ((auth-ttl-bug))"),
            one(RawRefKind::Block, "auth-ttl-bug")
        );
        assert!(refs("((two words))").is_empty());
        assert!(refs("`((x))`").is_empty());
    }

    #[test]
    fn markdown_links() {
        assert_eq!(
            refs("[the notes](../pages/auth.md)"),
            one(RawRefKind::MdLink, "../pages/auth.md")
        );
        assert_eq!(
            refs("[x](Project%20X/Auth%20Flow.md#setup)"),
            one(RawRefKind::MdLink, "Project%20X/Auth%20Flow.md#setup")
        );
        assert_eq!(
            refs("[x](<a b.md> \"title\")"),
            one(RawRefKind::MdLink, "a b.md")
        );
        assert_eq!(
            refs("[x](/pages/a.md)"),
            one(RawRefKind::MdLink, "/pages/a.md")
        );
        assert!(refs("[site](https://example.com/a.md)").is_empty());
        assert!(refs("![diagram](pic.md)").is_empty());
        assert!(refs("[x] (a.md)").is_empty());
        assert!(refs("`[x](a.md)`").is_empty());
        assert_eq!(
            refs("[see [[auth]]](a.md)"),
            vec![
                (RawRefKind::MdLink, "a.md".into()),
                (RawRefKind::Page, "auth".into())
            ]
        );
        assert!(
            refs("[x](a.md#[[nope]])")
                .iter()
                .all(|(k, _)| *k != RawRefKind::Page)
        );
    }

    #[test]
    fn attachments() {
        assert_eq!(
            refs("![flow](../assets/flow.png)"),
            one(RawRefKind::Attachment, "../assets/flow.png")
        );
        assert_eq!(
            refs("[spec](files/spec.pdf#page=2)"),
            one(RawRefKind::Attachment, "files/spec.pdf#page=2")
        );
        assert_eq!(
            refs("![x](</assets/a b.png> \"t\")"),
            one(RawRefKind::Attachment, "/assets/a b.png")
        );
        for text in [
            "![x](https://example.com/x.png)",
            "![x](data:image/png;base64,AAAA)",
            "![x](//cdn.example.com/x.png)",
            "[x](#setup)",
            "[x](mailto:a@example.com)",
            "`![x](a.png)`",
            "![x] (a.png)",
        ] {
            assert!(refs(text).is_empty(), "{text:?}");
        }
        // The alt text of an image is not scanned for references.
        assert_eq!(
            refs("![see [[auth]]](a.png)"),
            one(RawRefKind::Attachment, "a.png")
        );
        // A link around an image: both are attachments, the link text is still scanned.
        assert_eq!(
            refs("[![x](a.png)](b.png)"),
            vec![
                (RawRefKind::Attachment, "b.png".into()),
                (RawRefKind::Attachment, "a.png".into())
            ]
        );
    }

    #[test]
    fn markdown_link_destination_ranges() {
        for (text, dest) in [
            ("see [x](a.md) now", "a.md"),
            ("[x](<a b.md> \"t\")", "<a b.md>"),
            ("[x](a.md#s)", "a.md#s"),
            ("see ![x](a.png) now", "a.png"),
            ("![x](<a b.png> \"t\")", "<a b.png>"),
        ] {
            let r = &scan_refs(text)[0];
            assert_eq!(&text[r.dest.clone().unwrap()], dest, "{text:?}");
        }
        assert_eq!(scan_refs("[[a]]")[0].dest, None);
    }

    #[test]
    fn fenced_code_in_scanned_text() {
        assert_eq!(
            refs("[[a]]\n```\n[[b]]\n```\n[[c]]"),
            vec![
                (RawRefKind::Page, "a".into()),
                (RawRefKind::Page, "c".into())
            ]
        );
    }

    #[test]
    fn ranges_point_at_source_text() {
        let text = "x [[a]] y #b z ((c)) [t](d.md) #[[e f]] ![i](g.png)";
        for r in scan_refs(text) {
            let s = &text[r.range.clone()];
            match r.kind {
                RawRefKind::Page => assert_eq!(s, "[[a]]"),
                RawRefKind::Tag if r.text == "b" => assert_eq!(s, "#b"),
                RawRefKind::Tag => assert_eq!(s, "#[[e f]]"),
                RawRefKind::Block => assert_eq!(s, "((c))"),
                RawRefKind::MdLink => assert_eq!(s, "[t](d.md)"),
                RawRefKind::Attachment => assert_eq!(s, "![i](g.png)"),
            }
        }
        assert_eq!(scan_refs(text).len(), 6);
    }

    proptest::proptest! {
        #![proptest_config(proptest::test_runner::Config {
            // Keep regression seeds next to the source instead of in a top-level directory.
            failure_persistence: Some(Box::new(
                proptest::test_runner::FileFailurePersistence::WithSource("proptest-regressions"),
            )),
            ..Default::default()
        })]
        /// Parsing never panics, and the blocks' sources reassemble the text exactly
        /// (a text of only blank lines has no blocks).
        #[test]
        fn no_text_is_lost(lines in proptest::collection::vec(proptest::sample::select(vec![
            "", " ", "-", "- ", "- a", "  - b", "\t- c", "    -", "text", "  more", "```",
            "  ```", "~~~", "key:: v", "  id:: x", "TODO", "- DONE x", "\t", "#tag [[p]]",
        ]), 0..24), crlf in proptest::bool::ANY, newline in proptest::bool::ANY) {
            let mut text = lines.join(if crlf { "\r\n" } else { "\n" });
            if newline {
                text.push('\n');
            }
            let text = text.replace("\r\n", "\n");
            let blocks = parse_blocks(&text);
            if text.trim().is_empty() {
                proptest::prop_assert!(blocks.is_empty());
            } else {
                proptest::prop_assert_eq!(reassembled(&blocks), text);
            }
        }
    }
}
