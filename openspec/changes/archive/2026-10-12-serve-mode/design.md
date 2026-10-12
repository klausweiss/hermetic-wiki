# Design

## Context

`site::build` loads the graph and calls `render_into`, which writes every page, the site-wide pages, the embedded assets (`assets::EMBEDDED`), and attachments (`assets::copy_attachments`) straight to a temporary directory before swapping it into place. `main.rs` parses arguments by hand. The frontend is a Bun workspace of light-DOM web components bundled into `static/dist/app.js`; the sidebar listens to document events for cross-component requests. A full release build of 5,580 pages takes about 0.8 s including writing 43 MB to disk, with 33 MB peak memory.

Constraints: no new Rust dependencies, stable Rust, single binary, the built site must not change. Behavior is in `specs/serve-mode`.

## Goals / Non-Goals

**Goals:**
- One rendering path for `build` and `serve`.
- A small, auditable HTTP server that can only serve files of the current build.
- Live reload precise enough that editing other notes never disturbs the page being read.

**Non-Goals:**
- Incremental rendering (full rebuilds measured fast enough).
- HTTPS, compression, range requests, HTTP/2.
- Editing in the browser (`browser-editing`) and search (`search`).
- Opening a browser automatically.

## Decisions

### Rendering produces an in-memory site
`site::render(graph, title, notes, diagnostics) -> SiteFiles`, where `SiteFiles` is a `BTreeMap<PathBuf, Content>` and `Content` is either `Bytes(Vec<u8>)` (pages, site-wide pages, embedded assets) or `File(PathBuf)` (attachments, read from the notes directory when served or written, so large files are not held in memory). `build` writes a `SiteFiles` into its temporary directory and keeps all its output-directory safety; `serve` keeps the latest `SiteFiles` behind an `Arc<RwLock<…>>`. Attachment conflicts and symlink skipping move from `copy_attachments` into `render` unchanged.

*Alternative:* `serve` building into a hidden directory and serving files from disk. Rejected: writes to disk, which the user ruled out, and needs path-safety checks against the file system instead of a map lookup.

### Change detection by polling
A watcher thread scans the notes directory's inputs every 250 ms, comparing `(path, size, mtime)` snapshots. A detected change starts a 150 ms quiet period; further changes extend it; then one rebuild runs. Scanning ~5,000 files is a few milliseconds of `stat` calls.

*Alternative:* the `notify` crate. Rejected: several transitive dependencies and platform-specific back ends, for no user-visible gain at this scale.

### HTTP server on `std::net`
`TcpListener` with one thread per connection, HTTP/1.1, `Connection: close` for ordinary responses. The request line and headers are parsed with size limits (8 KiB). Only `GET`/`HEAD` are served. The path is percent-decoded and looked up in `SiteFiles`; anything not in the map is 404, so traversal is impossible by construction. `Content-Type` comes from a small extension table. Every response carries `Cache-Control: no-store`. HTML responses get `<hermetic-live-reload></hermetic-live-reload>` inserted before `</body>`.

The `Host` check runs before routing when bound to the default loopback address: allowed hosts are `localhost`, `127.0.0.1`, and `[::1]`, with the bound port.

### Live reload over Server-Sent Events
- **Server:** `GET /_hermetic/events` is a long-lived `text/event-stream` response. Each rebuild compares the new `SiteFiles` with the previous one by content: equal bytes means unchanged, and for `File` entries size and mtime are compared. It broadcasts `{"changed": [...], "removed": [...]}` with root-relative, percent-decoded site paths; the page and the sidebar decode their own paths before comparing, which avoids differences in how browsers percent-encode URL paths. A comment line every 15 s keeps idle connections alive. The broadcast list is a `Mutex<Vec<Sender>>` pruned when a send fails.
- **Client:** a new workspace package `@hermetic/live-reload` defines `<hermetic-live-reload>`. It opens an `EventSource` to `<root>/_hermetic/events` (`EventSource` reconnects by itself). On a message:
  - if the current page's path is in either list, it stores `scrollY` in `sessionStorage` keyed by path and calls `location.reload()`; after the reload the element restores the scroll position once;
  - otherwise it dispatches `hermetic-pages-changed` on `document` with the lists.
- **Sidebar:** `<hermetic-sidebar>` listens for `hermetic-pages-changed`, reloads panes whose paths are in `changed`, and closes panes in `removed`.

Built sites never contain the element, because only the server injects it.

*Alternative:* the element always present in built HTML and switched on by a meta tag. Rejected: it adds dead markup to every built page.

### Ports and addresses
Binding tries `127.0.0.1:8000…8099` unless `--port` is given, in which case only that port is tried. `--host` parses an `IpAddr`. `--public` binds `0.0.0.0` (IPv4 only for now; IPv6 can be added when needed) and finds the local-network address by connecting a UDP socket to a documentation address (`192.0.2.1:9`) and reading its local address. UDP `connect` sends no packets, so no network traffic happens. It prints the `localhost` URL and, if one was found, the network URL.

### The file:// placeholder
The sidebar's `file://` text recommends `hermetic serve` in place of `python3 -m http.server`.

## Risks / Trade-offs

- [A thread per connection could be exhausted by many idle SSE clients] → it's a local, single-user tool; connections are cheap threads and are closed when a send fails.
- [Polling misses changes within the mtime granularity] → size is compared too, and the next real change triggers a rebuild anyway; editors rarely write identical sizes in the same tick.
- [Byte comparison of 43 MB per rebuild] → a few milliseconds; done once per rebuild.
- [HTML injection by string search for `</body>`] → hermetic generates every page, and each has exactly one `</body>`.
- [`--public` exposes the notes to the local network without authentication] → opt-in, and the printed URLs make it visible; documented in `--help`.
- [Bundle grows by a few KB for the live-reload element even in built sites] → it stays inert without the injected tag.
