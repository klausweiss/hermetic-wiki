# Tasks

## 1. Graph data

- [x] 1.1 Extract the graph view's nodes (non-journal pages; implicit `index` skipped as in `site::render`), weighted undirected edges (links from a block's page plus co-mentions among its direct references, deduplicated per block, journal pages excluded), and per-node mention counts (distinct referencing pages, excluding the page itself); verify with Rust tests for the `Edges` scenarios (`Link between pages`, `Links in both directions` = weight 2, `Co-mention in a journal`, `Journal reference alone`), `Nodes` (implicit page included, journals excluded, unconnected page kept), and `Node size` (mention counts)

## 2. Layout

- [x] 2.1 Pure layout module: FNV-1a name seeds on a disc, spring/repulsion/gravity forces with a Barnes–Hut quadtree, fixed iterations with cooling, and a final overlap pass using radii; verify unit tests: same input gives identical positions, the two-cluster case (`Connected pages are near`), no two circles overlap after the final pass, and an empty or one-node input works
- [x] 2.2 Ignored release-mode perf test with a synthetic 5,000-node / 20,000-edge graph asserting ≤ 300 ms (`Large wiki`); verify with `make test-perf` and record the timing in this file

  Results (2026-10-13), `lays_out_five_thousand_nodes_quickly`, wiki-like graph (communities of 50 grown by preferential attachment, 5,000 nodes, 21,608 edges), release build, CPUs pinned with `taskset`: 8 CPUs 207 ms, 4 CPUs 263–271 ms, 1 CPU 495 ms. The first version (quadratic springs, 300 iterations, single-threaded) took 2.2 s; the budget was met by linear attraction, 150 iterations (no visible difference from 300 in the `layout_preview` renders at 500 and 5,000 nodes), contiguous quadtree bodies, and computing repulsion on up to 8 threads (output identical whatever the thread count). On a single core the target is missed, but the layout still fits the 2-second serve rebuild budget it exists for.

## 3. graph.html

- [x] 3.1 Render `graph.html`: `<article class="graph-view">` with a heading, a `<hermetic-graph>` wrapping the SVG (viewBox from the bounds plus a margin, edge paths per weight bucket, one `<a>` per node with relative href, `<title>`, circle, label, `data-i`/`data-n`, label tier class, `implicit` class), or the empty-state paragraph; add it to `site::render`'s site-wide files; verify Rust tests for `Graph page exists`, `Only journals`, `Click a node without JavaScript` (relative hrefs from the root, including encoded names), the `<title>` tooltip, and `Deterministic` (two builds byte-identical)
- [x] 3.2 Header link "Graph" in `html::document`; update `Output layout`/`Site navigation` tests in `src/site/html.rs` and `tests/site.rs` (`Site-wide pages`, `Header on every page`); verify the `Graph ignores journal content` scenario with a test that adds 1,000 unreferencing journal days and compares `graph.html` bytes
- [x] 3.3 Graph styles in `static/style.css`: full-width main via `main:has(> article.graph-view)`, capped height, theme colours only, edge widths per bucket, implicit nodes dashed, only `t0` labels shown without the script; verify by building `tests/fixtures/notes` (`make site`) and opening `graph.html` from `file://` with JavaScript disabled in light and dark themes (`Graph themes`, `Graph without JavaScript`)
- [x] 3.4 Mention the graph in `src/help/pages.txt` (site-wide pages, and `g g` under Reading); verify `cargo test --test help` and `hermetic help pages` output

## 4. Interactive graph element

- [x] 4.1 New `static/packages/graph` package (`index.ts`, `register.ts`, Makefile, tsconfig) with `define()`, registered in `app/src/main.ts` and in `PACKAGES` in `static/Makefile`; verify `make -C static check build`
- [x] 4.2 Pan/zoom: world transform, wheel zoom around the pointer, drag pan and two-pointer pinch with Pointer Events, zoom limits, fit, script-added +/−/fit controls; pure transform maths (zoom around a point, clamping, fit to bounds) in its own module with `bun test` unit tests; verify in a browser `Wheel zoom`, `Drag to pan` (page does not scroll), `Fit`
- [x] 4.3 Click suppression after a drag and `data-zoom` label tiers (plus hiding small labels while moving); verify `Drag starting on a node` and `Zooming in reveals labels` in a browser
- [x] 4.4 Neighbour highlight on hover and focus from `data-n` with an overlay edge path; verify `Hover a node`, and tabbing to a node plus Enter opens its page (`Keyboard`)

## 5. Links, panes, and shortcuts

- [x] 5.1 `anchorHref` helper in `core/src/links.ts` resolving HTML and SVG anchors; use it in the sidebar's click handling for Shift+click and for carrying the sidebar state; verify `bun test` for the helper and, in a browser over `http://`, `Shift+click a graph node` (zoom and pan unchanged) and `Plain click on a graph node keeps panes`; over `file://` Shift+click keeps the default behavior
- [x] 5.2 `g g` in `GO_TARGETS` and the `?` help list; verify in a browser `Go to the graph` on a journal page and on a nested page, and that `g` then `j` still works

## 6. Integration

- [x] 6.1 Rebuild `static/dist/app.js`; run `make lint` and `make test`; build the 5,000-journal bench corpus and `tests/fixtures/notes`, check `graph.html` size and smoothness of panning and zooming in Chromium and Firefox, and that `hermetic serve` still rebuilds within 2 seconds on that corpus; record results here

  Results (2026-10-13):
  - `make lint` is clean; `make test` passes 246 Rust tests and 77 TypeScript tests. `make test-perf`: layout of 5,000 nodes 256 ms (unpinned, alongside other tests), 5,000 journals loaded in 95 ms, and `change_served_within_two_seconds_at_5000_pages` rebuilt 5,251 pages in 433 ms and served the change after 916 ms.
  - Browser checks ran in headless Chromium and Firefox (Playwright, as a throwaway tool outside the repository) against generated wikis served over `http://` by a minimal static server, and from `file://`. All 38 checks pass in both browsers, plus 2 touch checks in Chromium only (multi-touch is dispatched through the DevTools protocol). They cover every `graph-view` scenario that needs a browser (wheel zoom keeps the node under the pointer, drag to pan without page scroll, a drag from a node opens nothing, fit, labels revealed after four zoom steps and kept the same size on screen, hover highlight and its end, click and keyboard opening, no horizontal scroll at 360px, pinch zoom without page zoom, tap after pinch), `Shift+click a graph node` (pane opens, zoom and pan unchanged), `Plain click on a graph node keeps panes`, Shift+click on HTML links unchanged, Shift+click over `file://` keeping the browser's default, `g g` from a journal and a nested page, `g j` unchanged, and `g g` in the `?` list. Without JavaScript over `file://`, the fixture graph renders in light and dark themes and clicking a node opens its page.
  - Sizes: `tests/fixtures/notes` 8 nodes, 2.7 KB. The 5,000-journal performance corpus has only 284 non-journal pages: 58 KB, built in 0.8 s. A generated wiki of 5,000 pages with 16,784 edges: `graph.html` 1.55 MB (290 KB gzipped), built in 0.76 s.
  - Smoothness (frame intervals while panning and wheel-zooming, 1440×900): at 300 pages both browsers hold 60 fps throughout. At 5,000 pages Chromium pans at 60 fps (wheel-zoom bursts p95 83–117 ms); headless Firefox pans at about 20 fps (median 50 ms) and wheel-zoom bursts reach p95 300 ms. Headless Firefox renders in software, so this likely overstates a real desktop; hinting a compositing layer (`will-change: transform`) while moving made no difference and was not kept.

## 7. Search in the graph

- [x] 7.1 Pure matching (`foldText`, every query word a substring of the folded name) in its own module; verify `bun test` for `Matching nodes`, `Several words and accents` (`flow AUTH`, `zurich`), and an empty query matching nothing
- [x] 7.2 Search control and box in `<hermetic-graph>`: Ctrl+F on the graph page (left alone when the box has focus), live emphasis and count, Enter / Shift+Enter through matches with centring and zoom, Ctrl+Enter to open, Escape to restore; styles; `Ctrl+F` in the `?` list on the graph page only; verify in Chromium and Firefox every `Search in the graph` scenario, `Graph search in the shortcut list`, and that the earlier graph and pane checks still pass
- [x] 7.3 Mention graph search in `src/help/pages.txt`; rebuild `static/dist/app.js`; run `make lint` and `make test`; record results here

  Results (2026-10-13):
  - `make lint` is clean (including `static/dist/app.js` being current); `make test` passes 246 Rust tests and 82 TypeScript tests (5 new for matching). The bundle grew from 72.4 KB to 75.7 KB.
  - Browser checks in headless Chromium and Firefox, on the generated 300-page wiki plus `Project X/Auth Flow` and `Zürich`: all 24 graph-search checks pass. They cover Ctrl+F opening the box, `Matching nodes` (31 of 31 expected, labels shown, others at opacity 0.2, count), `Several words and accents`, no matches, `Going through matches` (each match centred to the pixel, all labels showing), Shift+Enter, `Open a match` with Ctrl+Enter, `Escape` (box closed, nothing dimmed, previous view restored exactly), `Browser find stays reachable`, `Graph search in the shortcut list` (graph page only), `Other pages` (a journal page, also with the graph open in a sidebar pane), and search over `file://`. A headless browser shows no find bar, so `Browser find stays reachable` is checked as the key press not being taken over and focus staying in the box. The earlier 38 + 2 graph, pane, and shortcut checks still pass.
