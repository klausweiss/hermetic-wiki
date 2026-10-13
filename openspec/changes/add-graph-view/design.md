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
- Live simulation, dragging individual nodes, or filters (journal toggle, search-in-graph, hiding orphans).
- Namespace parent/child edges.
- Keeping zoom/pan in the URL.

## Decisions

### Layout in Rust at build time, rendered as inline SVG
Alternatives: (a) emit JSON and simulate in the browser on a canvas, like Logseq. It needs `fetch` (so `http:` only), shows nothing without JS, and means writing a force simulation, a canvas renderer, and hit-testing in TypeScript. (b) Emit JSON plus a static SVG fallback, which duplicates the data. With the layout done once in Rust, the plain HTML is the full feature, and the script only adds interaction, which is how every other hermetic feature works.

### Graph extraction (`src/graph.rs` or a small `src/site/graph_view.rs`)
- Nodes: pages where `!is_journal()`, skipping the implicit `index` exactly as `site::render` does. Order: `PageId` order (scan order, deterministic).
- Edges: for every block `b` located on page `L`, let `S = direct_pages(b)` restricted to non-journal pages. The block contributes the pairs `{L, q}` for `q ∈ S \ {L}` when `L` is not a journal (links), plus all pairs within `S \ {L}` (co-mentions, matching `reference-graph`'s rule that the containing page does not count). Pairs are deduplicated per block and then counted, so the weight is the number of distinct blocks. Use a `BTreeMap<(PageId, PageId), u32>` keyed by `(min, max)` to keep the order deterministic.
- Mentions (node size): the number of distinct pages of `backlinks(p)`, excluding `p`. Radius `r = R_MIN + K·sqrt(mentions)`, clamped to `R_MAX`.

### Layout algorithm (`src/site/layout_graph.rs`, pure)
- Force-directed (Fruchterman–Reingold style): spring attraction along edges (scaled by `ln(1 + weight)`), pairwise repulsion approximated with a Barnes–Hut quadtree (θ ≈ 0.9), and a weak linear gravity toward the origin so disconnected components and orphans stay nearby without overlapping the main cluster. A fixed number of iterations (about 300) with linear cooling of the maximum step. No convergence test, so the running time is predictable.
- A short final pass pushes overlapping circles apart, using their radii.
- Seeds: each node's starting point comes from FNV-1a over its `NameKey` bytes (never `std`'s randomly-keyed hasher), mapped onto a disc whose radius grows with `sqrt(n)`. A page keeps its starting point across builds, which limits reshuffling when unrelated pages change.
- Determinism: single-threaded, plain `f64` arithmetic in fixed iteration order, and coordinates written rounded to one decimal. Byte-identical output is required only for the same binary, which this satisfies.
- Cost: O(iterations · (n log n + e)). That's about 300 · (5,000·13 + 20,000) ≈ 25M steps, which should be well under the 300 ms target. Checked by an ignored release-mode test, like the existing `test-perf` checks.

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

### Links: SVG anchors through the existing click handling
The sidebar's click handler (`sidebar/src/index.ts:192`) finds the anchor with `closest("a[href]")` and reads `a.href`. On an `SVGAElement` that is an `SVGAnimatedString`. A small helper in `core/src/links.ts` (`anchorHref(a: Element): string`) will resolve `getAttribute("href")` against `document.baseURI` for both kinds of anchor. The sidebar uses it for Shift+click panes and for carrying the sidebar state on plain clicks. Graph node clicks then reuse all of the multi-pane behavior unchanged.

### Shortcut `g g`
Add `g: "graph.html"` to `GO_TARGETS` and a line to the help dialog. The chord logic already resets `pendingG` before it checks the target, so the second `g` resolves the chord instead of starting a new one.

## Risks / Trade-offs

- [The layout reshuffles when the graph changes, since force layouts are chaotic] → Seeds come from page names, so unchanged pages start where they did before. Accepted otherwise. A live simulation is out of scope.
- [`graph.html` is large for big wikis: about 5,000 nodes × ~130 B plus 20,000 edges × ~20 B ≈ 1 MB] → Compact edge paths and neighbour lists. The bounded-size exception is in the spec. Measured in the release perf test.
- [SVG paint cost while panning with thousands of `<a>`/`<text>` elements] → Transform a single group (`will-change: transform`). While a gesture is active, hide labels below `t0` with a `data-moving` attribute. Check with the 5,000-page bench corpus in Chromium and Firefox.
- [The serve rebuild budget] → The layout is under 300 ms at 5,000 pages (tested). If it becomes a problem, positions could be cached keyed on the edge set. Not done now.
- [Wheel capture can trap page scrolling] → The graph's height is capped (`min(80vh, …)`) so the page header and footer stay reachable. Zoom applies only while the pointer is over the graph.

## Open Questions

- The exact constants (iterations, spring length, gravity, R_MIN/R_MAX, tier cutoffs) will be tuned visually against `tests/fixtures/notes` and the bench corpus. They don't affect the spec or the task breakdown.
