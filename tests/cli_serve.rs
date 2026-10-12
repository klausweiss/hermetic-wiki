//! `hermetic serve` through the built binary: output, options, ports, and exit codes.

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use common::Notes;

/// A running `hermetic serve`, killed on drop.
struct Server {
    child: Child,
    lines: mpsc::Receiver<String>,
}

impl Server {
    fn start(cwd: &Path, args: &[&str]) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_hermetic"))
            .arg("serve")
            .args(args)
            .current_dir(cwd)
            .stderr(Stdio::piped())
            .spawn()
            .expect("runs");
        let stderr = child.stderr.take().unwrap();
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let _ = tx.send(line);
            }
        });
        Server { child, lines }
    }

    /// Lines printed up to and including the one containing `needle`.
    fn until(&self, needle: &str) -> Vec<String> {
        let mut seen = Vec::new();
        while let Ok(line) = self.lines.recv_timeout(Duration::from_secs(10)) {
            let done = line.contains(needle);
            seen.push(line);
            if done {
                return seen;
            }
        }
        panic!("`{needle}` not printed; got {seen:?}");
    }

    fn url(&self, label: &str) -> String {
        let line = self.until(label).pop().unwrap();
        line.split_whitespace().last().unwrap().to_owned()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn status_of(url: &str) -> u16 {
    let rest = url.strip_prefix("http://").unwrap();
    let (authority, path) = rest.split_once('/').unwrap();
    let mut stream = TcpStream::connect(authority).unwrap();
    write!(stream, "GET /{path} HTTP/1.1\r\nHost: {authority}\r\n\r\n").unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response[9..12].parse().unwrap()
}

fn run(cwd: &Path, args: &[&str]) -> (Option<i32>, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_hermetic"))
        .arg("serve")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    (
        o.status.code(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

#[test]
fn serve_the_current_directory() {
    let notes = Notes::new(&[
        ("pages/a.md", "- a"),
        ("journals/2026-10-12.md", "- [[a]] ((nope))"),
    ]);
    let server = Server::start(notes.path(), &[]);
    let lines = server.until("Local:");
    assert!(
        lines
            .iter()
            .any(|l| l.contains("warning: journals/2026-10-12.md:1: no block has id `nope`")),
        "{lines:?}"
    );
    let url = lines
        .last()
        .unwrap()
        .split_whitespace()
        .last()
        .unwrap()
        .to_owned();
    assert!(url.starts_with("http://127.0.0.1:80"), "{url}");
    assert_eq!(status_of(&url), 200);
    assert_eq!(status_of(&format!("{url}pages/a.html")), 200);
    assert!(
        !notes.path().join("site").exists(),
        "serve must not write the site"
    );
}

#[test]
fn default_binding_skips_a_busy_port() {
    let notes = Notes::new(&[("pages/a.md", "- a")]);
    // Hold 8000 if it is free; otherwise something else already does.
    let _holder = TcpListener::bind("127.0.0.1:8000");
    let server = Server::start(notes.path(), &[]);
    let url = server.url("Local:");
    let port: u16 = url
        .trim_end_matches('/')
        .rsplit(':')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!((8001..=8099).contains(&port), "{url}");
}

#[test]
fn exact_port_that_is_busy_fails() {
    let notes = Notes::new(&[("pages/a.md", "- a")]);
    let holder = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = holder.local_addr().unwrap().port().to_string();
    let (code, err) = run(notes.path(), &["--port", &port]);
    assert_eq!(code, Some(1), "{err}");
    assert!(err.contains(&format!("port {port}")), "{err}");
}

#[test]
fn exact_port_and_host() {
    let notes = Notes::new(&[("pages/a.md", "- a")]);
    let free = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
        .to_string();
    let server = Server::start(notes.path(), &["--host", "127.0.0.1", "--port", &free]);
    let url = server.url("Local:");
    assert_eq!(url, format!("http://127.0.0.1:{free}/"));
    assert_eq!(status_of(&url), 200);
}

#[test]
fn public_prints_and_answers_on_the_network_address() {
    let notes = Notes::new(&[("pages/a.md", "- a")]);
    let free = TcpListener::bind("0.0.0.0:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
        .to_string();
    let server = Server::start(notes.path(), &["--public", "--port", &free]);
    let lines = server.until("Local:");
    let local = lines
        .last()
        .unwrap()
        .split_whitespace()
        .last()
        .unwrap()
        .to_owned();
    assert_eq!(local, format!("http://localhost:{free}/"));
    assert_eq!(status_of(&format!("http://127.0.0.1:{free}/")), 200);
    // A machine without a network has no local-network address to show.
    match server.lines.recv_timeout(Duration::from_secs(2)) {
        Ok(line) if line.contains("On your network:") => {
            let network = line.split_whitespace().last().unwrap();
            assert!(
                !network.contains("127.0.0.1") && !network.contains("localhost"),
                "{network}"
            );
            assert_eq!(status_of(network), 200, "{network}");
        }
        other => eprintln!("no network URL printed: {other:?}"),
    }
}

#[test]
fn errors_and_usage() {
    let notes = Notes::empty();
    let (code, err) = run(notes.path(), &["nope"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("nope is not a directory"), "{err}");
    for args in [
        &["--port", "abc"][..],
        &["--port"],
        &["--host", "not-an-ip"],
        &["--host", "127.0.0.1", "--public"],
        &["--frob"],
        &["a", "b"],
    ] {
        let (code, err) = run(notes.path(), args);
        assert_eq!(code, Some(2), "{args:?}: {err}");
        assert!(err.contains("Usage:"), "{args:?}");
    }
}
