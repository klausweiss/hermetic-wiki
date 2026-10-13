# Tasks

## 1. Page properties

- [x] 1.1 `Graph::page_props(page)`: props of the first root block when its content is empty, else empty; tests in `tests/reference_graph.rs` for the three `block-model` "Page properties" scenarios (top-of-file lines, properties-only first bullet, first block with content)
- [x] 1.2 Document page properties in the Properties section of `src/help/syntax.txt`; `tests/help.rs` still passes

## 2. Aliases in the graph

- [x] 2.1 `DiagnosticKind::InvalidAlias` in `src/diag.rs`; alias value splitting (comma-separated, bare or `[[name]]`, normalized and validated) with unit tests
- [x] 2.2 `Graph::aliases` table and `Page::aliases`, built in `load` after `build_id_table` and before resolution. Pages are visited in scan order. Diagnostics for: date, existing file page, namespace prefix of a file page, earlier claim (naming both files), invalid name. Self-alias is ignored silently.
- [x] 2.3 Page links and tags resolve through `aliases` first (an alias never names a page with a file), then `by_key`, then create an implicit page; markdown links and `find_page` are unchanged
- [x] 2.4 Drop refs of the `alias` property on the page-properties block in `add_blocks`
- [x] 2.5 Tests in `tests/reference_graph.rs` for every "Page aliases" scenario and the "Namespaced alias" scenario: link through an alias, case-insensitivity, alias property not a reference, alias of an existing page, claimed twice, date alias, alias not in the first block, markdown link to an alias name. Also check that an aliased reference shows in linked references, tasks, and co-mentions of the page.

## 3. Site and search

- [x] 3.1 `src/site/render.rs`: show the `alias` property of the page-properties block as comma-separated plain names; test in `tests/site.rs` (`alias: authentication, login`, no links)
- [x] 3.2 Test in `tests/site.rs` that `[[login]]` through an alias links to `pages/auth.html` and that no `pages/login.html` is written
- [x] 3.3 `src/search/index.rs`: one page document per alias after the page's own document; test in `tests/search_index.rs` that the alias term has a name-field posting on a document of the page
- [x] 3.4 `static/packages/search/src/results.ts`: `resolve` drops page results whose path was already returned; add a case to `query.test.ts` (or a results test) where `kafka` matches both the name and the fixture alias `kafka streams` and the page appears once, plus a search for the alias `event log`; rebuild `static/dist/app.js`
- [x] 3.5 Document aliases in `src/help/syntax.txt`: they go at the top of the page, names with commas cannot be aliases, and markdown links are not aliased

## 4. Verification

- [ ] 4.1 Run `make lint` and `make test`; build a scratch notes folder with an aliased page and check in a browser that alias links, the alias property display, search, and `[[` completion work; record results here

  Results (2026-10-14): `make lint` is clean; `make test` passes (Rust and bun). On a scratch copy of `wiki/` with `- alias:: ai` on `AI-generated content` and a journal linking `[[ai]]`, the build reports 0 warnings. The link goes to `pages/AI-generated%20content.html`, no `pages/ai.html` is written, the page shows `alias: ai` and lists the journal block, and the search module run on the built index ranks the page first and once for `ai`. Not yet checked in a browser (including `[[` completion).
