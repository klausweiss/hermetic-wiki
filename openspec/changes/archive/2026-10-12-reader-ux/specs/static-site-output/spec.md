# Spec Delta

## MODIFIED Requirements

### Requirement: Output layout
The output SHALL contain one HTML file per page in the graph, including implicit pages. A page backed by a file SHALL be written at its source path with `.md` replaced by `.html`; an implicit regular page at the encoded path of its name under `pages/`; an implicit journal page at `journals/YYYY-MM-DD.html`. Site-wide pages SHALL be `index.html` (home page), `journals.html` (journal list), `pages.html` (all pages), and `tasks.html` (open tasks). Files belonging to hermetic itself SHALL be under `_hermetic/`. Continuation files SHALL be under `refs/` and `tasks/`.

#### Scenario: Pages, journals, and implicit pages
- **WHEN** the notes contain `journals/2026-10-12.md` linking to `[[kafka]]` and `[[2026-10-01]]`, and `pages/Project X/Auth Flow.md`
- **THEN** the output contains `journals/2026-10-12.html`, `journals/2026-10-01.html`, `pages/kafka.html`, `pages/Project X/Auth Flow.html`, and the implicit namespace page `pages/Project X.html`

#### Scenario: Encoded page names
- **WHEN** the notes contain `pages/C#%3A a.md`
- **THEN** the output contains `pages/C#%3A a.html`, and links to it use the href `C%23%253A%20a.html` relative to `pages/`

#### Scenario: Site-wide pages
- **WHEN** any notes directory is built
- **THEN** the output contains `index.html`, `journals.html`, `pages.html`, and `tasks.html`

### Requirement: Works offline and without JavaScript
Every generated page SHALL be fully usable when opened directly from disk (`file://`) with JavaScript disabled: all content, navigation, linked references, related pages, and tasks SHALL be present in the HTML. Pages SHALL NOT load any resource from outside the output directory, and all links between generated files SHALL be relative. Pages MAY load hermetic's own bundled script from `_hermetic/`; nothing on a page SHALL depend on it to be shown or reached.

#### Scenario: Opened from disk
- **WHEN** a generated page is opened via `file://` in a browser with JavaScript disabled
- **THEN** its content, linked references, and navigation links work, and stylesheets and fonts load

#### Scenario: No external resources
- **WHEN** any generated HTML or CSS file is inspected
- **THEN** it references no URL with a scheme or a protocol-relative URL, except links written by the author in note content

#### Scenario: Only hermetic's own script
- **WHEN** any generated HTML file is inspected
- **THEN** its only `<script>` element loads `_hermetic/app.js` via a relative URL, and it has no inline scripts

### Requirement: Site navigation
Every page SHALL include a header whose site title links to the home page and which links to the journal list, the all-pages list, and the open-tasks page, and SHALL use the notes directory's name as the site title. Each page SHALL contain exactly one `<main>` element holding all page-specific content, and SHALL declare the relative path from itself to the output root in a `<meta name="hermetic-root">` element.

#### Scenario: Header on every page
- **WHEN** any generated page is opened
- **THEN** its site title links to `index.html`, and it links to `journals.html`, `pages.html`, and `tasks.html` via correct relative paths

#### Scenario: Root path declared
- **WHEN** `pages/Project X/Auth Flow.html` is inspected
- **THEN** it contains `<meta name="hermetic-root" content="../..">`

### Requirement: Journal pages
A journal page SHALL be titled with its date and weekday, and SHALL link to the nearest earlier and later journal days that have a source file, when they exist. The journal list (`journals.html`) SHALL list all journal days that have a source file, newest first, grouped by month, each linking to its page.

#### Scenario: Previous and next day
- **WHEN** journals exist for 2026-10-09, 2026-10-12, and 2026-10-15
- **THEN** the page for 2026-10-12 is titled with "Monday" and links to 2026-10-09 as previous and 2026-10-15 as next

#### Scenario: Journal index
- **WHEN** the site is built
- **THEN** `journals.html` lists journal days newest first under month headings

## ADDED Requirements

### Requirement: Home page
When the notes root contains `index.md`, `index.html` SHALL be the HTML of the page `index` (its blocks, linked references, related pages, and open tasks, like any page). When there is no `index.md`, `index.html` SHALL show the journal list, followed by an unobtrusive note that adding `index.md` to the notes root replaces this page.

#### Scenario: Custom home page
- **WHEN** the notes root contains `index.md` with `- Start at [[auth]]` and `pages/auth.md` exists
- **THEN** `index.html` shows "Start at" with a link to `pages/auth.html`, and `pages/auth.html` lists a linked reference from the page `index`

#### Scenario: Default home page
- **WHEN** the notes root has no `index.md`
- **THEN** `index.html` lists the journal days like `journals.html` and includes a short hint naming `index.md`

#### Scenario: Linking to the home page
- **WHEN** `index.md` exists and a journal contains `[[index]]`
- **THEN** the journal's link points to `index.html`
