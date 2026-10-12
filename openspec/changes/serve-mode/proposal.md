# Proposal

## Why

While writing, the notes should be viewable live, with the features that need `http://` (sidebar panes, journal timeline), without installing or running a separate web server and without rebuilding by hand. Today a reader who opens the built site from disk gets the reduced `file://` experience and has to reach for `python3 -m http.server`.

## What Changes

- Add `hermetic serve [<dir>]` (notes directory defaults to the current directory): builds the site in memory and serves it over HTTP from the same single binary, with no configuration. Nothing is written to disk; `hermetic build` remains the way to produce a site directory.
- Watch the notes directory and rebuild after changes; the server keeps serving the last good build while rebuilding.
- Live reload: after a rebuild the server tells open tabs which pages changed. A tab reloads only if its own page changed (keeping the scroll position), and open sidebar panes whose pages changed refresh in place; edits to unrelated notes do not disturb the page being read.
- Network binding: localhost only by default (`127.0.0.1`, port 8000 or the next free port), printing the URL. `--port` picks a port, `--host` binds a chosen address, and `--public` binds all interfaces and also prints the address other devices on the local network can use (as Storybook does).
- The sidebar's `file://` placeholder points to `hermetic serve`.
- No new dependencies: the HTTP server and the change detection are small, standard-library-only components.

## Capabilities

### New Capabilities
- `serve-mode`: the `serve` command and its options, in-memory serving, change detection and rebuilds, live reload, network binding and its safety defaults.

### Modified Capabilities
<!-- none: the `cli` requirements (single executable, usage, exit codes) already cover a new command; serve-specific behavior lives in `serve-mode`. -->

## Impact

- Depends on `static-build` and `reader-ux` (archived).
- `site::build` is split so that rendering produces an in-memory set of files, which `build` writes to disk and `serve` keeps in memory.
- New Rust module for the HTTP server and file watching; `main.rs` gains the `serve` command and its flags.
- New web component package `@hermetic/live-reload` (`<hermetic-live-reload>`), included in `app.js`; the server injects the element into the pages it serves, so built sites are unchanged.
- Out of scope: search (the `search` change), editing (`browser-editing`), incremental rendering (full in-memory rebuilds are fast enough at the sizes measured; revisit if not).
