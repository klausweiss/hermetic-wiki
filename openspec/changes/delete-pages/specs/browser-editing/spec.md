# Spec Delta

## ADDED Requirements

### Requirement: Deleting pages
Where editing is available, a page that has a file (a regular page, a journal day, or the home page's `index.md`) SHALL offer "Delete page" in its header. Choosing it SHALL ask for confirmation, naming the file and the number of blocks on other pages that reference the page; nothing SHALL change unless the author confirms. A confirmed deletion SHALL move the file to the trash (see "Safe writes"), open the home page, and leave every other file unchanged. A page that is still referenced SHALL remain as a page without a file. Pages without a file SHALL NOT offer the control.

#### Scenario: Delete a page
- **WHEN** the author deletes the page `scratch` (`pages/scratch.md`), which nothing references, and confirms
- **THEN** `pages/scratch.md` is gone, a copy is under `.trash/`, the home page opens, and `pages/scratch.html` is no longer served

#### Scenario: Still referenced
- **WHEN** the author deletes `pages/redis.md` while a journal contains `[[redis]]`
- **THEN** the confirmation mentions 1 referencing block, and after deleting `pages/redis.html` shows the page without content and with its linked reference

#### Scenario: Cancel
- **WHEN** the author chooses "Delete page" and cancels the confirmation
- **THEN** nothing changes

#### Scenario: Empty namespace directory
- **WHEN** the author deletes `pages/Project X/Auth.md`, the only file in `pages/Project X/`
- **THEN** the directory `pages/Project X/` is removed and `pages/` remains

## MODIFIED Requirements

### Requirement: Edit API
The server SHALL offer one page's source at `/_hermetic/source?path=<site path>`, where `<site path>` is the page's HTML path relative to the site root (for example `pages/Project%20X/Auth.html`; the home page is `index.html`).

`GET` SHALL return, as JSON, whether the page has a file, a version identifying the file's current bytes, the file's text, and its blocks in document order. Each block SHALL have its line range, depth, whether it is a bullet, and its anchor on the page. A page without a file SHALL have empty text and the version of a missing file.

`PUT` SHALL replace the file with the request body. It SHALL be accepted only when the request names the version it is based on and that version is the file's current one; otherwise the response SHALL be 409 with the current version and text. The response to a successful `PUT` SHALL be sent after the site has been rebuilt with the change, and SHALL give the new version and blocks.

`DELETE` SHALL delete the page's file as "Deleting pages" requires, accepted only when the request names the version it is based on and that version is the file's current one (otherwise 409 with the current version and text); a page without a file SHALL get 404. Its response SHALL be sent after the site has been rebuilt without the page.

The server SHALL answer 404 for a site path that is not an editable page, 400 for a body that is not UTF-8 text, and 413 for a body larger than 8 MiB.

#### Scenario: Read a page
- **WHEN** the editor requests `GET /_hermetic/source?path=pages/auth.html`
- **THEN** the response contains the exact text of `pages/auth.md`, its version, and one entry per block with its anchor (`b1`, `b2`, … or `id-<id>`)

#### Scenario: Stale version
- **WHEN** a `PUT` names a version that is no longer the file's current version
- **THEN** the response is 409 with the current version and text, and the file is unchanged

#### Scenario: Not editable
- **WHEN** a `PUT` targets `path=tasks.html` or `path=refs/pages/kafka/2.html`
- **THEN** the response is 404 and nothing is written

#### Scenario: Delete with a stale version
- **WHEN** a `DELETE` names a version that is no longer the file's current version
- **THEN** the response is 409 and the file is not moved

### Requirement: Request authenticity
The server SHALL answer every request to the edit API with 403, and change nothing, unless all of these hold: it comes from a loopback address; its `Host` header names `localhost`, `127.0.0.1`, or `[::1]` with the server's port; and, for `PUT` and `DELETE`, it carries an `Origin` header equal to `http://` followed by that `Host`, and the header `X-Hermetic-Edit: 1`. This SHALL hold for every binding, including `--host` and `--public`. The edit API SHALL send no CORS headers.

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

### Requirement: Safe writes
The file written for a page SHALL be the page's existing source file or, for a page without one, the path derived from its name by the `source-layout` rules. It SHALL never be a path taken from the request. Before writing, the server SHALL verify that the target, with symbolic links resolved, is inside the notes directory under `journals/` or `pages/` (or is the root `index.md`), and SHALL refuse with 403 otherwise. A deletion SHALL move the file, after the same checks, to `.trash/<timestamp>/<original relative path>` in the notes folder, never outside it, and SHALL remove directories under `pages/` that this leaves empty (never `pages/` or `journals/` themselves). Writes SHALL be atomic: a reader SHALL see either the old or the new content, never a partial file. Temporary files SHALL begin with `.`, so the build and the watcher ignore them, and SHALL be removed if the write fails.

#### Scenario: Symbolic link out of the notes
- **WHEN** `pages/out.md` is a symbolic link to a file outside the notes directory and the editor saves the page `out`
- **THEN** the response is 403 and the outside file is unchanged

#### Scenario: Namespaced new page
- **WHEN** the page `Project X/Auth` without a file is saved
- **THEN** `pages/Project X/Auth.md` is created, and no file outside `pages/` is created or changed

#### Scenario: Crash during write
- **WHEN** the server is killed while writing `pages/auth.md`
- **THEN** `pages/auth.md` holds either its old or its new content
