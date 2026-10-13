//! `hermetic serve`: builds the site in memory, serves it over HTTP, rebuilds when the
//! notes change, and tells open pages which files changed.

pub mod edit;
pub mod http;
pub mod watch;

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{self, BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use crate::diag::Diagnostic;
use crate::graph::Graph;
use crate::site::{self, Content, SiteFiles};

/// Which addresses the server listens on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bind {
    /// `127.0.0.1`, with `Host` checks against DNS rebinding.
    Loopback,
    /// A chosen address (`--host`).
    Host(IpAddr),
    /// All interfaces (`--public`).
    Public,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub notes: PathBuf,
    pub bind: Bind,
    /// Exact port; `None` tries 8000 to 8099.
    pub port: Option<u16>,
    /// Site title; `None` uses the notes directory's name.
    pub title: Option<String>,
    pub timing: watch::Timing,
}

#[derive(Debug)]
pub enum ServeError {
    NotADirectory(PathBuf),
    Bind(String),
    Build(io::Error),
}

impl fmt::Display for ServeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServeError::NotADirectory(p) => write!(f, "{} is not a directory", p.display()),
            ServeError::Bind(msg) => f.write_str(msg),
            ServeError::Build(e) => write!(f, "cannot build the site: {e}"),
        }
    }
}

impl std::error::Error for ServeError {}

/// Default port range when no port is given.
const PORTS: std::ops::RangeInclusive<u16> = 8000..=8099;
/// Interval of keep-alive comments on idle event streams.
const KEEP_ALIVE: Duration = Duration::from_secs(15);

/// One successful build.
struct Build {
    files: SiteFiles,
    /// Size and modification time of each attachment, to detect attachment changes.
    attachments: BTreeMap<PathBuf, (u64, Option<SystemTime>)>,
    /// Pages the editor may change, by site path.
    editable: edit::EditMap,
}

struct State {
    notes: PathBuf,
    title: String,
    bind: Bind,
    port: u16,
    build: RwLock<Arc<Build>>,
    clients: Mutex<Vec<Sender<String>>>,
    /// Held while rebuilding or saving, so saves and rebuilds never interleave.
    rebuilding: Mutex<()>,
    /// The inputs as of the last rebuild; saves update it so the watcher does not
    /// rebuild a second time for them.
    snapshot: Mutex<watch::Snapshot>,
    /// Diagnostics printed after the last rebuild, to print only when they change.
    diagnostics: Mutex<Vec<String>>,
    /// Number of rebuilds since the start.
    rebuilds: AtomicUsize,
}

/// A running server.
pub struct Running {
    pub addr: SocketAddr,
    /// URLs to print: the local one first, then a local-network one for `--public`.
    pub urls: Vec<String>,
    /// Diagnostics of the initial build.
    pub diagnostics: Vec<Diagnostic>,
    pub pages: usize,
    accept: thread::JoinHandle<()>,
    state: Arc<State>,
}

impl Running {
    /// Number of rebuilds since the server started.
    pub fn rebuilds(&self) -> usize {
        self.state.rebuilds.load(Ordering::SeqCst)
    }

    /// Blocks while the server runs (until the process is interrupted).
    pub fn wait(self) {
        let _ = self.accept.join();
    }
}

/// Builds the site, binds the listener, and starts serving and watching in background
/// threads. Rebuild reports are printed to standard error.
pub fn start(options: Options) -> Result<Running, ServeError> {
    let notes = match fs::canonicalize(&options.notes) {
        Ok(p) if p.is_dir() => p,
        _ => return Err(ServeError::NotADirectory(options.notes.clone())),
    };
    let title = options.title.unwrap_or_else(|| site::title_for(&notes));
    let initial_snapshot = watch::snapshot(&notes);
    let (build, pages, diagnostics) = build_site(&notes, &title).map_err(ServeError::Build)?;

    let listener = bind(options.bind, options.port)?;
    let addr = listener
        .local_addr()
        .map_err(|e| ServeError::Bind(e.to_string()))?;
    let urls = urls(options.bind, addr);

    let state = Arc::new(State {
        notes,
        title,
        bind: options.bind,
        port: addr.port(),
        build: RwLock::new(Arc::new(build)),
        clients: Mutex::new(Vec::new()),
        rebuilding: Mutex::new(()),
        snapshot: Mutex::new(initial_snapshot),
        diagnostics: Mutex::new(diagnostics.iter().map(ToString::to_string).collect()),
        rebuilds: AtomicUsize::new(0),
    });

    let watcher = Arc::clone(&state);
    thread::spawn(move || {
        loop {
            watch::wait_for_change(&watcher.notes, &watcher.snapshot, options.timing);
            let _rebuilding = watcher.rebuilding.lock().expect("rebuild lock");
            // A save may have rebuilt for this change already.
            let now = watch::snapshot(&watcher.notes);
            if now == *watcher.snapshot.lock().expect("snapshot lock") {
                continue;
            }
            *watcher.snapshot.lock().expect("snapshot lock") = now;
            rebuild(&watcher);
        }
    });

    let server = Arc::clone(&state);
    let accept = thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let state = Arc::clone(&server);
            thread::spawn(move || {
                let _ = handle(&state, stream);
            });
        }
    });

    Ok(Running {
        addr,
        urls,
        diagnostics,
        pages,
        accept,
        state,
    })
}

fn build_site(notes: &Path, title: &str) -> io::Result<(Build, usize, Vec<Diagnostic>)> {
    let (graph, mut diagnostics) = Graph::load(notes);
    let rendered = site::render(&graph, title, notes, &mut diagnostics)?;
    let attachments = rendered
        .files
        .iter()
        .filter_map(|(rel, content)| match content {
            Content::File(source) => Some((rel.clone(), fingerprint(source))),
            Content::Bytes(_) => None,
        })
        .collect();
    let build = Build {
        files: rendered.files,
        attachments,
        editable: edit::editable_pages(&graph),
    };
    Ok((build, rendered.pages, diagnostics))
}

fn fingerprint(path: &Path) -> (u64, Option<SystemTime>) {
    fs::metadata(path).map_or((0, None), |m| (m.len(), m.modified().ok()))
}

/// Rebuilds after a change, swaps the served build, and notifies open pages. Callers
/// hold `state.rebuilding`.
fn rebuild(state: &State) {
    let started = Instant::now();
    let (build, pages, diagnostics) = match build_site(&state.notes, &state.title) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: rebuild failed, still serving the previous build: {e}");
            return;
        }
    };
    state.rebuilds.fetch_add(1, Ordering::SeqCst);
    let old = Arc::clone(&state.build.read().expect("build lock"));
    let (changed, removed) = diff(&old, &build);
    *state.build.write().expect("build lock") = Arc::new(build);

    let messages: Vec<String> = diagnostics.iter().map(ToString::to_string).collect();
    let mut last_diagnostics = state.diagnostics.lock().expect("diagnostics lock");
    if messages != *last_diagnostics {
        for m in &messages {
            eprintln!("warning: {m}");
        }
        *last_diagnostics = messages;
    }
    drop(last_diagnostics);
    eprintln!(
        "rebuilt {pages} pages in {} ms: {} changed, {} removed ({} warnings)",
        started.elapsed().as_millis(),
        changed.len(),
        removed.len(),
        diagnostics.len()
    );
    if !changed.is_empty() || !removed.is_empty() {
        broadcast(state, &event_json(&changed, &removed));
    }
}

/// Site paths whose content differs between two builds, and paths that disappeared.
fn diff(old: &Build, new: &Build) -> (Vec<String>, Vec<String>) {
    let mut changed = Vec::new();
    for (path, content) in &new.files {
        let same = match (old.files.get(path), content) {
            (Some(Content::Bytes(a)), Content::Bytes(b)) => a == b,
            (Some(Content::File(_)), Content::File(_)) => {
                old.attachments.get(path) == new.attachments.get(path)
            }
            _ => false,
        };
        if !same {
            changed.push(url_path(path));
        }
    }
    let removed = old
        .files
        .keys()
        .filter(|p| !new.files.contains_key(*p))
        .map(|p| url_path(p))
        .collect();
    (changed, removed)
}

/// A site path with `/` separators, not percent-encoded.
fn url_path(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn event_json(changed: &[String], removed: &[String]) -> String {
    let list = |items: &[String]| {
        let quoted: Vec<String> = items.iter().map(|s| json_string(s)).collect();
        format!("[{}]", quoted.join(","))
    };
    format!(
        r#"{{"changed":{},"removed":{}}}"#,
        list(changed),
        list(removed)
    )
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn broadcast(state: &State, json: &str) {
    let mut clients = state.clients.lock().expect("clients lock");
    clients.retain(|tx| tx.send(json.to_owned()).is_ok());
}

fn handle(state: &State, mut stream: TcpStream) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let peer = stream.peer_addr()?.ip();
    let mut reader = BufReader::new(stream.try_clone()?);
    let request = match http::read_request(&mut reader) {
        Ok(r) => r,
        Err(_) => {
            return stream.write_all(&http::response(400, "text/plain", b"Bad Request\n", false));
        }
    };
    if state.bind == Bind::Loopback
        && !request
            .header("host")
            .is_some_and(|h| http::is_loopback_host(h, state.port))
    {
        return stream.write_all(&http::response(403, "text/plain", b"Forbidden\n", false));
    }
    let same_machine = edit::same_machine(peer, &request, state.port);
    if request.target.split(['?', '#']).next() == Some(http::SOURCE_PATH) {
        let (status, body) = source(state, &request, &mut reader, same_machine);
        let content_type = if status == 200 || status == 409 {
            "application/json"
        } else {
            "text/plain"
        };
        return stream.write_all(&http::response(status, content_type, &body, false));
    }
    let head = request.method == "HEAD";
    if request.method != "GET" && !head {
        return stream.write_all(&http::response(405, "text/plain", b"", false));
    }
    if request.target.split('?').next() == Some(http::EVENTS_PATH) {
        return events(state, stream);
    }
    let build = Arc::clone(&state.build.read().expect("build lock"));
    let found = http::decoded_path(&request.target).and_then(|p| http::lookup(&build.files, &p));
    let Some((rel, content)) = found else {
        let page = http::error_page(404);
        return stream.write_all(&http::response(
            404,
            "text/html; charset=utf-8",
            &page,
            head,
        ));
    };
    let body = match content {
        Content::Bytes(bytes) => bytes.clone(),
        Content::File(source) => match fs::read(source) {
            Ok(bytes) => bytes,
            Err(_) => {
                let page = http::error_page(404);
                return stream.write_all(&http::response(
                    404,
                    "text/html; charset=utf-8",
                    &page,
                    head,
                ));
            }
        },
    };
    let content_type = http::content_type(&rel);
    let body = if content_type.starts_with("text/html") {
        http::inject_elements(&body, same_machine)
    } else {
        body
    };
    stream.write_all(&http::response(200, content_type, &body, head))
}

/// The edit API: `GET` returns a page's source payload, `PUT` saves it, `DELETE` moves
/// its file to the trash (see `browser-editing`). Returns the status and body.
fn source(
    state: &State,
    request: &http::Request,
    reader: &mut impl io::Read,
    same_machine: bool,
) -> (u16, Vec<u8>) {
    let put = request.method == "PUT";
    let delete = request.method == "DELETE";
    if !put && !delete && request.method != "GET" {
        return (405, Vec::new());
    }
    if !same_machine || ((put || delete) && !edit::from_editor(request)) {
        return (403, Vec::new());
    }
    let body = if put {
        match http::read_body(reader, request, edit::MAX_BODY) {
            Ok(body) => Some(body),
            Err(status) => return (status, Vec::new()),
        }
    } else {
        None
    };
    let build = Arc::clone(&state.build.read().expect("build lock"));
    let Some(page) = http::query_param(&request.target, "path")
        .and_then(|path| edit::resolve(&build.editable, &path))
    else {
        return (404, b"not an editable page\n".to_vec());
    };
    drop(build);

    let _rebuilding = state.rebuilding.lock().expect("rebuild lock");
    let target = match edit::checked_target(&state.notes, &edit::source_path(&page)) {
        Ok(target) => target,
        Err(edit::Unsafe::Outside) => return (403, b"outside the notes\n".to_vec()),
        Err(edit::Unsafe::Io(e)) => return (500, format!("{e}\n").into_bytes()),
    };
    let current = match fs::read(&target) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return (500, format!("{e}\n").into_bytes()),
    };
    let file = edit::source_path(&page);
    let payload = |bytes: Option<&[u8]>| match edit::source_json(&file, bytes) {
        Some(json) => (200, json.into_bytes()),
        None => (500, b"the file is not UTF-8\n".to_vec()),
    };
    if delete {
        let Some(bytes) = current else {
            return (404, b"the page has no file\n".to_vec());
        };
        if request.header(edit::VERSION_HEADER) != Some(edit::version(Some(&bytes)).as_str()) {
            let (status, json) = payload(Some(&bytes));
            return (if status == 200 { 409 } else { status }, json);
        }
        let trashed = match edit::move_to_trash(&state.notes, &target) {
            Ok(trashed) => trashed,
            Err(edit::Unsafe::Outside) => return (403, b"outside the notes\n".to_vec()),
            Err(edit::Unsafe::Io(e)) => return (500, format!("{e}\n").into_bytes()),
        };
        *state.snapshot.lock().expect("snapshot lock") = watch::snapshot(&state.notes);
        rebuild(state);
        let json = format!(
            "{{\"deleted\":true,\"trash\":{}}}",
            json_string(&trashed.to_string_lossy())
        );
        return (200, json.into_bytes());
    }
    let Some(body) = body else {
        return payload(current.as_deref());
    };

    let Ok(text) = String::from_utf8(body) else {
        return (400, b"the text is not UTF-8\n".to_vec());
    };
    let Some(base) = request.header(edit::VERSION_HEADER) else {
        return (400, b"missing version\n".to_vec());
    };
    if base != edit::version(current.as_deref()) {
        let (status, json) = payload(current.as_deref());
        return (if status == 200 { 409 } else { status }, json);
    }
    let bytes = edit::file_bytes(&text, current.as_deref());
    match edit::write_atomic(&state.notes, &target, &bytes) {
        Ok(()) => {}
        Err(edit::Unsafe::Outside) => return (403, b"outside the notes\n".to_vec()),
        Err(edit::Unsafe::Io(e)) => return (500, format!("{e}\n").into_bytes()),
    }
    *state.snapshot.lock().expect("snapshot lock") = watch::snapshot(&state.notes);
    rebuild(state);
    payload(Some(&bytes))
}

/// A server-sent event stream: one `data:` line per rebuild that changed something.
fn events(state: &State, mut stream: TcpStream) -> io::Result<()> {
    stream.write_all(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: keep-alive\r\n\r\n: connected\n\n",
    )?;
    let (tx, rx) = mpsc::channel();
    state.clients.lock().expect("clients lock").push(tx);
    loop {
        let chunk = match rx.recv_timeout(KEEP_ALIVE) {
            Ok(json) => format!("data: {json}\n\n"),
            Err(RecvTimeoutError::Timeout) => ": keep-alive\n\n".to_owned(),
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        };
        stream.write_all(chunk.as_bytes())?;
    }
}

fn bind(bind: Bind, port: Option<u16>) -> Result<TcpListener, ServeError> {
    let ip = match bind {
        Bind::Loopback => IpAddr::V4(Ipv4Addr::LOCALHOST),
        Bind::Host(ip) => ip,
        Bind::Public => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
    };
    let ports: Vec<u16> = match port {
        Some(p) => vec![p],
        None => PORTS.collect(),
    };
    let mut last = None;
    for p in &ports {
        match TcpListener::bind(SocketAddr::new(ip, *p)) {
            Ok(listener) => return Ok(listener),
            Err(e) => last = Some(e),
        }
    }
    let what = match port {
        Some(p) => format!("port {p}"),
        None => format!("any port from {} to {}", PORTS.start(), PORTS.end()),
    };
    Err(ServeError::Bind(format!(
        "cannot listen on {ip} at {what}: {}",
        last.map_or_else(|| "no port".to_owned(), |e| e.to_string())
    )))
}

/// The URLs to show for a listener: local first, then the local-network one if public.
fn urls(bind: Bind, addr: SocketAddr) -> Vec<String> {
    let port = addr.port();
    match bind {
        Bind::Loopback => vec![format!("http://127.0.0.1:{port}/")],
        Bind::Host(ip) => vec![format!("http://{}/", SocketAddr::new(ip, port))],
        Bind::Public => {
            let mut urls = vec![format!("http://localhost:{port}/")];
            if let Some(ip) = local_network_address() {
                urls.push(format!("http://{}/", SocketAddr::new(ip, port)));
            }
            urls
        }
    }
}

/// This machine's address on the local network: the source address the system would use
/// to reach a (documentation-only) remote address. `connect` on UDP sends no packets.
fn local_network_address() -> Option<IpAddr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect(("192.0.2.1", 9)).ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then_some(ip)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(files: &[(&str, &[u8])]) -> Build {
        Build {
            files: files
                .iter()
                .map(|(p, b)| (PathBuf::from(p), Content::Bytes(b.to_vec())))
                .collect(),
            attachments: BTreeMap::new(),
            editable: edit::EditMap::new(),
        }
    }

    #[test]
    fn diffs_builds() {
        let old = build(&[
            ("index.html", b"a"),
            ("pages/a.html", b"1"),
            ("pages/gone.html", b"x"),
        ]);
        let new = build(&[
            ("index.html", b"a"),
            ("pages/a.html", b"2"),
            ("pages/Project X/new.html", b"n"),
        ]);
        let (changed, removed) = diff(&old, &new);
        assert_eq!(changed, ["pages/Project X/new.html", "pages/a.html"]);
        assert_eq!(removed, ["pages/gone.html"]);
    }

    #[test]
    fn event_payload() {
        assert_eq!(
            event_json(&["pages/a \"b\".html".into()], &["x\\y".into()]),
            r#"{"changed":["pages/a \"b\".html"],"removed":["x\\y"]}"#
        );
    }
}
