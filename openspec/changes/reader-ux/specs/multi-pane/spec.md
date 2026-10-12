# Spec Delta

## Purpose

Lets a reader keep several pages open next to the current one, Logseq-style, in a right-hand sidebar whose contents are restored from the URL.

## ADDED Requirements

### Requirement: Open a page in the sidebar
When the script is active, Shift+click on a link to a generated page (in the main page or in a sidebar pane) SHALL open that page in a sidebar on the right instead of navigating. The pane SHALL show the target page's `<main>` content, fetched from the site, with all its links and images working relative to the target page. A link to a block anchor SHALL open its page in a pane scrolled to that block. Links to non-page resources (attachments, external URLs) SHALL keep their default behavior.

#### Scenario: Shift+click a page link
- **WHEN** the user Shift+clicks `[[redis]]` on a journal page
- **THEN** the sidebar shows the `redis` page, and the journal page stays in the main area

#### Scenario: Relative links inside a pane
- **WHEN** a pane shows `pages/Project X/Auth Flow.html` and the user clicks its link to `Project X`
- **THEN** the main area navigates to `pages/Project X.html`

#### Scenario: External link
- **WHEN** the user Shift+clicks a link to `https://example.com`
- **THEN** the browser's default behavior happens and no pane is opened

### Requirement: Pane stack
The sidebar SHALL stack panes vertically with the most recently opened on top. Opening a page that is already open SHALL move its pane to the top instead of duplicating it. Each pane SHALL have a title linking to the page, a control to collapse and expand it, and a control to close it. Closing the last pane SHALL hide the sidebar. Panes outside the visible area SHALL NOT be laid out or painted until scrolled into view.

#### Scenario: Reopening moves to top
- **WHEN** panes `auth` (top) and `redis` are open and the user Shift+clicks a link to `redis`
- **THEN** the sidebar shows `redis` on top and `auth` below, with no duplicate

#### Scenario: Closing the last pane
- **WHEN** one pane is open and the user closes it
- **THEN** the sidebar is hidden and the main area uses the full width

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

### Requirement: Sidebar on narrow screens
On viewports narrower than 900px the sidebar SHALL appear as an overlay covering the main area, which the user can hide and show again without closing panes.

#### Scenario: Phone width
- **WHEN** the viewport is 400px wide and a pane is open
- **THEN** the sidebar appears as an overlay with a visible control to hide it, and a visible control to show it again while panes remain open
