# static-site-output Specification

## Purpose

Defines the static HTML site produced from the notes graph: its layout, what every page shows, how references and tasks are rendered, and the guarantees that it works offline, without JavaScript, and with bounded file sizes.

## Requirements

### Requirement: Output layout
The output SHALL contain one HTML file per page in the graph, including implicit pages. A page backed by a file SHALL be written at its source path with `.md` replaced by `.html`; an implicit regular page at the encoded path of its name under `pages/`; an implicit journal page at `journals/YYYY-MM-DD.html`. Site-wide pages SHALL be `index.html` (journal index), `pages.html` (all pages), and `tasks.html` (open tasks). Files belonging to hermetic itself SHALL be under `_hermetic/`. Continuation files SHALL be under `refs/` and `tasks/`.

#### Scenario: Pages, journals, and implicit pages
- **WHEN** the notes contain `journals/2026-10-12.md` linking to `[[kafka]]` and `[[2026-10-01]]`, and `pages/Project X/Auth Flow.md`
- **THEN** the output contains `journals/2026-10-12.html`, `journals/2026-10-01.html`, `pages/kafka.html`, `pages/Project X/Auth Flow.html`, and the implicit namespace page `pages/Project X.html`

#### Scenario: Encoded page names
- **WHEN** the notes contain `pages/C#%3A a.md`
- **THEN** the output contains `pages/C#%3A a.html`, and links to it use the href `C%23%253A%20a.html` relative to `pages/`

### Requirement: Works offline and without JavaScript
Every generated page SHALL be fully usable when opened directly from disk (`file://`) with JavaScript disabled: all content, navigation, linked references, related pages, and tasks SHALL be present in the HTML. Pages SHALL NOT load any resource from outside the output directory, and all links between generated files SHALL be relative.

#### Scenario: Opened from disk
- **WHEN** a generated page is opened via `file://` in a browser with JavaScript disabled
- **THEN** its content, linked references, and navigation links work, and stylesheets and fonts load

#### Scenario: No external resources
- **WHEN** any generated HTML or CSS file is inspected
- **THEN** it references no URL with a scheme or a protocol-relative URL, except links written by the author in note content

### Requirement: Site navigation
Every page SHALL include a header linking to the journal index, the all-pages list, and the open-tasks page, and SHALL use the notes directory's name as the site title. Each page SHALL contain exactly one `<main>` element holding all page-specific content.

#### Scenario: Header on every page
- **WHEN** any generated page is opened
- **THEN** it links to `index.html`, `pages.html`, and `tasks.html` via correct relative paths

### Requirement: Page content
A page SHALL show its title, its blocks as a nested list in document order, and, for namespaced pages, links to each namespace ancestor and a list of its namespace children. Block content SHALL be rendered as CommonMark with tables, strikethrough, task lists, and footnotes. Block properties other than `id` SHALL be shown as key/value pairs. Task blocks SHALL show their state (`TODO`, `DOING`, `DONE`) distinctly from their content. An implicit page SHALL state that it has no content of its own.

#### Scenario: Nested blocks and properties
- **WHEN** a page contains `- decided to use JWT` with `status:: accepted` and `id:: jwt`, and a child block `- **because** it is simple`
- **THEN** the HTML shows the block with a `status: accepted` pair, no visible `id` pair, and a nested child rendered with bold text

#### Scenario: Namespaced page
- **WHEN** the page `Project X/Auth Flow` has a child page `Project X/Auth Flow/Tokens`
- **THEN** its HTML links to `Project X`, and lists `Tokens` as a child

### Requirement: Journal pages
A journal page SHALL be titled with its date and weekday, and SHALL link to the nearest earlier and later journal days that have a source file, when they exist. The journal index (`index.html`) SHALL list all journal days that have a source file, newest first, grouped by month, each linking to its page.

#### Scenario: Previous and next day
- **WHEN** journals exist for 2026-10-09, 2026-10-12, and 2026-10-15
- **THEN** the page for 2026-10-12 is titled with "Monday" and links to 2026-10-09 as previous and 2026-10-15 as next

#### Scenario: Journal index
- **WHEN** the site is built
- **THEN** `index.html` lists journal days newest first under month headings

### Requirement: Reference rendering
Page links and tags SHALL render as links to the target page's HTML, showing the name as written (tags keep their `#`). Markdown links to notes SHALL be rewritten to link to the target page's HTML, keeping any `#fragment`. A block reference SHALL render the target block's content inline, together with a link to the target block's anchor (the content itself is not a link, because it may contain links); block references inside that rendered content SHALL be shown as links without being expanded further. Unresolved block references and markdown links SHALL render visibly broken, showing the original text.

#### Scenario: Rewritten markdown link
- **WHEN** a journal contains `[auth notes](../pages/auth.md#setup)` and `pages/auth.md` exists
- **THEN** the HTML links to `../pages/auth.html#setup`

#### Scenario: Block reference shows target content
- **WHEN** a block contains `((ttl-bug))` and the block with that id reads `TTL defaults were wrong`
- **THEN** the HTML shows `TTL defaults were wrong` inline together with a link to that block's anchor on its page

#### Scenario: Self-referencing block refs do not loop
- **WHEN** block `a` contains `((b))` and block `b` contains `((a))`
- **THEN** the build completes and each renders the other's content once with the nested reference shown as a plain link

#### Scenario: Broken references
- **WHEN** a block contains `((missing))` or a link to a missing note file
- **THEN** the HTML shows the original text marked as broken, not as a working link

### Requirement: Block anchors
Every rendered block SHALL have an HTML anchor that is unique within its page and stable across builds while the page's content does not change: `id-<id>` for blocks with an `id` property, otherwise `b<n>` where `<n>` is the block's 1-based position in the page's document order.

#### Scenario: Anchors
- **WHEN** a page's second block has `id:: ttl-bug` and its third block has no id
- **THEN** their anchors are `id-ttl-bug` and `b3`

### Requirement: Linked references section
Each page SHALL show its linked references as defined by the graph, grouped by source page in the graph's order, each group headed by a link to the source page. Each linked reference SHALL show its breadcrumb (ancestor blocks' content, linking to them) and the referencing block with its full subtree. A page SHALL show at most 100 linked references; further references SHALL be split into continuation files of at most 100 each at `refs/<page path>/<n>.html` (`n` starting at 2), linked from the page and from each other in order.

#### Scenario: References with breadcrumb
- **WHEN** journal 2026-10-12 has `- standup` with child `- [[auth]] token refresh shipped` with child `- details`
- **THEN** `pages/auth.html` shows a group for 2026-10-12 containing the breadcrumb `standup`, the block, and its child `details`

#### Scenario: Pagination
- **WHEN** a page has 250 linked references
- **THEN** its HTML shows the first 100 and links to `refs/<page path>/2.html` (references 101–200), which links to `refs/<page path>/3.html` (201–250)

### Requirement: Open tasks
Each page SHALL show its open tasks (as defined by the graph) in a section separate from its content, each linking to the task block's anchor on its own page. `tasks.html` SHALL list all open tasks in the graph grouped by page, showing at most 100 tasks per file and continuing in `tasks/<n>.html` (`n` starting at 2).

#### Scenario: Open tasks of a page
- **WHEN** a journal block `meeting about [[auth]]` has children `TODO rotate keys` and `DONE drop endpoint`
- **THEN** `pages/auth.html` lists `rotate keys` as an open task linking to its block in the journal, and does not list `drop endpoint`

### Requirement: Related pages section
Each page SHALL show its co-mentioned pages in the graph's order, each with its weight and a link to the related page, expandable without JavaScript (e.g. `<details>`) to show the shared blocks.

#### Scenario: Related pages
- **WHEN** `[[claude]]` and `[[hermetic]]` appear together in 3 blocks
- **THEN** `pages/claude.html` lists `hermetic` with weight 3, and expanding it shows the 3 blocks

### Requirement: All pages list
`pages.html` SHALL list every page that is not a journal, ordered case-insensitively by name, each linking to its HTML, marking implicit pages.

#### Scenario: Implicit pages listed
- **WHEN** the graph has the file-backed page `auth` and the implicit page `kafka`
- **THEN** `pages.html` lists both, with `kafka` marked as having no file

### Requirement: Bounded file sizes
Apart from the journal index and the all-pages list, which list only names, the size of every generated file SHALL depend only on the content of one page, its linked references and related blocks up to the pagination limit, or one page of tasks, and SHALL NOT grow with the total size of the wiki.

#### Scenario: Unrelated growth
- **WHEN** 1,000 journal days unrelated to page `auth` are added
- **THEN** the size of `pages/auth.html` does not change

### Requirement: Attachments
Non-markdown files under `journals/`, `pages/`, and a root-level `assets/` directory (excluding names beginning with `.`) SHALL be copied unchanged to the same relative paths in the output. If an attachment's output path equals a generated page's path, the generated page SHALL win and a diagnostic SHALL be reported.

#### Scenario: Image next to a page
- **WHEN** `pages/auth.md` contains `![flow](auth-flow.png)` and `pages/auth-flow.png` exists
- **THEN** `pages/auth-flow.png` exists in the output and the image displays on `pages/auth.html`

### Requirement: Embedded stylesheet and fonts
The output SHALL include hermetic's stylesheet and the Inter and JetBrains Mono fonts under `_hermetic/`, together with the fonts' license texts, and pages SHALL use only these fonts (Inter for text, JetBrains Mono for code), never fonts installed on the viewer's system except as a last-resort generic fallback.

#### Scenario: Fonts and licenses shipped
- **WHEN** the site is built
- **THEN** `_hermetic/` contains the stylesheet, the Inter and JetBrains Mono font files, and their OFL license texts

### Requirement: Deterministic output
Building the same notes directory twice SHALL produce byte-identical output.

#### Scenario: Rebuild is identical
- **WHEN** the same notes are built into two output directories
- **THEN** both directories contain the same files with identical bytes
