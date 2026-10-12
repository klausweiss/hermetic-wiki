# Tasks

## 1. Home page as the page `index`

- [x] 1.1 Make `layout::scan` pick up a root-level `index.md` as the regular page `index`; verify `Home page source` and `Root index and pages index` (collision diagnostic, root file wins) with graph tests
- [x] 1.2 Allow markdown links to resolve to the root `index.md` in `resolve_link_path`; verify `Link to the home page` and that other root files (e.g. `../README.md`) are still reported as outside the notes
- [x] 1.3 Add a root `index.md` to `tests/fixtures/notes`, update the end-to-end snapshot with `UPDATE_SNAPSHOTS=1`, and review that `index` shows up in linked references and related pages as expected

## 2. Site-wide pages and HTML hooks

- [x] 2.1 Render `journals.html` (the existing list); when there is no file-backed page `index`, render `index.html` as the list plus a hint naming `index.md`; verify the `Custom home page`, `Default home page`, `Linking to the home page`, and `Site-wide pages` scenarios, and that an implicit `[[index]]` without `index.md` goes to `pages/index.html`
- [x] 2.2 Change the header: site title → `index.html`, "Journals" → `journals.html`; add `<meta name="hermetic-root">` and the `<script src=".../_hermetic/app.js">` tag in `<head>`; verify `Header on every page`, `Root path declared` (`../..` for `pages/Project X/Auth Flow.html`, `.` at root), and update existing tests that asserted no `<script>`
- [x] 2.3 Add `data-view` (`journals`, `pages`, `tasks`, and `home` on the default home page) to generated site-wide articles; verify with a Rust test that every hook named in the design exists in generated HTML
- [x] 2.4 Update the `No external resources` test to allow exactly one external-free `<script src>` pointing at `_hermetic/app.js` and no inline scripts (`Only hermetic's own script`)

## 3. Frontend toolchain

- [x] 3.1 Create `static/package.json` (private, `packageManager: bun@1.4.2`, devDependency `typescript` only), `tsconfig.json` (strict, `noUncheckedIndexedAccess`, `noEmit`, DOM libs), and run `bun install` to produce `bun.lock`; verify `bun pm ls` lists only `typescript`
- [x] 3.2 Create `static/banner.txt` (the generated-file comment from the design) and `static/Makefile` with `build` (passing the banner, with the Bun version filled in, via `--banner`), `check`, `test`, `verify` (rebuild, compare, and confirm the file starts with the banner), and `clean`; verify each target runs on an empty `ts/main.ts` and that `dist/app.js` begins with the banner after minification
- [x] 3.3 Embed `static/dist/app.js` as `_hermetic/app.js` in `assets.rs`; hook `make -C static check verify` into top-level `make lint` and `make -C static test` into `make test`; add `static/node_modules/` to `.gitignore`; verify `cargo build` works with Bun absent from `PATH`

## 4. Script foundations

- [x] 4.1 `env.ts`: read the site root from the meta tag and report whether interactive features are enabled (http/https only); `links.ts`: classify links (page, page with fragment, other) relative to the site root; verify with `bun test` cases for nested paths, `_hermetic/` assets, attachments, external and `mailto:` links
- [x] 4.2 `state.ts`: encode/decode pane lists (order, collapsed flag) to and from `URLSearchParams`, preserving unrelated parameters; verify round trips, unknown params, and malformed input with `bun test`
- [x] 4.3 `fetchpage.ts`: fetch a page, extract `main > article`, absolutize `href`/`src` against the page URL, locate the fragment target, strip `id`s; unit-test the URL rewriting function with `bun test`
- [x] 4.4 `theme.ts` + CSS: apply the stored theme before paint, add the system/light/dark toggle to the header, and restructure CSS variables into `:root:not([data-theme=light])` media rules plus explicit `[data-theme]` blocks; verify `Dark system preference` without JS and `Manual choice persists` in a browser

## 5. Sidebar

- [x] 5.1 Shift+click handling, pane creation with title link, collapse and close controls, newest-on-top ordering, and de-duplication; verify `Shift+click a page link`, `Reopening moves to top`, `Closing the last pane`, `External link` in a browser
- [x] 5.2 Keep panes across navigation (carry `?p=` on plain page-link clicks) and restore them on load, dropping missing pages silently; verify `Navigate main from a pane`, `Reload restores panes`, `Shared link`, `Missing page in the URL`, `Relative links inside a pane`
- [x] 5.3 Sidebar CSS: two-column grid, `content-visibility: auto` panes with lazy fetch near the viewport, overlay with hide/show controls below 900px; verify `Many panes` (50 panes stay responsive) and `Phone width`

## 6. Timeline and keyboard

- [x] 6.1 Timeline: list/timeline toggle on `journals.html` reflected in `?view=timeline`, batches of 5 days via `IntersectionObserver`, day headings linking to day pages, end marker; verify `Switch to timeline`, `Reload keeps the view`, `Scrolling loads older days` (400-day generated site, ≤10 fetches on first load via the network panel), `End of the journal`, and `Open a topic from the timeline`
- [x] 6.2 Keyboard shortcuts (`g h/j/p/t`, `Escape`, `\`, `?` with a help dialog), ignored in text fields; verify `Go to journals` and `Typing is not intercepted`

## 7. Design polish and verification

- [x] 7.1 Typography and layout pass on `static/style.css` (measure ≤ ~75ch, ≥16px, line-height ≥1.5, tables/code scroll within themselves, no horizontal page scroll at 320px); verify `Wide screen reading` and the responsive `Phone width` scenario in a browser
- [x] 7.2 Verify progressive enhancement: `file://` with JS enabled shows no sidebar/timeline/toggle controls and Shift+click behaves natively; `http://` with JS disabled shows none of them either
- [x] 7.3 Run `make lint`, `make test`, `make test-perf`, and `make -C static verify`, and confirm all pass; record the manual browser checks from groups 4–7 in this file

## 8. Always-visible sidebar (feedback)

- [x] 8.1 Show the sidebar on every page when the script runs (wide screens), with a placeholder while empty; over `file://` show it with an explanation that opening pages there needs `http://`; keep closing the last pane from hiding it; verify `Empty sidebar`, `Opened from disk`, `Closing the last pane`, and `Hide and show` in Chromium and Firefox
- [x] 8.2 Start hidden below 900px with a visible show control, showing as an overlay when opened or when a pane is opened; verify `Phone width`
- [x] 8.3 Re-run the browser checks in Chromium and Firefox, plus `make lint` and `make test`, and update the verification notes

## 9. Sidebar size and toggle (feedback)

- [x] 9.1 Keep the empty sidebar narrow (about 15rem) and widen it when a pane opens; verify `Narrow while empty` in Chromium and Firefox
- [x] 9.2 Replace the separate Hide and Sidebar buttons with one toggle fixed at the same position in both states, keeping the header clear of it; verify `Toggle stays in place`, `Hide and show`, and `Phone width`

## 10. Web components in a Bun workspace (feedback)

- [x] 10.1 Restructure `static/` into a Bun workspace (`packages/core`, `theme-toggle`, `sidebar`, `timeline`, `shortcuts`, `app`), each with `package.json`, `tsconfig.json` extending `tsconfig.base.json`, a `Makefile`, and tests where there is logic; keep TypeScript as the only dependency; verify `bun install --frozen-lockfile` and `bun pm ls` at the root
- [x] 10.2 Move the pure logic (env, links, state, fetchPage) into `@hermetic/core` with its tests, plus `whenParsed()`; verify `make -C static/packages/core check test`
- [x] 10.3 Turn theme toggle, sidebar, timeline, and shortcuts into light-DOM custom elements with `define()` and a standalone `register.ts`; verify each package's `make build` produces a working standalone bundle and `make check` passes
- [x] 10.4 Build `dist/app.js` (with the banner) from the `app` package; root `static/Makefile` runs `build`, `check`, `test`, and `verify` across packages; verify `make lint` and `make test` at the repository root
- [x] 10.5 Emit `<hermetic-theme-toggle>`, `<hermetic-sidebar>`, `<hermetic-shortcuts>`, and `<hermetic-timeline>` from Rust and update the CSS selectors; verify with Rust tests that the tags are present and that pages without JavaScript look unchanged
- [x] 10.6 Re-run all browser checks in Chromium and Firefox and update the verification notes

## 11. No layout animation on page load (feedback)

- [x] 11.1 Apply the sidebar's open/hidden/empty layout classes on `<html>` from the script in `<head>` before first paint (from the URL panes, the protocol, and the viewport width), and enable width transitions only after the page has settled; verify `No movement when a page loads` in Chromium and Firefox (no transitions and no shift of the main area during load), plus the existing sidebar checks

## Verification notes

Browser checks for groups 4–7 were run on 2026-10-12 with headless Chromium (Playwright, used as a throwaway tool outside the repository) against a site built from `tests/fixtures/notes` plus 399 generated journal days, served over `http://` by a minimal static server. All 42 checks passed at first; after the feedback round (section 8) the set grew to 59 checks (sections 8–11), run in both headless Chromium and headless Firefox against the web-component version, all passing:

- Sidebar: always shown on wide screens, as a narrow strip with a placeholder while empty that widens when a pane opens; one hide/show toggle that stays in the same place and never covers the header; Shift+click opens a pane; newest on top; reopening moves to top without duplicates; panes in the URL; links inside panes rewritten; plain click in a pane navigates main and keeps panes; reload and a shared URL restore panes; a missing page in the URL is dropped silently; a block link opens its pane at the block; external links open no pane; Escape closes the top pane; closing the last pane shows the placeholder again; hiding (`\\` or the control) frees the width and showing restores the same panes; sticky pane headers are not covered by content scrolled under them (this check was confirmed to fail without the fix).
- 50 panes: only panes near the viewport were fetched, and scrolling stayed responsive.
- 400px viewport: the sidebar starts hidden behind a show control and opens as a full-width overlay with working hide/show controls. 320px: no horizontal page scroll; a wide table scrolls inside its box.
- Timeline: toggle present; newest day first; first load fetched at most 10 day pages; view kept in the URL and across reload; scrolling loaded every listed day and then the end marker; no requests after the end; Shift+click inside the timeline opens a pane.
- Keyboard: `g j` opens the journal list; typing `g j` in an input is not intercepted; `?` shows the shortcut list.
- Theme: dark system preference applies without JavaScript; a manual light choice persists across pages.
- Typography: Inter at 17px, line height 1.6, about 64 characters per line; Inter is loaded.
- Page load: on load and reload, with and without panes in the URL, no CSS transition runs and the main area does not move between `DOMContentLoaded` and the settled layout; opening the first pane still animates. Against the previously committed build the same measurement showed the main area sliding from 380px to 253px (empty sidebar) or 91px (with a pane), confirming the check catches the problem.
- Web components: all four elements are defined by `app.js`; each component's standalone bundle (`packages/*/dist/hermetic-*.js`) loaded alone in place of `app.js` defines only its own element, renders, and logs no errors.
- Progressive enhancement: `http://` with JavaScript disabled shows no script-only controls; `file://` with JavaScript enabled opens no pane on Shift+click, shows no timeline control, and the sidebar explains that `http://` is needed; no script errors on any page.

These checks are automated and headless. The user tested in Firefox: Shift+click opened a new window when the site was opened from disk (the designed `file://` fallback, now explained in the sidebar) and works over `http://`; the transparent pane header they reported is fixed.
