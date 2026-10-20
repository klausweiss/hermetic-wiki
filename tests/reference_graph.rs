//! Scenarios from the `reference-graph` and `block-model` specs, at graph level.

mod common;

use common::*;
use hermetic::DiagnosticKind;
use hermetic::graph::{PageKind, RefLocation, RefTarget};
use hermetic::parse::RawRefKind;

#[test]
fn link_in_a_journal_block() {
    let notes = Notes::new(&[("journals/2026-10-12.md", "- met with X about [[auth]]")]);
    let (g, _) = notes.load();
    let b = block(&g, "met with X about [[auth]]");
    assert_eq!(g.block(b).refs[0].target, RefTarget::Page(page(&g, "auth")));
}

#[test]
fn tags_and_multi_word_tags() {
    let notes = Notes::new(&[(
        "journals/2026-10-12.md",
        "- fixed the bug #backend.\n- #[[Project X]]",
    )]);
    let (g, _) = notes.load();
    let backend = page(&g, "backend");
    let px = page(&g, "Project X");
    assert_eq!(g.backlinks(backend).len(), 1);
    assert_eq!(g.backlinks(px).len(), 1);
    assert_eq!(
        g.block(g.backlinks(backend)[0]).refs[0].kind,
        RawRefKind::Tag
    );
}

#[test]
fn relative_markdown_link_from_a_journal() {
    let notes = Notes::new(&[
        (
            "journals/2026-10-12.md",
            "- [the auth notes](../pages/auth.md)",
        ),
        ("pages/auth.md", "- x"),
    ]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{diags:?}");
    let auth = page(&g, "auth");
    assert_eq!(g.backlinks(auth).len(), 1);
    assert_eq!(
        g.block(g.backlinks(auth)[0]).refs[0].kind,
        RawRefKind::MdLink
    );
}

#[test]
fn markdown_link_with_encoded_characters() {
    let notes = Notes::new(&[
        (
            "pages/links.md",
            "- [x](Project%20X/Auth%20Flow.md#setup)\n- [y](/pages/C%2523.md)",
        ),
        ("pages/Project X/Auth Flow.md", "- x"),
        ("pages/C%23.md", "- c sharp"),
    ]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(g.backlinks(page(&g, "Project X/Auth Flow")).len(), 1);
    assert_eq!(g.backlinks(page(&g, "C#")).len(), 1);
}

#[test]
fn markdown_link_to_a_missing_file() {
    let notes = Notes::new(&[("pages/a.md", "- [x](nope.md)")]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::BrokenLink]);
    assert_eq!(diags[0].line, Some(1));
    assert!(g.find_page("nope").is_none());
    let b = block(&g, "[x](nope.md)");
    assert_eq!(g.block(b).refs[0].target, RefTarget::Unresolved);
}

#[test]
fn markdown_link_to_implicit_page_is_broken() {
    let notes = Notes::new(&[("pages/a.md", "- [[ghost]] [x](ghost.md)")]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::BrokenLink]);
    assert!(g.page(page(&g, "ghost")).is_implicit());
}

#[test]
fn markdown_link_escaping_the_notes_root() {
    let notes = Notes::new(&[("pages/a.md", "- [x](../../secrets.md)\n- [y](../README.md)")]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::LinkOutsideRoot; 2]);
    assert!(g.blocks().all(|(_, b)| b.refs.is_empty()));
}

#[test]
fn external_and_image_links_are_ignored() {
    let notes = Notes::new(&[(
        "pages/a.md",
        "- [site](https://example.com/a.md) ![diagram](pic.md)",
    )]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty());
    assert!(g.blocks().all(|(_, b)| b.refs.is_empty()));
}

#[test]
fn attachments_resolve_against_their_note() {
    let notes = Notes::new(&[
        (
            "journals/2026-10-17.md",
            "- ![shot](../assets/shot.png) [[auth]]",
        ),
        (
            "pages/Project X/Auth.md",
            "- ![seq](../../assets/seq%20one.png) [spec](/assets/spec.pdf#page=2)",
        ),
        ("pages/auth.md", "- ![flow](auth-flow.png)"),
    ]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{diags:?}");
    let attachments = |text: &str| -> Vec<String> {
        g.block(block(&g, text))
            .refs
            .iter()
            .filter_map(|r| match r.target {
                RefTarget::Attachment(a) => {
                    assert_eq!(r.kind, RawRefKind::Attachment);
                    Some(g.attachment(a).to_string_lossy().into_owned())
                }
                _ => None,
            })
            .collect()
    };
    assert_eq!(
        attachments("![shot](../assets/shot.png) [[auth]]"),
        ["assets/shot.png"]
    );
    assert_eq!(
        attachments("![seq](../../assets/seq%20one.png) [spec](/assets/spec.pdf#page=2)"),
        ["assets/seq one.png", "assets/spec.pdf"]
    );
    assert_eq!(
        attachments("![flow](auth-flow.png)"),
        ["pages/auth-flow.png"]
    );
}

#[test]
fn attachments_are_not_page_references() {
    let notes = Notes::new(&[
        (
            "journals/2026-10-17.md",
            "- ![x](../pages/auth.png) [[auth]]",
        ),
        ("pages/auth.md", "- ![y](../assets/y.png)"),
    ]);
    let (g, _) = notes.load();
    let auth = page(&g, "auth");
    assert_eq!(g.backlinks(auth).len(), 1);
    assert!(g.related(auth).is_empty());
}

#[test]
fn attachments_outside_the_notes_are_left_alone() {
    let notes = Notes::new(&[(
        "pages/a.md",
        "- ![x](../../outside.png) ![y](../) ![z](https://example.com/z.png)",
    )]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{diags:?}");
    assert!(g.blocks().all(|(_, b)| b.refs.is_empty()));
}

#[test]
fn resolved_block_ref() {
    let notes = Notes::new(&[
        ("pages/redis.md", "- the TTL was wrong\n  id:: auth-ttl-bug"),
        ("journals/2026-10-12.md", "- see ((auth-ttl-bug))"),
    ]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{diags:?}");
    let target = block(&g, "the TTL was wrong");
    let source = block(&g, "see ((auth-ttl-bug))");
    assert_eq!(g.block(source).refs[0].target, RefTarget::Block(target));
    let redis = page(&g, "redis");
    assert_eq!(
        g.direct_pages(source).into_iter().collect::<Vec<_>>(),
        [redis]
    );
    assert_eq!(g.backlinks(redis), [source]);
}

#[test]
fn duplicate_ids() {
    let notes = Notes::new(&[
        ("pages/a.md", "- first\n  id:: x"),
        ("pages/b.md", "- second\n  id:: x"),
        ("pages/c.md", "- ((x))"),
    ]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::DuplicateBlockId]);
    assert!(diags[0].message.contains("pages/a.md:1") && diags[0].message.contains("pages/b.md:1"));
    let r = &g.block(block(&g, "((x))")).refs[0];
    assert_eq!(r.target, RefTarget::Block(block(&g, "first")));
}

#[test]
fn dangling_block_ref() {
    let notes = Notes::new(&[("journals/2026-10-12.md", "- a\n\n\n- ((missing))")]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::DanglingBlockRef]);
    assert_eq!(diags[0].line, Some(4));
    assert_eq!(
        g.block(block(&g, "((missing))")).refs[0].target,
        RefTarget::Unresolved
    );
}

#[test]
fn invalid_block_id() {
    let notes = Notes::new(&[("pages/a.md", "- x\n  id:: two words")]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::InvalidBlockId]);
    assert_eq!(g.block(block(&g, "x")).id, None);
}

#[test]
fn page_created_on_demand() {
    let notes = Notes::new(&[("journals/2026-10-12.md", "- trying [[kafka]]")]);
    let (g, _) = notes.load();
    let kafka = page(&g, "kafka");
    assert!(g.page(kafka).is_implicit());
    assert!(g.page(kafka).roots.is_empty());
    let groups = g.linked_references(kafka);
    assert_eq!(groups.len(), 1);
    assert_eq!(
        contents(&g, groups[0].refs.iter().map(|r| r.block)),
        ["trying [[kafka]]"]
    );
}

#[test]
fn link_to_a_day() {
    let notes = Notes::new(&[("pages/a.md", "- see [[2026-10-01]]")]);
    let (g, _) = notes.load();
    let day = g.page(page(&g, "2026-10-01"));
    assert!(day.is_implicit());
    assert!(matches!(day.kind, PageKind::Journal(_)));
}

#[test]
fn path_traversal_attempt() {
    let notes = Notes::new(&[("pages/a.md", "- [[../../etc/passwd]] [[a//b]] [[/a]]")]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::InvalidPageName; 3]);
    assert_eq!(g.pages().count(), 1);
    assert!(g.blocks().all(|(_, b)| b.refs.is_empty()));
}

#[test]
fn references_in_property_values() {
    let notes = Notes::new(&[("pages/a.md", "- decision\n  related:: [[auth]]")]);
    let (g, _) = notes.load();
    let b = g.block(block(&g, "decision"));
    assert_eq!(b.refs[0].location, RefLocation::Property(0));
    assert_eq!(g.backlinks(page(&g, "auth")).len(), 1);
}

#[test]
fn child_inherits_parent_reference() {
    let notes = Notes::new(&[(
        "journals/2026-10-12.md",
        "- meeting about [[auth]]\n  - TODO rotate signing keys",
    )]);
    let (g, _) = notes.load();
    let auth = page(&g, "auth");
    let child = block(&g, "rotate signing keys");
    assert!(g.direct_pages(child).is_empty());
    assert!(g.inherited_pages(child).contains(&auth));
    assert_eq!(g.inherited_targets(child), [RefTarget::Page(auth)]);
}

#[test]
fn breadcrumb_and_subtree() {
    let notes = Notes::new(&[(
        "journals/2026-10-12.md",
        "- standup\n  - [[auth]] token refresh shipped\n    - details",
    )]);
    let (g, _) = notes.load();
    let groups = g.linked_references(page(&g, "auth"));
    let r = &groups[0].refs[0];
    assert_eq!(g.block(r.block).content, "[[auth]] token refresh shipped");
    assert_eq!(contents(&g, r.breadcrumb.clone()), ["standup"]);
    assert_eq!(contents(&g, g.block(r.block).children.clone()), ["details"]);
}

#[test]
fn nested_references_collapse() {
    let notes = Notes::new(&[(
        "journals/2026-10-12.md",
        "- [[auth]] outer\n  - inner [[auth]]\n- [[auth]] sibling",
    )]);
    let (g, _) = notes.load();
    let groups = g.linked_references(page(&g, "auth"));
    assert_eq!(
        contents(&g, groups[0].refs.iter().map(|r| r.block)),
        ["[[auth]] outer", "[[auth]] sibling"]
    );
}

#[test]
fn self_references_excluded() {
    let notes = Notes::new(&[("pages/auth.md", "- see [[auth]]")]);
    let (g, _) = notes.load();
    assert!(g.linked_references(page(&g, "auth")).is_empty());
}

#[test]
fn linked_reference_ordering() {
    let notes = Notes::new(&[
        ("journals/2026-09-30.md", "- [[auth]] old"),
        ("journals/2026-10-12.md", "- [[auth]] new\n- again [[auth]]"),
        ("pages/redis.md", "- [[auth]] r"),
        ("pages/architecture.md", "- [[auth]] a"),
        ("pages/auth.md", "- the page"),
    ]);
    let (g, _) = notes.load();
    let groups = g.linked_references(page(&g, "auth"));
    assert_eq!(
        names(&g, groups.iter().map(|gr| gr.page)),
        ["2026-10-12", "2026-09-30", "architecture", "redis"]
    );
    assert_eq!(
        contents(&g, groups[0].refs.iter().map(|r| r.block)),
        ["[[auth]] new", "again [[auth]]"]
    );
}

#[test]
fn tasks_under_a_referencing_block() {
    let notes = Notes::new(&[
        (
            "journals/2026-10-12.md",
            "- meeting about [[auth]]\n  - TODO rotate signing keys\n  - DONE drop legacy endpoint\n- TODO unrelated",
        ),
        ("pages/auth.md", "- DOING write ADR"),
    ]);
    let (g, _) = notes.load();
    let auth = page(&g, "auth");
    assert_eq!(
        contents(&g, g.page_tasks(auth).iter().copied()),
        ["rotate signing keys", "drop legacy endpoint", "write ADR"]
    );
    assert_eq!(
        contents(&g, g.open_page_tasks(auth)),
        ["rotate signing keys", "write ADR"]
    );
}

#[test]
fn tasks_referencing_a_block_count_for_its_page() {
    let notes = Notes::new(&[
        ("pages/redis.md", "- ttl\n  id:: ttl"),
        ("journals/2026-10-12.md", "- TODO fix ((ttl))"),
    ]);
    let (g, _) = notes.load();
    assert_eq!(
        contents(&g, g.open_page_tasks(page(&g, "redis"))),
        ["fix ((ttl))"]
    );
}

#[test]
fn global_open_tasks() {
    let notes = Notes::new(&[
        ("journals/2026-10-12.md", "- TODO a\n- DONE b"),
        ("pages/x.md", "- DOING c\n- d"),
    ]);
    let (g, _) = notes.load();
    assert_eq!(contents(&g, g.open_tasks()), ["a", "c"]);
}

#[test]
fn namespace_children() {
    let notes = Notes::new(&[
        ("pages/Project X/Billing.md", "- b"),
        ("pages/Project X/Auth Flow.md", "- a"),
        ("pages/Project X/Auth Flow/Tokens.md", "- t"),
    ]);
    let (g, _) = notes.load();
    let px = page(&g, "Project X");
    let leaves: Vec<&str> = g
        .namespace_children(px)
        .iter()
        .map(|&c| g.page(c).leaf_name())
        .collect();
    assert_eq!(leaves, ["Auth Flow", "Billing"]);
    assert_eq!(
        names(
            &g,
            g.namespace_children(page(&g, "Project X/Auth Flow"))
                .iter()
                .copied()
        ),
        ["Project X/Auth Flow/Tokens"]
    );
}

#[test]
fn namespace_prefix_of_a_link() {
    let notes = Notes::new(&[("journals/2026-10-12.md", "- [[a/b/c]]")]);
    let (g, _) = notes.load();
    let b = page(&g, "a/b");
    assert!(g.page(b).is_implicit());
    assert_eq!(g.page(page(&g, "a/b/c")).ns_parent, Some(b));
    assert_eq!(g.page(b).ns_parent, Some(page(&g, "a")));
}

#[test]
fn two_pages_in_one_block_are_co_mentioned() {
    let notes = Notes::new(&[(
        "journals/2026-10-12.md",
        "- [[claude]] helped me design a proposal for [[hermetic]]",
    )]);
    let (g, _) = notes.load();
    let (claude, hermetic) = (page(&g, "claude"), page(&g, "hermetic"));
    let related = g.related(claude);
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].page, hermetic);
    assert_eq!(related[0].weight(), 1);
    assert_eq!(
        related[0].recency.map(|d| d.to_string()).as_deref(),
        Some("2026-10-12")
    );
    assert_eq!(g.related(hermetic)[0].page, claude);
}

#[test]
fn repeated_mentions_in_one_block_count_once() {
    let notes = Notes::new(&[("pages/x.md", "- [[a]] and [[a]] with [[b]]")]);
    let (g, _) = notes.load();
    let a = page(&g, "a");
    let related = g.related(a);
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].page, page(&g, "b"));
    assert_eq!(related[0].weight(), 1);
    assert_eq!(related[0].recency, None);
}

#[test]
fn child_blocks_do_not_inherit_co_mentions() {
    let notes = Notes::new(&[(
        "journals/2026-10-12.md",
        "- [[claude]] helped with [[hermetic]]\n  - it suggested [[rust]]",
    )]);
    let (g, _) = notes.load();
    assert!(g.related(page(&g, "rust")).is_empty());
}

#[test]
fn containing_page_does_not_count() {
    let notes = Notes::new(&[("pages/hermetic.md", "- [[claude]] only")]);
    let (g, _) = notes.load();
    assert!(g.related(page(&g, "claude")).is_empty());
    assert!(g.related(page(&g, "hermetic")).is_empty());
}

#[test]
fn co_mentions_include_markdown_links_and_block_refs() {
    let notes = Notes::new(&[
        ("pages/redis.md", "- ttl\n  id:: ttl"),
        ("pages/auth.md", "- x"),
        (
            "journals/2026-10-12.md",
            "- [a](../pages/auth.md) ((ttl)) #ops",
        ),
    ]);
    let (g, _) = notes.load();
    assert_eq!(
        names(&g, g.related(page(&g, "auth")).iter().map(|r| r.page)),
        ["ops", "redis"]
    );
}

#[test]
fn co_mention_ordering() {
    let notes = Notes::new(&[
        (
            "journals/2026-01-01.md",
            "- [[claude]] [[hermetic]]\n- [[claude]] [[hermetic]]\n- [[claude]] [[zed]]",
        ),
        (
            "journals/2026-03-01.md",
            "- [[claude]] [[hermetic]]\n- [[claude]] [[rust]]",
        ),
        ("pages/notes.md", "- [[claude]] [[Alpha]]"),
    ]);
    let (g, _) = notes.load();
    let related = g.related(page(&g, "claude"));
    assert_eq!(
        names(&g, related.iter().map(|r| r.page)),
        // weight 3 first; then weight 1 by recency (newest first, undated last), then name
        ["hermetic", "rust", "zed", "Alpha"]
    );
    assert_eq!(related[0].weight(), 3);
}

#[test]
fn markdown_link_to_the_home_page() {
    let notes = Notes::new(&[
        ("index.md", "- home"),
        (
            "pages/auth.md",
            "- [home](../index.md) and [readme](../README.md)",
        ),
        ("journals/2026-10-12.md", "- [home](/index.md)"),
    ]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::LinkOutsideRoot]);
    assert!(diags[0].message.contains("README.md"));
    assert_eq!(g.backlinks(page(&g, "index")).len(), 2);
}

// Page properties and aliases.

fn prop_pairs(g: &hermetic::Graph, name: &str) -> Vec<(String, String)> {
    g.page_props(page(g, name))
        .iter()
        .map(|p| (p.key.clone(), p.value.clone()))
        .collect()
}

#[test]
fn page_properties_from_lines_at_the_top() {
    let notes = Notes::new(&[(
        "pages/auth.md",
        "alias:: authentication\ntags:: security\n\n- first note",
    )]);
    let (g, _) = notes.load();
    assert_eq!(
        prop_pairs(&g, "auth"),
        [
            ("alias".to_owned(), "authentication".to_owned()),
            ("tags".to_owned(), "security".to_owned())
        ]
    );
}

#[test]
fn page_properties_from_a_properties_only_first_bullet() {
    let notes = Notes::new(&[("pages/auth.md", "- alias:: authentication\n- first note")]);
    let (g, _) = notes.load();
    assert_eq!(
        prop_pairs(&g, "auth"),
        [("alias".to_owned(), "authentication".to_owned())]
    );
}

#[test]
fn no_page_properties_when_the_first_block_has_content() {
    let notes = Notes::new(&[("pages/auth.md", "- decided to use JWT\n  status:: accepted")]);
    let (g, _) = notes.load();
    assert!(prop_pairs(&g, "auth").is_empty());
    let b = block(&g, "decided to use JWT");
    assert_eq!(g.block(b).prop("status"), Some("accepted"));
}

#[test]
fn link_through_an_alias() {
    let notes = Notes::new(&[
        (
            "pages/auth.md",
            "alias:: authentication, [[login]]\n\n- notes",
        ),
        (
            "journals/2026-10-12.md",
            "- fixed [[authentication]] and #login\n  - TODO rotate keys",
        ),
    ]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{diags:?}");
    let auth = page(&g, "auth");
    assert_eq!(g.page(auth).aliases, ["authentication", "login"]);
    let b = block(&g, "fixed [[authentication]] and #login");
    assert_eq!(
        g.block(b).refs.iter().map(|r| r.target).collect::<Vec<_>>(),
        [RefTarget::Page(auth); 2]
    );
    assert_eq!(g.direct_pages(b).into_iter().collect::<Vec<_>>(), [auth]);
    let groups = g.linked_references(auth);
    assert_eq!(groups.len(), 1);
    assert_eq!(
        contents(&g, groups[0].refs.iter().map(|r| r.block)),
        ["fixed [[authentication]] and #login"]
    );
    assert_eq!(contents(&g, g.open_page_tasks(auth)), ["rotate keys"]);
    assert!(g.find_page("authentication").is_none());
    assert!(g.find_page("login").is_none());
}

#[test]
fn aliases_are_case_insensitive() {
    let notes = Notes::new(&[
        ("pages/auth.md", "- alias:: Authentication"),
        ("pages/notes.md", "- see [[authentication]]"),
    ]);
    let (g, _) = notes.load();
    let b = block(&g, "see [[authentication]]");
    assert_eq!(g.block(b).refs[0].target, RefTarget::Page(page(&g, "auth")));
}

#[test]
fn aliased_references_count_as_co_mentions() {
    let notes = Notes::new(&[
        ("pages/auth.md", "- alias:: login"),
        ("journals/2026-10-12.md", "- [[login]] uses [[redis]]"),
    ]);
    let (g, _) = notes.load();
    let related: Vec<_> = g
        .related(page(&g, "redis"))
        .iter()
        .map(|r| r.page)
        .collect();
    assert_eq!(names(&g, related), ["auth"]);
}

#[test]
fn alias_property_is_not_a_reference() {
    let notes = Notes::new(&[("pages/auth.md", "- alias:: [[login]], #sso")]);
    let (g, _) = notes.load();
    let auth = page(&g, "auth");
    let first = g.page(auth).roots[0];
    assert!(g.block(first).refs.is_empty());
    assert!(g.linked_references(auth).is_empty());
    assert!(g.find_page("login").is_none());
}

#[test]
fn alias_of_an_existing_page() {
    let notes = Notes::new(&[
        ("pages/auth.md", "- alias:: redis"),
        ("pages/redis.md", "- cache"),
        ("pages/notes.md", "- [[redis]]"),
    ]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::InvalidAlias]);
    assert!(diags[0].message.contains("pages/redis.md"), "{diags:?}");
    assert!(g.page(page(&g, "auth")).aliases.is_empty());
    let b = block(&g, "[[redis]]");
    assert_eq!(
        g.block(b).refs[0].target,
        RefTarget::Page(page(&g, "redis"))
    );
}

#[test]
fn alias_claimed_twice() {
    let notes = Notes::new(&[
        ("pages/auth.md", "- alias:: sso"),
        ("pages/login.md", "- alias:: SSO"),
        ("pages/notes.md", "- [[sso]]"),
    ]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::InvalidAlias]);
    let d = &diags[0];
    assert_eq!(d.file, std::path::Path::new("pages/login.md"));
    assert!(d.message.contains("pages/auth.md"), "{d:?}");
    let b = block(&g, "[[sso]]");
    assert_eq!(g.block(b).refs[0].target, RefTarget::Page(page(&g, "auth")));
}

#[test]
fn date_alias() {
    let notes = Notes::new(&[
        ("pages/launch.md", "- alias:: 2026-10-01"),
        ("pages/notes.md", "- [[2026-10-01]]"),
    ]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::InvalidAlias]);
    let b = block(&g, "[[2026-10-01]]");
    let RefTarget::Page(day) = g.block(b).refs[0].target else {
        panic!("not a page")
    };
    assert!(matches!(g.page(day).kind, PageKind::Journal(_)));
}

#[test]
fn invalid_and_self_aliases() {
    let notes = Notes::new(&[("pages/auth.md", "- alias:: Auth, a//b, ../x")]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::InvalidAlias; 2]);
    assert!(g.page(page(&g, "auth")).aliases.is_empty());
}

#[test]
fn alias_of_a_namespace() {
    let notes = Notes::new(&[
        ("pages/auth.md", "- alias:: Project X"),
        ("pages/Project X/Billing.md", "- b"),
    ]);
    let (_, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::InvalidAlias]);
}

#[test]
fn alias_not_in_the_first_block() {
    let notes = Notes::new(&[
        ("pages/auth.md", "- intro\n- alias:: other"),
        ("pages/notes.md", "- [[other]]"),
    ]);
    let (g, _) = notes.load();
    assert!(g.page(page(&g, "other")).is_implicit());
}

#[test]
fn markdown_link_to_an_alias_name() {
    let notes = Notes::new(&[
        ("pages/auth.md", "- alias:: login"),
        ("pages/notes.md", "- [x](login.md)"),
    ]);
    let (g, diags) = notes.load();
    assert_eq!(kinds(&diags), [DiagnosticKind::BrokenLink]);
    let b = block(&g, "[x](login.md)");
    assert_eq!(g.block(b).refs[0].target, RefTarget::Unresolved);
}

#[test]
fn namespaced_alias() {
    let notes = Notes::new(&[
        ("pages/auth.md", "- alias:: Project X/Login"),
        ("pages/notes.md", "- [[Project X/Login]]"),
    ]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{diags:?}");
    let b = block(&g, "[[Project X/Login]]");
    assert_eq!(g.block(b).refs[0].target, RefTarget::Page(page(&g, "auth")));
    assert!(g.find_page("Project X/Login").is_none());
    let px = page(&g, "Project X");
    assert!(g.page(px).is_implicit());
    assert_eq!(g.namespace_listing(px), [("Login", page(&g, "auth"))]);
    assert!(g.namespace_children(px).is_empty());
    assert_eq!(g.page(page(&g, "auth")).ns_parent, None);
}

#[test]
fn namespaced_aliases_are_listed_with_children() {
    let notes = Notes::new(&[
        ("pages/alternatives.md", "- tools"),
        ("pages/alternatives/obsidian.md", "- o"),
        ("pages/emanote.md", "- alias:: alternatives/Emanote"),
        ("pages/quartz.md", "- alias:: alternatives/quartz"),
    ]);
    let (g, diags) = notes.load();
    assert!(diags.is_empty(), "{diags:?}");
    let alt = page(&g, "alternatives");
    assert_eq!(
        g.namespace_listing(alt),
        [
            ("Emanote", page(&g, "emanote")),
            ("obsidian", page(&g, "alternatives/obsidian")),
            ("quartz", page(&g, "quartz")),
        ]
    );
    assert_eq!(g.page(page(&g, "emanote")).ns_parent, None);
}

#[test]
fn namespaced_alias_that_is_also_a_prefix() {
    let notes = Notes::new(&[
        ("pages/auth.md", "- alias:: x/login"),
        ("pages/notes.md", "- [[x/login/sso]]"),
    ]);
    let (g, _) = notes.load();
    let login = page(&g, "x/login");
    assert!(g.page(login).is_implicit());
    assert_eq!(g.namespace_listing(page(&g, "x")), [("login", login)]);
}
