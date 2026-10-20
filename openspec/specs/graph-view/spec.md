# graph-view Specification

## Purpose

Shows the whole wiki as a graph of pages and their connections on one page, so a reader can see clusters, hubs, and unconnected pages at a glance and open any of them, with and without JavaScript.

## Requirements

### Requirement: Graph page
The output SHALL contain `graph.html`, showing a graph of the wiki as an inline SVG drawing inside the page's `<main>`, using the full width of the window rather than the text measure. When the graph has no nodes, the page SHALL instead say that there are no pages to show yet.

#### Scenario: Graph page exists
- **WHEN** any notes directory is built
- **THEN** the output contains `graph.html`

#### Scenario: Only journals
- **WHEN** the notes contain only journal files that reference no other page
- **THEN** `graph.html` contains no graph drawing and says there are no pages to show

### Requirement: Nodes
The graph SHALL have one node for every page that is not a journal page, including implicit pages, and including pages without any connection. The page `index` SHALL be a node only when the notes root contains `index.md`. Each node SHALL show the page's display name as its label, and SHALL mark implicit pages differently from pages backed by a file.

#### Scenario: Pages and implicit pages
- **WHEN** the graph contains file-backed pages `auth` and `redis`, the implicit page `kafka`, and journal days
- **THEN** `graph.html` has nodes for `auth`, `redis`, and `kafka`, `kafka` is marked as implicit, and no journal day has a node

#### Scenario: Unconnected page
- **WHEN** `pages/scratch.md` exists and neither references nor is referenced by any page
- **THEN** `graph.html` has a node for `scratch` with no edges

### Requirement: Edges
Two distinct nodes P and Q SHALL be connected by one undirected edge when at least one block connects them: a block located on P that directly references Q or a block of Q (or the reverse), or a block anywhere, including on journal pages, in which P and Q are co-mentioned (see `reference-graph`), or one of P and Q is listed under the other as a namespace child or through a namespaced alias (see `reference-graph`, Namespace children). The edge's weight SHALL be the number of distinct such blocks, plus one when one page is listed under the other. Edges SHALL be drawn thicker for higher weights, up to a maximum thickness. Inherited references SHALL NOT create edges, and a page SHALL NOT be connected to itself.

#### Scenario: Link between pages
- **WHEN** `pages/auth.md` contains `- uses [[redis]]`
- **THEN** `auth` and `redis` are connected by an edge of weight 1

#### Scenario: Links in both directions
- **WHEN** `pages/auth.md` contains one block linking `[[redis]]` and `pages/redis.md` contains one block linking `[[auth]]`
- **THEN** there is exactly one edge between `auth` and `redis`, with weight 2

#### Scenario: Co-mention in a journal
- **WHEN** a journal block contains `[[claude]] helped design [[hermetic]]` and no page links one to the other
- **THEN** `claude` and `hermetic` are connected by an edge of weight 1

#### Scenario: Namespace child
- **WHEN** `pages/Project X/Billing.md` exists and `pages/emanote.md` declares `alias:: alternatives/emanote`, with no links between these pages
- **THEN** `Project X` and `Project X/Billing` are connected by an edge of weight 1, and so are `alternatives` and `emanote`

#### Scenario: Journal reference alone
- **WHEN** a journal block contains only `[[kafka]]`
- **THEN** `kafka` has a node but no edge comes from that block

### Requirement: Node size
A node's size SHALL grow with the number of distinct pages, journal pages included, containing a block that directly references the node's page or one of its blocks, excluding the page itself. Sizes SHALL grow sub-linearly so that hubs stay readable, between a fixed minimum and maximum.

#### Scenario: Referenced page is bigger
- **WHEN** `auth` is referenced from 10 journal days and `redis` from 1 page
- **THEN** the node of `auth` is drawn larger than the node of `redis`, and `redis` larger than an unreferenced page

### Requirement: Layout
Node positions SHALL be computed when the site is built, by a force-directed layout in which connected nodes attract and all nodes repel, so that connected pages end up close together and unconnected ones do not overlap them. Building the same notes twice SHALL produce identical positions. A page's starting position SHALL depend only on its name, so that adding or removing an unrelated page changes the layout as little as practical. For about 5,000 nodes and 20,000 edges, the layout SHALL take at most 300 milliseconds in a release build.

#### Scenario: Connected pages are near
- **WHEN** pages `a`, `b`, and `c` link to each other and pages `x`, `y`, and `z` link to each other, with no link between the two groups
- **THEN** in `graph.html` every node of one group is closer to the other nodes of its group than to any node of the other group

#### Scenario: Deterministic
- **WHEN** the same notes are built twice
- **THEN** both `graph.html` files are byte-identical

#### Scenario: Large wiki
- **WHEN** a graph of 5,000 non-journal pages with 20,000 edges is laid out in a release build
- **THEN** the layout finishes within 300 milliseconds

### Requirement: Graph without JavaScript
Without JavaScript, and when opened from `file://`, `graph.html` SHALL show the whole graph scaled to fit the page, every node SHALL be a relative link to its page, and every node SHALL expose the page name as an accessible name and as a tooltip. The labels of the largest nodes SHALL be visible; the others MAY be hidden until the script reveals them.

#### Scenario: Click a node without JavaScript
- **WHEN** `graph.html` is opened via `file://` with JavaScript disabled and the user clicks the node of `auth`
- **THEN** the browser opens `pages/auth.html`

#### Scenario: Tooltip
- **WHEN** the user hovers a node whose label is hidden
- **THEN** the browser shows the page name as a tooltip

### Requirement: Pan and zoom
When the script runs, the reader SHALL be able to zoom the graph with the mouse wheel or trackpad (centred on the pointer) and with a pinch gesture, pan it by dragging the background or a node, and use controls to zoom in, zoom out, and fit the whole graph in view. Zoom SHALL be limited to a range in which the whole graph fits at the low end and node labels are comfortably readable at the high end. Panning and zooming SHALL NOT scroll or zoom the rest of the page. A drag that starts on a node SHALL NOT open its page.

#### Scenario: Wheel zoom
- **WHEN** the pointer is over the node of `auth` and the user scrolls the wheel up
- **THEN** the graph zooms in and the node of `auth` stays under the pointer

#### Scenario: Drag to pan
- **WHEN** the user drags the background of the graph
- **THEN** the graph moves with the pointer and the page does not scroll

#### Scenario: Drag starting on a node
- **WHEN** the user presses on a node, drags, and releases
- **THEN** the graph is panned and the node's page is not opened

#### Scenario: Fit
- **WHEN** the user has zoomed in and activates the fit control
- **THEN** the whole graph is visible again

### Requirement: Labels while zooming
When the script runs, labels SHALL be shown for more nodes the further the reader zooms in, starting with the largest nodes, so that labels do not overlap into an unreadable mass when zoomed out. The label of a hovered or focused node and of its neighbours SHALL always be shown.

#### Scenario: Zooming in reveals labels
- **WHEN** the graph is shown fitted to the view and the user zooms in four times
- **THEN** labels of smaller nodes that were hidden become visible

### Requirement: Neighbour highlight
When the script runs and the pointer is over a node, or a node has keyboard focus, that node, its neighbours, and the edges between them SHALL be emphasized and the rest of the graph de-emphasized, until the pointer leaves or focus moves.

#### Scenario: Hover a node
- **WHEN** the user hovers the node of `auth`, which is connected to `redis` and `kafka`
- **THEN** `auth`, `redis`, `kafka` and their edges to `auth` are emphasized and other nodes and edges are dimmed

### Requirement: Open a page from the graph
A click on a node SHALL open its page in the main area. Nodes SHALL be reachable with the keyboard and SHALL open their page with Enter. Shift+click SHALL behave like Shift+click on any page link (see `multi-pane`).

#### Scenario: Click a node
- **WHEN** the user clicks the node of `redis` on `graph.html`
- **THEN** `pages/redis.html` opens in the main area

#### Scenario: Keyboard
- **WHEN** the user tabs to the node of `redis` and presses Enter
- **THEN** `pages/redis.html` opens

### Requirement: Search in the graph
When the script runs, the graph SHALL offer a search box, opened with a control next to the zoom controls or with `Ctrl+F` (`Cmd+F` on macOS) on the graph page. Pressing `Ctrl+F` again while the search box has focus SHALL open the browser's own find instead. A node SHALL match when every word of the query appears in its page name, ignoring case and diacritics as site search does. While the query is not empty, matching nodes SHALL be emphasized with their labels shown, the rest de-emphasized, and the number of matches shown. The down arrow SHALL select the next match and the up arrow the previous one, wrapping around at either end, centring the selected match, zooming in until its label is readable, and highlighting its neighbours. `Enter` SHALL open the selected match's page; with no match selected, it SHALL open the match when there is exactly one, and otherwise select the first. `Escape` SHALL clear the query, close the search box, and return to the view from before the search. Searching SHALL work over `file://` as well. On other pages, `Ctrl+F` SHALL keep the browser's behavior.

#### Scenario: Matching nodes
- **WHEN** the graph has the pages `Project X/Auth Flow`, `auth`, and `redis`, and the user types `auth` into the graph search
- **THEN** `Project X/Auth Flow` and `auth` are emphasized with their labels shown, `redis` is dimmed, and the box reports 2 matches

#### Scenario: Several words and accents
- **WHEN** the user types `flow AUTH` or `zurich`
- **THEN** `Project X/Auth Flow` matches the first query and `Zürich` the second

#### Scenario: Going through matches
- **WHEN** a query has two matches and the user presses the down arrow twice, then the up arrow
- **THEN** each press centres the selected match with its label readable: the first, the second, then the first again

#### Scenario: Wrapping around
- **WHEN** the last of three matches is selected and the user presses the down arrow
- **THEN** the first match is selected

#### Scenario: Open a match
- **WHEN** a match is selected and the user presses `Enter`
- **THEN** that match's page opens

#### Scenario: Enter with one match
- **WHEN** a query has exactly one match, none is selected, and the user presses `Enter`
- **THEN** that match's page opens

#### Scenario: Enter with several matches
- **WHEN** a query has several matches, none is selected, and the user presses `Enter`
- **THEN** the first match is selected and centred, and no page opens

#### Scenario: Escape
- **WHEN** the user has searched and selected a match, then presses `Escape`
- **THEN** the search box closes, no node is dimmed, and the graph shows the view it had before the search

#### Scenario: Browser find stays reachable
- **WHEN** the graph search box has focus and the user presses `Ctrl+F`
- **THEN** the browser's own find opens

#### Scenario: Other pages
- **WHEN** the user presses `Ctrl+F` on a journal page
- **THEN** the browser's own find opens

### Requirement: Graph themes
The graph SHALL use the site's theme colours, so that it follows the light and dark themes like the rest of the page, with and without the script.

#### Scenario: Dark theme
- **WHEN** the dark theme is active and `graph.html` is shown
- **THEN** the graph is drawn with light nodes, labels, and edges on the dark background
