# Tasks

## 1. Page properties

- [ ] 1.1 `Graph::page_props(page)`: props of the first root block when its content is empty, else empty; tests in `tests/reference_graph.rs` for the three `block-model` "Page properties" scenarios (top-of-file lines, properties-only first bullet, first block with content)
- [ ] 1.2 Document page properties in the Properties section of `src/help/syntax.txt`; `tests/help.rs` still passes

## 2. Aliases in the graph

- [ ] 2.1 `DiagnosticKind::InvalidAlias` in `src/diag.rs`; alias value splitting (comma-separated, bare or `[[name]]`, normalized and validated) with unit tests
- [ ] 2.2 `Graph::aliases` table and `Page::aliases`, built in `load` after `build_id_table` and before resolution. Pages are visited in scan order. Diagnostics for: date, existing file page, namespace prefix of a file page, earlier claim (naming both files), invalid name. Self-alias is ignored silently.
- [ ] 2.3 Page links and tags resolve through `aliases` after `by_key` and before creating an implicit page; markdown links and `find_page` are unchanged
- [ ] 2.4 Drop refs of the `alias` property on the page-properties block in `add_blocks`
- [ ] 2.5 Tests in `tests/reference_graph.rs` for every "Page aliases" scenario and the "Namespaced alias" scenario: link through an alias, case-insensitivity, alias property not a reference, alias of an existing page, claimed twice, date alias, alias not in the first block, markdown link to an alias name. Also check that an aliased reference shows in linked references, tasks, and co-mentions of the page.

## 3. Site and search

- [ ] 3.1 `src/site/render.rs`: show the `alias` property of the page-properties block as comma-separated plain names; test in `tests/site.rs` (`alias: authentication, login`, no links)
- [ ] 3.2 Test in `tests/site.rs` that `[[login]]` through an alias links to `pages/auth.html` and that no `pages/login.html` is written
- [ ] 3.3 `src/search/index.rs`: one page document per alias after the page's own document; test in `tests/search_index.rs` that the alias term has a name-field posting on a document of the page
- [ ] 3.4 `static/packages/search/src/results.ts`: `resolve` drops page results whose path was already returned; add a case to `query.test.ts` (or a results test) where `auth` matches both the name and the alias `authentication` and the page appears once and first; rebuild `static/dist/app.js`
- [ ] 3.5 Document aliases in `src/help/syntax.txt`: they go at the top of the page, names with commas cannot be aliases, and markdown links are not aliased

## 4. Verification

- [ ] 4.1 Run `make lint` and `make test`; build a scratch notes folder with an aliased page and check in a browser that alias links, the alias property display, search, and `[[` completion work; record results here
