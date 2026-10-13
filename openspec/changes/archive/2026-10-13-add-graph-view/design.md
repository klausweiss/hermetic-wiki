# Design

## Context

`src/graph.rs` already resolves every reference: `direct_pages(block)` gives a block's direct page references, `backlinks(page)` the blocks referencing a page, and `related(page)` the co-mentions. The site is rendered in memory by `site::render` (`src/site/mod.rs`) into `SiteFiles`, written by `build` or served by `serve`, which re-renders on every change with a 2-second budget at about 5,000 pages.

Constraints from existing specs: pages must work without JavaScript and from `file://` (no `fetch`), the only script is `_hermetic/app.js` (no inline scripts), output is byte-deterministic, and the frontend has no runtime dependencies. Every page goes through `html::document`, which wraps the page in `<main>` with a narrow text measure.

## Goals / Non-Goals

**Goals:**
- One build-time pass from the reference graph to a finished SVG; the browser only transforms it.
- A layout core that is a pure function (seeds, radii, weighted edges → positions), so it can be unit-tested and benchmarked without building a site.

**Non-Goals:**
- A per-page "local graph" (worth doing later on top of the same layout code).
- Live simulation, dragging individual nodes, or filters (journal toggle, hiding orphans). Search in the graph was added after the first version; see "Search in the graph" below.
- Namespace parent/child edges.
- Keeping zoom/pan in the URL.

## Decisions

### Layout in Rust at build time, rendered as inline SVG
Alternatives: (a) emit JSON and simulate in the browser on a canvas, like Logseq. It needs `fetch` (so `http:` only), shows nothing without JS, and means writing a force simulation, a canvas renderer, and hit-testing in TypeScript. (b) Emit JSON plus a static SVG fallback, which duplicates the data. With the layout done once in Rust, the plain HTML is the full feature, and the script only adds interaction, which is how every other hermetic feature works.

### Graph extraction (`src/graph.rs` or a small `src/site/graph_view.rs`)
- Nodes: pages where `!is_journal()`, skipping the implicit `index` exactly as `site::render` does. Order: `PageId` order (scan order, deterministic).
- Edges: for every block `b` located on page `L`, let `S = direct_pages(b)` restricted to non-journal pages. The block contributes the pairs `{L, q}` for `q ∈ S \ {L}` when `L` is not a journal (links), plus all pairs within `S \ {L}` (co-mentions, matching `reference-graph`'s rule that the containing page does not count). Pairs are deduplicated per block and then counted, so the weight is the number of distinct blocks. Use a `BTreeMap<(PageId, PageId), u32>` keyed by `(min, max)` to keep the order deterministic.
- Mentions (node size): the number of distinct pages of `backlinks(p)`, excluding `p`. Radius `r = R_MIN + K·sqrt(mentions)`, clamped to `R_MAX`.

### Layout algorithm (`src/site/graph_layout.rs`, pure)
- Force-directed, ForceAtlas2-style. Edges attract linearly in their length, scaled by `1 + ln(weight)`. Every pair of nodes repels with magnitude k²·qᵢ·qⱼ/d, approximated with a Barnes–Hut quadtree (θ = 1.2). The charge q grows with the node's radius, so hubs get room. A linear gravity toward the origin keeps disconnected components and orphans nearby. There are 150 iterations with linear cooling of the maximum step, and no convergence test, so the running time is predictable. A quadratic (Fruchterman–Reingold) spring was tried first. It crushed graphs with many long-range edges into a jammed ball that the overlap pass could not untangle, and it was 2.2 s at 5,000 nodes.
- A final pass pushes overlapping circles apart, using their radii.
- Seeds: each node's starting point comes from FNV-1a (finished with a splitmix64 round) over its `NameKey` bytes, never `std`'s randomly-keyed hasher. It is mapped onto a disc whose radius grows with `sqrt(n)`. A page keeps its starting point across builds, which limits reshuffling when unrelated pages change.
- Determinism: plain `f64` arithmetic in a fixed order, with coordinates written rounded to one decimal. From 1,000 nodes up, repulsion is computed on up to 8 threads. Each body's force depends only on the previous iteration's positions and is computed by exactly one thread, so the result is identical whatever the thread count. Byte-identical output is required only for the same binary, which this satisfies.
- Cost: O(iterations · (n log n + e)). Measured at about 210 ms on 8 CPUs and 270 ms on 4 for 5,000 nodes and 21,600 edges, and about 500 ms on a single core (see tasks 2.2). Checked by an ignored release-mode test, like the existing `test-perf` checks. A dev-only ignored test, `layout_preview`, renders a wiki-like graph to SVG for tuning by eye.

### SVG structure
```html
<article class="graph-view">
  <h1>Graph</h1>
  <hermetic-graph>
    <svg viewBox="minx miny w h" class="graph">
      <g class="graph-world">
        <g class="graph-edges">
          <path class="w1" d="M x y L x y M …"/>   <!-- one path per weight bucket (1, 2–3, 4+) -->
        </g>
        <g class="graph-nodes">
          <a href="pages/auth.html" data-i="0" data-n="3 7 12" class="t0">
            <title>auth</title><circle cx cy r/><text x y>auth</text>
          </a>
          …
```
- Grouping edges into a few `<path>` elements by weight bucket keeps the file small (about 16 bytes per edge) and makes them cheap to paint. Stroke width comes from CSS per bucket.
- `data-n` lists each node's neighbours by index, so the script can highlight without per-edge elements. It draws the emphasized edges as one extra overlay path computed from the node coordinates.
- Label tiers `t0`…`t3` come from ranking nodes by size (`t0` is roughly the top 30 or the top 5%). CSS shows only `t0` by default. The script sets `data-zoom="0..3"` on the element, and CSS shows the tiers up to that level. Implicit pages get the class `implicit` (dashed outline, like the "no file" marking in `pages.html`).
- Colours come only from the existing CSS custom properties (`--text`, `--muted`, `--link`, `--border`, `--bg`), so the light and dark themes work with no extra code.
- Full width: `main:has(> article.graph-view)` drops the text measure. `html::document` stays unchanged.
- Empty graph: the article contains a paragraph and no `<hermetic-graph>`.

### `@hermetic/graph` element (frontend)
- A new package, following the usual layout (`src/index.ts`, `src/register.ts`, its Makefile), registered in `app/src/main.ts` and in `PACKAGES` in `static/Makefile`.
- Pan/zoom: a `transform` matrix on `.graph-world`, driven by Pointer Events. One pointer pans and two pointers pinch. `wheel` zooms around the pointer (`passive: false` plus `preventDefault`, so the page itself doesn't scroll). `touch-action: none` is set on the SVG. The zoom range runs from "fit" ÷ 1.5 up to the level where `t3` labels are 14px on screen.
- Click suppression: once a gesture moves more than 4 px, the next `click` is cancelled in the capture phase, so a drag that starts on a node doesn't navigate.
- Controls (+, −, fit) are added by the script only, as required by `reader-ui` progressive enhancement, and use the existing `icon()` helper. If a needed icon is missing, it is added to `core/src/icons.ts`.
- Highlight: on `pointerenter` or `focusin` of a node `a`, the script sets `data-focus` on the SVG and `.hl` on the node and its neighbours, and fills the overlay path. CSS dims everything else.
- The element works the same inside a sidebar pane. Panes adopt the article and the custom element upgrades there, so no special case is needed.

### Search in the graph
- Part of `<hermetic-graph>`: a script-added search control and box in `.graph-controls`. Matching uses `foldText` from `@hermetic/search`, so case and diacritics fold exactly as in site search. Each node's folded name is computed once at setup, and a node matches when every folded word of the query is a substring of it. The pure matching function sits in its own module with unit tests.
- Display: `data-search` on the SVG, with `.match` on matching nodes. CSS dims non-matches and shows match labels, reusing the highlight styles. The current match gets the existing neighbour highlight.
- Moving to a match: centre it (`centreOn`) and zoom to at least the level where all labels show (`labelLevel` 3), within the zoom limit. The view from before the search is saved when the box opens and restored on `Escape`.
- `Ctrl+F` is taken over by a `keydown` listener on `document` that only the graph element installs, so other pages never see it. When the search input already has focus, the key is left alone and the browser's find opens (the Discourse pattern). `shortcuts` lists `Ctrl+F` in the `?` dialog only when a `hermetic-graph` element is on the page.

### Links: SVG anchors through the existing click handling
The sidebar's click handler (`sidebar/src/index.ts:192`) finds the anchor with `closest("a[href]")` and reads `a.href`. On an `SVGAElement` that is an `SVGAnimatedString`. A small helper in `core/src/links.ts` (`anchorHref(a: Element): string`) will resolve `getAttribute("href")` against `document.baseURI` for both kinds of anchor. The sidebar uses it for Shift+click panes and for carrying the sidebar state on plain clicks. Graph node clicks then reuse all of the multi-pane behavior unchanged.

### Shortcut `g g`
Add `g: "graph.html"` to `GO_TARGETS` and a line to the help dialog. The chord logic already resets `pendingG` before it checks the target, so the second `g` resolves the chord instead of starting a new one.

## Risks / Trade-offs

- [The layout reshuffles when the graph changes, since force layouts are chaotic] → Seeds come from page names, so unchanged pages start where they did before. Accepted otherwise. A live simulation is out of scope.
- [`graph.html` is large for big wikis: measured at 1.55 MB (290 KB gzipped) for 5,000 pages and 16,800 edges, about 215 B per node (the name appears twice, as tooltip and label) plus about 28 B per edge] → Compact edge paths and neighbour lists. The bounded-size exception is in the spec.
- [SVG paint cost while panning with thousands of `<a>`/`<text>` elements] → Transform a single group, and while a gesture is active hide labels below `t0` (`data-moving`). Measured (tasks 6.1): 60 fps at 300 pages in both browsers. At 5,000 pages Chromium stays at 60 fps, but headless Firefox pans at about 20 fps; `will-change: transform` did not help. If real Firefox is also slow on very large wikis, the next step would be drawing to a canvas while moving.
- [The serve rebuild budget] → The layout takes under 300 ms at 5,000 pages on 4 or more CPUs, and about 500 ms on one (tested). If it becomes a problem, positions could be cached keyed on the edge set. Not done now.
- [Wheel capture can trap page scrolling] → The graph's height is capped (`min(80vh, …)`) so the page header and footer stay reachable. Zoom applies only while the pointer is over the graph.

## Open Questions

- The exact constants (iterations, spring length, gravity, R_MIN/R_MAX, tier cutoffs) will be tuned visually against `tests/fixtures/notes` and the bench corpus. They don't affect the spec or the task breakdown.
