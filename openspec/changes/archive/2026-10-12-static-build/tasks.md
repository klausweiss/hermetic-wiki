# Tasks

## 1. Groundwork

- [x] 1.1 Add `pulldown-cmark` (0.13, `default-features = false`, `features = ["html"]`) and verify `cargo tree --edges normal --depth 1` lists only it and `unicode-normalization`
- [x] 1.2 Add `dest: Option<Range<usize>>` to `parse::RawRef` and `graph::Ref` for markdown links, filled by the inline scanner; verify with a unit test that `&content[dest]` equals the destination for `[x](a.md)`, `[x](<a b.md> "t")`, and `[x](a.md#s)`
- [x] 1.3 Add `AttachmentConflict` and `SkippedSymlink` to `DiagnosticKind`; verify `cargo build` and existing tests pass
- [x] 1.4 Vendor fonts into `static/fonts/` (Inter v4.1 `InterVariable.woff2` + italic; JetBrains Mono v2.304 Regular/Italic/Bold woff2) with their OFL license texts, confirm the licenses declare no Reserved Font Name issue for unmodified files, and verify the files are byte-identical to the release archives (record sha256 in `static/fonts/SOURCES.md`)

## 2. Paths and HTML primitives

- [x] 2.1 Create `src/site/mod.rs` with `paths::output_path(page)` for file-backed, implicit regular, and implicit journal pages; verify with unit tests including `pages/C#%3A a.md` → `pages/C#%3A a.html` and names containing dots
- [x] 2.2 Implement `paths::href(from, to, fragment)` with per-segment percent-encoding of non-unreserved bytes; verify `pages/a.html` → `pages/C#%3A a.html` gives `C%23%253A%20a.html`, cross-directory `../` cases, and same-file fragment links
- [x] 2.3 Implement HTML escaping helpers (`escape_text`, `escape_attr`) and the document `layout` shell (doctype, charset, viewport, stylesheet link, header nav to `index.html`/`pages.html`/`tasks.html`, site title, single `<main>`, no `<script>`); verify with unit tests on relative nav hrefs from root, `pages/`, and nested namespace depths
- [x] 2.4 Implement weekday calculation and previous/next file-backed journal lookup; verify 2026-10-12 is Monday, 2024-02-29 is Thursday, and the prev/next scenario

## 3. Block rendering

- [x] 3.1 Implement placeholder substitution and markdown rendering of a block's content (tables, strikethrough, task lists, footnotes) with page links, tags, and resolved markdown links rewritten; verify the `Rewritten markdown link` scenario and that a page name containing `*_` renders literally
- [x] 3.2 Render block refs one level deep with nested refs as plain links and broken refs/links as marked original text; verify the `Block reference shows target content`, `Self-referencing block refs do not loop`, and `Broken references` scenarios
- [x] 3.3 Render a block tree: nested lists with `id-<id>`/`b<n>` anchors, task state badges, properties except `id`; verify the `Nested blocks and properties` and `Anchors` scenarios

## 4. Page sections

- [x] 4.1 Render the page body: title (journal date + weekday), namespace ancestor links and children list, implicit-page notice, blocks; verify the `Namespaced page` scenario and an implicit page's notice
- [x] 4.2 Render linked references grouped by source page with breadcrumbs and subtrees, paginated into `refs/<page path>/<n>.html` at 100 per file with prev/next links; verify the `References with breadcrumb` and `Pagination` (250 refs) scenarios
- [x] 4.3 Render each page's open-tasks section linking to task anchors; verify the `Open tasks of a page` scenario
- [x] 4.4 Render the related-pages section with weights and `<details>` holding shared blocks; verify the `Related pages` scenario
- [x] 4.5 Render journal prev/next navigation on journal pages; verify the prev/next scenario in generated HTML

## 5. Site-wide pages and assets

- [x] 5.1 Generate `index.html` (file-backed journal days, newest first, by month) and verify the `Journal index` scenario
- [x] 5.2 Generate `pages.html` (all non-journal pages, case-insensitive order, implicit pages marked); verify the `Implicit pages listed` scenario
- [x] 5.3 Generate `tasks.html` grouped by page with `tasks/<n>.html` continuation at 100 per file; verify with 150 open tasks
- [x] 5.4 Write the baseline `static/style.css` (Inter/JetBrains Mono `@font-face` via relative URLs, readable measure, nested bullets, task badges, broken-ref styling, `<details>` styling, light/dark via `prefers-color-scheme`) and emit it with fonts and licenses into `_hermetic/`; verify the `Fonts and licenses shipped` scenario
- [x] 5.5 Copy attachments from `journals/`, `pages/`, `assets/` (regular non-md, non-dot files), skipping symlinks and generated-path conflicts with diagnostics; verify the `Image next to a page` scenario plus symlink and conflict cases

## 6. Build pipeline and CLI

- [x] 6.1 Implement `site::build`: path resolution, safety checks, render into `<out>.hermetic-tmp-<pid>`, marker file, swap into place, cleanup on error; verify `Rebuilding replaces the previous output`, `Foreign directory is not touched`, `Output would contain the notes`, and that a failed build leaves the old output intact
- [x] 6.2 Implement `main.rs` argument handling (`build <dir> [<out>]` defaulting to `./site`, `--help`, `--version`, usage errors), diagnostics on stderr, summary line, and exit codes 0/1/2; verify with CLI tests using the built binary for `Default output directory`, `Explicit output directory`, `Missing notes directory`, `Unknown command`, and `Broken link reported, build succeeds`
- [x] 6.3 Verify `Output inside the notes root but outside note directories`: build `.` inside a notes dir twice and confirm the second graph has no pages from `site/`

## 7. Whole-site verification

- [x] 7.1 Build `tests/fixtures/notes` in an integration test and assert the expected file list, then verify `Pages, journals, and implicit pages` and `Encoded page names`
- [x] 7.2 Verify determinism: two builds of the fixture are byte-identical (`Rebuild is identical`)
- [x] 7.3 Verify `No external resources`: scan every generated HTML/CSS file for `src`/`href`/`url(` values with a scheme or `//` that do not come from note content, and confirm no `<script>` tags
- [x] 7.4 Verify `Unrelated growth`: add 1,000 unrelated journal days to a fixture and confirm `pages/auth.html` is byte-identical
- [x] 7.5 Verify `Build without network access` by confirming `cargo tree` contains no networking crates and by running a build under `unshare -n` (or equivalent) if available
- [x] 7.6 Open the fixture site from disk in a browser with JavaScript disabled and confirm pages, navigation, styles, and fonts work (`Opened from disk`); record the check in the change notes
  - Checked 2026-10-12 by the user: the fixture site (`make site`) opened from disk with JavaScript disabled works as expected.
- [x] 7.7 Add `make site` (builds `tests/fixtures/notes` into `./site`) and confirm `make lint`, `make test`, and `make test-perf` pass
