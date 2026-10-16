//! The examples in `hermetic help` behave as the reference says.

mod common;

use common::*;
use hermetic::graph::RefTarget;
use hermetic::help;
use hermetic::parse::{BlockKind, RawRefKind, TaskState};

/// The `syntax` section, with each example's 8-space indentation removed so that
/// snippets can be matched as the note text they stand for.
fn syntax_examples() -> String {
    help::topic("syntax")
        .unwrap()
        .lines()
        .map(|l| l.strip_prefix("        ").unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Checks that the reference shows `snippet` verbatim.
fn example(snippet: &str) -> &str {
    assert!(
        syntax_examples().contains(snippet),
        "not an example in syntax.txt:\n{snippet}"
    );
    snippet
}

/// Loads the example `snippet` as the journal 2026-10-12, next to `pages/auth.md`
/// holding the block with id `ttl-bug`.
fn load(snippet: &str) -> (Notes, hermetic::Graph) {
    let snippet = example(snippet);
    let notes = Notes::new(&[
        ("journals/2026-10-12.md", snippet),
        ("pages/auth.md", "- TTL defaults were wrong\n  id:: ttl-bug"),
    ]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{snippet}: {diags:?}");
    (notes, g)
}

/// The pages the journal's single block links to, by kind.
fn refs(snippet: &str) -> Vec<(RawRefKind, String)> {
    let (_notes, g) = load(snippet);
    let (_, b) = g
        .blocks()
        .find(|(_, b)| g.page(b.page).name == "2026-10-12")
        .unwrap();
    b.refs
        .iter()
        .map(|r| {
            let target = match r.target {
                RefTarget::Page(p) => g.page(p).name.clone(),
                RefTarget::Block(b) => format!("(({}))", g.block(b).id.as_deref().unwrap()),
                RefTarget::Attachment(a) => g.attachment(a).to_string_lossy().into_owned(),
                RefTarget::Unresolved => "unresolved".to_owned(),
            };
            (r.kind, target)
        })
        .collect()
}

#[test]
fn references() {
    use RawRefKind::*;
    let cases: &[(&str, &[(RawRefKind, &str)])] = &[
        ("- met with Ana about [[auth]]", &[(Page, "auth")]),
        (
            "- see [[Project X/Auth Flow]] for the namespaced page",
            &[(Page, "Project X/Auth Flow")],
        ),
        (
            "- follow up on [[2026-10-01]], the journal of that day",
            &[(Page, "2026-10-01")],
        ),
        ("- fixed the bug #backend", &[(Tag, "backend")]),
        ("- kicked off #[[Project X]]", &[(Tag, "Project X")]),
        (
            "- details in [the auth notes](../pages/auth.md)",
            &[(MdLink, "auth")],
        ),
        ("- root cause: ((ttl-bug))", &[(Block, "((ttl-bug))")]),
        ("- write `[[auth]]` to link a page", &[]),
        ("- \\[[auth]] is shown literally", &[]),
    ];
    for (snippet, expected) in cases {
        let expected: Vec<_> = expected.iter().map(|(k, t)| (*k, t.to_string())).collect();
        assert_eq!(refs(snippet), expected, "{snippet}");
    }
}

#[test]
fn links_create_pages() {
    let (_notes, g) = load("- follow up on [[2026-10-01]], the journal of that day");
    let day = g.page(page(&g, "2026-10-01"));
    assert!(day.is_journal() && day.is_implicit());

    let (_notes, g) = load("- see [[Project X/Auth Flow]] for the namespaced page");
    let flow = page(&g, "Project X/Auth Flow");
    assert!(g.page(flow).is_implicit());
    assert_eq!(g.page(flow).ns_parent, Some(page(&g, "Project X")));
}

#[test]
fn blocks_and_nesting() {
    let (_notes, g) = load(
        "- standup\n  - shipped token refresh\n    more detail about the refresh\n  - reviewed the PR",
    );
    let standup = block(&g, "standup");
    let shipped = block(&g, "shipped token refresh\nmore detail about the refresh");
    let reviewed = block(&g, "reviewed the PR");
    assert_eq!(g.block(standup).children, vec![shipped, reviewed]);
}

#[test]
fn prose_becomes_a_block() {
    // The reference says so in prose; there is no example to quote.
    let notes = Notes::new(&[("pages/p.md", "A paragraph.\n\nAnother one.")]);
    let (g, _) = notes.load();
    let kinds: Vec<_> = g.blocks().map(|(_, b)| b.kind).collect();
    assert_eq!(kinds, vec![BlockKind::Prose, BlockKind::Prose]);
}

#[test]
fn properties_ids_and_tasks() {
    let (_notes, g) = load("- decided to use JWT\n  status:: accepted");
    let b = g.block(block(&g, "decided to use JWT"));
    assert_eq!(b.prop("status"), Some("accepted"));

    let notes = Notes::new(&[(
        "pages/a.md",
        example("- TTL defaults were wrong\n  id:: ttl-bug"),
    )]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(
        g.find_block("ttl-bug"),
        Some(block(&g, "TTL defaults were wrong"))
    );

    let (_notes, g) = load(
        "- TODO rotate signing keys\n- DOING write the migration\n- DONE drop the legacy endpoint",
    );
    let states: Vec<_> = [
        "rotate signing keys",
        "write the migration",
        "drop the legacy endpoint",
    ]
    .iter()
    .map(|c| g.block(block(&g, c)).task)
    .collect();
    assert_eq!(
        states,
        vec![
            Some(TaskState::Todo),
            Some(TaskState::Doing),
            Some(TaskState::Done)
        ]
    );
    assert_eq!(g.open_tasks().len(), 2);
}

/// The worked example at the end of the `pages` section.
#[test]
fn worked_example() {
    let pages = help::topic("pages").unwrap();
    for (path, content) in help::EXAMPLE {
        for line in content.lines() {
            assert!(pages.contains(&format!("    {line}\n")), "{path}: {line}");
        }
    }

    let notes = Notes::new(help::EXAMPLE);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{diags:?}");
    let auth = page(&g, "auth");
    let redis = page(&g, "redis");
    let index = page(&g, "index");
    let journal = page(&g, "2026-10-12");
    let shipped = block(
        &g,
        "[[auth]] token refresh shipped, sessions now in [[redis]]",
    );
    let todo = block(&g, "rotate signing keys");
    let done = block(&g, "move sessions to [[redis]]");

    // auth: the journal block with breadcrumb and child, then the block from index.
    let groups = g.linked_references(auth);
    let sources: Vec<_> = groups.iter().map(|grp| grp.page).collect();
    assert_eq!(sources, vec![journal, index]);
    let r = &groups[0].refs[0];
    assert_eq!(r.block, shipped);
    assert_eq!(contents(&g, r.breadcrumb.clone()), vec!["standup"]);
    assert_eq!(g.block(shipped).children, vec![todo]);
    assert_eq!(g.open_page_tasks(auth), vec![todo]);
    assert!(g.page_tasks(auth).contains(&done));

    // Pages without a file.
    for name in ["redis", "Project X", "backend"] {
        assert!(g.page(page(&g, name)).is_implicit(), "{name}");
    }
    let redis_sources: Vec<_> = g
        .linked_references(redis)
        .iter()
        .map(|grp| grp.page)
        .collect();
    assert_eq!(redis_sources, vec![journal, auth]);
    assert_eq!(g.open_page_tasks(redis), vec![todo]);

    // Related pages.
    let related = |p| names(&g, g.related(p).iter().map(|r| r.page).collect::<Vec<_>>());
    assert_eq!(related(redis), vec!["auth"]);
    let mut auth_related = related(auth);
    auth_related.sort();
    assert_eq!(auth_related, vec!["Project X", "redis"]);

    // The home page.
    let (_holder, out, _) = build_site(notes.path());
    let home = std::fs::read_to_string(out.join("index.html")).unwrap();
    assert!(home.contains("Start at"), "{home}");
}

#[test]
fn web_links() {
    let (notes, _g) = load("- the book is at https://doc.rust-lang.org/book/.");
    let (_holder, out, _) = build_site(notes.path());
    let html = std::fs::read_to_string(out.join("journals/2026-10-12.html")).unwrap();
    assert!(
        html.contains(
            r#"<a href="https://doc.rust-lang.org/book/">https://doc.rust-lang.org/book/</a>."#
        ),
        "{html}"
    );
}

#[test]
fn fits_a_terminal() {
    let reference = help::reference();
    for line in reference.lines() {
        assert!(line.chars().count() <= 80, "too long: {line:?}");
    }
    assert!(!reference.contains('\x1b'));
    assert!(!reference.contains('\t'));
}

#[test]
fn page_properties_and_aliases() {
    let snippet = example("alias:: authentication, [[login]]\n\n- how we sign users in");
    let notes = Notes::new(&[
        ("pages/auth.md", snippet),
        (
            "journals/2026-10-12.md",
            "- fixed [[login]] for #authentication",
        ),
    ]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{diags:?}");
    let auth = page(&g, "auth");
    assert_eq!(g.page_props(auth)[0].key, "alias");
    let b = block(&g, "fixed [[login]] for #authentication");
    assert!(
        g.block(b)
            .refs
            .iter()
            .all(|r| r.target == RefTarget::Page(auth))
    );
    assert!(g.find_page("login").is_none());
}
