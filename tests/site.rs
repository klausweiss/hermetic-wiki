//! Scenarios from the `static-site-output` and `cli` specs, through `site::build`.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{Notes, build_site, kinds};
use hermetic::DiagnosticKind;
use hermetic::site::{self, BuildError, MARKER};

/// Builds `files` into a fresh output directory; returns (notes, out holder, out path).
fn built(files: &[(&str, &str)]) -> (Notes, Notes, PathBuf, Vec<hermetic::Diagnostic>) {
    let notes = Notes::new(files);
    let (holder, out, report) = build_site(notes.path());
    (notes, holder, out, report.diagnostics)
}

fn read(out: &Path, rel: &str) -> String {
    fs::read_to_string(out.join(rel)).unwrap_or_else(|e| panic!("cannot read {rel}: {e}"))
}

/// The `<main>` part of a page, to avoid matching the shared header.
fn main_of(out: &Path, rel: &str) -> String {
    let html = read(out, rel);
    let start = html.find("<main>").expect("has main");
    let end = html.find("</main>").expect("closes main");
    html[start..end].to_owned()
}

#[test]
fn rewritten_markdown_link() {
    let (_n, _h, out, _) = built(&[
        (
            "journals/2026-10-12.md",
            "- see [auth notes](../pages/auth.md#setup)",
        ),
        ("pages/auth.md", "- x"),
    ]);
    let html = main_of(&out, "journals/2026-10-12.html");
    assert!(
        html.contains(r#"<a href="../pages/auth.html#setup">auth notes</a>"#),
        "{html}"
    );
}

#[test]
fn page_names_with_markdown_characters_render_literally() {
    let (_n, _h, out, _) = built(&[("pages/a.md", "- see [[a*b_c*]] and #x_y_")]);
    let html = main_of(&out, "pages/a.html");
    assert!(html.contains(">a*b_c*</a>"), "{html}");
    assert!(html.contains(">#x_y_</a>"), "{html}");
    assert!(!html.contains("<em>"));
}

#[test]
fn block_reference_shows_target_content() {
    let (_n, _h, out, _) = built(&[
        (
            "pages/redis.md",
            "- TTL defaults were wrong\n  id:: ttl-bug",
        ),
        ("journals/2026-10-12.md", "- the bug was ((ttl-bug))"),
    ]);
    let html = main_of(&out, "journals/2026-10-12.html");
    assert!(html.contains(r#"<span class="block-ref">TTL defaults were wrong<a class="block-ref-link" href="../pages/redis.html#id-ttl-bug""#), "{html}");
}

#[test]
fn self_referencing_block_refs_do_not_loop() {
    let (_n, _h, out, _) = built(&[(
        "pages/a.md",
        "- alpha ((b))\n  id:: a\n- beta ((a))\n  id:: b",
    )]);
    let html = main_of(&out, "pages/a.html");
    // alpha expands beta, whose own ref back to alpha is a plain link.
    assert!(html.contains(r##"alpha <span class="block-ref">beta <a class="block-ref-link" href="#id-a">((a))</a>"##), "{html}");
    assert!(html.contains(r##"beta <span class="block-ref">alpha <a class="block-ref-link" href="#id-b">((b))</a>"##), "{html}");
}

#[test]
fn broken_references() {
    let (_n, _h, out, diags) = built(&[("pages/a.md", "- x ((missing)) [y](nope.md)")]);
    let html = main_of(&out, "pages/a.html");
    assert!(html.contains(r#"<span class="broken" title="Broken reference">((missing))</span>"#));
    assert!(html.contains(r#"<span class="broken" title="Broken reference">[y](nope.md)</span>"#));
    assert_eq!(diags.len(), 2);
}

#[test]
fn nested_blocks_and_properties() {
    let (_n, _h, out, _) = built(&[(
        "pages/a.md",
        "- decided to use JWT\n  status:: accepted\n  id:: jwt\n  - **because** it is simple",
    )]);
    let html = main_of(&out, "pages/a.html");
    assert!(html.contains("<dt>status</dt><dd>accepted</dd>"), "{html}");
    assert!(!html.contains("<dt>id</dt>"));
    assert!(html.contains(r#"<li class="block" id="id-jwt">"#));
    assert!(html.contains("<strong>because</strong> it is simple"));
    let child = html.find("because").unwrap();
    let nested = html
        .find(r#"<ul class="blocks"><li class="block" id="b2">"#)
        .unwrap();
    assert!(nested < child);
}

#[test]
fn bare_urls_link_next_to_page_links() {
    let (_n, _h, out, _) = built(&[
        ("pages/a.md", "- [[redis]]: https://redis.io."),
        ("pages/redis.md", "- x"),
    ]);
    let html = main_of(&out, "pages/a.html");
    assert!(
        html.contains(r#"<a class="page-ref" href="redis.html">redis</a>: <a href="https://redis.io">https://redis.io</a>."#),
        "{html}"
    );
    assert!(!html.contains("target="), "{html}");
}

#[test]
fn bare_url_in_property_value() {
    let (_n, _h, out, _) = built(&[(
        "pages/a.md",
        "- report\n  source:: https://example.com/report",
    )]);
    let html = main_of(&out, "pages/a.html");
    assert!(
        html.contains(r#"<dt>source</dt><dd><a href="https://example.com/report">https://example.com/report</a></dd>"#),
        "{html}"
    );
}

#[test]
fn anchors() {
    let (_n, _h, out, _) = built(&[("pages/a.md", "- one\n- two\n  id:: ttl-bug\n- three")]);
    let html = main_of(&out, "pages/a.html");
    assert!(
        html.contains(r#"id="b1""#)
            && html.contains(r#"id="id-ttl-bug""#)
            && html.contains(r#"id="b3""#)
    );
}

#[test]
fn task_badges() {
    let (_n, _h, out, _) = built(&[("pages/a.md", "- TODO a\n- DOING b\n- DONE c")]);
    let html = main_of(&out, "pages/a.html");
    for (class, marker) in [("todo", "TODO"), ("doing", "DOING"), ("done", "DONE")] {
        assert!(html.contains(&format!(
            r#"<span class="task-marker task-{class}">{marker}</span>"#
        )));
    }
}

#[test]
fn namespaced_page() {
    let (_n, _h, out, _) = built(&[
        ("pages/Project X/Auth Flow.md", "- a"),
        ("pages/Project X/Auth Flow/Tokens.md", "- t"),
    ]);
    let html = main_of(&out, "pages/Project X/Auth Flow.html");
    assert!(
        html.contains(
            r#"<nav class="namespace"><a href="../Project%20X.html">Project X</a> / </nav>"#
        ),
        "{html}"
    );
    assert!(
        html.contains(r#"<a href="Auth%20Flow/Tokens.html">Tokens</a>"#),
        "{html}"
    );
    assert!(html.contains("<h1>Auth Flow</h1>"));
}

#[test]
fn implicit_page_notice() {
    let (_n, _h, out, _) = built(&[("journals/2026-10-12.md", "- [[kafka]] [[2026-10-01]]")]);
    assert!(main_of(&out, "pages/kafka.html").contains("This page has no content of its own yet."));
    assert!(
        main_of(&out, "journals/2026-10-01.html")
            .contains("There is no journal entry for this day.")
    );
}

#[test]
fn references_with_breadcrumb() {
    let (_n, _h, out, _) = built(&[(
        "journals/2026-10-12.md",
        "- standup\n  - [[auth]] token refresh shipped\n    - details",
    )]);
    let html = main_of(&out, "pages/auth.html");
    let refs = &html[html.find("linked-refs").unwrap()..];
    assert!(
        refs.contains(r#"<h3><a href="../journals/2026-10-12.html">2026-10-12 Monday</a></h3>"#),
        "{refs}"
    );
    assert!(
        refs.contains(
            r#"<nav class="breadcrumb"><a href="../journals/2026-10-12.html#b1">standup</a></nav>"#
        ),
        "{refs}"
    );
    assert!(refs.contains("token refresh shipped"));
    assert!(refs.contains("details"));
    assert!(
        !refs.contains(" id=\""),
        "linked refs must not repeat anchors"
    );
}

#[test]
fn pagination_of_linked_references() {
    let journal: String = (0..250).map(|i| format!("- ref {i} [[auth]]\n")).collect();
    let (_n, _h, out, _) = built(&[("journals/2026-10-12.md", &journal)]);
    let first = main_of(&out, "pages/auth.html");
    assert_eq!(first.matches(r#"<div class="linked-ref">"#).count(), 100);
    assert!(first.contains("ref 0 ") && !first.contains("ref 100 "));
    assert!(
        first.contains(r#"<a rel="next" href="../refs/pages/auth/2.html">"#),
        "{first}"
    );
    let second = main_of(&out, "refs/pages/auth/2.html");
    assert_eq!(second.matches(r#"<div class="linked-ref">"#).count(), 100);
    assert!(second.contains("ref 100 ") && second.contains("ref 199 "));
    assert!(
        second.contains(r#"<a rel="prev" href="../../../pages/auth.html">"#),
        "{second}"
    );
    assert!(second.contains(r#"<a rel="next" href="3.html">"#));
    let third = main_of(&out, "refs/pages/auth/3.html");
    assert_eq!(third.matches(r#"<div class="linked-ref">"#).count(), 50);
    assert!(!third.contains(r#"rel="next""#));
    assert!(!out.join("refs/pages/auth/4.html").exists());
}

#[test]
fn open_tasks_of_a_page() {
    let (_n, _h, out, _) = built(&[(
        "journals/2026-10-12.md",
        "- meeting about [[auth]]\n  - TODO rotate keys\n  - DONE drop endpoint",
    )]);
    let html = main_of(&out, "pages/auth.html");
    let tasks = &html[html.find("open-tasks").unwrap()..html.find("linked-refs").unwrap()];
    assert!(tasks.contains("rotate keys"));
    assert!(
        tasks.contains(r#"href="../journals/2026-10-12.html#b2""#),
        "{tasks}"
    );
    assert!(!tasks.contains("drop endpoint"));
}

#[test]
fn related_pages() {
    let (_n, _h, out, _) = built(&[(
        "journals/2026-10-12.md",
        "- [[claude]] and [[hermetic]] one\n- [[claude]] and [[hermetic]] two\n- [[claude]] and [[hermetic]] three",
    )]);
    let html = main_of(&out, "pages/claude.html");
    let related =
        &html[html.find(r#"class="related""#).unwrap()..html.find("linked-refs").unwrap()];
    assert!(related.contains(r#"<summary><a href="hermetic.html">hermetic</a> <span class="count">3</span></summary>"#), "{related}");
    assert!(related.contains("one") && related.contains("two") && related.contains("three"));
    assert!(related.contains("<details>"));
}

#[test]
fn previous_and_next_day() {
    let (_n, _h, out, _) = built(&[
        ("journals/2026-10-09.md", "- a"),
        ("journals/2026-10-12.md", "- b [[2026-10-13]]"),
        ("journals/2026-10-15.md", "- c"),
    ]);
    let html = main_of(&out, "journals/2026-10-12.html");
    assert!(html.contains("<h1>2026-10-12 <span class=\"weekday\">Monday</span></h1>"));
    assert!(html.contains(r#"<a rel="prev" href="2026-10-09.html">← 2026-10-09</a>"#));
    assert!(html.contains(r#"<a rel="next" href="2026-10-15.html">2026-10-15 →</a>"#));
    // An implicit day links to its file-backed neighbours.
    let implicit = main_of(&out, "journals/2026-10-13.html");
    assert!(implicit.contains("2026-10-12.html") && implicit.contains("2026-10-15.html"));
    let first = main_of(&out, "journals/2026-10-09.html");
    assert!(!first.contains(r#"rel="prev""#));
}

#[test]
fn journal_index() {
    let (_n, _h, out, _) = built(&[
        ("journals/2026-09-30.md", "- a [[2026-11-01]]"),
        ("journals/2026-10-12.md", "- b"),
        ("journals/2026-10-01.md", "- c"),
    ]);
    let html = main_of(&out, "journals.html");
    assert!(html.contains(r#"data-view="journals""#));
    let pos = |s: &str| {
        html.find(s)
            .unwrap_or_else(|| panic!("missing {s}: {html}"))
    };
    assert!(pos("<h2>October 2026</h2>") < pos("2026-10-12.html"));
    assert!(pos("2026-10-12.html") < pos("2026-10-01.html"));
    assert!(pos("2026-10-01.html") < pos("<h2>September 2026</h2>"));
    assert!(!html.contains("2026-11-01"), "implicit days are not listed");
}

#[test]
fn all_pages_list() {
    let (_n, _h, out, _) = built(&[
        ("pages/auth.md", "- x"),
        ("pages/Beta.md", "- [[kafka]]"),
        ("journals/2026-10-12.md", "- y"),
    ]);
    let html = main_of(&out, "pages.html");
    let pos = |s: &str| {
        html.find(s)
            .unwrap_or_else(|| panic!("missing {s}: {html}"))
    };
    assert!(pos(">auth</a>") < pos(">Beta</a>") && pos(">Beta</a>") < pos(">kafka</a>"));
    assert!(html.contains(r#">kafka</a> <span class="implicit""#));
    assert!(!html.contains("auth</a> <span class=\"implicit\""));
    assert!(!html.contains("2026-10-12"));
}

#[test]
fn global_tasks_are_paginated() {
    let page: String = (0..150).map(|i| format!("- TODO task {i}\n")).collect();
    let (_n, _h, out, _) = built(&[("pages/a.md", &page), ("pages/b.md", "- DONE closed")]);
    let first = main_of(&out, "tasks.html");
    assert_eq!(first.matches("task-marker task-todo").count(), 100);
    assert!(first.contains(r#"<h1>Open tasks <span class="count">150</span></h1>"#));
    assert!(first.contains(r#"<a rel="next" href="tasks/2.html">"#));
    let second = main_of(&out, "tasks/2.html");
    assert_eq!(second.matches("task-marker task-todo").count(), 50);
    assert!(second.contains(r#"<a rel="prev" href="../tasks.html">"#));
    assert!(!first.contains("closed") && !second.contains("closed"));
}

#[test]
fn fonts_and_licenses_shipped() {
    let (_n, _h, out, _) = built(&[("pages/a.md", "- x")]);
    for f in [
        "_hermetic/style.css",
        "_hermetic/fonts/InterVariable.woff2",
        "_hermetic/fonts/InterVariable-Italic.woff2",
        "_hermetic/fonts/JetBrainsMono-Regular.woff2",
        "_hermetic/fonts/LICENSE-Inter.txt",
        "_hermetic/fonts/LICENSE-JetBrainsMono.txt",
        "_hermetic/LICENSE-Lucide.txt",
    ] {
        assert!(out.join(f).is_file(), "{f}");
    }
    let css = read(&out, "_hermetic/style.css");
    assert!(css.contains(r#"url("fonts/InterVariable.woff2")"#));
    assert!(css.contains(r#"font-family: "Inter", sans-serif;"#));
    assert!(read(&out, "_hermetic/fonts/LICENSE-Inter.txt").contains("SIL Open Font License"));
    assert!(read(&out, "_hermetic/LICENSE-Lucide.txt").contains("ISC License"));
}

#[test]
fn image_next_to_a_page() {
    let notes = Notes::new(&[
        ("pages/auth.md", "- ![flow](auth-flow.png)"),
        ("assets/logo.svg", "<svg/>"),
    ]);
    notes.write("pages/auth-flow.png", b"\x89PNG fake");
    let (_h, out, _) = build_site(notes.path());
    assert_eq!(
        fs::read(out.join("pages/auth-flow.png")).unwrap(),
        b"\x89PNG fake"
    );
    assert!(out.join("assets/logo.svg").is_file());
    assert!(main_of(&out, "pages/auth.html").contains(r#"<img src="auth-flow.png" alt="flow" />"#));
}

#[test]
fn attachment_conflicts_and_symlinks() {
    let notes = Notes::new(&[
        ("pages/auth.md", "- x"),
        ("pages/auth.html", "<p>hand-written</p>"),
        ("pages/.secret.txt", "s"),
    ]);
    std::os::unix::fs::symlink("/etc/hostname", notes.path().join("pages/host.txt")).unwrap();
    let (_h, out, report) = build_site(notes.path());
    let kinds = kinds(&report.diagnostics);
    assert!(
        kinds.contains(&DiagnosticKind::AttachmentConflict),
        "{kinds:?}"
    );
    assert!(kinds.contains(&DiagnosticKind::SkippedSymlink), "{kinds:?}");
    assert!(!read(&out, "pages/auth.html").contains("hand-written"));
    assert!(!out.join("pages/host.txt").exists());
    assert!(!out.join("pages/.secret.txt").exists());
}

#[test]
fn rebuilding_replaces_the_previous_output() {
    let notes = Notes::new(&[("pages/a.md", "- a"), ("pages/b.md", "- b")]);
    let holder = Notes::empty();
    let out = holder.path().join("site");
    site::build(notes.path(), &out, None).unwrap();
    assert!(out.join("pages/b.html").exists());
    fs::remove_file(notes.path().join("pages/b.md")).unwrap();
    site::build(notes.path(), &out, None).unwrap();
    assert!(!out.join("pages/b.html").exists());
    assert!(out.join("pages/a.html").exists() && out.join(MARKER).exists());
}

#[test]
fn foreign_directory_is_not_touched() {
    let notes = Notes::new(&[("pages/a.md", "- a")]);
    let holder = Notes::new(&[("site/report.pdf", "pdf")]);
    let out = holder.path().join("site");
    let err = site::build(notes.path(), &out, None).unwrap_err();
    assert!(matches!(err, BuildError::UnsafeOutput { .. }), "{err}");
    assert_eq!(fs::read_dir(&out).unwrap().count(), 1);
    // An empty directory is fine.
    let empty = Notes::empty();
    fs::create_dir(empty.path().join("site")).unwrap();
    site::build(notes.path(), &empty.path().join("site"), None).unwrap();
}

#[test]
fn output_would_contain_or_be_inside_the_notes() {
    let notes = Notes::new(&[("pages/a.md", "- a")]);
    for out in [
        notes.path().to_path_buf(),
        notes.path().parent().unwrap().to_path_buf(),
        notes.path().join("pages/site"),
        notes.path().join("journals"),
        notes.path().join("assets/out"),
        notes.path().join("x/../pages"),
    ] {
        let err = site::build(notes.path(), &out, None).unwrap_err();
        assert!(
            matches!(err, BuildError::UnsafeOutput { .. }),
            "{}: {err}",
            out.display()
        );
    }
    assert!(notes.path().join("pages/a.md").exists());
}

#[test]
fn missing_notes_directory() {
    let holder = Notes::empty();
    let err = site::build(
        &holder.path().join("nope"),
        &holder.path().join("site"),
        None,
    )
    .unwrap_err();
    assert!(matches!(err, BuildError::NotADirectory(_)));
    assert!(!holder.path().join("site").exists());
}

#[cfg(unix)]
#[test]
fn failed_build_leaves_old_output_intact() {
    use std::os::unix::fs::PermissionsExt;
    let notes = Notes::new(&[("pages/a.md", "- a")]);
    let holder = Notes::empty();
    let parent = holder.path().join("parent");
    let out = parent.join("site");
    site::build(notes.path(), &out, None).unwrap();
    let before = read(&out, "pages/a.html");
    fs::write(notes.path().join("pages/a.md"), "- changed").unwrap();
    // A read-only parent makes creating the temporary directory fail.
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o555)).unwrap();
    let result = site::build(notes.path(), &out, None);
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o755)).unwrap();
    if nix_is_root() {
        return; // root ignores permissions; nothing to check
    }
    assert!(matches!(result, Err(BuildError::Io { .. })), "{result:?}");
    assert_eq!(read(&out, "pages/a.html"), before);
}

fn nix_is_root() -> bool {
    std::env::var("USER").is_ok_and(|u| u == "root")
}

#[test]
fn output_inside_notes_root_is_not_read_as_notes() {
    let notes = Notes::new(&[("pages/a.md", "- a")]);
    let out = notes.path().join("site");
    site::build(notes.path(), &out, None).unwrap();
    let report = site::build(notes.path(), &out, None).unwrap();
    assert_eq!(report.pages, 1);
    assert!(report.diagnostics.is_empty());
    let (g, _) = hermetic::Graph::load(notes.path());
    assert_eq!(g.pages().count(), 1);
}

#[test]
fn site_wide_pages() {
    let (_n, _h, out, _) = built(&[("pages/a.md", "- x")]);
    for f in [
        "index.html",
        "journals.html",
        "pages.html",
        "tasks.html",
        "search.html",
        "graph.html",
    ] {
        assert!(out.join(f).is_file(), "{f}");
    }
}

#[test]
fn default_home_page() {
    let (_n, _h, out, _) = built(&[("journals/2026-10-12.md", "- a")]);
    let home = main_of(&out, "index.html");
    assert!(home.contains(r#"data-view="home""#));
    assert!(home.contains("2026-10-12.html"));
    assert!(home.contains(r#"<p class="hint">"#) && home.contains("<code>index.md</code>"));
    assert!(!main_of(&out, "journals.html").contains("class=\"hint\""));
}

#[test]
fn custom_home_page() {
    let (_n, _h, out, _) = built(&[
        ("index.md", "- Start at [[auth]]"),
        ("pages/auth.md", "- x"),
        ("journals/2026-10-12.md", "- see [[index]]"),
    ]);
    let home = main_of(&out, "index.html");
    assert!(
        home.contains(r#"Start at <a class="page-ref" href="pages/auth.html">auth</a>"#),
        "{home}"
    );
    assert!(!home.contains(r#"class="hint""#));
    let auth = main_of(&out, "pages/auth.html");
    assert!(
        auth.contains(r#"<h3><a href="../index.html">index</a></h3>"#),
        "{auth}"
    );
    let journal = main_of(&out, "journals/2026-10-12.html");
    assert!(
        journal.contains(r#"<a class="page-ref" href="../index.html">index</a>"#),
        "{journal}"
    );
    assert!(main_of(&out, "pages.html").contains(r#"<a href="index.html">index</a>"#));
    assert!(!out.join("pages/index.html").exists());
}

#[test]
fn implicit_index_page_is_the_home_page() {
    let (_n, _h, out, _) = built(&[("journals/2026-10-12.md", "- see [[index]]")]);
    assert!(!out.join("pages/index.html").exists());
    assert!(main_of(&out, "index.html").contains(r#"data-view="home""#));
    let journal = main_of(&out, "journals/2026-10-12.html");
    assert!(
        journal.contains(r#"href="../index.html">index</a>"#),
        "{journal}"
    );
}

#[test]
fn html_hooks_for_the_script() {
    let (_n, _h, out, _) = built(&[
        ("journals/2026-10-12.md", "- [[auth]]"),
        ("pages/auth.md", "- TODO x"),
    ]);
    for (file, view) in [
        ("journals.html", "journals"),
        ("pages.html", "pages"),
        ("tasks.html", "tasks"),
        ("index.html", "home"),
    ] {
        let html = read(&out, file);
        assert!(
            html.contains(r#"<meta name="hermetic-root" content=".">"#),
            "{file}"
        );
        assert!(
            main_of(&out, file).contains(&format!(
                r#"<article class="page index" data-view="{view}">"#
            )),
            "{file}"
        );
    }
    let page = read(&out, "pages/auth.html");
    assert!(page.contains(r#"<meta name="hermetic-root" content="..">"#));
    assert!(page.contains(r#"<section class="content">"#));
    assert!(page.contains(r#"<section class="linked-refs">"#));
    assert!(read(&out, "journals.html").contains(r#"<ul class="days">"#));
}

#[test]
fn web_component_tags() {
    let (_n, _h, out, _) = built(&[("journals/2026-10-12.md", "- [[auth]]")]);
    for file in [
        "index.html",
        "journals.html",
        "pages/auth.html",
        "journals/2026-10-12.html",
    ] {
        let html = read(&out, file);
        // Empty, so pages look the same without the script that defines them.
        for tag in [
            "hermetic-theme-toggle",
            "hermetic-search",
            "hermetic-shortcuts",
        ] {
            assert_eq!(
                html.matches(&format!("<{tag}></{tag}>")).count(),
                1,
                "{file}: {tag}"
            );
        }
    }
    // The sidebar holds only the site footer, which without the script follows the page.
    for file in ["index.html", "pages/auth.html"] {
        let html = read(&out, file);
        let start = html.find("<hermetic-sidebar>").expect("sidebar element");
        let end = html.find("</hermetic-sidebar>").unwrap();
        let inner = &html[start + "<hermetic-sidebar>".len()..end];
        assert!(
            inner.starts_with(r#"<footer class="site-footer">"#) && inner.ends_with("</footer>"),
            "{file}: {inner}"
        );
        assert!(
            start > html.find("</main>").unwrap(),
            "{file}: footer after main"
        );
    }
    // The timeline wraps the journal list on journals.html only.
    let journals = main_of(&out, "journals.html");
    let start = journals
        .find("<hermetic-timeline>")
        .expect("timeline on journals.html");
    let end = journals.find("</hermetic-timeline>").unwrap();
    assert!(journals[start..end].contains(r#"<ul class="days">"#));
    assert!(!main_of(&out, "index.html").contains("hermetic-timeline"));
}

#[test]
fn footer_names_the_commit() {
    let (_n, _h, out, _) = built(&[("pages/a.md", "- x")]);
    let html = read(&out, "pages/a.html");
    let git = std::process::Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8(o.stdout).unwrap().trim().to_owned());
    let expected = match git {
        Some(commit) => format!(
            r#"Generated by <a href="https://github.com/klausweiss/hermetic-wiki">hermetic</a> <code>{commit}</code>"#
        ),
        None => r#"Generated by <a href="https://github.com/klausweiss/hermetic-wiki">hermetic</a></span>"#.to_owned(),
    };
    assert!(html.contains(&expected), "{html}");
}

#[test]
fn alias_property_is_shown_without_links() {
    let (_n, _h, out, _) = built(&[(
        "pages/auth.md",
        "alias:: authentication, [[login]]\n\n- notes",
    )]);
    let html = main_of(&out, "pages/auth.html");
    assert!(
        html.contains("<dt>alias</dt><dd>authentication, login</dd>"),
        "{html}"
    );
    assert!(!html.contains("login.html"), "{html}");
}

#[test]
fn link_through_an_alias_goes_to_the_page() {
    let (_n, _h, out, diags) = built(&[
        ("pages/auth.md", "- alias:: login"),
        ("journals/2026-10-12.md", "- [[login]] and #login"),
    ]);
    assert!(diags.is_empty(), "{diags:?}");
    let html = main_of(&out, "journals/2026-10-12.html");
    assert!(
        html.contains(r#"<a class="page-ref" href="../pages/auth.html">login</a>"#),
        "{html}"
    );
    assert!(
        html.contains(r#"<a class="tag" href="../pages/auth.html">#login</a>"#),
        "{html}"
    );
    assert!(!out.join("pages/login.html").exists());
    let auth = main_of(&out, "pages/auth.html");
    assert!(auth.contains("2026-10-12"), "{auth}");
}
