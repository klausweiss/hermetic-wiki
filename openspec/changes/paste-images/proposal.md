# Proposal

## Why

Adding a screenshot or diagram to a note today means saving the file into the notes folder by hand and typing its relative path. Edit mode should let the author paste or drop an image into a block the way Logseq does. Pasting will also expose a rendering bug that hand-written images mostly avoided: relative image paths are emitted exactly as written, so an image breaks wherever its block appears outside its own page (linked references, continuation pages, the timeline, sidebar panes).

## What Changes

- In edit mode, pasting an image into the block being edited, or dropping image files on the page, uploads each image to the root `assets/` folder and inserts `![<name>](<relative path to assets/…>)` into the block (Logseq convention, with the `..` count computed for the block's own source file, so namespaced pages work).
- New edit API endpoint for uploading an image. It is guarded by the same same-machine and editor-request checks as the existing edit API. The server, not the request, chooses the file name and location. Only PNG, JPEG, GIF, and WebP are accepted, recognized by their content; SVG and everything else is refused.
- Relative and root-relative image and link destinations that point to attachments (non-markdown files in the notes) are resolved against the block's source file and rendered relative to the page that shows the block, everywhere blocks are rendered. This also fixes images written by hand.
- The serve-mode HTTP rules allow `POST` to the new upload endpoint.

Out of scope, and planned as the follow-up change `maintenance-page`: finding and trashing unused images, and reporting broken image links.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `browser-editing`: new requirements for pasting and dropping images in edit mode and for the image upload endpoint.
- `static-site-output`: new requirement that attachment destinations in block content are rebased to the page that renders the block.
- `serve-mode`: "HTTP behavior" allows `POST` to the image upload endpoint.

## Impact

- `src/parse.rs`, `src/graph.rs`: images and links to non-note files become references with destination ranges, resolved against the source file like note links.
- `src/site/render.rs`: rewrites attachment destinations relative to the rendering page.
- `src/serve/mod.rs`, `src/serve/edit.rs`, `src/serve/http.rs`: new `POST /_hermetic/asset` route, content sniffing, file naming, atomic write into `assets/`, rebuild before responding.
- `static/packages/editor`: paste and drop handlers, upload client in `api.ts`, insertion into the block model, upload state and error notices.
- `src/help/commands.txt`, `src/help/layout.txt`: describe pasting, dropping, and where images go.
- No new dependencies. No change to built sites beyond correct image paths on pages that show blocks from other notes.
