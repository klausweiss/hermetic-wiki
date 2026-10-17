//! A minimal HTTP/1.1 layer: reading a request, routing it to a file of the current
//! build, and writing a response. Only paths present in the build can be served.

use std::io::{self, BufRead, Read};
use std::path::{Path, PathBuf};

use crate::site::{Content, SiteFiles};

/// Upper bound on the request line plus headers.
const MAX_HEAD: usize = 8 * 1024;

/// The element the server adds to every HTML page it serves.
pub const LIVE_RELOAD_TAG: &str = "<hermetic-live-reload></hermetic-live-reload>";
/// The server-sent events endpoint the live-reload element connects to.
pub const EVENTS_PATH: &str = "/_hermetic/events";
/// The element the server adds to pages it serves to a browser on the same machine.
pub const EDITOR_TAG: &str = "<hermetic-editor></hermetic-editor>";
/// The edit API: one page's source, by site path.
pub const SOURCE_PATH: &str = "/_hermetic/source";
/// The edit API's image upload, by the site path of the page the image is for.
pub const ASSET_PATH: &str = "/_hermetic/asset";
/// The maintenance API: the attachments nothing uses, and moving them to the trash.
pub const UNUSED_ASSETS_PATH: &str = "/_hermetic/unused-assets";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    /// The request target as sent, e.g. `/pages/a%20b.html?x=1`.
    pub target: String,
    /// Header names are lowercased.
    pub headers: Vec<(String, String)>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Reads a request line and headers. Errors on malformed or oversized input.
pub fn read_request(reader: &mut impl BufRead) -> io::Result<Request> {
    let invalid = |msg: &str| io::Error::new(io::ErrorKind::InvalidData, msg.to_owned());
    let mut limited = reader.take(MAX_HEAD as u64);
    let mut line = String::new();
    let mut lines = Vec::new();
    loop {
        line.clear();
        if limited.read_line(&mut line)? == 0 {
            return Err(invalid("request head is incomplete or too large"));
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        lines.push(trimmed.to_owned());
    }
    let mut first = lines
        .first()
        .ok_or_else(|| invalid("empty request"))?
        .split(' ');
    let (Some(method), Some(target), Some(version)) = (first.next(), first.next(), first.next())
    else {
        return Err(invalid("malformed request line"));
    };
    if !version.starts_with("HTTP/1.") {
        return Err(invalid("unsupported HTTP version"));
    }
    let headers = lines[1..]
        .iter()
        .filter_map(|h| h.split_once(':'))
        .map(|(n, v)| (n.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    Ok(Request {
        method: method.to_owned(),
        target: target.to_owned(),
        headers,
    })
}

/// The path part of a request target, percent-decoded, without the leading `/`.
/// `None` for targets that are not origin-form paths or do not decode to UTF-8.
pub fn decoded_path(target: &str) -> Option<String> {
    let path = target.split(['?', '#']).next()?;
    let rest = path.strip_prefix('/')?;
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = rest.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// The value of a query parameter, percent-decoded (`+` is a space).
pub fn query_param(target: &str, name: &str) -> Option<String> {
    let query = target.split_once('?')?.1.split('#').next()?;
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key != name {
            return None;
        }
        let value = value.replace('+', " ");
        decoded_path(&format!("/{value}"))
    })
}

/// Reads a request body of `Content-Length` bytes, at most `max`. Errors are the
/// response status: 411 without a length (or with a transfer encoding), 413 when too
/// large, 400 when the connection ends early.
pub fn read_body(reader: &mut impl Read, request: &Request, max: usize) -> Result<Vec<u8>, u16> {
    if request.header("transfer-encoding").is_some() {
        return Err(411);
    }
    let len: usize = request
        .header("content-length")
        .ok_or(411u16)?
        .parse()
        .map_err(|_| 400u16)?;
    if len > max {
        // Read (and drop) what was sent, so the client sees the response, not a reset.
        let _ = io::copy(&mut reader.take(len.min(4 * max) as u64), &mut io::sink());
        return Err(413);
    }
    let mut body = vec![0; len];
    reader.read_exact(&mut body).map_err(|_| 400u16)?;
    Ok(body)
}

/// Finds the build file for a decoded path: `""` and paths ending in `/` map to the
/// directory's `index.html`. Paths with `.`/`..` segments never match.
pub fn lookup<'f>(files: &'f SiteFiles, path: &str) -> Option<(PathBuf, &'f Content)> {
    let mut rel = path.to_owned();
    if rel.is_empty() || rel.ends_with('/') {
        rel.push_str("index.html");
    }
    if rel
        .split('/')
        .any(|s| s.is_empty() || s == "." || s == "..")
    {
        return None;
    }
    let rel = PathBuf::from(rel);
    files.get(&rel).map(|c| (rel, c))
}

pub fn content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("txt" | "md") => "text/plain; charset=utf-8",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        Some("ico") => "image/x-icon",
        Some("pdf") => "application/pdf",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mp3") => "audio/mpeg",
        _ => "application/octet-stream",
    }
}

/// Adds the live-reload element, and the editor element when `editor` is set, before
/// `</body>` of a generated page.
pub fn inject_elements(html: &[u8], editor: bool) -> Vec<u8> {
    const END: &[u8] = b"</body>";
    let mut tags = String::from(LIVE_RELOAD_TAG);
    if editor {
        tags.push_str(EDITOR_TAG);
    }
    match html.windows(END.len()).rposition(|w| w == END) {
        Some(i) => [&html[..i], tags.as_bytes(), b"\n", &html[i..]].concat(),
        None => html.to_vec(),
    }
}

/// Whether a `Host` header names this machine's loopback address (with or without the
/// port), as required when bound to loopback: other names indicate DNS rebinding.
pub fn is_loopback_host(host: &str, port: u16) -> bool {
    let host = host.to_ascii_lowercase();
    let port_ok = |p: &str| p.is_empty() || p == format!(":{port}");
    if let Some(rest) = host.strip_prefix('[') {
        // An IPv6 literal: `[::1]` or `[::1]:8000`.
        return rest
            .split_once(']')
            .is_some_and(|(addr, tail)| addr == "::1" && port_ok(tail));
    }
    let (name, tail) = match host.find(':') {
        Some(i) => host.split_at(i),
        None => (host.as_str(), ""),
    };
    (name == "localhost" || name == "127.0.0.1") && port_ok(tail)
}

pub fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        411 => "Length Required",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        _ => "Internal Server Error",
    }
}

/// A complete response with `Connection: close`; the body is omitted for HEAD.
pub fn response(status: u16, content_type: &str, body: &[u8], head: bool) -> Vec<u8> {
    let mut out = format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n",
        status_text(status),
        body.len()
    );
    if status == 405 {
        out.push_str("Allow: GET, HEAD\r\n");
    }
    out.push_str("\r\n");
    let mut out = out.into_bytes();
    if !head {
        out.extend_from_slice(body);
    }
    out
}

/// A small error page linking back to the home page.
pub fn error_page(status: u16) -> Vec<u8> {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\"><title>{status} {text}</title>\
         <link rel=\"stylesheet\" href=\"/_hermetic/style.css\"></head>\n\
         <body><main><h1>{text}</h1><p><a href=\"/\">Go to the home page</a></p></main></body>\n</html>\n",
        text = status_text(status)
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> io::Result<Request> {
        read_request(&mut io::BufReader::new(raw.as_bytes()))
    }

    fn site() -> SiteFiles {
        let mut files = SiteFiles::new();
        for p in [
            "index.html",
            "pages/Project X/Auth Flow.html",
            "pages/a.html",
            "journals.html",
        ] {
            files.insert(PathBuf::from(p), Content::Bytes(p.as_bytes().to_vec()));
        }
        files
    }

    #[test]
    fn parses_requests() {
        let r =
            parse("GET /pages/a.html?x=1 HTTP/1.1\r\nHost: localhost:8000\r\nX-Test:  v \r\n\r\n")
                .unwrap();
        assert_eq!(r.method, "GET");
        assert_eq!(r.target, "/pages/a.html?x=1");
        assert_eq!(r.header("host"), Some("localhost:8000"));
        assert_eq!(r.header("x-test"), Some("v"));
        assert!(parse("GET /\r\n\r\n").is_err());
        assert!(parse("GET / SPDY/3\r\n\r\n").is_err());
        assert!(
            parse("GET / HTTP/1.1\r\nHost: x\r\n").is_err(),
            "unterminated head"
        );
        let huge = format!("GET / HTTP/1.1\r\nX: {}\r\n\r\n", "a".repeat(MAX_HEAD));
        assert!(parse(&huge).is_err(), "oversized head");
    }

    #[test]
    fn encoded_path() {
        let files = site();
        let path = decoded_path("/pages/Project%20X/Auth%20Flow.html").unwrap();
        let (rel, _) = lookup(&files, &path).unwrap();
        assert_eq!(rel, PathBuf::from("pages/Project X/Auth Flow.html"));
        assert_eq!(content_type(&rel), "text/html; charset=utf-8");
        assert_eq!(decoded_path("/a%zz"), None);
        assert_eq!(decoded_path("/%FF"), None);
        assert_eq!(decoded_path("http://x/a"), None);
    }

    #[test]
    fn path_outside_the_site() {
        let files = site();
        for target in [
            "/../../etc/passwd",
            "/%2e%2e/notes.md",
            "/pages/../index.html",
            "/pages//a.html",
            "/./index.html",
        ] {
            let found = decoded_path(target).and_then(|p| lookup(&files, &p).map(|(r, _)| r));
            assert_eq!(found, None, "{target}");
        }
    }

    #[test]
    fn directory_index() {
        let files = site();
        assert_eq!(lookup(&files, "").unwrap().0, PathBuf::from("index.html"));
        assert!(lookup(&files, "pages/").is_none());
    }

    #[test]
    fn responses() {
        let r = String::from_utf8(response(200, "text/plain", b"hi", false)).unwrap();
        assert!(r.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(r.contains("Cache-Control: no-store\r\n") && r.contains("Content-Length: 2\r\n"));
        assert!(r.ends_with("\r\n\r\nhi"));
        let head = String::from_utf8(response(200, "text/plain", b"hi", true)).unwrap();
        assert!(head.ends_with("\r\n\r\n") && head.contains("Content-Length: 2"));
        assert!(
            String::from_utf8(response(405, "text/plain", b"", false))
                .unwrap()
                .contains("Allow: GET, HEAD")
        );
        assert!(
            String::from_utf8(error_page(404))
                .unwrap()
                .contains("<a href=\"/\">")
        );
    }

    #[test]
    fn content_types() {
        assert_eq!(
            content_type(Path::new("_hermetic/app.js")),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(content_type(Path::new("x.WOFF2")), "font/woff2");
        assert_eq!(
            content_type(Path::new("pages/data.bin")),
            "application/octet-stream"
        );
        assert_eq!(
            content_type(Path::new(".hermetic-site")),
            "application/octet-stream"
        );
    }

    #[test]
    fn rebinding_protection() {
        for ok in [
            "localhost:8000",
            "127.0.0.1:8000",
            "[::1]:8000",
            "localhost",
            "LOCALHOST:8000",
            "[::1]",
        ] {
            assert!(is_loopback_host(ok, 8000), "{ok}");
        }
        for bad in [
            "evil.example:8000",
            "evil.example",
            "localhost:9000",
            "127.0.0.2:8000",
            "localhost.evil.example:8000",
            "",
        ] {
            assert!(!is_loopback_host(bad, 8000), "{bad}");
        }
    }

    #[test]
    fn injection() {
        let page = b"<html><body><main>x</main>\n</body>\n</html>\n";
        let out = String::from_utf8(inject_elements(page, false)).unwrap();
        assert_eq!(
            out,
            format!("<html><body><main>x</main>\n{LIVE_RELOAD_TAG}\n</body>\n</html>\n")
        );
        let out = String::from_utf8(inject_elements(page, true)).unwrap();
        assert!(out.contains(&format!("{LIVE_RELOAD_TAG}{EDITOR_TAG}\n</body>")));
        assert_eq!(inject_elements(b"no body", true), b"no body");
    }

    #[test]
    fn query_params() {
        let t = "/_hermetic/source?x=1&path=pages/Project%20X/A+b.html#frag";
        assert_eq!(
            query_param(t, "path").as_deref(),
            Some("pages/Project X/A b.html")
        );
        assert_eq!(query_param(t, "x").as_deref(), Some("1"));
        assert_eq!(query_param(t, "y"), None);
        assert_eq!(query_param("/_hermetic/source", "path"), None);
        assert_eq!(query_param("/s?path=%2B%25", "path").as_deref(), Some("+%"));
        assert_eq!(query_param("/s?path=%ff", "path"), None);
    }

    #[test]
    fn bodies() {
        let body = |head: &str, rest: &str, max: usize| {
            let r = parse(head).unwrap();
            read_body(&mut rest.as_bytes(), &r, max)
        };
        let head = "PUT / HTTP/1.1\r\nContent-Length: 3\r\n\r\n";
        assert_eq!(body(head, "abcdef", 10), Ok(b"abc".to_vec()));
        assert_eq!(body(head, "ab", 10), Err(400));
        assert_eq!(body(head, "abc", 2), Err(413));
        assert_eq!(body("PUT / HTTP/1.1\r\n\r\n", "abc", 10), Err(411));
        assert_eq!(
            body(
                "PUT / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n",
                "",
                10
            ),
            Err(411)
        );
        assert_eq!(
            body("PUT / HTTP/1.1\r\nContent-Length: x\r\n\r\n", "", 10),
            Err(400)
        );
    }
}
