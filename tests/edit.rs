//! Scenarios from the `browser-editing` spec that the edit API decides, against a real
//! server on an ephemeral port.

mod common;

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

use common::Notes;
use hermetic::serve::{self, Bind, Running, watch::Timing};

fn start(notes: &Path) -> Running {
    serve::start(serve::Options {
        notes: notes.to_path_buf(),
        bind: Bind::Loopback,
        port: Some(0),
        title: None,
        timing: Timing {
            poll: Duration::from_millis(30),
            quiet: Duration::from_millis(60),
        },
    })
    .expect("server starts")
}

struct Response {
    status: u16,
    body: String,
}

/// Sends a request with the given extra headers and body.
fn send(
    addr: SocketAddr,
    method: &str,
    target: &str,
    headers: &[(&str, String)],
    body: &str,
) -> Response {
    let mut stream = TcpStream::connect(addr).unwrap();
    let mut head = format!("{method} {target} HTTP/1.1\r\n");
    if !headers.iter().any(|(n, _)| *n == "Host") {
        head.push_str(&format!("Host: 127.0.0.1:{}\r\n", addr.port()));
    }
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    if method == "PUT" {
        head.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes()).unwrap();
    stream.write_all(body.as_bytes()).unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let raw = String::from_utf8(raw).unwrap();
    let (head, body) = raw.split_once("\r\n\r\n").expect("header end");
    Response {
        status: head[9..12].parse().unwrap(),
        body: body.to_owned(),
    }
}

fn source_url(path: &str) -> String {
    format!("/_hermetic/source?path={}", path.replace(' ', "%20"))
}

fn get_source(addr: SocketAddr, path: &str) -> Response {
    send(addr, "GET", &source_url(path), &[], "")
}

/// The headers hermetic's editor sends with a save.
fn editor_headers(addr: SocketAddr, version: &str) -> Vec<(&'static str, String)> {
    vec![
        ("Origin", format!("http://127.0.0.1:{}", addr.port())),
        ("X-Hermetic-Edit", "1".to_owned()),
        ("X-Hermetic-Version", version.to_owned()),
    ]
}

fn put_source(addr: SocketAddr, path: &str, version: &str, text: &str) -> Response {
    send(
        addr,
        "PUT",
        &source_url(path),
        &editor_headers(addr, version),
        text,
    )
}

/// A string field of a JSON payload (no escapes in the values these tests read).
fn field(json: &str, name: &str) -> String {
    let start = json.find(&format!("\"{name}\":\"")).expect(name) + name.len() + 4;
    json[start..start + json[start..].find('"').unwrap()].to_owned()
}

fn page(addr: SocketAddr, path: &str) -> String {
    send(
        addr,
        "GET",
        &format!("/{}", path.replace(' ', "%20")),
        &[],
        "",
    )
    .body
}

#[test]
fn read_a_page() {
    let notes = Notes::new(&[(
        "pages/auth.md",
        "- decided to use [[JWT]]\n  status:: accepted\n- b\n  id:: ttl-bug\n",
    )]);
    let server = start(notes.path());
    let r = get_source(server.addr, "pages/auth.html");
    assert_eq!(r.status, 200);
    assert!(r.body.starts_with(r#"{"exists":true,"#), "{}", r.body);
    assert!(
        r.body
            .contains(r#""anchor":"b1","body":"decided to use [[JWT]]\u000astatus:: accepted""#),
        "{}",
        r.body
    );
    assert!(r.body.contains(r#""anchor":"id-ttl-bug""#));
    assert_eq!(field(&r.body, "version").len(), 16);

    // A page that is only referenced has no file yet.
    let r = get_source(server.addr, "pages/JWT.html");
    assert_eq!(r.status, 200);
    assert!(
        r.body
            .starts_with(r#"{"exists":false,"file":"pages/JWT.md","version":"missing","text":"""#),
        "{}",
        r.body
    );
}

#[test]
fn save_and_stale_version() {
    let notes = Notes::new(&[("pages/auth.md", "- old\n")]);
    let server = start(notes.path());
    let version = field(&get_source(server.addr, "pages/auth.html").body, "version");
    let before = server.rebuilds();
    let r = put_source(server.addr, "pages/auth.html", &version, "- new text\n");
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(
        fs::read_to_string(notes.path().join("pages/auth.md")).unwrap(),
        "- new text\n"
    );
    // The response comes after the rebuild: the page already shows the change.
    assert!(page(server.addr, "pages/auth.html").contains("new text"));
    let new_version = field(&r.body, "version");
    assert_ne!(new_version, version);
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(server.rebuilds(), before + 1, "one rebuild per save");

    // Saving again from the old version is refused, and the file is unchanged.
    let r = put_source(server.addr, "pages/auth.html", &version, "- lost\n");
    assert_eq!(r.status, 409);
    assert_eq!(field(&r.body, "version"), new_version);
    assert!(r.body.contains("new text"));
    assert_eq!(
        fs::read_to_string(notes.path().join("pages/auth.md")).unwrap(),
        "- new text\n"
    );

    // An external change makes the browser's version stale too.
    notes.write("pages/auth.md", b"- from $EDITOR\n");
    let r = put_source(server.addr, "pages/auth.html", &new_version, "- browser\n");
    assert_eq!(r.status, 409);
    assert_eq!(
        fs::read_to_string(notes.path().join("pages/auth.md")).unwrap(),
        "- from $EDITOR\n"
    );
}

#[test]
fn line_endings_are_kept() {
    let notes = Notes::new(&[("pages/w.md", "- a\r\n- b\r\n")]);
    let server = start(notes.path());
    let r = get_source(server.addr, "pages/w.html");
    assert!(
        r.body.contains(r#""text":"- a\u000a- b\u000a""#),
        "{}",
        r.body
    );
    let r = put_source(
        server.addr,
        "pages/w.html",
        &field(&r.body, "version"),
        "- a\n- c",
    );
    assert_eq!(r.status, 200);
    assert_eq!(
        fs::read(notes.path().join("pages/w.md")).unwrap(),
        b"- a\r\n- c\r\n"
    );
}

#[test]
fn new_pages_and_days() {
    let notes = Notes::new(&[("journals/2026-10-12.md", "- see [[Project X/Auth]]\n")]);
    let server = start(notes.path());
    // A namespaced page that is only referenced.
    let r = put_source(
        server.addr,
        "pages/Project X/Auth.html",
        "missing",
        "- first note\n",
    );
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(
        fs::read_to_string(notes.path().join("pages/Project X/Auth.md")).unwrap(),
        "- first note\n"
    );
    // A journal day nobody references (the "Today" control).
    let r = put_source(server.addr, "journals/2026-10-13.html", "missing", "- a\n");
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(
        fs::read_to_string(notes.path().join("journals/2026-10-13.md")).unwrap(),
        "- a\n"
    );
    assert!(page(server.addr, "journals/2026-10-13.html").contains("2026-10-13"));
    // Creating again does not replace the file.
    let r = put_source(
        server.addr,
        "journals/2026-10-13.html",
        "missing",
        "- other\n",
    );
    assert_eq!(r.status, 409);
    assert_eq!(
        fs::read_to_string(notes.path().join("journals/2026-10-13.md")).unwrap(),
        "- a\n"
    );
    // No stray files anywhere.
    let mut all = Vec::new();
    for dir in ["journals", "pages", "pages/Project X"] {
        for e in fs::read_dir(notes.path().join(dir)).unwrap().flatten() {
            all.push(e.file_name().to_string_lossy().into_owned());
        }
    }
    assert!(all.iter().all(|n| !n.starts_with('.')), "{all:?}");
}

#[test]
fn pages_without_content_have_no_file() {
    let notes = Notes::new(&[("journals/2026-10-12.md", "- standup [[empty]]\n")]);
    let server = start(notes.path());
    let day = notes.path().join("journals/2026-10-13.md");

    // A day without a file is shown, ready to write in, without creating its file.
    let html = page(server.addr, "journals/2026-10-13.html");
    assert!(
        html.contains("<h1>2026-10-13 <span class=\"weekday\">"),
        "{html}"
    );
    assert!(html.contains(r#"<a rel="prev" href="2026-10-12.html">"#));
    assert!(html.contains("<hermetic-editor></hermetic-editor>"));
    assert!(!day.exists());
    assert!(page(server.addr, "journals/2026-13-01.html").contains("404"));
    assert!(page(server.addr, "journals/notes.html").contains("404"));

    // Saving it empty creates nothing.
    let r = put_source(server.addr, "journals/2026-10-13.html", "missing", "-\n");
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(field(&r.body, "version"), "missing");
    assert!(!day.exists());

    // Content creates the file; emptying it again removes the file.
    let r = put_source(server.addr, "journals/2026-10-13.html", "missing", "- a\n");
    assert_eq!(r.status, 200, "{}", r.body);
    assert!(day.exists());
    let version = field(&r.body, "version");
    let r = put_source(server.addr, "journals/2026-10-13.html", &version, "-\n");
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(field(&r.body, "version"), "missing");
    assert!(!day.exists());
    assert!(!notes.path().join(".trash").exists());
    assert!(!page(server.addr, "journals.html").contains("2026-10-13"));

    // So do other pages, also one that nothing references, which can be written again.
    let file = notes.path().join("pages/Project X/Auth.md");
    let r = put_source(server.addr, "pages/Project X/Auth.html", "missing", "- a\n");
    assert_eq!(r.status, 200, "{}", r.body);
    assert!(file.exists());
    let version = field(&r.body, "version");
    let r = put_source(server.addr, "pages/Project X/Auth.html", &version, "-\n");
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(field(&r.body, "version"), "missing");
    assert!(!notes.path().join("pages/Project X").exists());
    assert!(page(server.addr, "pages/Project X/Auth.html").contains("404"));
    let r = put_source(server.addr, "pages/Project X/Auth.html", "missing", "- b\n");
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(fs::read_to_string(&file).unwrap(), "- b\n");

    // A referenced page stays as a page without a file.
    let r = put_source(server.addr, "pages/empty.html", "missing", "-\n");
    assert_eq!(r.status, 200, "{}", r.body);
    assert!(!notes.path().join("pages/empty.md").exists());
    assert!(page(server.addr, "pages/empty.html").contains("standup"));

    // The home page keeps its file: "Write a home page" creates it with an empty block.
    let r = put_source(server.addr, "index.html", "missing", "-\n");
    assert_eq!(r.status, 200, "{}", r.body);
    assert!(notes.path().join("index.md").exists());
}

#[test]
fn not_editable() {
    let notes = Notes::new(&[
        ("pages/kafka.md", "- k\n"),
        ("journals/2026-10-12.md", "- [[kafka]]\n"),
    ]);
    let server = start(notes.path());
    for path in [
        "tasks.html",
        "journals.html",
        "pages.html",
        "search.html",
        "graph.html",
        "refs/pages/kafka/2.html",
        "pages/kafka.md",
        "../outside.html",
        "journals/2026-13-01.html",
    ] {
        assert_eq!(get_source(server.addr, path).status, 404, "GET {path}");
        assert_eq!(
            put_source(server.addr, path, "missing", "- x\n").status,
            404,
            "PUT {path}"
        );
    }
    assert_eq!(
        send(server.addr, "GET", "/_hermetic/source", &[], "").status,
        404
    );
    assert!(!notes.path().join("tasks.md").exists() && !notes.path().join("index.md").exists());
}

#[test]
fn home_page_with_index_md() {
    let notes = Notes::new(&[("index.md", "- welcome\n")]);
    let server = start(notes.path());
    let r = get_source(server.addr, "index.html");
    assert_eq!(r.status, 200);
    let r = put_source(
        server.addr,
        "index.html",
        &field(&r.body, "version"),
        "- hello\n",
    );
    assert_eq!(r.status, 200);
    assert_eq!(
        fs::read_to_string(notes.path().join("index.md")).unwrap(),
        "- hello\n"
    );
}

#[test]
fn write_a_home_page() {
    let notes = Notes::new(&[("journals/2026-10-12.md", "- [[index]]\n")]);
    let server = start(notes.path());
    let r = get_source(server.addr, "index.html");
    assert_eq!(r.status, 200);
    assert!(r.body.starts_with(r#"{"exists":false,"#), "{}", r.body);
    let r = put_source(server.addr, "index.html", "missing", "- welcome\n");
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(
        fs::read_to_string(notes.path().join("index.md")).unwrap(),
        "- welcome\n"
    );
    assert!(!notes.path().join("pages/index.md").exists());
    assert!(page(server.addr, "index.html").contains("welcome"));
    // Also when nothing references the page `index`.
    let empty = Notes::new(&[("journals/2026-10-12.md", "- day\n")]);
    let server = start(empty.path());
    assert_eq!(
        put_source(server.addr, "index.html", "missing", "- hi\n").status,
        200
    );
    assert_eq!(
        fs::read_to_string(empty.path().join("index.md")).unwrap(),
        "- hi\n"
    );
}

#[test]
fn request_authenticity() {
    let notes = Notes::new(&[("pages/auth.md", "- a\n")]);
    let server = start(notes.path());
    let addr = server.addr;
    let port = addr.port();
    let target = source_url("pages/auth.html");
    let version = field(&get_source(addr, "pages/auth.html").body, "version");
    let ok = editor_headers(addr, &version);

    let refused = |headers: Vec<(&'static str, String)>, why: &str| {
        let r = send(addr, "PUT", &target, &headers, "- changed\n");
        assert_eq!(r.status, 403, "{why}");
        assert_eq!(
            fs::read_to_string(notes.path().join("pages/auth.md")).unwrap(),
            "- a\n",
            "{why}"
        );
    };
    refused(
        vec![ok[0].clone(), ok[2].clone()],
        "without the edit header",
    );
    refused(vec![ok[1].clone(), ok[2].clone()], "without an origin");
    refused(
        vec![
            ("Origin", "https://evil.example".to_owned()),
            ok[1].clone(),
            ok[2].clone(),
        ],
        "another website",
    );
    // DNS rebinding: a foreign name with a matching origin.
    let evil = format!("evil.example:{port}");
    refused(
        vec![
            ("Host", evil.clone()),
            ("Origin", format!("http://{evil}")),
            ok[1].clone(),
            ok[2].clone(),
        ],
        "rebinding",
    );
    assert_eq!(
        send(addr, "GET", &target, &[("Host", evil)], "").status,
        403
    );
    // Preflights are not answered, so browsers never send cross-site writes.
    assert_eq!(send(addr, "OPTIONS", &target, &[], "").status, 405);
    // Other methods on pages.
    assert_eq!(
        send(addr, "PUT", "/pages/auth.html", &ok, "- x\n").status,
        405
    );
    assert_eq!(send(addr, "POST", "/pages/auth.html", &[], "").status, 405);
    // Bodies.
    let mut no_length = ok.clone();
    no_length.push(("Transfer-Encoding", "chunked".to_owned()));
    assert_eq!(send(addr, "PUT", &target, &no_length, "").status, 411);
    let big = "x".repeat(8 * 1024 * 1024 + 1);
    assert_eq!(send(addr, "PUT", &target, &ok, &big).status, 413);
    assert_eq!(
        fs::read_to_string(notes.path().join("pages/auth.md")).unwrap(),
        "- a\n"
    );
}

#[test]
fn editor_element_only_for_this_machine() {
    let notes = Notes::new(&[("pages/a.md", "- a\n")]);
    let server = start(notes.path());
    assert!(page(server.addr, "pages/a.html").contains("<hermetic-editor></hermetic-editor>"));
    let (_dir, site, _) = common::build_site(notes.path());
    let built = fs::read_to_string(site.join("pages/a.html")).unwrap();
    assert!(!built.contains("hermetic-editor"));
}

#[cfg(unix)]
#[test]
fn symbolic_link_out_of_the_notes() {
    let outside = Notes::new(&[("secret.md", "- secret\n")]);
    let notes = Notes::new(&[("journals/2026-10-12.md", "- [[out]]\n")]);
    fs::create_dir_all(notes.path().join("pages")).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("secret.md"),
        notes.path().join("pages/out.md"),
    )
    .unwrap();
    let server = start(notes.path());
    assert_eq!(get_source(server.addr, "pages/out.html").status, 403);
    let r = put_source(server.addr, "pages/out.html", "missing", "- overwritten\n");
    assert_eq!(r.status, 403);
    assert_eq!(
        fs::read_to_string(outside.path().join("secret.md")).unwrap(),
        "- secret\n"
    );
}

/// The source payloads of `tests/fixtures/editing/` are the input of the editor's
/// TypeScript tests (`static/packages/editor/test-fixtures/`), so both sides agree on
/// how files are split into blocks.
#[test]
fn typescript_test_fixtures_are_current() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = root.join("static/packages/editor/test-fixtures");
    let mut expected = std::collections::BTreeMap::new();
    for entry in fs::read_dir(root.join("tests/fixtures/editing"))
        .unwrap()
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().replace(".md", ".json");
        let json = hermetic::serve::edit::source_json(
            &Path::new("pages").join(entry.file_name()),
            Some(&fs::read(entry.path()).unwrap()),
        )
        .unwrap();
        expected.insert(
            std::path::PathBuf::from(name),
            format!("{json}\n").into_bytes(),
        );
    }
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        let _ = fs::remove_dir_all(&target);
        fs::create_dir_all(&target).unwrap();
        for (name, bytes) in &expected {
            fs::write(target.join(name), bytes).unwrap();
        }
    }
    assert!(
        target.is_dir() && common::tree(&target) == expected,
        "static/packages/editor/test-fixtures is out of date: run UPDATE_SNAPSHOTS=1 cargo test --test edit"
    );
}

fn delete_source(addr: SocketAddr, path: &str, version: &str) -> Response {
    send(
        addr,
        "DELETE",
        &source_url(path),
        &editor_headers(addr, version),
        "",
    )
}

/// Files under `.trash/`, relative to it, except its `.gitignore`.
fn trashed(notes: &Path) -> Vec<String> {
    common::tree(&notes.join(".trash"))
        .keys()
        .filter(|p| p.as_os_str() != ".gitignore")
        .map(|p| p.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn delete_a_page() {
    let notes = Notes::new(&[
        ("pages/scratch.md", "- temporary\n"),
        ("journals/2026-10-12.md", "- day\n"),
    ]);
    let server = start(notes.path());
    let version = field(
        &get_source(server.addr, "pages/scratch.html").body,
        "version",
    );
    let before = server.rebuilds();
    let r = delete_source(server.addr, "pages/scratch.html", &version);
    assert_eq!(r.status, 200, "{}", r.body);
    assert!(!notes.path().join("pages/scratch.md").exists());
    let trash = trashed(notes.path());
    assert_eq!(trash.len(), 1);
    assert!(
        trash[0].ends_with("pages/scratch.md") && r.body.contains(&trash[0]),
        "{trash:?} {}",
        r.body
    );
    assert_eq!(
        fs::read_to_string(notes.path().join(".trash").join(&trash[0])).unwrap(),
        "- temporary\n"
    );
    // Rebuilt before the response; once, and the trash is not picked up.
    assert!(!page(server.addr, "pages/scratch.html").contains("temporary"));
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(server.rebuilds(), before + 1);
    assert_eq!(
        fs::read_to_string(notes.path().join("journals/2026-10-12.md")).unwrap(),
        "- day\n"
    );
    // Git ignores the trash.
    let ignore = fs::read_to_string(notes.path().join(".trash/.gitignore")).unwrap();
    assert!(ignore.lines().any(|l| l == "*"), "{ignore}");
}

#[test]
fn delete_keeps_an_existing_trash_gitignore() {
    let notes = Notes::new(&[
        ("pages/a.md", "- a\n"),
        ("pages/b.md", "- b\n"),
        (".trash/.gitignore", ""),
    ]);
    let server = start(notes.path());
    for name in ["a", "b"] {
        let path = format!("pages/{name}.html");
        let v = field(&get_source(server.addr, &path).body, "version");
        assert_eq!(delete_source(server.addr, &path, &v).status, 200);
    }
    assert_eq!(trashed(notes.path()).len(), 2);
    // Empty, as the author left it.
    assert_eq!(
        fs::read_to_string(notes.path().join(".trash/.gitignore")).unwrap(),
        ""
    );
}

#[test]
fn delete_keeps_referenced_pages_and_prunes_directories() {
    let notes = Notes::new(&[
        ("pages/redis.md", "- cache\n"),
        ("pages/Project X/Auth.md", "- auth\n"),
        ("journals/2026-10-12.md", "- [[redis]]\n"),
    ]);
    let server = start(notes.path());
    let v = field(&get_source(server.addr, "pages/redis.html").body, "version");
    assert_eq!(
        delete_source(server.addr, "pages/redis.html", &v).status,
        200
    );
    let redis = page(server.addr, "pages/redis.html");
    assert!(
        !redis.contains("cache") && redis.contains("Linked references"),
        "{redis}"
    );
    // The page has no file now: deleting again is 404.
    assert_eq!(
        delete_source(server.addr, "pages/redis.html", &v).status,
        404
    );

    let v = field(
        &get_source(server.addr, "pages/Project X/Auth.html").body,
        "version",
    );
    assert_eq!(
        delete_source(server.addr, "pages/Project X/Auth.html", &v).status,
        200
    );
    assert!(!notes.path().join("pages/Project X").exists());
    assert!(notes.path().join("pages").is_dir());
    assert_eq!(trashed(notes.path()).len(), 2);
}

#[test]
fn delete_with_a_stale_version_or_from_elsewhere() {
    let notes = Notes::new(&[("pages/a.md", "- a\n")]);
    let server = start(notes.path());
    let addr = server.addr;
    let v = field(&get_source(addr, "pages/a.html").body, "version");
    notes.write("pages/a.md", b"- changed\n");
    let r = delete_source(addr, "pages/a.html", &v);
    assert_eq!(r.status, 409);
    assert!(r.body.contains("changed"));
    let v = field(&r.body, "version");
    // Cross-site: other origin, or no edit header.
    let target = source_url("pages/a.html");
    let evil = vec![
        ("Origin", "https://evil.example".to_owned()),
        ("X-Hermetic-Edit", "1".to_owned()),
        ("X-Hermetic-Version", v.clone()),
    ];
    assert_eq!(send(addr, "DELETE", &target, &evil, "").status, 403);
    let no_header = vec![
        editor_headers(addr, &v)[0].clone(),
        ("X-Hermetic-Version", v.clone()),
    ];
    assert_eq!(send(addr, "DELETE", &target, &no_header, "").status, 403);
    assert_eq!(
        send(
            addr,
            "DELETE",
            "/pages/a.html",
            &editor_headers(addr, &v),
            ""
        )
        .status,
        405
    );
    assert!(notes.path().join("pages/a.md").exists());
    assert!(!notes.path().join(".trash").exists());
}

#[cfg(unix)]
#[test]
fn delete_never_leaves_the_notes() {
    let outside = Notes::empty();
    let notes = Notes::new(&[("pages/a.md", "- a\n")]);
    // A `.trash` that is a symbolic link out of the notes is refused.
    std::os::unix::fs::symlink(outside.path(), notes.path().join(".trash")).unwrap();
    let server = start(notes.path());
    let v = field(&get_source(server.addr, "pages/a.html").body, "version");
    assert_eq!(delete_source(server.addr, "pages/a.html", &v).status, 403);
    assert!(notes.path().join("pages/a.md").exists());
    // Not even an empty directory was created there.
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}
