//! Scenarios from the `source-layout` spec.

mod common;

use std::path::PathBuf;

use common::*;
use hermetic::DiagnosticKind;
use hermetic::graph::PageKind;
use hermetic::layout::Date;

#[test]
fn journals_and_pages_are_discovered() {
    let notes = Notes::new(&[("journals/2026-10-12.md", "- hi"), ("pages/auth.md", "- x")]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{diags:?}");
    let j = g.page(page(&g, "2026-10-12"));
    assert_eq!(
        j.kind,
        PageKind::Journal(Date::parse("2026-10-12").unwrap())
    );
    assert_eq!(j.file, Some(PathBuf::from("journals/2026-10-12.md")));
    assert_eq!(g.page(page(&g, "auth")).kind, PageKind::Regular);
}

#[test]
fn unrelated_files_are_ignored() {
    let notes = Notes::new(&[
        ("README.md", "- [[x]]"),
        ("pages/diagram.png", "png"),
        ("pages/.drafts/x.md", "- [[y]]"),
        ("journals/.hidden.md", "- [[z]]"),
        ("other/a.md", "- [[w]]"),
    ]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(g.pages().count(), 0);
}

#[test]
fn missing_directories() {
    let notes = Notes::empty();
    let (g, diags) = notes.load();
    assert!(diags.is_empty());
    assert_eq!(g.pages().count(), 0);

    let notes = Notes::new(&[("pages/a.md", "- a")]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty());
    assert_eq!(g.pages().count(), 1);
}

#[test]
fn valid_journal_file() {
    let notes = Notes::new(&[("journals/2024-02-29.md", "- leap")]);
    let (g, _) = notes.load();
    assert_eq!(
        g.page(page(&g, "2024-02-29")).date(),
        Some(Date {
            year: 2024,
            month: 2,
            day: 29
        })
    );
}

#[test]
fn invalid_journal_file_names() {
    let notes = Notes::new(&[
        ("journals/2026-13-40.md", "- a"),
        ("journals/notes.md", "- b"),
    ]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::InvalidJournalName; 2]);
    assert_eq!(g.page(page(&g, "2026-13-40")).kind, PageKind::Regular);
    assert_eq!(g.page(page(&g, "notes")).kind, PageKind::Regular);
}

#[test]
fn namespaced_page_file() {
    let notes = Notes::new(&[("pages/Project X/Auth Flow.md", "- x")]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty());
    let child = page(&g, "Project X/Auth Flow");
    let parent = page(&g, "Project X");
    assert_eq!(g.page(child).ns_parent, Some(parent));
    assert!(g.page(parent).is_implicit());
    assert_eq!(g.page(child).leaf_name(), "Auth Flow");
}

#[test]
fn page_and_namespace_directory_side_by_side() {
    let notes = Notes::new(&[
        ("pages/Project X.md", "- the project"),
        ("pages/Project X/Auth Flow.md", "- x"),
    ]);
    let (g, _) = notes.load();
    let parent = page(&g, "Project X");
    assert!(!g.page(parent).is_implicit());
    assert_eq!(contents(&g, g.page(parent).roots.clone()), ["the project"]);
    assert_eq!(
        g.page(page(&g, "Project X/Auth Flow")).ns_parent,
        Some(parent)
    );
}

#[test]
fn encoded_file_names_are_decoded() {
    let notes = Notes::new(&[
        ("pages/C#%3A a%7Cb%3F.md", "- x"),
        ("pages/50%25 done.md", "- y"),
        ("pages/%63on.md", "- z"),
        ("pages/a:b.md", "- hand made"),
    ]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{diags:?}");
    for name in ["C#: a|b?", "50% done", "con", "a:b"] {
        page(&g, name);
    }
}

#[test]
fn invalid_name_from_file_is_skipped() {
    let notes = Notes::new(&[("pages/%2E%2E.md", "- x"), ("pages/ok.md", "- y")]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::InvalidPageName]);
    assert_eq!(names(&g, g.pages().map(|(id, _)| id)), ["ok"]);
}

#[test]
fn different_casing_resolves_to_one_page() {
    let notes = Notes::new(&[
        ("pages/Auth.md", "- x"),
        ("journals/2026-01-01.md", "- [[auth]] and [[AUTH]]"),
    ]);
    let (g, _) = notes.load();
    let auth = page(&g, "auth");
    assert_eq!(g.page(auth).name, "Auth");
    assert_eq!(g.backlinks(auth).len(), 1);
    assert_eq!(g.pages().count(), 2);
}

#[test]
fn nfd_file_name_matches_nfc_link() {
    let notes = Notes::new(&[
        ("pages/Zu\u{308}rich.md", "- x"),
        ("journals/2026-01-01.md", "- [[Z\u{fc}rich]]"),
    ]);
    let (g, _) = notes.load();
    assert_eq!(g.pages().count(), 2);
    let z = page(&g, "Z\u{fc}rich");
    assert!(!g.page(z).is_implicit());
    assert_eq!(g.backlinks(z).len(), 1);
}

#[test]
fn implicit_page_display_name_is_first_mention() {
    let notes = Notes::new(&[
        ("journals/2026-02-01.md", "- [[redis]]"),
        ("journals/2026-01-01.md", "- [[Redis]]"),
    ]);
    let (g, _) = notes.load();
    assert_eq!(g.page(page(&g, "redis")).name, "Redis");
}

// Needs a case-sensitive file system to create both files.
#[cfg(target_os = "linux")]
#[test]
fn case_collision() {
    let notes = Notes::new(&[("pages/auth.md", "- lower"), ("pages/Auth.md", "- upper")]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::CollidingFiles]);
    assert!(
        diags[0].message.contains("pages/Auth.md") && diags[0].message.contains("pages/auth.md")
    );
    let auth = page(&g, "auth");
    assert_eq!(g.page(auth).file, Some(PathBuf::from("pages/Auth.md")));
    assert_eq!(contents(&g, g.page(auth).roots.clone()), ["upper"]);
    assert_eq!(g.blocks().count(), 1);
}

#[test]
fn invalid_utf8_is_reported_and_skipped() {
    let notes = Notes::new(&[("pages/ok.md", "- fine")]);
    notes.write("pages/broken.md", b"- \xff\xfe");
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::InvalidUtf8]);
    assert_eq!(diags[0].file, PathBuf::from("pages/broken.md"));
    assert_eq!(names(&g, g.pages().map(|(id, _)| id)), ["ok"]);
}

#[test]
fn crlf_parses_like_lf() {
    let crlf = Notes::new(&[("pages/a.md", "- a\r\n  - b [[x]]\r\n  cont\r\n- c\r\n")]);
    let lf = Notes::new(&[("pages/a.md", "- a\n  - b [[x]]\n  cont\n- c\n")]);
    let (g1, _) = crlf.load();
    let (g2, _) = lf.load();
    let dump = |g: &hermetic::Graph| {
        g.blocks()
            .map(|(_, b)| (b.content.clone(), b.parent, b.refs.len()))
            .collect::<Vec<_>>()
    };
    assert_eq!(dump(&g1), dump(&g2));
}

#[test]
fn scan_order_is_journals_by_date_then_pages_by_path() {
    let notes = Notes::new(&[
        ("pages/b.md", "- b"),
        ("pages/a/z.md", "- az"),
        ("journals/2026-01-02.md", "- j2"),
        ("journals/2025-12-31.md", "- j1"),
        ("pages/a.md", "- a"),
    ]);
    let (g, _) = notes.load();
    let order: Vec<String> = g.blocks().map(|(_, b)| b.content.clone()).collect();
    assert_eq!(order, ["j1", "j2", "a", "az", "b"]);
}

#[test]
fn home_page_source() {
    let notes = Notes::new(&[
        ("index.md", "- Start at [[auth]]"),
        ("pages/auth.md", "- x"),
    ]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{diags:?}");
    let index = g.page(page(&g, "index"));
    assert_eq!(index.kind, PageKind::Regular);
    assert_eq!(index.file, Some(PathBuf::from("index.md")));
    // The home page takes part in the graph like any page.
    assert_eq!(g.backlinks(page(&g, "auth")).len(), 1);
}

#[test]
fn root_index_and_pages_index() {
    let notes = Notes::new(&[("index.md", "- home"), ("pages/index.md", "- other")]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::ReservedName]);
    assert_eq!(diags[0].file, PathBuf::from("pages/index.md"));
    let index = g.page(page(&g, "index"));
    assert_eq!(index.file, Some(PathBuf::from("index.md")));
    assert_eq!(contents(&g, index.roots.clone()), ["home"]);
    let other = g.page(page(&g, "index (in pages)"));
    assert_eq!(other.file, Some(PathBuf::from("pages/index.md")));
    assert_eq!(contents(&g, other.roots.clone()), ["other"]);
}

#[test]
fn pages_index_alone() {
    let notes = Notes::new(&[
        ("pages/index.md", "- other"),
        (
            "journals/2026-10-12.md",
            "- [[index]] and [file](../pages/index.md)",
        ),
    ]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::ReservedName]);
    let other = page(&g, "index (in pages)");
    assert_eq!(g.page(other).file, Some(PathBuf::from("pages/index.md")));
    // `[[index]]` means the home page, which has no file here; a link to the file
    // reaches the file's own page.
    assert!(g.page(page(&g, "index")).is_implicit());
    assert_eq!(g.backlinks(other).len(), 1);
}

#[test]
fn other_root_files_are_still_ignored() {
    let notes = Notes::new(&[("README.md", "- [[x]]"), ("notes.md", "- [[y]]")]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty());
    assert_eq!(g.pages().count(), 0);
}
