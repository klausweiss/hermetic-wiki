//! Whole-site properties: file layout, determinism, no external resources, bounded growth.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use common::{FIXTURE, Notes, build_site};

/// All files under `dir` with their bytes, keyed by relative path.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(&path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

#[test]
fn fixture_file_list() {
    let (_h, out, _) = build_site(Path::new(FIXTURE));
    let mut files: Vec<String> = snapshot(&out)
        .keys()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    files.sort();
    let mut expected = [
        ".hermetic-site",
        "_hermetic/LICENSE-Lucide.txt",
        "_hermetic/app.js",
        "_hermetic/fonts/InterVariable-Italic.woff2",
        "_hermetic/fonts/InterVariable.woff2",
        "_hermetic/fonts/JetBrainsMono-Bold.woff2",
        "_hermetic/fonts/JetBrainsMono-Italic.woff2",
        "_hermetic/fonts/JetBrainsMono-Regular.woff2",
        "_hermetic/fonts/LICENSE-Inter.txt",
        "_hermetic/fonts/LICENSE-JetBrainsMono.txt",
        "_hermetic/style.css",
        "graph.html",
        "index.html",
        "journals.html",
        "journals/2026-09-30.html",
        "journals/2026-10-12.html",
        "journals/2026-13-01.html",
        "pages.html",
        "pages/Claude.html",
        "pages/Project X.html",
        "pages/Project X/Auth Flow.html",
        "pages/Zu\u{308}rich.html",
        "pages/ops.html",
        "pages/redis.html",
        "search.html",
        "search/meta.json",
        "tasks.html",
    ];
    expected.sort();
    // Index shards and document chunks depend on the words in the notes; they are
    // covered by tests/search_index.rs.
    let (index, files): (Vec<String>, Vec<String>) = files
        .into_iter()
        .partition(|f| f.starts_with("search/t/") || f.starts_with("search/d/"));
    assert_eq!(files, expected);
    assert!(index.iter().any(|f| f == "search/d/0.json"));
    assert!(index.iter().filter(|f| f.starts_with("search/t/")).count() > 10);
}

#[test]
fn pages_journals_and_implicit_pages() {
    let notes = Notes::new(&[
        ("journals/2026-10-12.md", "- [[kafka]] [[2026-10-01]]"),
        ("pages/Project X/Auth Flow.md", "- x"),
    ]);
    let (_h, out, _) = build_site(notes.path());
    for f in [
        "journals/2026-10-12.html",
        "journals/2026-10-01.html",
        "pages/kafka.html",
        "pages/Project X/Auth Flow.html",
        "pages/Project X.html",
    ] {
        assert!(out.join(f).is_file(), "{f}");
    }
}

#[test]
fn encoded_page_names() {
    let notes = Notes::new(&[
        ("pages/C#%3A a.md", "- x"),
        ("pages/index.md", "- [[C#: a]]"),
    ]);
    let (_h, out, _) = build_site(notes.path());
    assert!(out.join("pages/C#%3A a.html").is_file());
    let html = fs::read_to_string(out.join("pages/index.html")).unwrap();
    assert!(html.contains(r#"href="C%23%253A%20a.html""#), "{html}");
}

#[test]
fn index_is_the_home_page() {
    // Without index.md, [[index]] links to the default home page, which stays in place.
    let notes = Notes::new(&[("journals/2026-10-12.md", "- see [[index]]")]);
    let (_h, out, _) = build_site(notes.path());
    let day = fs::read_to_string(out.join("journals/2026-10-12.html")).unwrap();
    assert!(day.contains(r#"href="../index.html""#), "{day}");
    assert!(!out.join("pages/index.html").exists());
    let home = fs::read_to_string(out.join("index.html")).unwrap();
    assert!(home.contains("data-view=\"home\""), "{home}");
    assert!(!home.contains("class=\"warning\""));
}

#[test]
fn conflicting_pages_index() {
    for with_root in [false, true] {
        let mut files = vec![("pages/index.md", "- other")];
        if with_root {
            files.push(("index.md", "- home"));
        }
        let notes = Notes::new(&files);
        let (_h, out, _) = build_site(notes.path());
        let home = fs::read_to_string(out.join("index.html")).unwrap();
        let warning = home
            .find("class=\"warning\"")
            .expect("warning on the home page");
        assert!(
            home[warning..].contains(r#"<code>pages/index.md</code>"#),
            "{home}"
        );
        assert!(
            home[warning..].contains(r#"href="pages/index.html""#),
            "{home}"
        );
        let other = fs::read_to_string(out.join("pages/index.html")).unwrap();
        assert!(
            other.contains("<h1>index (in pages)</h1>") && other.contains("other"),
            "{other}"
        );
    }
}

#[test]
fn rebuild_is_identical() {
    let (_h1, a, _) = build_site(Path::new(FIXTURE));
    let (_h2, b, _) = build_site(Path::new(FIXTURE));
    assert!(snapshot(&a) == snapshot(&b), "two builds differ");
}

/// Extracts attribute and `url(...)` values that may point at resources.
fn urls(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for marker in ["href=\"", "src=\"", "url(\""] {
        let mut rest = text;
        while let Some(i) = rest.find(marker) {
            rest = &rest[i + marker.len()..];
            let end = rest.find('"').unwrap_or(rest.len());
            out.push(rest[..end].to_owned());
        }
    }
    out
}

/// Whether `url` points outside the site; `data:` URLs carry their content inline.
fn is_external(url: &str) -> bool {
    url.starts_with("//")
        || url.split_once(':').is_some_and(|(scheme, _)| {
            !scheme.is_empty()
                && scheme != "data"
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
        })
}

#[test]
fn no_external_resources() {
    // Author-written external links are allowed, and the footer's link to hermetic's
    // project page; they are the only expected exceptions.
    let notes = Notes::new(&[
        (
            "pages/a.md",
            "- [site](https://example.com) and [[b]] ![img](pic.png)",
        ),
        ("pages/b.md", "- TODO x ((nope))"),
    ]);
    let (_h, out, _) = build_site(notes.path());
    let mut checked = 0;
    for (path, bytes) in snapshot(&out) {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default();
        if ext != "html" && ext != "css" {
            continue;
        }
        let text = String::from_utf8(bytes).unwrap();
        if ext == "html" {
            // Only hermetic's own script, loaded by URL, never inline.
            let depth = path.components().count() - 1;
            let expected = format!("{}_hermetic/app.js", "../".repeat(depth));
            assert_eq!(text.matches("<script").count(), 1, "{}", path.display());
            assert!(
                text.contains(&format!(r#"<script src="{expected}"></script>"#)),
                "{} loads another script",
                path.display()
            );
        }
        for url in urls(&text) {
            if is_external(&url) {
                assert!(
                    url == "https://example.com" || url == hermetic::site::PROJECT_URL,
                    "{} links to {url}",
                    path.display()
                );
            }
        }
        checked += 1;
    }
    assert!(checked >= 6);
}

#[test]
fn unrelated_growth_does_not_change_a_page() {
    let notes = Notes::new(&[
        ("pages/auth.md", "- the auth page"),
        ("journals/2026-10-12.md", "- about [[auth]] and [[redis]]"),
    ]);
    let (_h1, before, _) = build_site(notes.path());
    for i in 0..1000 {
        let date = format!(
            "{}-{:02}-{:02}",
            2000 + i / 336,
            1 + (i / 28) % 12,
            1 + i % 28
        );
        notes.write(
            &format!("journals/{date}.md"),
            format!("- unrelated {i} [[topic {i}]] #misc").as_bytes(),
        );
    }
    let (_h2, after, _) = build_site(notes.path());
    for page in ["pages/auth.html", "pages/redis.html"] {
        assert_eq!(
            fs::read(before.join(page)).unwrap(),
            fs::read(after.join(page)).unwrap(),
            "{page} changed"
        );
    }
}
