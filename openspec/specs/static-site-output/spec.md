# static-site-output Specification

## Purpose

Defines the static HTML site produced from the notes graph: its layout, what every page shows, how references and tasks are rendered, and the guarantees that it works offline, without JavaScript, and with bounded file sizes.

## Requirements

### Requirement: Output layout
The output SHALL contain one HTML file per page in the graph, including implicit pages. A page backed by a file SHALL be written at its source path with `.md` replaced by `.html`; an implicit regular page at the encoded path of its name under `pages/`; an implicit journal page at `journals/YYYY-MM-DD.html`. Site-wide pages SHALL be `index.html` (home page), `journals.html` (journal list), `pages.html` (all pages), `tasks.html` (open tasks), `search.html` (search results), and `graph.html` (the graph view, see `graph-view`). Files belonging to hermetic itself SHALL be under `_hermetic/`. Continuation files SHALL be under `refs/` and `tasks/`. The search index SHALL be under `search/`.

#### Scenario: Pages, journals, and implicit pages
- **WHEN** the notes contain `journals/2026-10-12.md` linking to `[[kafka]]` and `[[2026-10-01]]`, and `pages/Project X/Auth Flow.md`
- **THEN** the output contains `journals/2026-10-12.html`, `journals/2026-10-01.html`, `pages/kafka.html`, `pages/Project X/Auth Flow.html`, and the implicit namespace page `pages/Project X.html`

#### Scenario: Encoded page names
- **WHEN** the notes contain `pages/C#%3A a.md`
- **THEN** the output contains `pages/C#%3A a.html`, and links to it use the href `C%23%253A%20a.html` relative to `pages/`

#### Scenario: Site-wide pages
- **WHEN** any notes directory is built
- **THEN** the output contains `index.html`, `journals.html`, `pages.html`, `tasks.html`, `search.html`, `graph.html`, and `search/meta.json`

### Requirement: Works offline and without JavaScript
Every generated page SHALL be fully usable when opened directly from disk (`file://`) with JavaScript disabled: all content, navigation, linked references, related pages, and tasks SHALL be present in the HTML. Pages SHALL NOT load any resource from outside the output directory, and all links between generated files SHALL be relative. Pages MAY load hermetic's own bundled script from `_hermetic/`; nothing on a page SHALL depend on it to be shown or reached.

#### Scenario: Opened from disk
- **WHEN** a generated page is opened via `file://` in a browser with JavaScript disabled
- **THEN** its content, linked references, and navigation links work, and stylesheets and fonts load

#### Scenario: No external resources
- **WHEN** any generated HTML or CSS file is inspected
- **THEN** it references no URL with a scheme or a protocol-relative URL, except links written by the author in note content, the site footer's link to hermetic's project page, and `data:` URLs, which carry their content inline

#### Scenario: Only hermetic's own script
- **WHEN** any generated HTML file is inspected
- **THEN** its only `<script>` element loads `_hermetic/app.js` via a relative URL, and it has no inline scripts

### Requirement: Site navigation
Every page SHALL include a header whose site title links to the home page and which links to the journal list, the open-tasks page, and the graph view, and SHALL use the `--title` value as the site title when given, or the notes directory's name otherwise. When the script runs and can load the site's data files, the link to the open-tasks page SHALL show the number of open tasks in the graph, unless it is zero. The number SHALL NOT be part of the page's HTML, so that a change in the number of open tasks alone changes no page other than the open-tasks list; without the script, and where the data file cannot be loaded (such as a page opened via `file://`), the link SHALL be shown without a number. Every page SHALL also include a site footer with a link to the all-pages list and the text "Generated with hermetic", followed by the commit of hermetic's source the program was built from when that is known. The word "hermetic" SHALL link to hermetic's project page, `https://github.com/klausweiss/hermetic-wiki`. Links SHALL be visually distinct from the footer's text. Without the script the footer SHALL appear after the page's content, and with the script at the bottom of the sidebar (see `multi-pane`). The footer SHALL contain nothing that changes between builds of the same notes by the same program. Each page SHALL contain exactly one `<main>` element holding all page-specific content, and SHALL declare the relative path from itself to the output root in a `<meta name="hermetic-root">` element.

#### Scenario: Header on every page
- **WHEN** any generated page is opened
- **THEN** its site title links to `index.html`, and it links to `journals.html`, `tasks.html`, and `graph.html` via correct relative paths

#### Scenario: Open-tasks count shown by the script
- **WHEN** the notes contain 3 open tasks and a generated page is opened over HTTP with JavaScript enabled
- **THEN** the link to `tasks.html` shows the number 3

#### Scenario: No count without the script
- **WHEN** the notes contain 3 open tasks and a generated page is opened with JavaScript disabled, or via `file://`
- **THEN** the link to `tasks.html` is shown and works, without a number

#### Scenario: No count when there are no open tasks
- **WHEN** the notes contain no open tasks and a generated page is opened with JavaScript enabled
- **THEN** the link to `tasks.html` shows no number

#### Scenario: Ticking off a task changes only the pages that show it
- **WHEN** the notes are built, a `TODO` block in `pages/auth.md` that references no page is changed to `DONE`, and the notes are built again
- **THEN** among the HTML files, only `pages/auth.html` and the open-tasks list files differ between the two builds

#### Scenario: Footer on every page
- **WHEN** `pages/Project X/Auth Flow.html` is opened with JavaScript disabled
- **THEN** after the page's content there is a footer with a link to `../../pages.html` and the text "Generated with hermetic" with "hermetic" linking to `https://github.com/klausweiss/hermetic-wiki`

#### Scenario: Commit in the footer
- **WHEN** hermetic was built from a git checkout at commit `473790c`
- **THEN** every page's footer reads "Generated with hermetic 473790c"

#### Scenario: Root path declared
- **WHEN** `pages/Project X/Auth Flow.html` is inspected
- **THEN** it contains `<meta name="hermetic-root" content="../..">`

### Requirement: Link previews
Every page SHALL declare its title, the site title, and its type as Open Graph metadata (`og:title`, `og:site_name`, `og:type` of `website`), and a `twitter:card` of `summary`, so that other sites and apps can show a preview of a link to it. A note's page SHALL also have a description, given both as `<meta name="description">` and as `og:description`: the page's `description` property, with link brackets removed, when it has one, or else the plain text of its first block with any text, in document order, not counting the page's properties. The description SHALL have its whitespace collapsed and, when longer than 200 characters, SHALL be cut at a word boundary and end with "…". Pages with nothing to describe, such as the site-wide lists and continuation files, SHALL have no description. Pages SHALL NOT declare `og:url` or `og:image`, which must be absolute URLs, as the site's address is not known.

#### Scenario: Description from the first block
- **WHEN** `pages/a.md` is `source:: x`, followed by an empty block whose child reads `see [[b]]`
- **THEN** `pages/a.html` contains `<meta name="description" content="see b">` and `<meta property="og:description" content="see b">`

#### Scenario: Description property
- **WHEN** `pages/b.md` has the page property `description:: about [[a]]`
- **THEN** `pages/b.html` contains `<meta name="description" content="about a">`

#### Scenario: No description on lists
- **WHEN** `journals.html` is inspected
- **THEN** it contains `<meta property="og:title" content="Journals">` and no description

### Requirement: Page content
A page SHALL show its title, its blocks as a nested list in document order, and, for namespaced pages, links to each namespace ancestor and a list of its namespace children. Block content SHALL be rendered as CommonMark with tables, strikethrough, task lists, and footnotes. Block properties other than `id` SHALL be shown as key/value pairs. The page's `alias` property SHALL be shown as its alias names separated by commas, as plain text without link syntax. Task blocks SHALL show their state (`TODO`, `DOING`, `DONE`) distinctly from their content. An implicit page SHALL state that it has no content of its own.

#### Scenario: Nested blocks and properties
- **WHEN** a page contains `- decided to use JWT` with `status:: accepted` and `id:: jwt`, and a child block `- **because** it is simple`
- **THEN** the HTML shows the block with a `status: accepted` pair, no visible `id` pair, and a nested child rendered with bold text

#### Scenario: Namespaced page
- **WHEN** the page `Project X/Auth Flow` has a child page `Project X/Auth Flow/Tokens`
- **THEN** its HTML links to `Project X`, and lists `Tokens` as a child

#### Scenario: Aliases
- **WHEN** `pages/auth.md` starts with `alias:: authentication, [[login]]`
- **THEN** `pages/auth.html` shows an `alias: authentication, login` pair with no links

### Requirement: Journal pages
A journal page SHALL be titled with its date and weekday, and SHALL link to the nearest earlier and later journal days that have a source file, when they exist. The journal list (`journals.html`) SHALL list all journal days that have a source file, newest first, grouped by month, each linking to its page.

#### Scenario: Previous and next day
- **WHEN** journals exist for 2026-10-09, 2026-10-12, and 2026-10-15
- **THEN** the page for 2026-10-12 is titled with "Monday" and links to 2026-10-09 as previous and 2026-10-15 as next

#### Scenario: Journal index
- **WHEN** the site is built
- **THEN** `journals.html` lists journal days newest first under month headings

### Requirement: Reference rendering
Page links and tags SHALL render as links to the target page's HTML, showing the name as written (tags keep their `#`). A page link or tag that resolves through an alias SHALL link to the HTML of the page that declares the alias; no HTML SHALL be written for an alias name. Markdown links to notes SHALL be rewritten to link to the target page's HTML, keeping any `#fragment`. A block reference SHALL render the target block's content inline, together with a link to the target block's anchor (the content itself is not a link, because it may contain links); block references inside that rendered content SHALL be shown as links without being expanded further. Unresolved block references and markdown links SHALL render visibly broken, showing the original text.

#### Scenario: Rewritten markdown link
- **WHEN** a journal contains `[auth notes](../pages/auth.md#setup)` and `pages/auth.md` exists
- **THEN** the HTML links to `../pages/auth.html#setup`

#### Scenario: Link through an alias
- **WHEN** `pages/auth.md` declares `alias:: login` and a journal contains `[[login]]`
- **THEN** the HTML shows `login` linking to `../pages/auth.html`, and the site has no `pages/login.html`

#### Scenario: Block reference shows target content
- **WHEN** a block contains `((ttl-bug))` and the block with that id reads `TTL defaults were wrong`
- **THEN** the HTML shows `TTL defaults were wrong` inline together with a link to that block's anchor on its page

#### Scenario: Self-referencing block refs do not loop
- **WHEN** block `a` contains `((b))` and block `b` contains `((a))`
- **THEN** the build completes and each renders the other's content once with the nested reference shown as a plain link

#### Scenario: Broken references
- **WHEN** a block contains `((missing))` or a link to a missing note file
- **THEN** the HTML shows the original text marked as broken, not as a working link

### Requirement: External links
A bare URL starting with `http://` or `https://` in block content or a property value SHALL render as a link to that URL, showing the URL as written. The URL SHALL end at the first whitespace or `<`. Trailing `.`, `,`, `:`, `;`, `!`, `?`, `*`, `_`, `~`, `'`, and `"` SHALL NOT be part of it, and neither SHALL a trailing `)` that has no matching `(` in the URL. A URL inside inline code, a code block, or an existing link SHALL NOT be linked again. Every link in block content or a property value whose destination starts with `http://` or `https://` (a bare URL, an `<…>` autolink, or a markdown link) SHALL be visually distinct from links to generated pages, by a marker after its text that is shown without JavaScript. External links SHALL open the way plain links do: the page SHALL NOT set a `target` on them.

#### Scenario: Bare URL
- **WHEN** a block contains `see http://127.0.0.1:8000/index.html.`
- **THEN** the HTML links `http://127.0.0.1:8000/index.html` to that URL, and the final `.` is plain text after the link

#### Scenario: Parentheses
- **WHEN** a block contains `(docs: https://en.wikipedia.org/wiki/Rust_(programming_language))`
- **THEN** the link's destination is `https://en.wikipedia.org/wiki/Rust_(programming_language)`, and the last `)` is plain text

#### Scenario: URL in code or an existing link
- **WHEN** a block contains `` `curl https://example.com` `` and `[site](https://example.com)`
- **THEN** the code span holds no link, and the markdown link is a single link with the text `site`

#### Scenario: Distinct from page links
- **WHEN** a block contains `[[redis]]` and `https://redis.io`
- **THEN** the URL is shown with the external-link marker and the page link is not, and neither link has a `target` attribute

#### Scenario: URL in a property value
- **WHEN** a block has the property `source:: https://example.com/report`
- **THEN** the property's value links to `https://example.com/report`

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
Apart from the journal index and the all-pages list, which list only names, the graph view, whose size grows only with the number of non-journal pages and the connections between them, and the search index under `search/`, whose size is governed by the `search` capability, the size of every generated file SHALL depend only on the content of one page, its linked references and related blocks up to the pagination limit, or one page of tasks, and SHALL NOT grow with the total size of the wiki.

#### Scenario: Unrelated growth
- **WHEN** 1,000 journal days unrelated to page `auth` are added
- **THEN** the size of `pages/auth.html` does not change

#### Scenario: Graph ignores journal content
- **WHEN** 1,000 journal days are added that reference no page
- **THEN** the size of `graph.html` does not change

### Requirement: Attachments
Non-markdown files under `journals/`, `pages/`, and a root-level `assets/` directory (excluding names beginning with `.`) SHALL be copied unchanged to the same relative paths in the output. If an attachment's output path equals a generated page's path, the generated page SHALL win and a diagnostic SHALL be reported.

#### Scenario: Image next to a page
- **WHEN** `pages/auth.md` contains `![flow](auth-flow.png)` and `pages/auth-flow.png` exists
- **THEN** `pages/auth-flow.png` exists in the output and the image displays on `pages/auth.html`

### Requirement: Attachment links
An inline markdown image or link (`![alt](dest)` or `[text](dest)`) in block content or a property value SHALL be rendered with its destination rewritten when the destination has no URL scheme, does not start with `#` or `//`, does not point at a note (`.md`), and stays inside the notes folder. A destination starting with `/` SHALL mean the notes root; any other destination SHALL be resolved against the folder of the block's source file, after percent-decoding. The rewritten destination SHALL be the relative href from the page being rendered to that file's path in the output, percent-encoded as other hrefs between generated files are, keeping any `#fragment`. This SHALL apply wherever a block is rendered: its own page, linked references and their continuation pages, block references, related pages, the timeline, and the HTML used by sidebar panes. The destination SHALL be rewritten whether or not the file exists. A destination that leaves the notes folder, reference-style links, and HTML elements written in the note SHALL be left as written.

#### Scenario: Journal image in linked references
- **WHEN** `journals/2026_10_17.md` contains `- [[auth]] ![flow](../assets/flow.png)` and `assets/flow.png` exists
- **THEN** the image's `src` is `../assets/flow.png` on `journals/2026-10-17.html`, and `../assets/flow.png` in the linked references of `pages/auth.html`

#### Scenario: Continuation page
- **WHEN** that block is listed on `refs/pages/auth/2.html`
- **THEN** the image's `src` is `../../../assets/flow.png`

#### Scenario: Namespaced page
- **WHEN** `pages/Project X/Auth.md` contains `![seq](../../assets/seq.png)` and a journal block embeds that block with `((…))`
- **THEN** the image displays on `pages/Project X/Auth.html` and on the journal's page

#### Scenario: Image next to its note
- **WHEN** `pages/auth.md` contains the block `- ![flow](auth-flow.png)` with `id:: flow`, and `journals/2026_10_17.md` contains `- see ((flow))`
- **THEN** the image's `src` is `auth-flow.png` on `pages/auth.html` and `../pages/auth-flow.png` on `journals/2026-10-17.html`

#### Scenario: Root-relative destination
- **WHEN** `pages/auth.md` contains `![logo](/assets/logo.png)`
- **THEN** the image's `src` on `pages/auth.html` is `../assets/logo.png`, which also works when the site is opened via `file://`

#### Scenario: Left as written
- **WHEN** a block contains `![x](https://example.com/x.png)`, `![y](data:image/png;base64,…)`, and `![z](../../../outside.png)` from `pages/auth.md`
- **THEN** all three destinations are rendered exactly as written

### Requirement: Embedded stylesheet and fonts
The output SHALL include hermetic's stylesheet and the Inter and JetBrains Mono fonts under `_hermetic/`, together with the fonts' license texts and the license text of the icons the script uses (`_hermetic/LICENSE-Lucide.txt`), and pages SHALL use only these fonts (Inter for text, JetBrains Mono for code), never fonts installed on the viewer's system except as a last-resort generic fallback.

#### Scenario: Fonts and licenses shipped
- **WHEN** the site is built
- **THEN** `_hermetic/` contains the stylesheet, the Inter and JetBrains Mono font files, their OFL license texts, and the Lucide ISC license text

### Requirement: Deterministic output
Building the same notes directory twice SHALL produce byte-identical output.

#### Scenario: Rebuild is identical
- **WHEN** the same notes are built into two output directories
- **THEN** both directories contain the same files with identical bytes

### Requirement: Home page
When the notes root contains `index.md`, `index.html` SHALL be the HTML of the page `index` (its blocks, linked references, related pages, and open tasks, like any page). When there is no `index.md`, `index.html` SHALL show the journal list, followed by an unobtrusive note that adding `index.md` to the notes root replaces this page. References to the page `index` SHALL always link to `index.html`, also without `index.md`; the page `index` SHALL then have no page of its own. When the page `index (in pages)` exists (see `source-layout`), `index.html` SHALL start with a warning that `pages/index.md` conflicts with the home page, linking to that page.

#### Scenario: Custom home page
- **WHEN** the notes root contains `index.md` with `- Start at [[auth]]` and `pages/auth.md` exists
- **THEN** `index.html` shows "Start at" with a link to `pages/auth.html`, and `pages/auth.html` lists a linked reference from the page `index`

#### Scenario: Default home page
- **WHEN** the notes root has no `index.md`
- **THEN** `index.html` lists the journal days like `journals.html` and includes a short hint naming `index.md`

#### Scenario: Linking to the home page
- **WHEN** `index.md` exists and a journal contains `[[index]]`
- **THEN** the journal's link points to `index.html`

#### Scenario: Linking to the home page without index.md
- **WHEN** the notes root has no `index.md` and a journal contains `[[index]]`
- **THEN** the journal's link points to `index.html`, and no `pages/index.html` is written for it

#### Scenario: Conflicting pages/index.md
- **WHEN** the notes contain `pages/index.md`
- **THEN** `index.html` starts with a warning naming `pages/index.md` and linking to `pages/index.html`, which shows that file's content under the title `index (in pages)`
