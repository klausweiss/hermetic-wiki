# serve-mode Specification

## Purpose

Lets the author read the notes as a live wiki while writing them: `hermetic serve` builds the site in memory, serves it over HTTP with every enhancement available, rebuilds when notes change, and updates open tabs without disturbing what is being read.

## Requirements

### Requirement: Serve command
`hermetic serve [<dir>] [--port <n>] [--host <address>] [--public] [--open]` SHALL build the notes directory `<dir>` (default: the current directory) and serve the result over HTTP until interrupted. The served site SHALL have the same files and content as `hermetic build` would produce for the same notes, except that served HTML pages additionally contain the live-reload element and, for requests from the same machine, the editor element. It SHALL NOT write the site to disk, and SHALL write nothing else except note files saved through browser editing and files it moves to `.trash/` when a page is deleted, along with `.trash/.gitignore` when that does not exist yet (see `browser-editing`). Diagnostics SHALL be printed as for `build`. When `<dir>` is not a directory, or an option is invalid, it SHALL print an error and exit with status 1 (status 2 for usage errors) without serving.

#### Scenario: Serve the current directory
- **WHEN** the user runs `hermetic serve` inside a notes directory
- **THEN** it prints a URL such as `http://127.0.0.1:8000/`, opening it shows the home page, and no `site/` directory is created

#### Scenario: Same output as build
- **WHEN** a notes directory is served and also built with `hermetic build`
- **THEN** every built file is served at the same path with the same bytes, except HTML pages, which differ only by the injected live-reload and editor elements

#### Scenario: Missing notes directory
- **WHEN** the user runs `hermetic serve nope` and `nope` does not exist
- **THEN** an error is printed and the exit status is 1

### Requirement: HTTP behavior
The server SHALL answer `GET` and `HEAD` requests for the site's files with status 200 and a correct `Content-Type` for HTML, CSS, JavaScript, fonts, images, and other common types (`application/octet-stream` otherwise). A path ending in `/` SHALL serve that directory's `index.html` when the site has one. Paths SHALL be percent-decoded before lookup, and any path that is not a file of the current build SHALL get a 404 page linking to the home page. Other methods SHALL get 405, except `PUT` and `DELETE` to the page source of the edit API of `browser-editing`, `POST` to its image upload, and `POST` to the maintenance API of `maintenance`. The maintenance pages of `maintenance` SHALL be served to a browser on the same machine although they are not files of the build. Responses SHALL not be cached by the browser (`Cache-Control: no-store`), so a reload always shows the current build.

#### Scenario: Encoded path
- **WHEN** the browser requests `/pages/Project%20X/Auth%20Flow.html`
- **THEN** the page `Project X/Auth Flow` is returned with `Content-Type: text/html; charset=utf-8`

#### Scenario: Path outside the site
- **WHEN** a client requests `/../../etc/passwd` or `/%2e%2e/notes.md`
- **THEN** the response is 404 and no file outside the build is read

#### Scenario: Directory index
- **WHEN** the browser requests `/`
- **THEN** the home page (`index.html`) is returned

#### Scenario: Other methods
- **WHEN** a client sends `POST /pages/auth.html`, `PUT /pages/auth.html`, `DELETE /pages/auth.html`, `PUT /_hermetic/asset?path=index.html`, or `DELETE /_hermetic/unused-assets`
- **THEN** the response is 405 and nothing is written

### Requirement: Rebuild on change
The server SHALL detect created, modified, and deleted notes and attachments (files under `journals/`, `pages/`, `assets/`, and the root `index.md`) and rebuild the site. Several changes in quick succession SHALL lead to one rebuild. A file saved through browser editing SHALL be rebuilt immediately, without waiting for change detection, and SHALL NOT cause a second rebuild when change detection notices it. For a notes directory of about 5,000 pages, a change SHALL be served within 2 seconds of the file being saved. While rebuilding, and if a rebuild fails, the server SHALL keep serving the last successful build and SHALL report the failure on standard error.

#### Scenario: New page appears
- **WHEN** the server is running and the author saves a new journal containing `[[kafka]]`
- **THEN** within 2 seconds `/journals/<that day>.html` and `/pages/kafka.html` are served and the journal list includes the day

#### Scenario: Deleted note disappears
- **WHEN** a page file is deleted while the server runs
- **THEN** after the rebuild its URL returns 404 and pages that linked to it show the updated state

#### Scenario: Burst of saves
- **WHEN** an editor writes the same file several times within 100 milliseconds
- **THEN** the server rebuilds once for that burst

### Requirement: Removing empty notes
Saves from the browser delete empty pages at once (see "Pages without content" in `browser-editing`); this rule catches notes left empty otherwise, for example by another editor. When it starts and before every rebuild, the server SHALL delete each note file — a page file under `journals/` or `pages/`, or the root `index.md`; a regular file reached without symbolic links — whose content is empty — only blank lines, or a single bullet without text (`-`) and nothing else — and that has not been modified for at least one hour. The file SHALL be deleted, not moved to the trash, directories under `pages/` that this leaves empty SHALL be removed (never `pages/` or `journals/` themselves), and the deletion SHALL be reported on standard error. Other files SHALL NOT be deleted by this rule. The deletion SHALL be part of that rebuild and SHALL NOT cause a second one.

#### Scenario: Left empty
- **WHEN** `journals/2026-10-12.md` contains only `-`, has not changed for two hours, and the author saves another page
- **THEN** `journals/2026-10-12.md` no longer exists and the journal list and timeline do not show that day

#### Scenario: Empty page
- **WHEN** `pages/Project X/Auth.md` contains only `-`, is the only file in `pages/Project X/`, and has not changed for two hours, and the server rebuilds
- **THEN** neither `pages/Project X/Auth.md` nor `pages/Project X/` exists, and the page remains only if other pages reference it

#### Scenario: Just written elsewhere
- **WHEN** another editor has just saved `journals/2026-10-13.md` or `pages/draft.md` with only `-` and the server rebuilds
- **THEN** both files still exist

#### Scenario: Content kept
- **WHEN** `journals/2026-10-01.md` contains `- standup` or two empty bullets, or `pages/about.md` contains only `title:: About`
- **THEN** none of them is deleted

### Requirement: Live reload
Pages served by `hermetic serve` SHALL receive, after every rebuild, the list of site paths whose content changed or which were removed. A tab SHALL reload only when its own page is in that list, keeping its scroll position and open sidebar panes, unless the page is open for editing, in which case it SHALL update in place as `browser-editing` requires; a sidebar pane whose page is in the list SHALL refresh its content in place, and a pane whose page was removed SHALL close. When the number of open tasks changed, every open tab SHALL show the new number on its open-tasks link (see "Site navigation" in `static-site-output`) without reloading. When the connection to the server is lost, the page SHALL keep working and reconnect when the server is back. Built sites (`hermetic build`) SHALL contain no live-reload behavior.

#### Scenario: Editing the page being read
- **WHEN** `pages/auth.html` is open on another device (where it is not open for editing), scrolled halfway, and the author edits `pages/auth.md`
- **THEN** the tab reloads and returns to about the same scroll position

#### Scenario: Editing the page open for editing
- **WHEN** `pages/auth.html` is open on the same machine (open for editing), scrolled halfway, and the author edits `pages/auth.md` in another editor
- **THEN** the tab does not reload, shows the new content, and keeps its scroll position

#### Scenario: Pages that are not editable still reload
- **WHEN** `journals.html` is open on the same machine and a new journal day is saved
- **THEN** the tab reloads and lists the new day

#### Scenario: Editing an unrelated note
- **WHEN** `pages/auth.html` is open and the author edits a note that neither links to `auth` nor is linked from it
- **THEN** the tab does not reload

#### Scenario: Adding a task elsewhere
- **WHEN** `pages/auth.html` is open showing 3 open tasks on its open-tasks link, and the author adds a `TODO` block to a note that neither links to `auth` nor is linked from it
- **THEN** the tab does not reload and its open-tasks link shows 4

#### Scenario: Pane refresh
- **WHEN** `journals/2026-10-12.html` is open with a `redis` pane and the author edits `pages/redis.md`
- **THEN** the pane shows the new content and the main page does not reload

#### Scenario: Built site has no live reload
- **WHEN** the output of `hermetic build` is served by another web server
- **THEN** its pages make no live-reload connections

### Requirement: Network binding
By default the server SHALL listen on `127.0.0.1`, on port 8000 or, if that port is taken, the next free port up to 8099, and SHALL print the URL it serves. `--port <n>` SHALL use exactly that port and fail with status 1 if it is taken. `--host <address>` SHALL listen on the given address. `--public` SHALL listen on all interfaces and print both the local URL and a URL using this machine's local-network address. Unless `--host` or `--public` is given, the server SHALL reject requests whose `Host` header names anything other than `localhost`, `127.0.0.1`, or `[::1]` (with the port), so that other websites cannot read the notes through DNS rebinding. `--host` and `--public` SHALL NOT be combined. `--open` SHALL, once the server listens, open the first printed URL in the default browser; when no browser can be started it SHALL print a warning and keep serving.

#### Scenario: Default binding
- **WHEN** the user runs `hermetic serve` while port 8000 is in use
- **THEN** it listens on `127.0.0.1:8001` and prints `http://127.0.0.1:8001/`

#### Scenario: Public
- **WHEN** the user runs `hermetic serve --public` on a machine whose local-network address is 192.168.1.23
- **THEN** it prints `http://localhost:8000/` and `http://192.168.1.23:8000/`, and another device on the network can open the latter

#### Scenario: Open in the browser
- **WHEN** the user runs `hermetic serve --open` and the server listens on `127.0.0.1:8000`
- **THEN** the default browser opens `http://127.0.0.1:8000/`

#### Scenario: Rebinding protection
- **WHEN** the server runs with default binding and receives a request with `Host: evil.example:8000`
- **THEN** it responds with status 403 and no content of the notes
