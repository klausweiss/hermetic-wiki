//! Scenarios from the `graph-view` spec.

mod common;

use common::*;
use hermetic::Graph;
use hermetic::graph::Overview;

/// The overview's edges by page name, each pair in name order.
fn edges(g: &Graph, o: &Overview) -> Vec<(String, String, u32)> {
    let name = |n: u32| g.page(o.nodes[n as usize]).name.clone();
    let mut out: Vec<_> = o
        .edges
        .iter()
        .map(|&(a, b, w)| {
            let (a, b) = (name(a), name(b));
            if a <= b { (a, b, w) } else { (b, a, w) }
        })
        .collect();
    out.sort();
    out
}

fn edge(a: &str, b: &str, w: u32) -> (String, String, u32) {
    (a.to_owned(), b.to_owned(), w)
}

fn mentions(g: &Graph, o: &Overview, name: &str) -> u32 {
    let p = page(g, name);
    o.mentions[o.nodes.iter().position(|&n| n == p).unwrap()]
}

#[test]
fn nodes_are_non_journal_pages() {
    let notes = Notes::new(&[
        ("pages/auth.md", "- uses [[redis]]"),
        ("pages/redis.md", "- x"),
        ("pages/scratch.md", "- nothing here"),
        ("journals/2026-10-12.md", "- [[kafka]] and [[2026-10-01]]"),
    ]);
    let (g, _) = notes.load();
    let o = g.overview();
    let mut nodes = names(&g, o.nodes.iter().copied());
    nodes.sort();
    assert_eq!(nodes, ["auth", "kafka", "redis", "scratch"]);
    assert!(g.page(page(&g, "kafka")).is_implicit());
}

#[test]
fn home_page_is_a_node_only_with_index_md() {
    let without = Notes::new(&[("pages/auth.md", "- see [[index]]")]);
    let (g, _) = without.load();
    assert_eq!(names(&g, g.overview().nodes), ["auth"]);

    let with = Notes::new(&[
        ("index.md", "- start at [[auth]]"),
        ("pages/auth.md", "- x"),
    ]);
    let (g, _) = with.load();
    let o = g.overview();
    let mut nodes = names(&g, o.nodes.iter().copied());
    nodes.sort();
    assert_eq!(nodes, ["auth", "index"]);
    assert_eq!(edges(&g, &o), [edge("auth", "index", 1)]);
}

#[test]
fn link_between_pages() {
    let notes = Notes::new(&[("pages/auth.md", "- uses [[redis]]")]);
    let (g, _) = notes.load();
    let o = g.overview();
    assert_eq!(edges(&g, &o), [edge("auth", "redis", 1)]);
}

#[test]
fn links_in_both_directions() {
    let notes = Notes::new(&[
        ("pages/auth.md", "- uses [[redis]]"),
        ("pages/redis.md", "- used by [[auth]]"),
    ]);
    let (g, _) = notes.load();
    let o = g.overview();
    assert_eq!(edges(&g, &o), [edge("auth", "redis", 2)]);
}

#[test]
fn co_mention_in_a_journal() {
    let notes = Notes::new(&[(
        "journals/2026-10-12.md",
        "- [[claude]] helped design [[hermetic]]",
    )]);
    let (g, _) = notes.load();
    let o = g.overview();
    assert_eq!(edges(&g, &o), [edge("claude", "hermetic", 1)]);
}

#[test]
fn link_and_co_mention_in_one_block_count_once() {
    // The block links `auth` to `redis` and `kafka`, and co-mentions those two.
    let notes = Notes::new(&[("pages/auth.md", "- [[redis]] or [[kafka]] or [[redis]]")]);
    let (g, _) = notes.load();
    let o = g.overview();
    assert_eq!(
        edges(&g, &o),
        [
            edge("auth", "kafka", 1),
            edge("auth", "redis", 1),
            edge("kafka", "redis", 1)
        ]
    );
}

#[test]
fn journal_reference_alone() {
    let notes = Notes::new(&[("journals/2026-10-12.md", "- [[kafka]] and [[2026-10-01]]")]);
    let (g, _) = notes.load();
    let o = g.overview();
    assert_eq!(names(&g, o.nodes.iter().copied()), ["kafka"]);
    assert!(o.edges.is_empty());
}

#[test]
fn inherited_references_and_self_links_make_no_edges() {
    let notes = Notes::new(&[
        (
            "pages/auth.md",
            "- about [[auth]]\n- [[redis]]\n  - child [[kafka]]",
        ),
        ("pages/kafka.md", "- x"),
    ]);
    let (g, _) = notes.load();
    let o = g.overview();
    // The child links `auth` (its page) to `kafka`, but not `redis` to `kafka`.
    assert_eq!(
        edges(&g, &o),
        [edge("auth", "kafka", 1), edge("auth", "redis", 1)]
    );
}

#[test]
fn namespace_children_and_aliases_are_connected() {
    let notes = Notes::new(&[
        ("pages/alternatives/obsidian.md", "- o"),
        ("pages/emanote.md", "- alias:: alternatives/emanote"),
        ("pages/quartz.md", "- alias:: alternatives/quartz\n- see [[alternatives]]"),
    ]);
    let (g, _) = notes.load();
    let o = g.overview();
    assert_eq!(
        edges(&g, &o),
        [
            edge("alternatives", "alternatives/obsidian", 1),
            edge("alternatives", "emanote", 1),
            edge("alternatives", "quartz", 2),
        ]
    );
}

#[test]
fn block_refs_connect_to_the_target_page() {
    let notes = Notes::new(&[
        ("pages/redis.md", "- the bug\n  id:: ttl-bug"),
        ("pages/auth.md", "- see ((ttl-bug))"),
    ]);
    let (g, _) = notes.load();
    let o = g.overview();
    assert_eq!(edges(&g, &o), [edge("auth", "redis", 1)]);
}

#[test]
fn node_size_counts_referencing_pages() {
    let mut files: Vec<(String, String)> = (1..=9)
        .map(|d| {
            (
                format!("journals/2026-10-0{d}.md"),
                "- [[auth]]\n- again [[auth]]".to_owned(),
            )
        })
        .collect();
    files.push(("journals/2026-10-10.md".into(), "- [[auth]]".into()));
    files.push(("pages/auth.md".into(), "- self [[auth]], [[redis]]".into()));
    files.push(("pages/redis.md".into(), "- x".into()));
    files.push(("pages/scratch.md".into(), "- x".into()));
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, c)| (p.as_str(), c.as_str()))
        .collect();
    let notes = Notes::new(&files);
    let (g, _) = notes.load();
    let o = g.overview();
    assert_eq!(mentions(&g, &o, "auth"), 10);
    assert_eq!(mentions(&g, &o, "redis"), 1);
    assert_eq!(mentions(&g, &o, "scratch"), 0);
}

fn read(out: &std::path::Path, rel: &str) -> String {
    std::fs::read_to_string(out.join(rel)).unwrap_or_else(|e| panic!("cannot read {rel}: {e}"))
}

#[test]
fn graph_page_exists() {
    let notes = Notes::new(&[("pages/auth.md", "- uses [[redis]]")]);
    let (_h, out, _) = build_site(notes.path());
    let html = read(&out, "graph.html");
    assert!(html.contains("<hermetic-graph>"));
    assert!(html.contains("<svg class=\"graph\" viewBox=\""));
    assert!(html.contains("<title>Graph · "));
}

#[test]
fn only_journals() {
    let notes = Notes::new(&[("journals/2026-10-12.md", "- nothing linked here")]);
    let (_h, out, _) = build_site(notes.path());
    let html = read(&out, "graph.html");
    assert!(!html.contains("<svg"), "{html}");
    assert!(html.contains("There are no pages to show yet"));
}

#[test]
fn nodes_link_to_their_pages() {
    let notes = Notes::new(&[
        (
            "pages/Project X/Auth Flow.md",
            "- see [[C#%3A a]] and [[kafka]]",
        ),
        ("pages/C#%3A a.md", "- x"),
    ]);
    let (_h, out, _) = build_site(notes.path());
    let html = read(&out, "graph.html");
    for (href, name) in [
        ("pages/Project%20X/Auth%20Flow.html", "Project X/Auth Flow"),
        ("pages/Project%20X.html", "Project X"),
        ("pages/C%23%253A%20a.html", "C#%3A a"),
        ("pages/kafka.html", "kafka"),
    ] {
        assert!(
            html.contains(&format!("<a href=\"{href}\"")),
            "{href} in {html}"
        );
        // The name is both the tooltip and the label.
        assert!(
            html.contains(&format!("<title>{name}</title>")),
            "{name} in {html}"
        );
        assert!(
            out.join(
                href.replace("%20", " ")
                    .replace("%23", "#")
                    .replace("%25", "%")
            )
            .exists()
        );
    }
    assert!(html.contains("class=\"t0 implicit\""), "kafka is implicit");
}

#[test]
fn graph_is_deterministic() {
    // Big enough for the layout to use several threads.
    let files: Vec<(String, String)> = (0..1200)
        .map(|i| {
            (
                format!("pages/p{i}.md"),
                format!("- [[p{}]] [[p{}]]", (i + 1) % 1200, (i * 7) % 1200),
            )
        })
        .collect();
    let files: Vec<(&str, &str)> = files
        .iter()
        .map(|(p, c)| (p.as_str(), c.as_str()))
        .collect();
    let notes = Notes::new(&files);
    let (_h1, a, _) = build_site(notes.path());
    let (_h2, b, _) = build_site(notes.path());
    assert!(
        read(&a, "graph.html") == read(&b, "graph.html"),
        "two builds differ"
    );
}

#[test]
fn graph_ignores_journal_content() {
    let notes = Notes::new(&[
        ("pages/auth.md", "- uses [[redis]]"),
        ("journals/2026-10-12.md", "- [[auth]] and [[kafka]]"),
    ]);
    let (_h1, before, _) = build_site(notes.path());
    for day in 0..1000u32 {
        let (year, month, d) = (2020 + day / 336, 1 + day / 28 % 12, 1 + day % 28);
        notes.write(
            &format!("journals/{year}-{month:02}-{d:02}.md"),
            format!("- standup {day}\n  - nothing linked\n").as_bytes(),
        );
    }
    let (_h2, after, _) = build_site(notes.path());
    assert!(after.join("journals/2022-12-20.html").exists());
    assert!(
        read(&before, "graph.html") == read(&after, "graph.html"),
        "graph.html changed"
    );
}
