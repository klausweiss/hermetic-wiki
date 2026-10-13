//! Writing the search index (`search/`), read by `static/packages/search`.
//!
//! ```text
//! search/meta.json          { version, id, n_docs, avg_len: [name, text], k1, b,
//!                             journal_docs: [first, last] | null, doc_chunk, shards }
//! search/t/<hex key>.json   [[term, stem | "", postings], ...]
//! search/d/<n>.json         { pages: [[path, title], ...],
//!                             docs: [[page_index, anchor, parent_delta, preview], ...] }
//! ```
//! `postings` is a string of numbers packed with [`pack`]: for each document containing
//! the term, `doc_delta, tf * 2 + field, field_length`. `parent_delta` is the distance
//! back to the document of the block's parent (0 for top-level blocks and pages), from
//! which breadcrumbs are built.
//! Documents are pages (field 0: the name, then one more document per alias) and
//! blocks (field 1: the text), in scan order, so journal blocks come first and in date
//! order. A shard holds the terms
//! starting with the same two characters (hex-encoded in the file name).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use super::normalize::words;
use super::stem::stem;
use super::text::{PREVIEW_CHARS, block_text, truncate};
use crate::graph::{BlockId, Graph, PageId};

pub const VERSION: u32 = 1;
pub const DOC_CHUNK: usize = 256;
pub const K1: f64 = 1.2;
pub const B: f64 = 0.75;
const FIELD_NAME: u8 = 0;
const FIELD_TEXT: u8 = 1;

/// What the index needs to know about the rendered site.
pub struct SiteInfo<'a> {
    pub path: &'a dyn Fn(PageId) -> PathBuf,
    pub title: &'a dyn Fn(PageId) -> String,
    pub anchor: &'a dyn Fn(BlockId) -> String,
}

struct Doc {
    page: PageId,
    /// Empty for page documents.
    anchor: String,
    /// Document id of the block's parent block, if any.
    parent: Option<usize>,
    preview: String,
}

/// Postings of one term: (doc, field, tf, field length), in doc order.
type Postings = Vec<(usize, u8, u32, u32)>;

/// The index files, keyed by their path relative to the site root.
pub fn build(g: &Graph, site: &SiteInfo) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut docs: Vec<Doc> = Vec::new();
    let mut terms: BTreeMap<String, Postings> = BTreeMap::new();
    let mut lengths = [(0u64, 0u64); 2];
    let mut journal_docs: Option<(usize, usize)> = None;

    let mut add = |docs: &mut Vec<Doc>, doc: Doc, field: u8, text: &str| {
        let id = docs.len();
        docs.push(doc);
        let ws = words(text);
        let mut tf: BTreeMap<String, u32> = BTreeMap::new();
        for w in &ws {
            *tf.entry(w.clone()).or_default() += 1;
        }
        let len = ws.len() as u32;
        lengths[field as usize].0 += u64::from(len);
        lengths[field as usize].1 += 1;
        for (term, count) in tf {
            terms.entry(term).or_default().push((id, field, count, len));
        }
        id
    };

    for (page, p) in g.pages() {
        let first = add(
            &mut docs,
            Doc {
                page,
                anchor: String::new(),
                parent: None,
                preview: String::new(),
            },
            FIELD_NAME,
            &p.name,
        );
        let mut last = first;
        // One more page document per alias, so that each is an exact name of its own.
        for alias in &p.aliases {
            last = add(
                &mut docs,
                Doc {
                    page,
                    anchor: String::new(),
                    parent: None,
                    preview: String::new(),
                },
                FIELD_NAME,
                alias,
            );
        }
        // Depth first, so a parent's document always precedes its children's.
        let mut stack: Vec<(BlockId, Option<usize>)> =
            p.roots.iter().rev().map(|&b| (b, None)).collect();
        while let Some((b, parent)) = stack.pop() {
            let text = block_text(g, b);
            last = add(
                &mut docs,
                Doc {
                    page,
                    anchor: (site.anchor)(b),
                    parent,
                    preview: truncate(&text, PREVIEW_CHARS),
                },
                FIELD_TEXT,
                &text,
            );
            stack.extend(g.block(b).children.iter().rev().map(|&c| (c, Some(last))));
        }
        if p.is_journal() && !p.is_implicit() {
            let range = journal_docs.get_or_insert((first, last));
            range.1 = last;
        }
    }

    let mut files = BTreeMap::new();

    // Term shards.
    let mut shards: BTreeMap<String, String> = BTreeMap::new();
    for (term, postings) in &terms {
        let key: String = term.chars().take(2).collect();
        let shard = shards.entry(key).or_insert_with(|| String::from("["));
        if shard.len() > 1 {
            shard.push(',');
        }
        let s = stem(term);
        let mut packed = String::with_capacity(postings.len() * 3);
        let mut prev = 0;
        for &(doc, field, tf, len) in postings {
            pack(&mut packed, (doc - prev) as u64);
            pack(&mut packed, u64::from(tf) * 2 + u64::from(field));
            pack(&mut packed, u64::from(len));
            prev = doc;
        }
        let _ = write!(
            shard,
            "[{},{},\"{packed}\"]",
            json_string(term),
            json_string(if &s == term { "" } else { &s }),
        );
    }
    let keys: Vec<String> = shards.keys().map(|k| hex(k)).collect();
    for (key, mut body) in shards {
        body.push(']');
        files.insert(
            Path::new("search/t").join(format!("{}.json", hex(&key))),
            body.into_bytes(),
        );
    }

    // Document chunks, each with its own table of pages.
    for (n, chunk) in docs.chunks(DOC_CHUNK).enumerate() {
        let mut pages: Vec<PageId> = Vec::new();
        let mut entries = Vec::with_capacity(chunk.len());
        for (i, doc) in chunk.iter().enumerate() {
            let id = n * DOC_CHUNK + i;
            let index = pages
                .iter()
                .position(|&p| p == doc.page)
                .unwrap_or_else(|| {
                    pages.push(doc.page);
                    pages.len() - 1
                });
            entries.push(format!(
                "[{index},{},{},{}]",
                json_string(&doc.anchor),
                doc.parent.map_or(0, |p| id - p),
                json_string(&doc.preview)
            ));
        }
        let pages: Vec<String> = pages
            .iter()
            .map(|&p| {
                let path = (site.path)(p);
                format!(
                    "[{},{}]",
                    json_string(&url_path(&path)),
                    json_string(&(site.title)(p))
                )
            })
            .collect();
        let body = format!(
            "{{\"pages\":[{}],\"docs\":[{}]}}",
            pages.join(","),
            entries.join(",")
        );
        files.insert(
            Path::new("search/d").join(format!("{n}.json")),
            body.into_bytes(),
        );
    }

    // Metadata, with an id that changes whenever any index file does.
    let avg = |(sum, count): (u64, u64)| {
        if count == 0 {
            0.0
        } else {
            sum as f64 / count as f64
        }
    };
    let id = files
        .values()
        .fold(0xcbf2_9ce4_8422_2325u64, |h, bytes| fnv1a(h, bytes));
    let journal = journal_docs.map_or_else(|| "null".to_owned(), |(a, b)| format!("[{a},{b}]"));
    let meta = format!(
        "{{\"version\":{VERSION},\"id\":\"{id:016x}\",\"n_docs\":{},\"avg_len\":[{:.3},{:.3}],\"k1\":{K1},\"b\":{B},\"journal_docs\":{journal},\"doc_chunk\":{DOC_CHUNK},\"shards\":[{}]}}",
        docs.len(),
        avg(lengths[0]),
        avg(lengths[1]),
        keys.iter()
            .map(|k| json_string(k))
            .collect::<Vec<_>>()
            .join(",")
    );
    files.insert(PathBuf::from("search/meta.json"), meta.into_bytes());
    files
}

/// Digits of [`pack`]: the 64 characters that need no escaping in JSON or URLs.
pub const PACK_DIGITS: &[u8; 64] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-_";

/// Appends `n` as a variable-length number: five bits per character, low bits first,
/// with the sixth bit set on every character but the last.
pub fn pack(out: &mut String, mut n: u64) {
    loop {
        let low = (n & 31) as usize;
        n >>= 5;
        if n == 0 {
            out.push(PACK_DIGITS[low] as char);
            return;
        }
        out.push(PACK_DIGITS[low | 32] as char);
    }
}

/// Reads numbers written by [`pack`].
pub fn unpack(packed: &str) -> Vec<u64> {
    let mut out = Vec::new();
    let (mut n, mut shift) = (0u64, 0u32);
    for c in packed.bytes() {
        let v = PACK_DIGITS.iter().position(|&d| d == c).unwrap_or(0) as u64;
        n |= (v & 31) << shift;
        if v & 32 == 0 {
            out.push(n);
            n = 0;
            shift = 0;
        } else {
            shift += 5;
        }
    }
    out
}

/// Lowercase hex of the UTF-8 bytes: a file name that is safe for any script.
pub fn hex(key: &str) -> String {
    key.bytes().map(|b| format!("{b:02x}")).collect()
}

fn url_path(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

pub fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers() {
        assert_eq!(hex("re"), "7265");
        assert_eq!(hex("ż"), "c5bc");
        assert_eq!(json_string("a\"b\\c\n"), r#""a\"b\\c\u000a""#);
        let mut s = String::new();
        for n in [0, 1, 31, 32, 1000, 1 << 20] {
            pack(&mut s, n);
        }
        assert_eq!(s, "01VW1eVWWWW1");
        assert_eq!(unpack(&s), [0, 1, 31, 32, 1000, 1 << 20]);
    }
}
