# reference-graph Specification

## Purpose

Defines how references between pages and blocks are recognized and resolved, and how backlinks, inherited references, implicit pages, and task views are derived from them, so that every page can show everything ever written about it.

## Requirements

### Requirement: Page links
`[[name]]` in a block's content SHALL be a reference from that block to the page `name`. Links SHALL NOT nest, SHALL NOT span lines, and SHALL NOT be recognized inside inline code spans, fenced code, or after a backslash escape (`\[[`).

#### Scenario: Link in a journal block
- **WHEN** a journal block contains `met with X about [[auth]]`
- **THEN** the block references the page `auth`

#### Scenario: Link inside inline code
- **WHEN** a block contains `` `[[auth]]` ``
- **THEN** no reference is recorded

### Requirement: Tags
`#name` SHALL be a reference to the page `name` when the `#` is at the start of a line or preceded by whitespace and followed by a non-whitespace character other than `#`. The name SHALL extend up to the next whitespace, excluding trailing `.`, `,`, `;`, `:`, `!`, `?`, `)`. `#[[multi word]]` SHALL reference the page `multi word`. A tag SHALL be equivalent to a page link.

#### Scenario: Simple tag
- **WHEN** a block contains `fixed the bug #backend.`
- **THEN** the block references the page `backend`

#### Scenario: Markdown heading and URL fragment
- **WHEN** a block contains `# Heading` or `https://example.com/a#section`
- **THEN** no tag reference is recorded

#### Scenario: Multi-word tag
- **WHEN** a block contains `#[[Project X]]`
- **THEN** the block references the page `Project X`

### Requirement: Markdown links
A markdown inline link `[text](destination)` (not an image) whose destination is a relative path ending in `.md`, optionally followed by `#fragment`, SHALL be a reference to the page stored at that path. The destination SHALL be percent-decoded once and then resolved relative to the directory of the linking file, or relative to the notes root when it starts with `/`; the resulting file path SHALL then be mapped to a page name using the file-name decoding of `source-layout`. Destinations with a URL scheme (e.g. `https:`, `mailto:`) SHALL be ignored. A destination that resolves outside `journals/` or `pages/` of the notes root, other than the root `index.md`, SHALL be reported as a diagnostic and not treated as a reference. A destination that resolves to a non-existent file SHALL be reported as a diagnostic, recorded as an unresolved reference, and SHALL NOT create an implicit page. The same code-span, fence, and escape exclusions as page links SHALL apply.

#### Scenario: Relative link from a journal
- **WHEN** `journals/2026-10-12.md` contains `[the auth notes](../pages/auth.md)` and `pages/auth.md` exists
- **THEN** the block references the page `auth`

#### Scenario: Link with encoded characters
- **WHEN** a page in `pages/` contains `[x](Project%20X/Auth%20Flow.md#setup)` and `pages/Project X/Auth Flow.md` exists
- **THEN** the block references the page `Project X/Auth Flow`

#### Scenario: Link to a missing file
- **WHEN** a block contains `[x](nope.md)` and no such file exists
- **THEN** a diagnostic is reported, the reference is recorded as unresolved, and no page `nope` is created

#### Scenario: Link escaping the notes root
- **WHEN** a block contains `[x](../../secrets.md)`
- **THEN** a diagnostic is reported and no reference is recorded

#### Scenario: External and image links
- **WHEN** a block contains `[site](https://example.com/a.md)` or `![diagram](pic.md)`
- **THEN** no reference is recorded

#### Scenario: Link to the home page
- **WHEN** `pages/auth.md` contains `[home](../index.md)` and the root `index.md` exists
- **THEN** the block references the page `index`

### Requirement: Block references
`((id))` SHALL be a reference from the containing block to the block with that id, and SHALL also count as a reference to the page containing the target block. The same code-span, fence, and escape exclusions as page links SHALL apply.

#### Scenario: Resolved block ref
- **WHEN** a block on page `redis` has `id:: auth-ttl-bug` and a journal block contains `((auth-ttl-bug))`
- **THEN** the journal block references that block and the page `redis`

### Requirement: Id uniqueness and dangling references
Block ids SHALL be unique across the graph. When an id appears on several blocks, the first in scan order SHALL own it and every occurrence SHALL be reported in one diagnostic. A `((id))` with no matching block SHALL be reported as a diagnostic and recorded as an unresolved reference so it can be displayed as broken.

#### Scenario: Duplicate id
- **WHEN** two blocks both have `id:: x`
- **THEN** a diagnostic names both locations and `((x))` resolves to the first in scan order

#### Scenario: Dangling block ref
- **WHEN** a block contains `((missing))` and no block has that id
- **THEN** a diagnostic is reported and the reference is recorded as unresolved

### Requirement: Implicit pages
A page that is referenced (or is a namespace prefix of such a page) but has no file SHALL exist in the graph as an implicit page with no own blocks, unless the reference resolves through an alias. A reference whose name is a valid `YYYY-MM-DD` date SHALL resolve to the journal page of that date, implicit if it has no file.

#### Scenario: Page created on demand
- **WHEN** a journal block contains `[[kafka]]` and no `pages/kafka.md` exists
- **THEN** the graph contains an implicit page `kafka` whose linked references include that block

#### Scenario: Link to a day
- **WHEN** a block contains `[[2026-10-01]]` and that journal file does not exist
- **THEN** it resolves to an implicit journal page dated 2026-10-01

#### Scenario: Namespaced alias
- **WHEN** `auth` declares `alias:: Project X/Login` and a block contains `[[Project X/Login]]`
- **THEN** the block references `auth`, `Project X/Login` does not exist as a page, and `Project X` exists as an implicit page

### Requirement: Direct and inherited references
Each block SHALL have direct references (those written in its own content and properties) and inherited references (its direct references plus the inherited references of its parent).

#### Scenario: Child inherits parent's reference
- **WHEN** a block `meeting about [[auth]]` has a child `TODO rotate signing keys`
- **THEN** the child's inherited references include `auth` and its direct references do not

### Requirement: Linked references
The linked references of a page P SHALL be the blocks on other pages that directly reference P or a block of P. A referencing block whose ancestor is also a linked reference of P SHALL be represented only within that ancestor. Each linked reference SHALL expose the block's subtree and its ancestor chain (breadcrumb). References from blocks on P itself SHALL be excluded.

#### Scenario: Breadcrumb and subtree
- **WHEN** a journal block `- standup` has a child `- [[auth]] token refresh shipped` which has a child `- details`
- **THEN** page `auth` lists the block `[[auth]] token refresh shipped` with breadcrumb `standup` and its child `details`

#### Scenario: Nested references collapse
- **WHEN** a block referencing `auth` has a descendant that also references `auth`
- **THEN** page `auth` lists only the ancestor block, with the descendant inside its subtree

#### Scenario: Self references excluded
- **WHEN** a block on page `auth` contains `[[auth]]`
- **THEN** that block is not a linked reference of `auth`

### Requirement: Linked reference ordering
Linked references of a page SHALL be grouped by source page. Groups from journal pages SHALL come first, ordered by date with the newest first, followed by groups from regular pages ordered alphabetically by case-insensitive display name. Within a group, blocks SHALL be in document order.

#### Scenario: Mixed sources
- **WHEN** `auth` is referenced from journals 2026-09-30 and 2026-10-12 and from pages `redis` and `architecture`
- **THEN** the groups are ordered 2026-10-12, 2026-09-30, `architecture`, `redis`

### Requirement: Page tasks
The tasks of a page P SHALL be all task blocks located on P plus all task blocks on other pages whose inherited references include P or a block of P. The system SHALL be able to list open tasks (TODO, DOING) per page and across the whole graph.

#### Scenario: Task under a referencing block
- **WHEN** a journal block `meeting about [[auth]]` has children `TODO rotate signing keys` and `DONE drop legacy endpoint`
- **THEN** both are tasks of `auth`, and only `rotate signing keys` is an open task

#### Scenario: Global open tasks
- **WHEN** the graph contains tasks in TODO, DOING, and DONE states across several pages
- **THEN** the global open-task list contains exactly the TODO and DOING tasks

### Requirement: Co-mentions
Two distinct pages SHALL be co-mentioned when a single block directly references both of them (by page link, tag, markdown link, or block reference to one of their blocks). Only a block's direct references SHALL count; inherited references and the page the block is located on SHALL NOT. A co-mention SHALL be undirected and SHALL record the blocks in which it occurs; its weight SHALL be the number of such blocks, and its recency SHALL be the newest journal date among them (if any). The system SHALL list, for each page, its co-mentioned pages ordered by weight (highest first), then recency (newest first), then case-insensitive display name.

#### Scenario: Two pages in one block
- **WHEN** a journal block contains `[[claude]] helped me design a proposal for [[hermetic]]`
- **THEN** `claude` and `hermetic` are co-mentioned with weight 1, and each lists the other as related

#### Scenario: Repeated mentions in one block count once
- **WHEN** a block contains `[[a]] and [[a]] with [[b]]`
- **THEN** `a` and `b` are co-mentioned with weight 1 and `a` is not co-mentioned with itself

#### Scenario: Child blocks do not inherit co-mentions
- **WHEN** a block `[[claude]] helped with [[hermetic]]` has a child `it suggested [[rust]]`
- **THEN** `rust` is not co-mentioned with `claude` or `hermetic`

#### Scenario: Containing page does not count
- **WHEN** a block on page `hermetic` contains `[[claude]]` only
- **THEN** no co-mention is recorded

#### Scenario: Ordering
- **WHEN** `claude` is co-mentioned with `hermetic` in 3 blocks and with `rust` in 1 block
- **THEN** the related pages of `claude` are listed as `hermetic` (3) before `rust` (1)

### Requirement: Namespace children
The graph SHALL expose, for each page, the pages that are its direct namespace children. It SHALL also list under a page each page that declares an alias directly in its namespace (`alias:: <page>/x`), named by the alias's last segment, together with the children and ordered by key; that page's own namespace parent SHALL be unchanged. The namespace prefixes of an alias SHALL exist as pages, implicit if they have no file. When an implicit page has the alias's name, only that page SHALL be listed.

#### Scenario: Listing children
- **WHEN** pages `Project X/Auth Flow` and `Project X/Billing` exist
- **THEN** the namespace children of `Project X` are `Auth Flow` and `Billing`

#### Scenario: Listing a namespaced alias
- **WHEN** `pages/emanote.md` declares `alias:: alternatives/emanote`
- **THEN** `alternatives` lists `emanote` linking to the page `emanote`, and `emanote` has no namespace parent

### Requirement: Page aliases
The page property `alias` SHALL give the page additional names. Its value SHALL be split on commas; each part, with surrounding whitespace removed, SHALL be an alias, written either as a bare name or as a single page link `[[name]]`. Empty parts SHALL be ignored. Alias names SHALL be normalized and validated like page names and matched case-insensitively. A page link or tag whose name matches an alias SHALL reference the page that declares it, and SHALL count as a reference to that page everywhere (linked references, tasks, co-mentions). A reference through an alias SHALL NOT create a page. An implicit page with an alias's name SHALL exist only as the namespace prefix of another referenced name (`[[login/x]]`); page links and tags naming it still resolve to the page that declares the alias. Aliases SHALL NOT apply to markdown links, which name files. Page links and tags in the `alias` property value SHALL NOT be references of the block.

An alias SHALL be ignored, with a diagnostic naming the declaring file and line, when:
- it is invalid as a page name;
- it is a valid `YYYY-MM-DD` date;
- a page with that name has a source file, or the name is a namespace prefix of such a page;
- an earlier page in scan order already declares it; the diagnostic SHALL name both pages.

An alias equal to the declaring page's own name SHALL be ignored without a diagnostic. An alias SHALL always name a page that has a source file, so aliases never chain.

#### Scenario: Link through an alias
- **WHEN** `pages/auth.md` starts with `alias:: authentication, [[login]]` and a journal block contains `fixed [[authentication]] and #login`
- **THEN** the journal block references the page `auth` (once), it is listed in the linked references of `auth`, and no page `authentication` or `login` exists

#### Scenario: Case-insensitive
- **WHEN** `auth` declares `alias:: Authentication` and a block contains `[[authentication]]`
- **THEN** the block references the page `auth`

#### Scenario: Alias property is not a reference
- **WHEN** `auth` declares `alias:: [[login]]`
- **THEN** the alias block has no references and `auth` has no linked reference from its own alias

#### Scenario: Alias of an existing page
- **WHEN** `pages/auth.md` declares `alias:: redis` and `pages/redis.md` exists
- **THEN** a diagnostic is reported, and `[[redis]]` references the page `redis`

#### Scenario: Alias claimed twice
- **WHEN** `pages/auth.md` and `pages/login.md` both declare `alias:: sso`
- **THEN** a diagnostic names both files, and `[[sso]]` references the page that comes first in scan order

#### Scenario: Date alias
- **WHEN** a page declares `alias:: 2026-10-01`
- **THEN** a diagnostic is reported and `[[2026-10-01]]` references the journal page of that day

#### Scenario: Alias not in the first block
- **WHEN** a page's first block is `- intro` and a later block has `alias:: other`
- **THEN** `other` is not an alias and `[[other]]` references an implicit page `other`

#### Scenario: Markdown link to an alias name
- **WHEN** `auth` declares `alias:: login`, no `pages/login.md` exists, and a block contains `[x](login.md)`
- **THEN** the link is reported as pointing to a missing file and recorded as unresolved
