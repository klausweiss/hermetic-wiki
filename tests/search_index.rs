//! The search index written by the build: what is indexed and how.

mod common;

use std::fs;
use std::path::Path;

use common::{Notes, block, build_site};
use hermetic::search::index::{hex, unpack};
use hermetic::search::text::{block_text, breadcrumb};

/// The JSON of the shard holding `term`.
fn shard(out: &Path, term: &str) -> String {
    let key: String = term.chars().take(2).collect();
    fs::read_to_string(out.join(format!("search/t/{}.json", hex(&key)))).unwrap_or_default()
}

/// Postings of `term` as (doc, field, tf, len) tuples, parsed from its shard.
fn postings(out: &Path, term: &str) -> Vec<[u64; 4]> {
    let json = shard(out, term);
    // Entries look like ["term","stem","packed"].
    let marker = format!("[\"{term}\",\"");
    let Some(start) = json.find(&marker) else {
        return Vec::new();
    };
    let rest = &json[start + marker.len()..];
    let after_stem = &rest[rest.find("\",\"").unwrap() + 3..];
    let packed = &after_stem[..after_stem.find('"').unwrap()];
    let mut doc = 0;
    unpack(packed)
        .chunks(3)
        .map(|c| {
            doc += c[0];
            [doc, c[1] % 2, c[1] / 2, c[2]]
        })
        .collect()
}

#[test]
fn words_in_a_journal_block() {
    let notes = Notes::new(&[
        (
            "journals/2026-10-12.md",
            "- standup\n  - token refresh shipped for [[auth]]",
        ),
        ("pages/auth.md", "- the auth page"),
    ]);
    let (_h, out, _) = build_site(notes.path());
    let refresh = postings(&out, "refresh");
    assert_eq!(refresh.len(), 1, "linked references must not add postings");
    assert_eq!(refresh[0][1], 1, "text field");
    let auth = postings(&out, "auth");
    assert!(
        auth.iter().any(|p| p[1] == 0),
        "page name `auth` is indexed: {auth:?}"
    );
    assert!(
        auth.iter().any(|p| p[0] == refresh[0][0]),
        "the journal block mentions auth"
    );
    // The block's preview, and its parent (`standup`, one document back), are in its
    // document chunk.
    let docs = fs::read_to_string(out.join("search/d/0.json")).unwrap();
    assert!(docs.contains(r#""b1",0,"standup"]"#), "{docs}");
    assert!(
        docs.contains(r#""b2",1,"token refresh shipped for auth"]"#),
        "{docs}"
    );
    assert!(out.join("search.html").is_file() && out.join("search/meta.json").is_file());
}

#[test]
fn markup_is_not_indexed() {
    let notes = Notes::new(&[
        ("pages/redis.md", "- target block\n  id:: ttl-bug"),
        (
            "pages/a.md",
            "- see ((ttl-bug)) and [[Some Page]] #tagged **bold** [md](redis.md) `code_word`\n  status:: accepted\n  id:: own-id",
        ),
    ]);
    let (g, _) = hermetic::Graph::load(notes.path());
    let b = block(
        &g,
        "see ((ttl-bug)) and [[Some Page]] #tagged **bold** [md](redis.md) `code_word`",
    );
    let text = block_text(&g, b);
    assert_eq!(text, "see and Some Page tagged bold md code_word accepted");
    let (_h, out, _) = build_site(notes.path());
    for absent in ["ttl", "bug", "own", "id"] {
        assert!(
            postings(&out, absent).is_empty(),
            "`{absent}` must not be indexed"
        );
    }
    for present in ["code", "word", "accepted", "tagged", "some", "page"] {
        assert!(
            !postings(&out, present).is_empty(),
            "`{present}` must be indexed"
        );
    }
}

#[test]
fn breadcrumbs_and_stems() {
    let notes = Notes::new(&[(
        "pages/a.md",
        "- a very long parent block that talks about many different things at once\n  - we were testing the parser",
    )]);
    let (g, _) = hermetic::Graph::load(notes.path());
    let child = block(&g, "we were testing the parser");
    assert_eq!(
        breadcrumb(&g, child),
        "a very long parent block that talks…"
    );
    let (_h, out, _) = build_site(notes.path());
    assert!(
        shard(&out, "testing").contains(r#"["testing","test",""#),
        "{}",
        shard(&out, "testing")
    );
}

#[test]
fn journal_documents_come_first() {
    let notes = Notes::new(&[
        ("pages/a.md", "- page"),
        ("journals/2025-01-10.md", "- old"),
        ("journals/2026-10-01.md", "- new\n- newer"),
    ]);
    let (_h, out, _) = build_site(notes.path());
    let meta = fs::read_to_string(out.join("search/meta.json")).unwrap();
    // Two journal pages: page doc + 1 block, page doc + 2 blocks = documents 0..=4.
    assert!(meta.contains(r#""journal_docs":[0,4]"#), "{meta}");
}

/// Keeps `static/packages/search/test-index/` (the index the TypeScript query tests
/// read) equal to what the indexer produces for `tests/fixtures/search-notes`.
#[test]
fn typescript_test_index_is_current() {
    let notes = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/search-notes");
    let (_h, out, _) = build_site(&notes);
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("static/packages/search/test-index");
    let built = common::tree(&out.join("search"));
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        let _ = fs::remove_dir_all(&target);
        for (rel, bytes) in &built {
            let path = target.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
    }
    assert!(
        common::tree(&target) == built,
        "static/packages/search/test-index is out of date: run UPDATE_SNAPSHOTS=1 cargo test --test search_index"
    );
}

#[test]
fn aliases_are_page_names() {
    let notes = Notes::new(&[("pages/auth.md", "alias:: authentication\n\n- notes")]);
    let (_h, out, _) = build_site(notes.path());
    let all = postings(&out, "authentication");
    let alias: Vec<_> = all.iter().filter(|p| p[1] == 0).collect();
    assert_eq!(alias.len(), 1, "one name-field posting: {all:?}");
    assert_eq!(alias[0][0], 1, "right after the page's own document");
    assert_eq!(alias[0][3], 1, "the alias alone is the name");
    let docs = fs::read_to_string(out.join("search/d/0.json")).unwrap();
    assert!(
        docs.contains(r#""pages":[["pages/auth.html","auth"]]"#),
        "{docs}"
    );
    // The page's own document and the alias's both belong to the page.
    assert!(docs.contains(r#"[0,"",0,""],[0,"",0,""]"#), "{docs}");
}
