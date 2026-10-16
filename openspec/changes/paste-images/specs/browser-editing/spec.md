# Spec Delta

## ADDED Requirements

### Requirement: Pasting and dropping images
Where editing is available, the author SHALL be able to add PNG, JPEG, GIF, and WebP images to the page's own blocks by pasting or dropping them. Each image SHALL be stored through the image upload of the edit API and inserted into the block as the markdown that upload returns.

Pasting into the block being edited SHALL upload the clipboard's image files, in clipboard order, when the clipboard holds at least one image file and no non-empty plain text; otherwise pasting SHALL behave as before. Dropping files on a block row of the page's own blocks, whether or not it is being edited, SHALL add the dropped images to that block. Dropping files anywhere else in the page's own block area SHALL add them to a new last top-level block. Dropping files elsewhere on an editable page (linked references, related pages, sidebar panes, the timeline, the header) SHALL do nothing. On an editable page, the browser SHALL NOT open a dropped file in place of the page.

The markdown for an image SHALL be inserted where the cursor was when it was pasted, or, for a drop, at the end of the block's text before its property lines, separated from text before it by a space, as soon as its upload succeeds. It SHALL go into the same block even when editing has ended or moved to another block in the meantime, and into a new last top-level block when that block no longer exists. When several images arrive together, each SHALL be on its own line, in order. The inserted text SHALL be saved as "Saving" requires. While an upload is in progress, the page SHALL show that changes are not yet saved, and the browser SHALL warn before leaving the page.

Files that are not PNG, JPEG, GIF, or WebP SHALL be skipped. When anything was skipped, or an upload is refused or fails, the page SHALL show a notice naming the file and the reason, and nothing SHALL be inserted for that file. Uploads are not retried.

#### Scenario: Paste a screenshot
- **WHEN** the author edits a block of `journals/2026-10-17.html` and pastes a screenshot from the clipboard
- **THEN** a PNG file appears in `assets/`, and the block contains `![image.png](../assets/image_<timestamp>_0.png)` at the cursor, saved to `journals/2026_10_17.md`

#### Scenario: Text wins over images
- **WHEN** the author pastes cells copied from a spreadsheet, which put both plain text and an image on the clipboard
- **THEN** the text is pasted and nothing is uploaded

#### Scenario: Drop on a block that is not being edited
- **WHEN** the author drops `diagram.webp` on the row of the block `- architecture` of `pages/auth.html`
- **THEN** the block reads `architecture ![diagram.webp](../assets/diagram_<timestamp>_0.webp)` and is saved

#### Scenario: Drop several files on an empty area
- **WHEN** the author drops `a.png` and `b.jpg` below the last block of the page
- **THEN** a new last top-level block holds both images, each on its own line, in that order

#### Scenario: Namespaced page
- **WHEN** the author pastes an image into a block of the page `Project X/Auth`
- **THEN** the inserted destination is `../../assets/…`, and the image displays on `pages/Project X/Auth.html`

#### Scenario: Unsupported file
- **WHEN** the author drops `notes.pdf` and `logo.svg` on a block
- **THEN** nothing is uploaded or inserted, and a notice says that only PNG, JPEG, GIF, and WebP images can be added

#### Scenario: Editing ends during the upload
- **WHEN** the author pastes an image and presses Escape before the upload finishes
- **THEN** the image markdown is still inserted into that block at the paste position and saved

#### Scenario: Drop outside the page's blocks
- **WHEN** the author drops an image on the linked references section
- **THEN** nothing is uploaded and the browser stays on the page

### Requirement: Image upload API
The server SHALL accept an image for a page at `POST /_hermetic/asset?path=<site path>`, where `<site path>` names an editable page as the "Edit API" defines it (404 otherwise). The request body SHALL be the image's bytes. The request MAY carry the header `X-Hermetic-Name` with the image's original file name, percent-encoded as UTF-8.

The server SHALL recognize the format from the body's content alone, never from the name or the `Content-Type` header. It SHALL accept PNG, JPEG, GIF, and WebP, and answer 415 for anything else, including SVG. A body larger than 32 MiB SHALL get 413.

The server SHALL write the image to `assets/<stem>_<milliseconds since the Unix epoch>_<n>.<ext>` in the notes folder, creating `assets/` when it does not exist. `<stem>` is the original name without its extension, with every character other than ASCII letters, digits, `-`, and `_` replaced by `_` and cut to 64 characters, or `image` when no name is given or nothing is left. `<ext>` is `png`, `jpg`, `gif`, or `webp` according to the recognized format. `<n>` is the smallest number from 0 that names no existing file. An existing file SHALL never be replaced. The path SHALL never be taken from the request. The target, with symbolic links resolved, SHALL be inside `assets/` of the notes folder (403 otherwise). The write SHALL be atomic and follow the temporary-file rules of "Safe writes".

The upload SHALL NOT create or change the page's note file. The response SHALL be 201 and SHALL be sent after the site has been rebuilt with the new file. It SHALL give, as JSON, the file's path relative to the notes folder and the markdown to insert: `![<stem>.<ext>](<destination>)`, where `<destination>` is the path from the folder of the page's source file (the file "Safe writes" names, existing or not) to the image, with `/` separators and `..` segments.

#### Scenario: Upload for a journal day
- **WHEN** the editor posts a PNG with `X-Hermetic-Name: Screenshot%20from%202026-10-17.png` for `path=journals/2026-10-17.html`
- **THEN** the response is 201 with file `assets/Screenshot_from_2026-10-17_<ms>_0.png` and markdown `![Screenshot_from_2026-10-17.png](../assets/Screenshot_from_2026-10-17_<ms>_0.png)`, and the file serves at `/assets/Screenshot_from_2026-10-17_<ms>_0.png`

#### Scenario: Home page
- **WHEN** the editor posts an image for `path=index.html`
- **THEN** the markdown destination is `assets/<name>`, without `..`

#### Scenario: Name taken
- **WHEN** two images with the same name are posted within the same millisecond
- **THEN** they are stored as `…_0.png` and `…_1.png`, and neither replaces the other

#### Scenario: Disguised file
- **WHEN** the editor posts an SVG document or an HTML file with `X-Hermetic-Name: photo.png`
- **THEN** the response is 415 and nothing is written

#### Scenario: Too large
- **WHEN** the editor posts a 40 MiB JPEG
- **THEN** the response is 413 and nothing is written

#### Scenario: Not an editable page
- **WHEN** the editor posts an image for `path=tasks.html`
- **THEN** the response is 404 and nothing is written

#### Scenario: Symbolic link out of the notes
- **WHEN** `assets` is a symbolic link to a directory outside the notes folder
- **THEN** the response is 403 and nothing is written outside the notes folder

## MODIFIED Requirements

### Requirement: Request authenticity
The server SHALL answer every request to the edit API with 403, and change nothing, unless all of these hold: it comes from a loopback address; its `Host` header names `localhost`, `127.0.0.1`, or `[::1]` with the server's port; and, for `PUT`, `POST`, and `DELETE`, it carries an `Origin` header equal to `http://` followed by that `Host`, and the header `X-Hermetic-Edit: 1`. This SHALL hold for every binding, including `--host` and `--public`. The edit API SHALL send no CORS headers.

#### Scenario: Another website
- **WHEN** a page from `https://evil.example` in the author's browser sends a `PUT` to `http://127.0.0.1:8000/_hermetic/source?path=pages/auth.html`
- **THEN** the request is refused (by the browser for lack of CORS, or with 403 by the server) and `pages/auth.md` is unchanged

#### Scenario: DNS rebinding
- **WHEN** a request arrives with `Host: evil.example:8000` and a matching `Origin`
- **THEN** the response is 403, also when the server runs with `--public`

#### Scenario: Other device
- **WHEN** a device on the network sends a well-formed `PUT` to a server started with `--public`
- **THEN** the response is 403 and nothing is written

#### Scenario: Cross-site delete
- **WHEN** a page from another website sends `DELETE` to the edit API
- **THEN** the request is refused and no file is moved

#### Scenario: Cross-site upload
- **WHEN** a page from another website sends a form `POST` with an image to `http://127.0.0.1:8000/_hermetic/asset?path=index.html`
- **THEN** the response is 403 and nothing is written to `assets/`
