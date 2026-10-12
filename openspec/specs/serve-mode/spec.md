# serve-mode Specification

## Purpose

Lets the author read the notes as a live wiki while writing them: `hermetic serve` builds the site in memory, serves it over HTTP with every enhancement available, rebuilds when notes change, and updates open tabs without disturbing what is being read.

## Requirements

### Requirement: Serve command
`hermetic serve [<dir>] [--port <n>] [--host <address>] [--public]` SHALL build the notes directory `<dir>` (default: the current directory) and serve the result over HTTP until interrupted. The served site SHALL have the same files and content as `hermetic build` would produce for the same notes, except that served HTML pages additionally contain the live-reload element. It SHALL NOT write the site or anything else to disk. Diagnostics SHALL be printed as for `build`. When `<dir>` is not a directory, or an option is invalid, it SHALL print an error and exit with status 1 (status 2 for usage errors) without serving.

#### Scenario: Serve the current directory
- **WHEN** the user runs `hermetic serve` inside a notes directory
- **THEN** it prints a URL such as `http://127.0.0.1:8000/`, opening it shows the home page, and no `site/` directory is created

#### Scenario: Same output as build
- **WHEN** a notes directory is served and also built with `hermetic build`
- **THEN** every built file is served at the same path with the same bytes, except HTML pages, which differ only by the injected live-reload element

#### Scenario: Missing notes directory
- **WHEN** the user runs `hermetic serve nope` and `nope` does not exist
- **THEN** an error is printed and the exit status is 1

### Requirement: HTTP behavior
The server SHALL answer `GET` and `HEAD` requests for the site's files with status 200 and a correct `Content-Type` for HTML, CSS, JavaScript, fonts, images, and other common types (`application/octet-stream` otherwise). A path ending in `/` SHALL serve that directory's `index.html` when the site has one. Paths SHALL be percent-decoded before lookup, and any path that is not a file of the current build SHALL get a 404 page linking to the home page. Other methods SHALL get 405. Responses SHALL not be cached by the browser (`Cache-Control: no-store`), so a reload always shows the current build.

#### Scenario: Encoded path
- **WHEN** the browser requests `/pages/Project%20X/Auth%20Flow.html`
- **THEN** the page `Project X/Auth Flow` is returned with `Content-Type: text/html; charset=utf-8`

#### Scenario: Path outside the site
- **WHEN** a client requests `/../../etc/passwd` or `/%2e%2e/notes.md`
- **THEN** the response is 404 and no file outside the build is read

#### Scenario: Directory index
- **WHEN** the browser requests `/`
- **THEN** the home page (`index.html`) is returned

### Requirement: Rebuild on change
The server SHALL detect created, modified, and deleted notes and attachments (files under `journals/`, `pages/`, `assets/`, and the root `index.md`) and rebuild the site. Several changes in quick succession SHALL lead to one rebuild. For a notes directory of about 5,000 pages, a change SHALL be served within 2 seconds of the file being saved. While rebuilding, and if a rebuild fails, the server SHALL keep serving the last successful build and SHALL report the failure on standard error.

#### Scenario: New page appears
- **WHEN** the server is running and the author saves a new journal containing `[[kafka]]`
- **THEN** within 2 seconds `/journals/<that day>.html` and `/pages/kafka.html` are served and the journal list includes the day

#### Scenario: Deleted note disappears
- **WHEN** a page file is deleted while the server runs
- **THEN** after the rebuild its URL returns 404 and pages that linked to it show the updated state

#### Scenario: Burst of saves
- **WHEN** an editor writes the same file several times within 100 milliseconds
- **THEN** the server rebuilds once for that burst

### Requirement: Live reload
Pages served by `hermetic serve` SHALL receive, after every rebuild, the list of site paths whose content changed or which were removed. A tab SHALL reload only when its own page is in that list, keeping its scroll position and open sidebar panes; a sidebar pane whose page is in the list SHALL refresh its content in place, and a pane whose page was removed SHALL close. When the connection to the server is lost, the page SHALL keep working and reconnect when the server is back. Built sites (`hermetic build`) SHALL contain no live-reload behavior.

#### Scenario: Editing the page being read
- **WHEN** `pages/auth.html` is open, scrolled halfway, and the author edits `pages/auth.md`
- **THEN** the tab reloads and returns to about the same scroll position

#### Scenario: Editing an unrelated note
- **WHEN** `pages/auth.html` is open and the author edits a note that neither links to `auth` nor is linked from it
- **THEN** the tab does not reload

#### Scenario: Pane refresh
- **WHEN** `journals/2026-10-12.html` is open with a `redis` pane and the author edits `pages/redis.md`
- **THEN** the pane shows the new content and the main page does not reload

#### Scenario: Built site has no live reload
- **WHEN** the output of `hermetic build` is served by another web server
- **THEN** its pages make no live-reload connections

### Requirement: Network binding
By default the server SHALL listen on `127.0.0.1`, on port 8000 or, if that port is taken, the next free port up to 8099, and SHALL print the URL it serves. `--port <n>` SHALL use exactly that port and fail with status 1 if it is taken. `--host <address>` SHALL listen on the given address. `--public` SHALL listen on all interfaces and print both the local URL and a URL using this machine's local-network address. Unless `--host` or `--public` is given, the server SHALL reject requests whose `Host` header names anything other than `localhost`, `127.0.0.1`, or `[::1]` (with the port), so that other websites cannot read the notes through DNS rebinding. `--host` and `--public` SHALL NOT be combined.

#### Scenario: Default binding
- **WHEN** the user runs `hermetic serve` while port 8000 is in use
- **THEN** it listens on `127.0.0.1:8001` and prints `http://127.0.0.1:8001/`

#### Scenario: Public
- **WHEN** the user runs `hermetic serve --public` on a machine whose local-network address is 192.168.1.23
- **THEN** it prints `http://localhost:8000/` and `http://192.168.1.23:8000/`, and another device on the network can open the latter

#### Scenario: Rebinding protection
- **WHEN** the server runs with default binding and receives a request with `Host: evil.example:8000`
- **THEN** it responds with status 403 and no content of the notes
