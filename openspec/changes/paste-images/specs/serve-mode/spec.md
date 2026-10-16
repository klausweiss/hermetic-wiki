# Spec Delta

## MODIFIED Requirements

### Requirement: HTTP behavior
The server SHALL answer `GET` and `HEAD` requests for the site's files with status 200 and a correct `Content-Type` for HTML, CSS, JavaScript, fonts, images, and other common types (`application/octet-stream` otherwise). A path ending in `/` SHALL serve that directory's `index.html` when the site has one. Paths SHALL be percent-decoded before lookup, and any path that is not a file of the current build SHALL get a 404 page linking to the home page. Other methods SHALL get 405, except `PUT` and `DELETE` to the page source of the edit API of `browser-editing`, and `POST` to its image upload. Responses SHALL not be cached by the browser (`Cache-Control: no-store`), so a reload always shows the current build.

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
- **WHEN** a client sends `POST /pages/auth.html`, `PUT /pages/auth.html`, `DELETE /pages/auth.html`, or `PUT /_hermetic/asset?path=index.html`
- **THEN** the response is 405 and nothing is written
