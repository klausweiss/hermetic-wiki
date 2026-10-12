# Design

## Context

- **Server.** `hermetic serve` (`src/serve/`) is a std-only HTTP/1.1 server. Each connection runs on its own thread; the current build (`SiteFiles` plus attachment fingerprints) sits behind an `RwLock<Arc<Build>>`. A polling watcher (`watch.rs`, 250 ms poll, 150 ms quiet period) rebuilds the whole site and broadcasts changed site paths over server-sent events. Today it accepts only `GET` and `HEAD`, reads no request bodies, and checks the `Host` header only for the default loopback binding.
- **Parser.** It already keeps everything a lossless editor needs (`block-model`, "Source positions"). Each block has `lines` (a 0-based line range; the ranges of a file's blocks partition its lines), `source` (the exact text of those lines), `kind`, its parent, and its properties. Anchors are `id-<id>` or `b<n>` (`static-site-output`, "Block anchors").
- **Frontend.** Light-DOM web components, one Bun package each, bundled into the committed `static/dist/app.js`. `live-reload` reloads the tab when its own page changes and otherwise dispatches `PAGES_CHANGED`. The `search` package exposes `search()`, `resolve()`, and `FetchSource`.
- **No serde.** The crate depends only on `pulldown-cmark` and `unicode-normalization`. JSON is written by hand (see `search/index.rs`), and nothing on the server parses JSON today.

## Goals / Non-Goals

**Goals:**
- A writing experience close to Logseq's outliner, running entirely in the browser between saves.
- No path through the edit API can lose data or write outside the notes directory.
- A small server surface: one endpoint, two methods, no JSON parsing on the server.

**Non-Goals:**
- Collaborative editing or merging concurrent edits. Conflicts are detected, never merged.
- A WYSIWYG editor. The block being edited is plain markdown text.
- Making the editor usable from other devices, even with `--public`.

## Decisions

### 1. The browser owns the outline; the server stores whole files

The editor loads a page's source and block list and keeps the outline as a tree in the browser. Every outliner operation (split, indent, outdent, move, join, cycle task) is a pure function on that tree. A save serializes the tree to the full file text and sends it with `PUT`, together with the version it was based on.

- **Rejected: server-side block operations** (`{op: "indent", block: 7}` …). They would need a JSON parser and an operation language in Rust, and a round trip per keystroke-level action. They would also make the server surface larger and harder to audit.
- The whole-file approach keeps the server's job to "check, write atomically, rebuild". The outliner logic becomes testable TypeScript with no DOM.

### 2. Lossless serialization from the parser's own line ranges

`GET /_hermetic/source` returns the following JSON (written by hand in Rust):

```
{"exists": true, "version": "9f2c…", "text": "…", "indent": "\t",
 "blocks": [{"lines": [0, 3], "depth": 0, "bullet": true, "anchor": "b1",
             "body": "decided to use [[JWT]]\nstatus:: accepted", "column": 2}, …]}
```

- `body` is the text the editor shows: the block's source de-indented to its content column, with the bullet marker removed.
- Each tree node remembers its original `lines` and a `dirty` flag.
- Serialization walks the tree in document order:
  - an untouched node at its original depth emits its original lines verbatim, including trailing blank lines;
  - a dirty, new, or moved node emits its bullet line and its further lines at its content column (two spaces after the bullet's indentation), with empty lines kept empty;
  - **indentation** is chosen so that the parser's nesting rule (a bullet is the child of the nearest earlier bullet indented less) gives back the editor's tree. A node at its original depth keeps its original indentation if that is still deeper than its parent's and no deeper than its previous sibling's. Otherwise it takes its previous sibling's indentation, or its parent's plus the indentation unit. A randomized test applies operation sequences to every fixture and checks the nesting of the written text;
  - a non-bullet (prose) node is only ever edited at depth 0, and emits its body as is.
- **Indentation unit:** the server reports it as the leading whitespace of the first indented bullet line in the file (a tab, or that many spaces). It defaults to two spaces.
- **Line endings:** the browser always works with LF. The server converts the body to CRLF on `PUT` when the current file uses CRLF (decided by its first line ending), and appends a final line ending when one is missing.
- **Resync after save:** the `PUT` response returns the re-parsed blocks. The editor then rebuilds its tree from them (keeping the active block by its index in document order, and the cursor offset). Text typed into one block that the parser splits, such as a line starting with `- `, therefore becomes separate blocks naturally.

### 3. Versions are content hashes

The version is the 64-bit FNV-1a hash of the file's bytes, as 16 hex digits, or `missing` for a page without a file. It is sent in the header `X-Hermetic-Version`. The hash is computed under the write lock just before writing.

- **Rejected: modification times,** which are coarse on some filesystems and change on a touch.
- FNV is enough because this guards against accidents, not adversaries, and it needs no extra crate.

### 4. Addressing pages by site path, resolved by the server

Each `Build` gains a map from site path (for example `pages/Project X/Auth.html`) to `Editable { name, kind, file: Option<PathBuf> }`. It covers every page HTML the build wrote, plus `index.html` when `index.md` exists. Paths of the form `journals/YYYY-MM-DD.html` with a valid date and no page are resolved by parsing, so "Today" works for days nobody references. Everything else is 404.

- **Target file:** the page's existing `file`. For a page without a file, the target is `layout::page_file_path(name)` or `layout::journal_file_path(date)`. Request text never becomes a path.
- **Rejected: sending page names.** That would duplicate name normalization and encoding in TypeScript.
- **Rejected: adding `data-page` to the output.** That would change built HTML for a serve-only feature.

### 5. One write path, under one lock

`PUT` handling:

1. Authenticity checks (decision 6).
2. Read the body: requires `Content-Length`, at most 8 MiB, must be UTF-8.
3. Take the `rebuild` mutex, which is shared with the watcher.
4. Resolve the target and run the safety checks:
   - canonicalize the target if it exists, or else its deepest existing ancestor;
   - require it to be under the canonical root's `journals/` or `pages/`, or to be the root `index.md`;
   - create missing namespace directories, then re-check.
5. Compare versions.
6. Write a temporary file `.<stem>.hermetic-<pid>-<n>.tmp` in the target directory, `sync_all`, and `rename` it over the target. On error, remove it.
7. Rebuild synchronously and broadcast the change.
8. Refresh the watcher's snapshot (now a shared `Mutex<Snapshot>`), so the poller sees no difference and does not rebuild again.
9. Respond with the new version and blocks.

Answering only after the rebuild means that when the editor fetches the page HTML next, it is current. At 5,000 pages that costs about 0.4 s per save, but saves are asynchronous to typing (decision 8).

### 6. Request authenticity

The edit API checks, in order:

1. The peer is a loopback address (`TcpStream::peer_addr`, including IPv4-mapped `::ffff:127.0.0.1`).
2. `Host` names a loopback host with the server's port (the existing `is_loopback_host`, now applied for every binding).
3. For `PUT`: `Origin` equals `http://<Host>`, and `X-Hermetic-Edit: 1` is present.

Any failure gives 403 with an empty body.

- **Cross-site requests:** the custom header and the non-simple method force a CORS preflight, and preflight `OPTIONS` requests get 405 with no CORS headers. A browser therefore never sends a cross-site `PUT`, and the `Origin` check is a second line of defense.
- **DNS rebinding:** this passes `Origin == Host`, so the check also requires `Host` to be loopback. That defeats rebinding even under `--public`.
- **Editor element:** it is injected next to the live-reload element only when checks 1 and 2 pass for the page request.
- **Scope:** `GET` of the source also requires 1 and 2. It reveals nothing beyond the HTML, but keeping the API off the network is simpler to reason about.

### 7. A new `editor` package

`static/packages/editor` holds:

- `model.ts`: the tree and pure operations;
- `serialize.ts`;
- `api.ts`: `GET`/`PUT` with headers, and a per-page save queue;
- `element.ts`: `<hermetic-editor>`, which activates blocks, keeps an auto-growing `<textarea>`, and maps keys;
- `complete.ts`: `[[` and `((` lists built on `search()` / `resolve()` from `@hermetic/search`, filtered to page or block hits;
- `refs.ts`: "Copy reference" and the add-an-id path.

Each part in detail:

- **Textarea over contenteditable:** plain text gives exact control over what is saved. The browser's own undo works inside it, and IME and spellcheck behave.
- **Mapping DOM to model:** blocks map by anchor. The page's own `section.content li.block[id]` elements carry the anchors that the API returns.
- **Adding an id to another page's block:** GET that page, find the block by anchor, and insert `<indent>  id:: <id>` after its existing property lines (or its first line). Then PUT. On 409, retry once from a fresh GET.
- **Generated ids:** 8 characters from `crypto.getRandomValues` mapped onto `a–z0–9`. That gives 2.8 × 10¹² possibilities; a collision would surface as the existing duplicate-id diagnostic.
- **"Today":** a header link the editor inserts. It GETs `journals/<local date>.html`. If `exists` is false, it PUTs `-\n` with version `missing`; a 409 means another tab created the day, so it just proceeds. It then navigates with `#b1` and the editor starts editing the first block.

### 8. Saving and updating in place

- **When it saves:** after a structural operation, on blur or Escape, and 1 s after the last input.
- **Queue:** at most one `PUT` per page is in flight. A save requested meanwhile sends the latest text once the current one finishes.
- **Status:** an indicator next to "Today" shows Saved, Saving…, or Not saved.
- **Network failure:** the queue retries with backoff (1 s up to 10 s) and also when live reload reconnects. `beforeunload` warns while anything is unsaved.
- **Updating in place:** while the editor is active, `live-reload` does not reload for the tab's own page. It dispatches `PAGES_CHANGED` instead; the editor marks itself active with an attribute that `live-reload` checks, using a constant in `@hermetic/core`.
  - On `PAGES_CHANGED` for its page, the editor fetches the page HTML (`core/fetchpage`) and replaces the page's sections.
  - The block being edited keeps its live `<textarea>`, which is moved into the new `li`, refocused, and given back its selection.
  - If the file changed on disk and the active block has no unsaved changes, the editor also re-GETs the source and rebuilds its tree.

### 9. Conflicts

On 409 the editor:

1. takes the current text and blocks from the response and rebuilds its tree;
2. ends editing;
3. shows a notice above the page content that says the page changed elsewhere, holds the refused block text in a read-only field with "Copy", and has "Dismiss".

There is no automatic merge, because silent merges of prose are where data goes missing.

### 10. Keys and shortcuts

- `<hermetic-shortcuts>` already ignores key presses in text fields, so `g …`, `/`, `\`, and `?` do not fire while editing. Ctrl+K still focuses search.
- Its help dialog gains an "Editing" group, listed only when a `hermetic-editor` element is present.
- Alt+Shift+arrows and Ctrl/Cmd+Enter are handled on the textarea, which calls `preventDefault`.

### 11. Testing

**Rust:**
- unit tests for versioning, CRLF handling, target resolution, and the authenticity matrix (peer × Host × Origin × header × binding);
- HTTP tests in `tests/serve.rs`: GET/PUT round trip, 409, 404 on non-editable paths, 413, a symbolic link out of the tree (Unix only), a namespaced create, and one rebuild per PUT.

**Cross-language fixtures:**
- A Rust test writes the GET payload for each file in `tests/fixtures/editing/` into `static/packages/editor/test-fixtures/` (regenerated with `UPDATE_SNAPSHOTS=1`, like the search index).
- TypeScript tests check that an unchanged tree serializes byte-identically for every fixture, and that each outliner operation yields a hand-checked expected file.
- The fixtures include CRLF, tabs, irregular indentation, prose preambles, code fences, and properties.

**Browser (Playwright, Chromium and Firefox):**
- every scenario of `browser-editing` that a browser can exercise, against `hermetic serve` on a copy of a fixture notes directory;
- editing from a non-loopback `Host` is absent.

Implementing the source payload found that the parser panicked on any empty bullet (`-` or `- `), which also broke `hermetic build`. The fix is part of this change, together with a property test: parsing never panics, and the blocks' sources always reassemble the text.

## Risks / Trade-offs

- [A serializer bug rewrites untouched text] → Untouched blocks are copied by line range, never re-rendered. The fixture tests assert byte identity, and the server refuses stale versions, so damage is limited to what the browser sent for that page.
- [Every save rebuilds the whole site, about 0.4 s at 5,000 pages] → Saves are queued and asynchronous to typing. The 2-second visibility target already holds for serve. Incremental rebuilds stay out of scope.
- [No structural undo] → Text undo works inside a block. Deleting a block takes a deliberate Backspace on an empty block. Notes are expected to be in version control. Structural undo can come later without changing the API.
- [Another user on the same machine can write through loopback] → The same holds for the files themselves on such machines. The loopback restriction targets other devices and websites, not local users.
- [Moving the focused textarea during an in-place update loses browser undo history in some browsers] → Updates swap only when the build changed the page. Bodies are patched without moving the textarea when the block structure is unchanged (the common case of a text edit).
- [A file mixing LF and CRLF line endings is saved with one style, chosen by its first line ending] → Mixed files are rare and the change is only in line endings. Consistent files keep their style exactly.
- [`PUT` waits for the rebuild, so a slow rebuild blocks the save queue] → The queue shows "Saving…". The text stays in the browser, and the next save carries all newer text.

## Migration Plan

None. The feature is additive and exists only in `serve`. Built sites and their bytes are unchanged, so there is nothing to roll back beyond the binary.
