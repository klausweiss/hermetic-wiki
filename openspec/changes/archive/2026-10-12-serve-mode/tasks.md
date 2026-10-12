# Tasks

## 1. In-memory rendering

- [x] 1.1 Introduce `SiteFiles` (`BTreeMap<PathBuf, Content>` with `Bytes`/`File`) and `site::render(...)` that produces it, moving attachment collection (conflicts, symlinks) into it; verify existing site tests still pass unchanged
- [x] 1.2 Make `site::build` write a `SiteFiles` into its temporary directory, keeping all output safety; verify the whole-site determinism and file-list tests pass and the 5,000-journal build time does not regress (release, ±10%)

## 2. HTTP server

- [x] 2.1 Request parsing with size limits, `GET`/`HEAD` handling, 405 for other methods, percent-decoding, map lookup with directory `index.html`, 404 page, content types, `Cache-Control: no-store`; verify with unit tests on raw request bytes, including `Encoded path`, `Path outside the site`, and `Directory index`
- [x] 2.2 Host-header check for loopback binding; verify `Rebinding protection` and that `localhost`, `127.0.0.1`, `[::1]` with the bound port are accepted
- [x] 2.3 Inject `<hermetic-live-reload></hermetic-live-reload>` before `</body>` in served HTML; verify `Same output as build` by comparing a served fixture with a built one byte for byte except that insertion

## 3. Watching and rebuilding

- [x] 3.1 Polling watcher over `journals/`, `pages/`, `assets/`, and root `index.md` with a 150 ms quiet period; verify `Burst of saves` (one rebuild) and detection of create/modify/delete with a test against a temporary notes directory
- [x] 3.2 Rebuild loop: swap the served `SiteFiles` on success, keep the old one and print the error on failure, print diagnostics as for `build`; verify `New page appears` and `Deleted note disappears` over real HTTP within 2 seconds, and the 2-second target on the 5,000-journal set
- [x] 3.3 Change diffing (`changed`/`removed` paths) and the SSE endpoint with keep-alive comments and broadcast; verify with an integration test that an edit produces an event listing exactly the affected pages

## 4. CLI and binding

- [x] 4.1 `serve` command in `main.rs` with `--port`, `--host`, `--public` (mutually exclusive with `--host`), usage text, exit codes; verify `Serve the current directory`, `Missing notes directory`, and usage errors with CLI tests
- [x] 4.2 Port selection 8000–8099 and exact `--port`; verify `Default binding` with a busy port 8000 and failure on a busy `--port`
- [x] 4.3 `--public`: bind all interfaces, find the local-network address via a UDP socket without sending packets, print both URLs; verify `Public` (both URLs printed; the network URL answers when requested from this machine's network address)

## 5. Live reload in the browser

- [x] 5.1 New workspace package `@hermetic/live-reload` (`<hermetic-live-reload>`) with its Makefile and tests for path matching; include it in `app.js`; verify `make -C static check test verify`
- [x] 5.2 Reload when the current page changed, restoring the scroll position; otherwise dispatch `hermetic-pages-changed`; verify `Editing the page being read` and `Editing an unrelated note` in Chromium and Firefox
- [x] 5.3 Sidebar refreshes changed panes and closes removed ones on `hermetic-pages-changed`; verify `Pane refresh` in Chromium and Firefox
- [x] 5.4 Reconnection after a server restart, and no live-reload connection on built sites; verify `Built site has no live reload` and that a restarted server resumes updates

## 6. Finish

- [x] 6.1 Point the sidebar's `file://` placeholder at `hermetic serve`; add `make serve` (serves `tests/fixtures/notes`); update `--help`
- [x] 6.2 Run `make lint`, `make test`, `make test-perf`, and the browser checks in Chromium and Firefox; record the results in this file

## Verification notes

Run on 2026-10-12:

- `make lint` passes (rustfmt, clippy, TypeScript check of every package, committed `app.js` up to date); `make test` passes: 163 Rust tests and 12 `bun test` tests.
- `make test-perf`: graph load of 5,000 journals in about 96 ms; with `hermetic serve` running on 5,000 journals, a newly saved page is served after about 0.83 s (target: 2 s). A full release `hermetic build` of 5,580 pages takes about 0.93 s (0.81–0.89 s before the in-memory refactor), with peak memory rising from about 33 MB to 81 MB because the rendered site is now held in memory before writing.
- HTTP and CLI behavior are covered by integration tests against a real server (`tests/serve.rs`, `tests/cli_serve.rs`), including byte-for-byte equality with `hermetic build` apart from the injected element, DNS-rebinding rejection, a burst of saves producing exactly one rebuild event, an event listing exactly the pages that changed, and `--public` answering on this machine's network address (192.168.1.217 at the time).
- Live reload in headless Chromium and Firefox against a real `hermetic serve` (7 checks each, all passing): the page being edited reloads and keeps its scroll position; editing an unrelated note does not reload; a sidebar pane refreshes in place; a pane whose page is deleted closes; updates resume after a server restart; no script errors.
- The full reader browser suite (60 checks, including "built site has no live reload") passes in Chromium and Firefox.
