# Tasks

## 1. Edit API (Rust)

- [x] 1.1 Request bodies in `serve/http.rs`: read `Content-Length` bodies up to 8 MiB (411 without a length, 413 above the limit, 400 for non-UTF-8); accept `PUT` only for `/_hermetic/source` and keep 405 for every other method and path; verify with unit tests and the `Other methods` scenario
- [x] 1.2 Authenticity: peer loopback check (including IPv4-mapped addresses), loopback `Host` for every binding, and for `PUT` an exact `Origin` and `X-Hermetic-Edit: 1`; no CORS headers, `OPTIONS` → 405; inject the editor element only when peer and `Host` are loopback; verify a unit-test matrix (peer × Host × Origin × header × binding) and the `Another device`, `DNS rebinding`, and `Built site` scenarios
- [x] 1.3 Site-path → editable page map kept with each build (every page HTML, `index.html` only with `index.md`, plus parsed `journals/YYYY-MM-DD.html` for days without a page); 404 for everything else; verify the `Not editable` scenario and that `tasks.html`, `journals.html`, `pages.html`, `search.html`, and `refs/…` are rejected
- [x] 1.4 `GET /_hermetic/source`: JSON with `exists`, `version` (FNV-1a hex or `missing`), `text`, `indent`, and blocks (`lines`, `depth`, `bullet`, `anchor`, `body`, `column`); verify the `Read a page` scenario, including anchors `id-<id>` and `b<n>`, and a page without a file
- [x] 1.5 Safe writes: target from the page's file or the `source-layout` path of its name, canonical containment under `journals/`, `pages/`, or root `index.md`, namespace directories created and re-checked, temp file `.…tmp` + `sync_all` + rename, removed on failure; CRLF kept, final line ending added; verify `Symbolic link out of the notes` (Unix), `Namespaced new page`, CRLF and final-newline tests, and that a failed write leaves no temp file
- [x] 1.6 `PUT /_hermetic/source`: version check (409 with current version and text), write, synchronous rebuild under the shared rebuild lock, broadcast, refresh of the watcher's shared snapshot, response with new version and blocks; verify `Stale version`, that one `PUT` causes exactly one rebuild, and that the response is sent after the new page is served
- [x] 1.7 Cross-language fixtures: `tests/fixtures/editing/*.md` (CRLF, tabs, 2- and 4-space indentation, irregular indentation, prose preamble, blank lines, code fences, properties, ids, tasks) and a Rust test that writes their GET payloads to `static/packages/editor/test-fixtures/` (`UPDATE_SNAPSHOTS=1` regenerates; the test fails when they are stale); verify `cargo test`
- [x] 1.8 Update `serve-mode` behavior notes in `src/help.rs` (`commands`: editing on the same machine only, saved to note files); verify the `Editing is described` scenario and the help tests (80 columns, no control characters)

## 2. Editor model and serializer (TypeScript)

- [x] 2.1 Create the `static/packages/editor` Bun package (package.json depending on `@hermetic/core` and `@hermetic/search`, Makefile with `NAME := hermetic-editor`, tsconfig), add it to `static/Makefile` `PACKAGES` before `app`; verify `make -C static check test`
- [x] 2.2 `model.ts`: tree from the GET payload; `serialize.ts`: untouched nodes copied by line range, changed or re-indented nodes written with the file's indent unit and content column, prose nodes kept at depth 0; verify byte-identical round trips for every fixture
- [x] 2.3 Pure outliner operations: split at cursor (sibling or first child; no split inside an unclosed fence), indent, outdent (later siblings stay), move up/down, delete empty, join with previous, cycle task marker; verify each with hand-checked expected files, covering every `Outliner keys` scenario and the `Tab-indented file` scenario
- [x] 2.4 `api.ts`: GET/PUT with `X-Hermetic-Edit` and `X-Hermetic-Version`, per-page save queue (one in flight, latest text wins), retry with backoff on network errors, 409 surfaced to the caller; verify with unit tests against a fake fetch, including `Server stopped` ordering

## 3. Editing in the page (TypeScript)

- [x] 3.1 `<hermetic-editor>`: map `section.content li.block[id]` to model nodes, click-to-edit with an auto-growing textarea (links in non-edited blocks still navigate), one block at a time, Escape/click outside to end, empty-page starter block; verify `Raw text`, `Links still navigate`, and `Empty page` in the browser
- [x] 3.2 Key handling on the textarea for every outliner key, wired to the model operations, with resync of the tree from the `PUT` response (keeping the active block and cursor); verify `New sibling`, `Split in the middle`, `Indent and outdent`, `Outdent keeps later siblings`, `Enter inside a code fence`, `Join with previous`, `Cycle task`, and `Typed bullet` in the browser
- [x] 3.3 Saving triggers (structure change, blur/Escape, 1 s idle), status indicator, `beforeunload` warning, retry after reconnect; verify `Save while typing`, `Server stopped`, and `Backlinks follow` (within 2 seconds)
- [x] 3.4 In-place updates: `live-reload` dispatches `PAGES_CHANGED` instead of reloading while the editor is active (constant in `@hermetic/core`); the editor patches rendered bodies or swaps sections and re-attaches the live textarea with its selection; re-GET on external changes when nothing is unsaved; verify `Own save` and `External change while idle`
- [x] 3.5 Conflicts: on 409 rebuild from the response, end editing, show the notice with the refused text, Copy, and Dismiss; verify `Changed in $EDITOR`
- [x] 3.6 "Today" control and creation: header link, GET of today's site path, `PUT "-\n"` with `missing` when absent (409 → proceed), navigation to `#b1` with editing started; first save of a page without a file creates it; verify `Today without a file` and `New page from a link`

## 4. References and autocomplete (TypeScript)

- [x] 4.1 Bullet menu with "Copy reference": add `id::` after the block's first line and existing properties at the content column, 8-character id from `crypto.getRandomValues`, clipboard `((id))`; verify `Block without id` and `Block with id` (file otherwise byte-identical)
- [x] 4.2 `[[` page list using the search engine's page hits plus a "new page" entry for an unmatched name; arrows, Enter/Tab, Escape; verify `Link to an existing page` and `New page name`
- [x] 4.3 `((` block list using block hits; choosing a block without id adds one on its page (GET, insert, PUT, one retry on 409) and inserts `((id))`; verify `Block reference by search`
- [x] 4.4 Shortcuts help dialog lists an "Editing" group when `hermetic-editor` is present; verify the dialog in edit mode and its absence on built sites; rebuild `static/dist/app.js` and verify `make -C static verify`

## 6. Formatting keys

- [x] 6.1 `format.ts`: a pure toggle of inline markers for a selection or cursor (wrap, unwrap outside or inside the selection, insert or remove an empty pair, `*` runs telling italic from bold); verify unit tests for every `Formatting keys` scenario plus `~~` and `` ` ``
- [x] 6.2 Wire Ctrl/Cmd+B, I, Shift+S, and E on the textarea (applied with the browser's undoable text insertion, falling back to `setRangeText`), list them in the shortcuts dialog, rebuild `static/dist/app.js`; verify the scenarios in Chromium and Firefox, including that Ctrl+Z reverts and the browser's own bindings do not fire, and that the full editing suite still passes

  Results (2026-10-12): 7 unit tests for the toggle; the editing browser suite, now 43 checks, passes in Chromium and Firefox. It covers every formatting scenario, Ctrl+Z, saving, and the help dialog.

## 5. Integration

- [x] 5.1 Run `make lint`, `make test`, `make test-perf`, and the browser suites (reader, live reload, search, and a new editing suite covering every browser-observable `browser-editing` scenario) in Chromium and Firefox; record results here

  Results (2026-10-12):
  - `make lint` is clean.
  - `make test`: 210 Rust tests (including `tests/edit.rs`) and 59 TypeScript tests pass.
  - `make test-perf`: 5,000 journals load in 97 ms; serve shows a change after 0.93 s.
  - Browser suites, Chromium and Firefox: editing 35/35, reader 60/60, search 20/20, live reload 8/8.
  - The live-reload suite now expects in-place updates for pages open for editing, and reloads for pages that are not editable. The `serve-mode` delta scenarios say the same.
  - `serve --public` reached through the LAN address serves pages without the editor, and the edit API answers 403.
  - Found and fixed along the way: the parser panicked on empty bullets (`-`), which also crashed `hermetic build`.

