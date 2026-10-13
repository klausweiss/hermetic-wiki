# multi-pane Specification

## Purpose

Lets a reader keep several pages open next to the current one, Logseq-style, in a right-hand sidebar whose contents are restored from the URL.

## Requirements

### Requirement: Open a page in the sidebar
When the script is active, Shift+click on a link to a generated page (in the main page or in a sidebar pane, including the nodes of the graph view, see `graph-view`) SHALL open that page in a sidebar on the right instead of navigating. The pane SHALL show the target page's `<main>` content, fetched from the site, with all its links and images working relative to the target page. A link to a block anchor SHALL open its page in a pane scrolled to that block. Links to non-page resources (attachments, external URLs) SHALL keep their default behavior.

#### Scenario: Shift+click a page link
- **WHEN** the user Shift+clicks `[[redis]]` on a journal page
- **THEN** the sidebar shows the `redis` page, and the journal page stays in the main area

#### Scenario: Shift+click a graph node
- **WHEN** the page is served over `http://` and the user Shift+clicks the node of `redis` on `graph.html`
- **THEN** the sidebar shows the `redis` page, and the graph stays in the main area with its zoom and pan unchanged

#### Scenario: Plain click on a graph node keeps panes
- **WHEN** a pane shows `auth` and the user clicks the node of `redis` on `graph.html`
- **THEN** the main area shows `redis` and the `auth` pane is still open

#### Scenario: Relative links inside a pane
- **WHEN** a pane shows `pages/Project X/Auth Flow.html` and the user clicks its link to `Project X`
- **THEN** the main area navigates to `pages/Project X.html`

#### Scenario: External link
- **WHEN** the user Shift+clicks a link to `https://example.com`
- **THEN** the browser's default behavior happens and no pane is opened

### Requirement: Pane stack
The sidebar SHALL stack panes vertically with the most recently opened on top. Opening a page that is already open SHALL move its pane to the top instead of duplicating it. Each pane SHALL have a title linking to the page, a control to collapse and expand it, and a control to close it. Closing the last pane SHALL leave the sidebar showing its placeholder. Panes outside the visible area SHALL NOT be laid out or painted until scrolled into view.

#### Scenario: Reopening moves to top
- **WHEN** panes `auth` (top) and `redis` are open and the user Shift+clicks a link to `redis`
- **THEN** the sidebar shows `redis` on top and `auth` below, with no duplicate

#### Scenario: Closing the last pane
- **WHEN** one pane is open and the user closes it
- **THEN** the sidebar stays visible and shows its placeholder

#### Scenario: Many panes
- **WHEN** 50 panes are open
- **THEN** scrolling the sidebar stays responsive and only panes near the viewport are rendered

### Requirement: Clicking within the sidebar
A plain click on a page link inside a pane SHALL navigate the main area to that page, keeping the sidebar's panes.

#### Scenario: Navigate main from a pane
- **WHEN** a pane shows `redis` and the user clicks its link to `auth`
- **THEN** the main area shows `auth` and the `redis` pane is still open

### Requirement: Panes in the URL
The open panes, their order, and their collapsed state SHALL be stored in the page URL's query string, so that reloading, using back/forward, and opening a copied URL restore the same panes. The URL fragment SHALL remain available for block anchors of the main page. Navigating the main area through any link on the page SHALL keep the panes. A pane whose page no longer exists SHALL be dropped silently.

#### Scenario: Reload restores panes
- **WHEN** panes `auth` and `redis` are open and the user reloads the page
- **THEN** the same two panes are open in the same order

#### Scenario: Shared link
- **WHEN** a URL copied while panes were open is opened in a new browser window
- **THEN** the main page and the same panes are shown

#### Scenario: Missing page in the URL
- **WHEN** the URL lists a pane for a page that is no longer in the site
- **THEN** the other panes open and no error is shown

### Requirement: Sidebar always present
When the script runs on a viewport at least 900px wide, the sidebar SHALL be shown on every page, also when no pane is open. While no pane is open it SHALL show a placeholder explaining that Shift+click on a link opens the page there. When the site is opened from disk (`file:`), where pages cannot be opened in the sidebar, the placeholder SHALL instead explain that this needs the site to be served over `http://`. The sidebar's layout on page load SHALL be in place before the page is first painted, without animation. While no pane is open the sidebar SHALL be narrow (no wider than about 16rem), widening when a pane opens and narrowing again when the last pane closes. The reader SHALL be able to hide the sidebar and show it again without closing panes, with a single control that stays in the same place on screen whether the sidebar is shown or hidden, and with the `\` shortcut.

#### Scenario: Empty sidebar
- **WHEN** a page is opened over `http://` in a 1440px-wide window with no panes in the URL
- **THEN** the sidebar is visible and shows a placeholder mentioning Shift+click

#### Scenario: Opened from disk
- **WHEN** a page is opened via `file://` with JavaScript enabled in a 1440px-wide window
- **THEN** the sidebar is visible and its placeholder explains that the site must be served over `http://` to open pages there

#### Scenario: No movement when a page loads
- **WHEN** a page is opened or reloaded, with or without panes in the URL
- **THEN** the sidebar and main area appear in their final layout without sliding or animating; only changes the reader makes (opening the first pane, closing the last, hiding, showing) may animate

#### Scenario: Narrow while empty
- **WHEN** the sidebar is empty on a 1440px-wide window and the user then Shift+clicks a page link
- **THEN** the empty sidebar is at most about 16rem wide, and with the pane open it is noticeably wider

#### Scenario: Toggle stays in place
- **WHEN** the user hides the sidebar and then shows it again
- **THEN** the hide control and the show control occupy the same position on screen

#### Scenario: Hide and show
- **WHEN** the user hides the sidebar with its control or `\`
- **THEN** the main area uses the full width, a control to show the sidebar remains visible, and showing it again restores the same panes

### Requirement: Sidebar on narrow screens
On viewports narrower than 900px the sidebar SHALL start hidden, with a visible control to show it, and SHALL appear as an overlay covering the main area when shown or when a pane is opened. The user SHALL be able to hide it again without closing panes.

#### Scenario: Phone width
- **WHEN** the viewport is 400px wide and a pane is open
- **THEN** the sidebar appears as an overlay with a visible control to hide it, and a visible control to show it again while panes remain open

### Requirement: Sidebar footer
When the script runs, the site footer (see `static-site-output`) SHALL be shown at the bottom of the sidebar, below the placeholder or the panes, and SHALL stay in view while the panes scroll. It SHALL not appear anywhere else on the page then. When the sidebar is hidden, the footer is hidden with it; on narrow screens, where the sidebar starts hidden, it is reached by showing the sidebar.

#### Scenario: Footer in the sidebar
- **WHEN** a page is opened with the script running in a 1440px-wide window
- **THEN** the sidebar's bottom shows the link to the all-pages list and "Generated by hermetic", and nothing of the footer shows after the page's content

#### Scenario: Many panes
- **WHEN** enough panes are open that the sidebar scrolls
- **THEN** the footer stays at the bottom of the sidebar, visible

#### Scenario: Narrow screen
- **WHEN** a page is opened with the script running in a 360px-wide window and the reader shows the sidebar
- **THEN** the sidebar covers the page and its footer links to the all-pages list
