# Proposal

## Why

The reference graph already knows how every page relates to every other page (links, tags, block refs, co-mentions), but the site only shows this one page at a time. A whole-wiki graph view, like Logseq's, lets a reader see clusters, hubs, and orphans at a glance and jump into any of them.

## What Changes

- New site-wide page `graph.html` with a graph of all non-journal pages: one node per page, sized by how many pages reference it, and one edge per pair of pages that are connected by a link (either way) or by a co-mention (including co-mentions written in journals).
- The layout is computed at build time in Rust (seeded, deterministic force-directed layout) and emitted as inline SVG whose nodes are plain relative links, so the graph is visible and clickable without JavaScript and from `file://`.
- The bundled script adds pan and zoom (wheel, drag, pinch, zoom buttons, fit-to-view), highlights a node's neighbours on hover/focus, and reveals labels of smaller nodes as the reader zooms in.
- Click on a node opens its page; Shift+click opens it in a sidebar pane, like any page link.
- Every page's header links to the graph, and the shortcut `g` then `g` opens it.
- `graph.html` is exempt from the bounded-file-size rule: it grows with the number of non-journal pages and their connections (not with journals or block content).

## Capabilities

### New Capabilities
- `graph-view`: the graph page: which pages and connections it shows, node sizing, build-time layout, behavior without JavaScript, pan/zoom, highlighting, labels, and opening pages from it.

### Modified Capabilities
- `static-site-output`: `graph.html` joins the site-wide pages, the header links to it, and it gets an exception from "Bounded file sizes".
- `multi-pane`: Shift+click on a graph node opens the page in a pane (SVG links behave like HTML links).
- `reader-ui`: new shortcut `g` then `g` opens the graph.

## Impact

- Rust: edge and node-size extraction over `src/graph.rs`; a new build-time layout module; a new site-wide page in `src/site/pages.rs`; header in `src/site/html.rs`.
- Frontend: new `@hermetic/graph` package (pan/zoom/highlight element) bundled into `static/dist/app.js`; `sidebar` click handling learns SVG `<a>` elements; `shortcuts` gains `g g`; graph styles in `static/style.css`.
- Build time: the layout runs on every build and every `serve` rebuild; it must fit the existing 2-second rebuild budget for about 5,000 pages.
- No new dependencies on either side.
