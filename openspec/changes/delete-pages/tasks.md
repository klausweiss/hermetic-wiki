# Tasks

## 1. Edit API (Rust)

- [ ] 1.1 `DELETE /_hermetic/source`: routing (405 elsewhere), the same authenticity checks as `PUT`, version check (409), 404 for pages without a file; verify `Delete with a stale version`, `Cross-site delete`, and `Other methods`
- [ ] 1.2 Move to `.trash/<timestamp>/<path>` after the safe-path checks (symbolic links resolved, never outside the notes folder), remove emptied directories under `pages/`, rebuild before responding; verify `Empty namespace directory`, a symbolic link out of the notes (Unix), and that the build and watcher ignore `.trash/`
- [ ] 1.3 Help text (`commands`): deleting pages and the trash; verify the help tests

## 2. Editor (TypeScript)

- [ ] 2.1 "Delete page" in the page header for pages with a file; a confirmation dialog naming the file and the count of referencing blocks (from the page's linked references); on confirm `DELETE`, then open the home page; verify `Delete a page`, `Still referenced`, and `Cancel` in the browser

## 3. Integration

- [ ] 3.1 Rebuild `static/dist/app.js`; run `make lint`, `make test`, and the browser suites in Chromium and Firefox; record results here
