# Tasks

## 1. Keys

- [x] 1.1 Graph search box: down / up select the next / previous match (wrapping, centred, neighbours highlighted), Enter opens the selected match (with none selected: the only match, else select the first); Shift+Enter and Ctrl+Enter dropped; verify `make -C static check`
- [x] 1.2 Update the Graph section of `src/help/pages.txt`; verify `cargo test --test help`

## 2. Verification

- [x] 2.1 Rebuild `static/dist/app.js`; in Chromium and Firefox verify `Going through matches`, `Wrapping around`, `Open a match`, `Enter with one match`, `Enter with several matches`, `Escape`, and that the earlier graph-search checks still pass; run `make lint` and `make test`; record results here

  Results (2026-10-13): `make lint` is clean; `make test` passes 248 Rust tests and 82 TypeScript tests. In headless Chromium and Firefox (Playwright, outside the repository, on the 300-page test wiki), the 15 graph-search checks pass in each browser, adapted to the new keys: `Going through matches` (down twice, centred to the pixel, all labels showing), up arrow back, `Wrapping around` (up from the first selects the last, down from the last the first), `Enter with several matches` (selects and centres the first, no page opens), `Open a match` (Enter on the selected match), `Enter with one match`, `Escape` (previous view restored), plus the unchanged ones. The 22 earlier graph checks and 2 touch checks still pass.

