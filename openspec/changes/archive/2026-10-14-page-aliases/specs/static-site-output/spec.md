# Spec Delta

## MODIFIED Requirements

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
