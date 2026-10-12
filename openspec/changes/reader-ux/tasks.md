# Tasks

## 1. Home page as the page `index`

- [ ] 1.1 Make `layout::scan` pick up a root-level `index.md` as the regular page `index`; verify `Home page source` and `Root index and pages index` (collision diagnostic, root file wins) with graph tests
- [ ] 1.2 Allow markdown links to resolve to the root `index.md` in `resolve_link_path`; verify `Link to the home page` and that other root files (e.g. `../README.md`) are still reported as outside the notes
- [ ] 1.3 Add a root `index.md` to `tests/fixtures/notes`, update the end-to-end snapshot with `UPDATE_SNAPSHOTS=1`, and review that `index` shows up in linked references and related pages as expected

## 2. Site-wide pages and HTML hooks

- [ ] 2.1 Render `journals.html` (the existing list); when there is no file-backed page `index`, render `index.html` as the list plus a hint naming `index.md`; verify the `Custom home page`, `Default home page`, `Linking to the home page`, and `Site-wide pages` scenarios, and that an implicit `[[index]]` without `index.md` goes to `pages/index.html`
- [ ] 2.2 Change the header: site title → `index.html`, "Journals" → `journals.html`; add `<meta name="hermetic-root">` and the `<script src=".../_hermetic/app.js">` tag in `<head>`; verify `Header on every page`, `Root path declared` (`../..` for `pages/Project X/Auth Flow.html`, `.` at root), and update existing tests that asserted no `<script>`
- [ ] 2.3 Add `data-view` (`journals`, `pages`, `tasks`, and `home` on the default home page) to generated site-wide articles; verify with a Rust test that every hook named in the design exists in generated HTML
- [ ] 2.4 Update the `No external resources` test to allow exactly one external-free `<script src>` pointing at `_hermetic/app.js` and no inline scripts (`Only hermetic's own script`)

## 3. Frontend toolchain

- [ ] 3.1 Create `static/package.json` (private, `packageManager: bun@1.4.2`, devDependency `typescript` only), `tsconfig.json` (strict, `noUncheckedIndexedAccess`, `noEmit`, DOM libs), and run `bun install` to produce `bun.lock`; verify `bun pm ls` lists only `typescript`
- [ ] 3.2 Create `static/banner.txt` (the generated-file comment from the design) and `static/Makefile` with `build` (passing the banner, with the Bun version filled in, via `--banner`), `check`, `test`, `verify` (rebuild, compare, and confirm the file starts with the banner), and `clean`; verify each target runs on an empty `ts/main.ts` and that `dist/app.js` begins with the banner after minification
- [ ] 3.3 Embed `static/dist/app.js` as `_hermetic/app.js` in `assets.rs`; hook `make -C static check verify` into top-level `make lint` and `make -C static test` into `make test`; add `static/node_modules/` to `.gitignore`; verify `cargo build` works with Bun absent from `PATH`

## 4. Script foundations

- [ ] 4.1 `env.ts`: read the site root from the meta tag and report whether interactive features are enabled (http/https only); `links.ts`: classify links (page, page with fragment, other) relative to the site root; verify with `bun test` cases for nested paths, `_hermetic/` assets, attachments, external and `mailto:` links
- [ ] 4.2 `state.ts`: encode/decode pane lists (order, collapsed flag) to and from `URLSearchParams`, preserving unrelated parameters; verify round trips, unknown params, and malformed input with `bun test`
- [ ] 4.3 `fetchpage.ts`: fetch a page, extract `main > article`, absolutize `href`/`src` against the page URL, locate the fragment target, strip `id`s; unit-test the URL rewriting function with `bun test`
- [ ] 4.4 `theme.ts` + CSS: apply the stored theme before paint, add the system/light/dark toggle to the header, and restructure CSS variables into `:root:not([data-theme=light])` media rules plus explicit `[data-theme]` blocks; verify `Dark system preference` without JS and `Manual choice persists` in a browser

## 5. Sidebar

- [ ] 5.1 Shift+click handling, pane creation with title link, collapse and close controls, newest-on-top ordering, and de-duplication; verify `Shift+click a page link`, `Reopening moves to top`, `Closing the last pane`, `External link` in a browser
- [ ] 5.2 Keep panes across navigation (carry `?p=` on plain page-link clicks) and restore them on load, dropping missing pages silently; verify `Navigate main from a pane`, `Reload restores panes`, `Shared link`, `Missing page in the URL`, `Relative links inside a pane`
- [ ] 5.3 Sidebar CSS: two-column grid, `content-visibility: auto` panes with lazy fetch near the viewport, overlay with hide/show controls below 900px; verify `Many panes` (50 panes stay responsive) and `Phone width`

## 6. Timeline and keyboard

- [ ] 6.1 Timeline: list/timeline toggle on `journals.html` reflected in `?view=timeline`, batches of 5 days via `IntersectionObserver`, day headings linking to day pages, end marker; verify `Switch to timeline`, `Reload keeps the view`, `Scrolling loads older days` (400-day generated site, ≤10 fetches on first load via the network panel), `End of the journal`, and `Open a topic from the timeline`
- [ ] 6.2 Keyboard shortcuts (`g h/j/p/t`, `Escape`, `\`, `?` with a help dialog), ignored in text fields; verify `Go to journals` and `Typing is not intercepted`

## 7. Design polish and verification

- [ ] 7.1 Typography and layout pass on `static/style.css` (measure ≤ ~75ch, ≥16px, line-height ≥1.5, tables/code scroll within themselves, no horizontal page scroll at 320px); verify `Wide screen reading` and the responsive `Phone width` scenario in a browser
- [ ] 7.2 Verify progressive enhancement: `file://` with JS enabled shows no sidebar/timeline/toggle controls and Shift+click behaves natively; `http://` with JS disabled shows none of them either
- [ ] 7.3 Run `make lint`, `make test`, `make test-perf`, and `make -C static verify`, and confirm all pass; record the manual browser checks from groups 4–7 in this file
