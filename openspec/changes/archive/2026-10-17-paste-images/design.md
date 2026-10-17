# Design

## Context

The motivation is in proposal.md. This section covers only the current state that shapes the approach:

- **Parsing.** `src/parse.rs` (`scan_chunk`) finds inline markdown links but keeps only note links (`is_note_link`) as `RawRefKind::MdLink`, with the destination's byte range. It skips images entirely ("Images are never references"). `src/graph.rs` (`resolve_link_path`) resolves a note link against the source file's folder, treats a leading `/` as the notes root, and detects paths that leave the notes folder.
- **Rendering.** `src/site/render.rs` (`render_text`) rewrites the source text before CommonMark rendering: each resolved `MdLink` destination range is replaced by `page_href(from, …)`. Every other destination passes through unchanged, which is the bug described in the proposal. `from` is already passed everywhere a block is rendered (own page, linked references, continuations, block refs, timeline), and `paths::href(from, to, fragment)` already computes relative, percent-encoded hrefs.
- **Edit API.** `src/serve/mod.rs` routes `_hermetic/source` to a handler that checks `same_machine`, then `from_editor`, then reads the body with `http::read_body(…, MAX_BODY)`. It resolves `path` through `edit::resolve`, writes with `edit::checked_target` and `edit::write_atomic`, and rebuilds under `state.rebuilding`, updating the recorded inputs so the watcher does not rebuild a second time. `edit::allowed` permits only `journals/`, `pages/`, and the root `index.md`.
- **Editor.** `static/packages/editor` holds the page as a `Doc` of `Node` objects. Text is changed through `ops.ts` and saved by a queue with a save-state indicator. `beforeunload` already warns about unsaved changes. The `Payload` from `GET /_hermetic/source` includes `file`, the page's source path.
- **Publishing.** Attachments under `assets/` are already published (`site/assets.rs`), watched, and served with the right `Content-Type` for PNG, JPEG, GIF, and WebP.

## Goals / Non-Goals

**Goals:**
- One mechanism, used by both the build and serve mode, makes attachment destinations correct on every page that renders a block.
- The upload endpoint reuses the edit API's checks and write path, and gets no special-case security rules.
- Paste and drop share a single upload-and-insert path in the editor.

**Non-Goals:**
- Unused-image and broken-link reporting. These belong to the follow-up `maintenance-page` change, which will consume the attachment references added here.
- Resizing, compressing, or converting images. Bytes are stored as received.
- Rebasing reference-style links (`[x][ref]`) or raw HTML `<img>`.
- Pasting non-image files.

## Decisions

### 1. Attachment references in the parser and graph
Add `RawRefKind::Attachment`. `scan_chunk` records it for inline images and for inline links whose destination is relative or root-relative, has no scheme, and is not a note link. Like `MdLink`, it carries `dest` as a byte range. Image alt text keeps being skipped for other references, as it is today. In `graph.rs`, `resolve_link_path` is split into a shared path normalizer (`source folder + dest → notes-relative path | Outside`) used by both kinds. The resolved target becomes `RefTarget::Attachment(PathBuf)`, or `RefTarget::Unresolved` when the path is outside, in which case the destination is left as written.

Attachment refs never count as page references. They add nothing to backlinks, the graph view, or related pages.

- *Alternative: post-process the rendered HTML and rewrite `src`/`href`.* Rejected. By then the source file is unknown, and markup the author wrote would be rewritten too.
- *Alternative: rewrite inside a pulldown-cmark event stream in render.* Rejected because it splits link handling across two mechanisms. The source-text edit approach already handles note links and keeps byte ranges exact.

The maintenance page will get "which images are referenced" from the graph for free.

### 2. Rewriting in `render_text`
Add a match arm for `(Attachment, RefTarget::Attachment(path))` that pushes `(dest, paths::href(from, &path, fragment))`. The href is percent-encoded, so it never contains spaces or parentheses that would need `<…>` around it. The destination is rewritten whether or not the file exists, so it stays deterministic and independent of the filesystem. Attachment output paths equal their notes-relative paths (spec "Attachments"), so no lookup is needed.

### 3. Upload endpoint: `POST /_hermetic/asset?path=<site path>`
- **Routing.** A second `_hermetic` route next to `SOURCE_PATH` in `src/serve/mod.rs`. Method handling goes `POST` → the handler, anything else → 405, then `same_machine` and `from_editor` → 403. These are the same checks in the same order as the source handler.
- **Body.** `http::read_body(…, MAX_ASSET)` with a new `MAX_ASSET = 32 MiB` constant. The 8 MiB page limit is too small for photos, and a separate constant keeps the page limit unchanged.
- **Format.** Recognized by magic bytes only: PNG `89 50 4E 47 0D 0A 1A 0A`, JPEG `FF D8 FF`, GIF `GIF87a`/`GIF89a`, WebP `RIFF????WEBP`. Anything else gets 415. SVG is never recognized, so it can never land in `assets/` through the editor. This matters because served SVG runs scripts on the same origin as the edit API.
- **Name.** `<stem>_<unix ms>_<n>.<ext>`, following Logseq's `image_1697…_0.png` style. The stem is sanitized from `X-Hermetic-Name` (percent-decoded and limited to ASCII `[A-Za-z0-9_-]`), so the returned markdown never needs escaping and file names are safe on every filesystem.
- **No-replace atomic write.** Write a temporary `.`-prefixed file in `assets/` (the build and the watcher ignore it), then `fs::hard_link(tmp, target)`. That call fails with `AlreadyExists` instead of replacing, in which case retry with `n + 1`. Finally remove the temporary file. `rename` would silently replace an existing file. `edit::allowed` is extended to also accept paths under `assets/`, and `checked_target` stays the single safety gate, which resolves symlinks.
  - *Alternative: `OpenOptions::create_new` directly on the target.* It doesn't replace files, but a crash midway leaves a partial image under its final name, which breaks the "atomic" rule.
- **Rebuild and reply.** Under `state.rebuilding`: write, update the recorded inputs, rebuild, release. Then reply with 201 `{ "file": "assets/…", "markdown": "![…](…)" }`. The image is part of the build before the editor inserts the markdown, so the first render after the text is saved already shows it.
- **Markdown destination.** Computed on the server from `edit::source_path(&page)`: one `..` per component of its parent folder, then `assets/<name>`. The server already owns the page-to-file mapping (see the 10-15 journal note about it being duplicated), so the editor doesn't get another copy of it.

### 4. Editor: one `addImages(files, target)` path
- `api.ts` gains `upload(path, file): Promise<{file, markdown}>`. It sends the same `X-Hermetic-Edit` header, plus `X-Hermetic-Name: encodeURIComponent(file.name)`. A 415 or 413 throws `Refused` with its status, so the notice can say why.
- **Target.** Either `{ node, offset }` (paste: the edited node and the caret position in its body) or `{ node, end: true }` (drop on a row). Without a node (drop in the empty area), a new last root node is created on the first successful upload. `Node` objects keep their identity across edits, so the insertion target survives editing ending or moving. When the node is no longer in the `Doc`, the image goes into a new last root node.
- **Order.** Uploads run one at a time in input order, and each result is inserted when it arrives. With several images, each is joined with `\n` so it sits on its own line. Afterwards the normal save queue is triggered (`ops` + save, the same as typing).
- **Paste.** A `paste` listener on the textarea. If `clipboardData.getData("text/plain").trim()` is non-empty, do nothing (default paste). Otherwise collect the `clipboardData.files` that are images, call `preventDefault`, and add them.
- **Drop.** `dragover`/`drop` on the editor's own block `section` with `preventDefault` when `dataTransfer.types` includes `Files`. Rows are found with `closest()` on the block row element. A document-level `dragover`/`drop` listener calls `preventDefault` for `Files` everywhere else on an editable page and does nothing more, so the browser never navigates to a dropped file.
- **Client-side filter.** File types are filtered by `type` before uploading (`image/png|jpeg|gif|webp`), so an unsupported file gets an immediate notice instead of a round trip. The server check remains the authority.
- **Pending state.** While any upload is in flight, the save-state indicator shows "not saved" and `beforeunload` warns (one counter feeding the existing state).

## Risks / Trade-offs

- [The "text wins" rule may misfire in some apps: copying an image in a browser can also put its URL on the clipboard as plain text] → Check Firefox and Chromium "Copy image" and a desktop screenshot tool during implementation. If a browser adds plain text alongside the image, refine the rule to "plain text that is a single URL does not count as text" and update the scenario.
- [Every upload triggers a full rebuild, and so does the save that follows] → Fine for single pastes. Dropping N files means N + 1 rebuilds, run in sequence. Incremental rebuilds are tracked separately (10-15 journal note).
- [32 MiB bodies are held in memory] → One upload at a time from one local editor. Acceptable for now.
- [`hard_link` is unsupported on some filesystems (e.g. some FUSE or network mounts)] → On `Unsupported`, fall back to `create_new` and write directly, accepting a non-atomic write there. Log a warning on standard error.
- [Rewriting attachment destinations changes the HTML of built sites whose notes contain relative images shown outside their own page] → Intended: those images were broken before. Pages showing a block on its own page keep identical output whenever the destination was already relative to that page.
- [Uploaded images that end up unused (paste then undo) accumulate in `assets/`] → By design. The `maintenance-page` change lists them and moves them to the trash.

## Migration Plan

No data migration is needed. Existing notes render the same or better. Rollback means reverting the change, and uploaded images stay as ordinary attachments.
