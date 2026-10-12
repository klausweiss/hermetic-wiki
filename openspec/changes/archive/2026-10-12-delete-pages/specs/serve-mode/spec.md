# Spec Delta

## MODIFIED Requirements

### Requirement: Serve command
`hermetic serve [<dir>] [--port <n>] [--host <address>] [--public]` SHALL build the notes directory `<dir>` (default: the current directory) and serve the result over HTTP until interrupted. The served site SHALL have the same files and content as `hermetic build` would produce for the same notes, except that served HTML pages additionally contain the live-reload element and, for requests from the same machine, the editor element. It SHALL NOT write the site to disk, and SHALL write nothing else except note files saved through browser editing and files it moves to `.trash/` when a page is deleted (see `browser-editing`). Diagnostics SHALL be printed as for `build`. When `<dir>` is not a directory, or an option is invalid, it SHALL print an error and exit with status 1 (status 2 for usage errors) without serving.

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
The server SHALL answer `GET` and `HEAD` requests for the site's files with status 200 and a correct `Content-Type` for HTML, CSS, JavaScript, fonts, images, and other common types (`application/octet-stream` otherwise). A path ending in `/` SHALL serve that directory's `index.html` when the site has one. Paths SHALL be percent-decoded before lookup, and any path that is not a file of the current build SHALL get a 404 page linking to the home page. Other methods SHALL get 405, except `PUT` and `DELETE` to the edit API of `browser-editing`. Responses SHALL not be cached by the browser (`Cache-Control: no-store`), so a reload always shows the current build.

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
- **WHEN** a client sends `POST /pages/auth.html`, `PUT /pages/auth.html`, or `DELETE /pages/auth.html`
- **THEN** the response is 405 and nothing is written
