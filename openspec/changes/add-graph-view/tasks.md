# Tasks

## 1. Graph data

- [ ] 1.1 Extract the graph view's nodes (non-journal pages; implicit `index` skipped as in `site::render`), weighted undirected edges (links from a block's page plus co-mentions among its direct references, deduplicated per block, journal pages excluded), and per-node mention counts (distinct referencing pages, excluding the page itself); verify with Rust tests for the `Edges` scenarios (`Link between pages`, `Links in both directions` = weight 2, `Co-mention in a journal`, `Journal reference alone`), `Nodes` (implicit page included, journals excluded, unconnected page kept), and `Node size` (mention counts)

## 2. Layout

- [ ] 2.1 Pure layout module: FNV-1a name seeds on a disc, spring/repulsion/gravity forces with a Barnes–Hut quadtree, fixed iterations with cooling, and a final overlap pass using radii; verify unit tests: same input gives identical positions, the two-cluster case (`Connected pages are near`), no two circles overlap after the final pass, and an empty or one-node input works
- [ ] 2.2 Ignored release-mode perf test with a synthetic 5,000-node / 20,000-edge graph asserting ≤ 300 ms (`Large wiki`); verify with `make test-perf` and record the timing in this file

## 3. graph.html

- [ ] 3.1 Render `graph.html`: `<article class="graph-view">` with a heading, a `<hermetic-graph>` wrapping the SVG (viewBox from the bounds plus a margin, edge paths per weight bucket, one `<a>` per node with relative href, `<title>`, circle, label, `data-i`/`data-n`, label tier class, `implicit` class), or the empty-state paragraph; add it to `site::render`'s site-wide files; verify Rust tests for `Graph page exists`, `Only journals`, `Click a node without JavaScript` (relative hrefs from the root, including encoded names), the `<title>` tooltip, and `Deterministic` (two builds byte-identical)
- [ ] 3.2 Header link "Graph" in `html::document`; update `Output layout`/`Site navigation` tests in `src/site/html.rs` and `tests/site.rs` (`Site-wide pages`, `Header on every page`); verify the `Graph ignores journal content` scenario with a test that adds 1,000 unreferencing journal days and compares `graph.html` bytes
- [ ] 3.3 Graph styles in `static/style.css`: full-width main via `main:has(> article.graph-view)`, capped height, theme colours only, edge widths per bucket, implicit nodes dashed, only `t0` labels shown without the script; verify by building `tests/fixtures/notes` (`make site`) and opening `graph.html` from `file://` with JavaScript disabled in light and dark themes (`Graph themes`, `Graph without JavaScript`)
- [ ] 3.4 Mention the graph in `src/help/pages.txt` (site-wide pages, and `g g` under Reading); verify `cargo test --test help` and `hermetic help pages` output

## 4. Interactive graph element

- [ ] 4.1 New `static/packages/graph` package (`index.ts`, `register.ts`, Makefile, tsconfig) with `define()`, registered in `app/src/main.ts` and in `PACKAGES` in `static/Makefile`; verify `make -C static check build`
- [ ] 4.2 Pan/zoom: world transform, wheel zoom around the pointer, drag pan and two-pointer pinch with Pointer Events, zoom limits, fit, script-added +/−/fit controls; pure transform maths (zoom around a point, clamping, fit to bounds) in its own module with `bun test` unit tests; verify in a browser `Wheel zoom`, `Drag to pan` (page does not scroll), `Fit`
- [ ] 4.3 Click suppression after a drag and `data-zoom` label tiers (plus hiding small labels while moving); verify `Drag starting on a node` and `Zooming in reveals labels` in a browser
- [ ] 4.4 Neighbour highlight on hover and focus from `data-n` with an overlay edge path; verify `Hover a node`, and tabbing to a node plus Enter opens its page (`Keyboard`)

## 5. Links, panes, and shortcuts

- [ ] 5.1 `anchorHref` helper in `core/src/links.ts` resolving HTML and SVG anchors; use it in the sidebar's click handling for Shift+click and for carrying the sidebar state; verify `bun test` for the helper and, in a browser over `http://`, `Shift+click a graph node` (zoom and pan unchanged) and `Plain click on a graph node keeps panes`; over `file://` Shift+click keeps the default behavior
- [ ] 5.2 `g g` in `GO_TARGETS` and the `?` help list; verify in a browser `Go to the graph` on a journal page and on a nested page, and that `g` then `j` still works

## 6. Integration

- [ ] 6.1 Rebuild `static/dist/app.js`; run `make lint` and `make test`; build the 5,000-journal bench corpus and `tests/fixtures/notes`, check `graph.html` size and smoothness of panning and zooming in Chromium and Firefox, and that `hermetic serve` still rebuilds within 2 seconds on that corpus; record results here
