# Proposal

## Why

The core workflow is writing in today's journal and linking pages that come into existence when first referenced. Today that needs a separate editor next to the browser tab, and the tab only shows the result after a save. Writing in the same outliner view used for reading, as in Logseq, makes `hermetic serve` a complete replacement for Logseq on project notes, while the notes stay plain markdown files.

## What Changes

- In `hermetic serve`, pages and journal days can be edited from a browser on the same machine with a Logseq-style block outliner. Clicking a block edits its markdown in place. Enter starts a new block, Tab and Shift+Tab indent and outdent, Alt+Shift+Up/Down move a block, and Backspace at the start of a block joins or deletes it. Typing `[[` offers matching pages and `((` offers matching blocks, using the search engine.
- Edits are saved to the markdown files automatically. Blocks that were not edited keep their exact bytes; the file's indentation style and line endings are kept.
- A "Today" control opens today's journal and creates it if needed. Following a link to a page that has no file and writing in it creates the file.
- "Copy reference" on a block puts `((id))` on the clipboard, first adding a generated `id::` property to the block if it has none.
- Conflicts are detected: a save is refused when the file changed on disk (for example in `$EDITOR`) since the browser loaded it. The unsaved text stays available to the writer.
- A small edit API in the server: read and write one page's source, addressed by its site path. Requests are accepted only from the same machine, with a loopback `Host`, a same-origin `Origin`, and a custom header, so other websites and other devices cannot write. File paths are derived from page names by the `source-layout` rules, and every write is checked to stay inside the notes directory, then written atomically.
- After a save, the server rebuilds immediately. The tab being edited updates in place instead of reloading.
- Built sites, and pages served to other devices (`--public`, `--host`), are unchanged and read-only.

### Non-goals

- Editing inside sidebar panes, the timeline, linked references, or search results. Those stay read-only and update after saves.
- Undo across blocks (structural undo); text undo inside the block being edited is the browser's own.
- Multi-block selection, drag and drop, collapsing blocks, deleting or renaming pages.
- Editing from other devices on the network.

## Capabilities

### New Capabilities
- `browser-editing`: when editing is available, the block outliner and its keys, lossless saving, page and journal creation, page and block autocomplete, block references with generated ids, conflict handling, and the edit API with its path-safety and request-authenticity rules.

### Modified Capabilities
- `serve-mode`: served HTML may also contain the editor element (only for requests from this machine). `serve` writes note files when asked to by the editor. A `PUT` method is accepted for the edit API. Saves trigger an immediate rebuild. A page open for editing updates in place instead of reloading.
- `cli`: the `commands` section of `hermetic help` mentions browser editing in `serve` and that it is limited to the same machine.

## Impact

- Rust: `src/serve/` gains the edit API (request bodies, peer and origin checks, atomic writes, immediate rebuild, a site-path → page map kept with each build). `src/serve/http.rs` gains body reading with a size limit. `src/help.rs` is updated. No new crates.
- Frontend: a new Bun package `static/packages/editor` (`<hermetic-editor>`) bundled into `app.js`. `live-reload` hands page changes to the editor instead of reloading. `shortcuts` lists the editing keys in its help dialog when editing is available.
- Tests: Rust unit and HTTP tests for the API and its safety rules. Cross-language fixtures that check the TypeScript serializer against the Rust parser. Browser checks in Chromium and Firefox.
- Depends on the archived `serve-mode`, `block-model` (source line ranges), `source-layout` (name encoding), and `search` capabilities.
