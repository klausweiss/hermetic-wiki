//! Loads the fixture notes directory and checks the whole graph against a snapshot.

mod common;

use std::fmt::Write;
use std::path::Path;
use std::time::Instant;

use common::Notes;
use hermetic::graph::{PageKind, RefTarget};
use hermetic::{Diagnostic, Graph};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/notes");

/// A readable, deterministic dump of everything the graph exposes.
fn dump(g: &Graph, diags: &[Diagnostic]) -> String {
    let mut out = String::new();
    let target = |t: RefTarget| match t {
        RefTarget::Page(p) => format!("page {:?}", g.page(p).name),
        RefTarget::Block(b) => format!("block {:?}", g.block(b).content),
        RefTarget::Unresolved => "unresolved".to_owned(),
    };
    for (id, page) in g.pages() {
        let kind = match page.kind {
            PageKind::Journal(_) => "journal",
            PageKind::Regular => "page",
        };
        let file = page
            .file
            .as_ref()
            .map_or("implicit".to_owned(), |f| f.display().to_string());
        writeln!(out, "{kind} {:?} ({file})", page.name).unwrap();
        if let Some(parent) = page.ns_parent {
            writeln!(out, "  namespace parent: {:?}", g.page(parent).name).unwrap();
        }
        let mut stack: Vec<(usize, _)> = page.roots.iter().rev().map(|&b| (0, b)).collect();
        while let Some((depth, b)) = stack.pop() {
            let block = g.block(b);
            let task = block
                .task
                .map_or(String::new(), |t| format!("{} ", t.marker()));
            writeln!(
                out,
                "  {}- {task}{:?} @{}",
                "  ".repeat(depth),
                block.content,
                block.line
            )
            .unwrap();
            for p in &block.props {
                writeln!(out, "  {}    {}:: {:?}", "  ".repeat(depth), p.key, p.value).unwrap();
            }
            for r in &block.refs {
                writeln!(
                    out,
                    "  {}    -> {:?} {:?} = {}",
                    "  ".repeat(depth),
                    r.kind,
                    r.text,
                    target(r.target)
                )
                .unwrap();
            }
            stack.extend(block.children.iter().rev().map(|&c| (depth + 1, c)));
        }
        for group in g.linked_references(id) {
            for r in group.refs {
                let crumbs: Vec<String> = r
                    .breadcrumb
                    .iter()
                    .map(|&b| g.block(b).content.clone())
                    .collect();
                writeln!(
                    out,
                    "  linked from {:?}: {:?} via {:?}",
                    g.page(group.page).name,
                    g.block(r.block).content,
                    crumbs
                )
                .unwrap();
            }
        }
        for t in g.open_page_tasks(id) {
            writeln!(out, "  open task: {:?}", g.block(t).content).unwrap();
        }
        for r in g.related(id) {
            writeln!(out, "  related: {:?} x{}", g.page(r.page).name, r.weight()).unwrap();
        }
    }
    for t in g.open_tasks() {
        writeln!(out, "open task: {:?}", g.block(t).content).unwrap();
    }
    for d in diags {
        writeln!(out, "diagnostic {:?}: {d}", d.kind).unwrap();
    }
    out
}

#[test]
fn fixture_snapshot() {
    let (g, diags) = Graph::load(Path::new(FIXTURE));
    let actual = dump(&g, &diags);
    let snapshot = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/notes.snapshot");
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&snapshot, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(&snapshot).unwrap_or_default();
    assert_eq!(
        actual, expected,
        "\n--- actual ---\n{actual}\n(set UPDATE_SNAPSHOTS=1 to accept)"
    );
}

#[test]
fn loading_is_deterministic() {
    let (g1, d1) = Graph::load(Path::new(FIXTURE));
    let (g2, d2) = Graph::load(Path::new(FIXTURE));
    assert_eq!(dump(&g1, &d1), dump(&g2, &d2));
}

/// Run with `make test-perf` (release mode).
#[test]
#[ignore = "performance check; run in release mode"]
fn loads_five_thousand_journals_quickly() {
    let notes = Notes::empty();
    let mut day = 0u32;
    for year in 2010..2026 {
        for month in 1..=12 {
            for d in 1..=28 {
                if day == 5000 {
                    break;
                }
                day += 1;
                let text = format!(
                    "- standup about [[project {}]] and #topic{}\n  - TODO follow up with [[person {}]]\n    id:: b{day}\n  - see ((b{}))\n- notes\n  more notes\n",
                    day % 50,
                    day % 200,
                    day % 30,
                    day.saturating_sub(1).max(1)
                );
                notes.write(
                    &format!("journals/{year}-{month:02}-{d:02}.md"),
                    text.as_bytes(),
                );
            }
        }
    }
    let start = Instant::now();
    let (g, diags) = notes.load();
    let elapsed = start.elapsed();
    assert!(diags.is_empty(), "{:?}", &diags[..diags.len().min(3)]);
    assert_eq!(g.pages().filter(|(_, p)| !p.is_implicit()).count(), 5000);
    assert!(elapsed.as_secs_f64() < 1.0, "took {elapsed:?}");
    eprintln!("loaded 5000 journals in {elapsed:?}");
}
