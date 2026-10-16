# Tasks

## 1. Attachment references (parser and graph)

- [ ] 1.1 Add `RawRefKind::Attachment` in `src/parse.rs`: record inline images and inline links whose destination is relative or root-relative, has no scheme, does not start with `#` or `//`, and is not a note link, with `dest` set to the destination range. Keep skipping other refs inside image alt text. Verify with new unit tests in `parse.rs` (image, link to a pdf, external URL, `data:` URL, `#frag`, code span) and `cargo test parse`
- [ ] 1.2 Split `resolve_link_path` in `src/graph.rs` into a shared normalizer and add `RefTarget::Attachment(PathBuf)` (outside the notes → `Unresolved`). Make sure attachment refs add no backlinks, related pages, or graph edges. Verify with unit tests for `../assets/x.png` from a journal, `../../assets/x.png` from `pages/Project X/Auth.md`, `/assets/x.png`, a path escaping the root, and `tests/reference_graph.rs` passing unchanged

## 2. Rebasing attachment destinations in rendering

- [ ] 2.1 Add the `(Attachment, RefTarget::Attachment(path))` arm to `render_text` in `src/site/render.rs`, rewriting `dest` to `paths::href(from, path, fragment)`. Leave unresolved attachment refs as written (not marked broken). Verify with render unit tests for the own page, a linked-references page, and `refs/pages/auth/2.html`
- [ ] 2.2 Add site tests in `tests/site.rs` for each "Attachment links" scenario in `specs/static-site-output/spec.md` (journal image in linked refs, continuation page, namespaced page via block ref, image next to its note, root-relative, left as written). Verify with `cargo test --test site`
- [ ] 2.3 Update `src/help/layout.txt` (attachments paragraph) to say relative and `/`-rooted image paths work wherever a block is shown. Verify with `tests/help.rs` passing

## 3. Image upload endpoint

- [ ] 3.1 In `src/serve/edit.rs`, add `MAX_ASSET` (32 MiB), `sniff_image(&[u8]) -> Option<&'static str>` (png/jpg/gif/webp by magic bytes), `asset_stem(name: Option<&str>)` (percent-decode, sanitize to `[A-Za-z0-9_-]`, 64 chars, `image` fallback), and `asset_markdown(source: &Path, file_name)` (`..` per parent component). Verify with unit tests including an SVG and HTML disguised as `.png`, a Unicode name, and the home page, journal, and namespaced-page destinations
- [ ] 3.2 Extend `edit::allowed` to accept paths under `assets/` and add `write_new_atomic(root, dir, stem, ext, bytes)`: dot-prefixed temp file, `hard_link` without replacing, retry with `n + 1` on `AlreadyExists`, fall back to `create_new` with a warning on `Unsupported`, always remove the temp file. Verify with unit tests for name collisions, a symlinked `assets/` pointing outside (→ `Unsafe::Outside`), and no temp files left behind
- [ ] 3.3 Route `POST /_hermetic/asset` in `src/serve/mod.rs`: 405 for other methods, 403 unless `same_machine` and `from_editor`, `read_body` with `MAX_ASSET` (413), 404 for a non-editable `path`, 415 for unknown formats; write, update the recorded inputs, and rebuild under `state.rebuilding`; reply 201 with `{file, markdown}`. Update the 405 rule in `src/serve/http.rs` if it lists methods. Verify with tests in `tests/edit.rs` covering every "Image upload API" scenario and the "Cross-site upload" scenario, plus a `tests/serve.rs` case showing `PUT /_hermetic/asset` → 405 and the uploaded file served with `Content-Type: image/png` right after the 201
- [ ] 3.4 Verify that the watcher does not trigger a second rebuild for an uploaded file: assert the rebuild count in a `tests/serve.rs` test, the same way the existing save test does

## 4. Editor: paste and drop

- [ ] 4.1 Add `SourceApi.upload(path, file)` in `static/packages/editor/src/api.ts` (POST, `X-Hermetic-Edit`, `X-Hermetic-Name`, `Refused` with status on failure). Verify with cases in `api.test.ts` for 201, 413, 415, and a network error
- [ ] 4.2 Add an insertion operation in `ops.ts`/`model.ts` that puts markdown into a given `Node` at an offset or at the end of its text before property lines (space-separated), joins several images with `\n`, and falls back to a new last root node when the node is gone. Verify with `model.test.ts` cases for each of these, including a block with properties
- [ ] 4.3 Implement `addImages(files, target)` in `element.ts`: client-side type filter with a notice for skipped files, sequential uploads, insertion as each finishes, a save after insertion, and a pending-upload counter feeding the save-state indicator and `beforeunload`. Verify by running the editor tests, then manually in `hermetic serve` (via the `run` skill or a browser) with a pasted screenshot and with Escape pressed during the upload
- [ ] 4.4 Add the paste listener on the block textarea ("text wins" rule) and the drop handling: own-block rows, the empty block area for a new last block, and a document-level `preventDefault` for `Files` elsewhere on editable pages. Verify manually in Firefox and Chromium: screenshot paste, "Copy image" from a web page, spreadsheet cells (text only), drop on a non-edited row, drop of several files below the last block, drop on linked references (no-op, no navigation). Adjust the "text wins" rule and its scenario if "Copy image" puts a URL on the clipboard as plain text
- [ ] 4.5 Update the edit-mode paragraph in `src/help/commands.txt`: paste or drop PNG/JPEG/GIF/WebP images, which are stored in `assets/` and linked relative to the note. Verify with `tests/help.rs` passing

## 5. Integration check

- [ ] 5.1 Run `make test` and the editor package tests. Then, in `hermetic serve` on `wiki/`, paste an image into today's journal and a namespaced page, confirm the image shows on the page, in linked references, and on the home timeline, and confirm `hermetic build` output shows it when opened via `file://`. Run `openspec validate paste-images --strict`
