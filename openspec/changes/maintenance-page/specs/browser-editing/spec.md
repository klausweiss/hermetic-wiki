# Spec Delta

## MODIFIED Requirements

### Requirement: Request authenticity
The server SHALL answer every request to the edit API and to the maintenance API (see `maintenance`) with 403, and change nothing, unless all of these hold: it comes from a loopback address; its `Host` header names `localhost`, `127.0.0.1`, or `[::1]` with the server's port; and, for `PUT`, `POST`, and `DELETE`, it carries an `Origin` header equal to `http://` followed by that `Host`, and the header `X-Hermetic-Edit: 1`. This SHALL hold for every binding, including `--host` and `--public`. The edit API SHALL send no CORS headers.

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

#### Scenario: Cross-site trash
- **WHEN** a page from another website sends a `POST` to `http://127.0.0.1:8000/_hermetic/unused-assets` naming an unused asset
- **THEN** the request is refused and the file is not moved

### Requirement: Safe writes
The file written for a page SHALL be the page's existing source file or, for a page without one, the path derived from its name by the `source-layout` rules. It SHALL never be a path taken from the request. Before writing, the server SHALL verify that the target, with symbolic links resolved, is inside the notes directory under `journals/` or `pages/` (or is the root `index.md`), and SHALL refuse with 403 otherwise. A deletion SHALL move the file, after the same checks, to `.trash/<timestamp>/<original relative path>` in the notes folder, never outside it, and SHALL remove directories under `pages/` that this leaves empty (never `pages/` or `journals/` themselves). Moving an unused asset to the trash (see `maintenance`) SHALL follow the same rules, except that the file, with symbolic links resolved, SHALL be inside `journals/`, `pages/`, or `assets/` and SHALL NOT be a markdown file; `assets/` itself SHALL never be removed. When `.trash/.gitignore` does not exist, the deletion SHALL also create it with content that makes git ignore everything in `.trash/`; an existing `.trash/.gitignore`, whatever its content (even empty), SHALL be left unchanged. Failing to create it SHALL NOT fail the deletion. Nothing SHALL remove files from `.trash/`. Writes SHALL be atomic: a reader SHALL see either the old or the new content, never a partial file. Temporary files SHALL begin with `.`, so the build and the watcher ignore them, and SHALL be removed if the write fails.

#### Scenario: Symbolic link out of the notes
- **WHEN** `pages/out.md` is a symbolic link to a file outside the notes directory and the editor saves the page `out`
- **THEN** the response is 403 and the outside file is unchanged

#### Scenario: Namespaced new page
- **WHEN** the page `Project X/Auth` without a file is saved
- **THEN** `pages/Project X/Auth.md` is created, and no file outside `pages/` is created or changed

#### Scenario: Crash during write
- **WHEN** the server is killed while writing `pages/auth.md`
- **THEN** `pages/auth.md` holds either its old or its new content

#### Scenario: Trashing an asset
- **WHEN** the unused asset `assets/a.png` is moved to the trash
- **THEN** it is at `.trash/<timestamp>/assets/a.png` and `assets/` still exists, also when it is now empty
