# Spec Delta

## MODIFIED Requirements

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
