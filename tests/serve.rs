//! Scenarios from the `serve-mode` spec, against a real server on an ephemeral port.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use common::{FIXTURE, Notes, build_site};
use hermetic::serve::{self, Bind, Running, watch::Timing};

fn fast() -> Timing {
    Timing {
        poll: Duration::from_millis(30),
        quiet: Duration::from_millis(60),
    }
}

fn start(notes: &Path) -> Running {
    serve::start(serve::Options {
        notes: notes.to_path_buf(),
        bind: Bind::Loopback,
        port: Some(0),
        title: None,
        timing: fast(),
    })
    .expect("server starts")
}

struct Response {
    status: u16,
    headers: String,
    body: Vec<u8>,
}

fn request(addr: SocketAddr, method: &str, target: &str, host: Option<&str>) -> Response {
    let mut stream = TcpStream::connect(addr).unwrap();
    let host = host.map_or_else(|| format!("127.0.0.1:{}", addr.port()), str::to_owned);
    write!(stream, "{method} {target} HTTP/1.1\r\nHost: {host}\r\n\r\n").unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("header end");
    let headers = String::from_utf8(raw[..split].to_vec()).unwrap();
    let status = headers[9..12].parse().unwrap();
    Response {
        status,
        headers,
        body: raw[split + 4..].to_vec(),
    }
}

fn get(addr: SocketAddr, target: &str) -> Response {
    request(addr, "GET", target, None)
}

/// Every file under `dir`, keyed by its path relative to `dir`.
fn files(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                out.insert(
                    p.strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(&p).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

fn url(path: &Path) -> String {
    let segments: Vec<String> = path
        .components()
        .map(|c| hermetic::site::encode_url_segment(&c.as_os_str().to_string_lossy()))
        .collect();
    format!("/{}", segments.join("/"))
}

/// Waits until `cond` holds or `limit` passes; returns how long it took.
fn within(limit: Duration, mut cond: impl FnMut() -> bool) -> Option<Duration> {
    let start = Instant::now();
    while start.elapsed() < limit {
        if cond() {
            return Some(start.elapsed());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    None
}

/// Reads server-sent events from the events endpoint.
struct Events(BufReader<TcpStream>);

impl Events {
    fn connect(addr: SocketAddr) -> Events {
        let mut stream = TcpStream::connect(addr).unwrap();
        write!(
            stream,
            "GET /_hermetic/events HTTP/1.1\r\nHost: localhost:{}\r\n\r\n",
            addr.port()
        )
        .unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .unwrap();
        let mut events = Events(BufReader::new(stream));
        let first = events.line().expect("status line");
        assert!(first.starts_with("HTTP/1.1 200"), "{first}");
        events
    }

    fn line(&mut self) -> Option<String> {
        let mut line = String::new();
        match self.0.read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line),
        }
    }

    /// `data:` payloads received within `wait`.
    fn data(&mut self, wait: Duration) -> Vec<String> {
        let start = Instant::now();
        let mut out = Vec::new();
        while start.elapsed() < wait {
            if let Some(line) = self.line()
                && let Some(json) = line.strip_prefix("data: ")
            {
                out.push(json.trim().to_owned());
            }
        }
        out
    }
}

#[test]
fn custom_site_title() {
    let notes = Notes::new(&[("pages/a.md", "- a")]);
    let server = serve::start(serve::Options {
        notes: notes.path().to_path_buf(),
        bind: Bind::Loopback,
        port: Some(0),
        title: Some("My Wiki".to_owned()),
        timing: fast(),
    })
    .expect("server starts");
    let body = String::from_utf8(get(server.addr, "/pages/a.html").body).unwrap();
    assert!(body.contains("<title>a · My Wiki</title>"), "{body}");
}

#[test]
fn same_output_as_build() {
    let (_h, built, _) = build_site(Path::new(FIXTURE));
    let server = start(Path::new(FIXTURE));
    let mut html = 0;
    for (path, bytes) in files(&built) {
        let r = get(server.addr, &url(&path));
        assert_eq!(r.status, 200, "{}", path.display());
        if path.extension().is_some_and(|e| e == "html") {
            let expected = hermetic::serve::http::inject_elements(&bytes, true);
            assert_eq!(r.body, expected, "{}", path.display());
            assert_eq!(
                r.body.len(),
                bytes.len()
                    + "<hermetic-live-reload></hermetic-live-reload><hermetic-editor></hermetic-editor>\n"
                        .len()
            );
            html += 1;
        } else {
            assert_eq!(r.body, bytes, "{}", path.display());
        }
    }
    assert!(html > 5);
}

#[test]
fn http_behavior() {
    let notes = Notes::new(&[
        ("pages/Project X/Auth Flow.md", "- x"),
        ("pages/a.md", "- [[Project X/Auth Flow]]"),
    ]);
    notes.write("pages/pic.png", b"\x89PNG");
    let server = start(notes.path());
    let r = get(server.addr, "/pages/Project%20X/Auth%20Flow.html");
    assert_eq!(r.status, 200);
    assert!(r.headers.contains("Content-Type: text/html; charset=utf-8"));
    assert!(r.headers.contains("Cache-Control: no-store"));
    assert!(String::from_utf8(r.body).unwrap().contains(
        "<hermetic-live-reload></hermetic-live-reload><hermetic-editor></hermetic-editor>\n</body>"
    ));
    let r = get(server.addr, "/pages/pic.png");
    assert!(r.headers.contains("Content-Type: image/png") && r.body == b"\x89PNG");
    assert!(
        get(server.addr, "/_hermetic/app.js")
            .headers
            .contains("text/javascript")
    );
    let home = get(server.addr, "/");
    assert_eq!(home.status, 200);
    assert_eq!(home.body, get(server.addr, "/index.html").body);
    for target in [
        "/../../etc/passwd",
        "/%2e%2e/notes.md",
        "/pages/a.md",
        "/nope.html",
    ] {
        let r = get(server.addr, target);
        assert_eq!(r.status, 404, "{target}");
        assert!(
            String::from_utf8(r.body)
                .unwrap()
                .contains("<a href=\"/\">")
        );
    }
    let head = request(server.addr, "HEAD", "/pages/a.html", None);
    assert_eq!(head.status, 200);
    assert!(head.body.is_empty() && head.headers.contains("Content-Length: "));
    assert_eq!(
        request(server.addr, "POST", "/pages/a.html", None).status,
        405
    );
}

#[test]
fn rebinding_protection() {
    let notes = Notes::new(&[("pages/a.md", "- secret")]);
    let server = start(notes.path());
    let port = server.addr.port();
    let evil = request(
        server.addr,
        "GET",
        "/pages/a.html",
        Some(&format!("evil.example:{port}")),
    );
    assert_eq!(evil.status, 403);
    assert!(!String::from_utf8(evil.body).unwrap().contains("secret"));
    for host in [
        format!("localhost:{port}"),
        format!("127.0.0.1:{port}"),
        format!("[::1]:{port}"),
    ] {
        assert_eq!(
            request(server.addr, "GET", "/pages/a.html", Some(&host)).status,
            200,
            "{host}"
        );
    }
}

#[test]
fn new_page_appears_and_deleted_note_disappears() {
    let notes = Notes::new(&[
        ("journals/2026-10-12.md", "- hello"),
        ("pages/old.md", "- old"),
    ]);
    let server = start(notes.path());
    assert_eq!(get(server.addr, "/pages/kafka.html").status, 404);
    notes.write("journals/2026-10-13.md", b"- trying [[kafka]]");
    let took = within(Duration::from_secs(2), || {
        get(server.addr, "/pages/kafka.html").status == 200
            && get(server.addr, "/journals/2026-10-13.html").status == 200
    });
    assert!(took.is_some(), "new page not served within 2s");
    let list = String::from_utf8(get(server.addr, "/journals.html").body).unwrap();
    assert!(list.contains("2026-10-13"));

    fs::remove_file(notes.path().join("pages/old.md")).unwrap();
    assert!(
        within(Duration::from_secs(2), || get(
            server.addr,
            "/pages/old.html"
        )
        .status
            == 404)
        .is_some()
    );
}

#[test]
fn burst_of_saves_rebuilds_once() {
    let notes = Notes::new(&[("pages/a.md", "- v0")]);
    let server = start(notes.path());
    let mut events = Events::connect(server.addr);
    for i in 1..=5 {
        notes.write("pages/a.md", format!("- v{i}").as_bytes());
        std::thread::sleep(Duration::from_millis(20));
    }
    let data = events.data(Duration::from_millis(800));
    assert_eq!(data.len(), 1, "{data:?}");
    assert!(
        String::from_utf8(get(server.addr, "/pages/a.html").body)
            .unwrap()
            .contains("v5")
    );
}

#[test]
fn events_list_exactly_the_affected_pages() {
    let notes = Notes::new(&[
        ("pages/redis.md", "- cache"),
        ("pages/unrelated.md", "- other"),
        ("journals/2026-10-12.md", "- looked at [[redis]]"),
    ]);
    let before = {
        let (g, mut d) = hermetic::Graph::load(&fs::canonicalize(notes.path()).unwrap());
        hermetic::site::render(
            &g,
            &hermetic::site::title_for(notes.path()),
            notes.path(),
            &mut d,
        )
        .unwrap()
        .files
    };
    let server = start(notes.path());
    let mut events = Events::connect(server.addr);
    notes.write(
        "journals/2026-10-12.md",
        b"- looked at [[redis]] and [[kafka]]",
    );
    let data = events.data(Duration::from_millis(800));
    assert_eq!(data.len(), 1, "{data:?}");
    let after = {
        let (g, mut d) = hermetic::Graph::load(&fs::canonicalize(notes.path()).unwrap());
        hermetic::site::render(
            &g,
            &hermetic::site::title_for(notes.path()),
            notes.path(),
            &mut d,
        )
        .unwrap()
        .files
    };
    let mut expected: Vec<String> = after
        .iter()
        .filter(|(p, c)| before.get(*p) != Some(*c))
        .map(|(p, _)| p.to_string_lossy().into_owned())
        .collect();
    expected.sort();
    let json = &data[0];
    for path in &expected {
        assert!(
            json.contains(&format!("\"{path}\"")),
            "{path} missing from {json}"
        );
    }
    assert_eq!(
        json.matches(".html\"").count(),
        expected.iter().filter(|p| p.ends_with(".html")).count(),
        "{json} vs {expected:?}"
    );
    assert!(json.contains("\"pages/kafka.html\"") && json.contains("\"pages/redis.html\""));
    assert!(!json.contains("unrelated"), "{json}");
    assert!(json.ends_with(r#""removed":[]}"#), "{json}");
}

#[test]
fn adding_a_task_elsewhere_changes_only_the_count_and_its_pages() {
    let notes = Notes::new(&[
        ("pages/redis.md", "- cache\n- TODO tune"),
        ("pages/unrelated.md", "- other"),
        ("journals/2026-10-12.md", "- looked at [[redis]]"),
    ]);
    let server = start(notes.path());
    assert_eq!(
        get(server.addr, "/_hermetic/open-tasks.json").body,
        br#"{"count":1}"#
    );
    let mut events = Events::connect(server.addr);
    notes.write("pages/unrelated.md", b"- other\n- TODO new task");
    let data = events.data(Duration::from_millis(800));
    assert_eq!(data.len(), 1, "{data:?}");
    let json = &data[0];
    for path in [
        "_hermetic/open-tasks.json",
        "pages/unrelated.html",
        "tasks.html",
    ] {
        assert!(
            json.contains(&format!("\"{path}\"")),
            "{path} missing from {json}"
        );
    }
    assert_eq!(json.matches(".html\"").count(), 2, "{json}");
    assert!(
        !json.contains("redis") && !json.contains("journals/"),
        "{json}"
    );
    assert_eq!(
        get(server.addr, "/_hermetic/open-tasks.json").body,
        br#"{"count":2}"#
    );
}

#[test]
fn rebuild_failure_keeps_serving() {
    // An unreadable attachment directory makes rendering fail; the old build stays.
    use std::os::unix::fs::PermissionsExt;
    let notes = Notes::new(&[("pages/a.md", "- first"), ("assets/x.txt", "x")]);
    let server = start(notes.path());
    let assets = notes.path().join("assets");
    fs::set_permissions(&assets, fs::Permissions::from_mode(0o000)).unwrap();
    notes.write("pages/a.md", b"- second");
    std::thread::sleep(Duration::from_millis(500));
    let body = String::from_utf8(get(server.addr, "/pages/a.html").body).unwrap();
    fs::set_permissions(&assets, fs::Permissions::from_mode(0o755)).unwrap();
    if std::env::var("USER").is_ok_and(|u| u == "root") {
        return; // root can read anyway
    }
    assert!(body.contains("first"), "{body}");
}

/// Run with `make test-perf` (release mode).
#[test]
#[ignore = "performance check; run in release mode"]
fn change_served_within_two_seconds_at_5000_pages() {
    let notes = Notes::empty();
    for i in 0..5000u32 {
        let day = 1 + i % 28;
        let month = 1 + (i / 28) % 12;
        let year = 2000 + i / 336;
        notes.write(
            &format!("journals/{year}-{month:02}-{day:02}.md"),
            format!(
                "- standup about [[project {}]] and #topic{}\n  - TODO follow up\n",
                i % 50,
                i % 200
            )
            .as_bytes(),
        );
    }
    let server = serve::start(serve::Options {
        notes: notes.path().to_path_buf(),
        bind: Bind::Loopback,
        port: Some(0),
        title: None,
        timing: Timing::default(),
    })
    .unwrap();
    notes.write("pages/fresh.md", b"- a brand new page");
    let took = within(Duration::from_secs(5), || {
        get(server.addr, "/pages/fresh.html").status == 200
    });
    eprintln!("change served after {took:?}");
    assert!(took.is_some_and(|t| t < Duration::from_secs(2)), "{took:?}");
}
