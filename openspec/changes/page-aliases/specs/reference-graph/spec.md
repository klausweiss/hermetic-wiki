# Spec Delta

## ADDED Requirements

### Requirement: Page aliases
The page property `alias` SHALL give the page additional names. Its value SHALL be split on commas; each part, with surrounding whitespace removed, SHALL be an alias, written either as a bare name or as a single page link `[[name]]`. Empty parts SHALL be ignored. Alias names SHALL be normalized and validated like page names and matched case-insensitively. A page link or tag whose name matches an alias SHALL reference the page that declares it, and SHALL count as a reference to that page everywhere (linked references, tasks, co-mentions). No page SHALL exist for an alias name. Aliases SHALL NOT apply to markdown links, which name files. Page links and tags in the `alias` property value SHALL NOT be references of the block.

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

## MODIFIED Requirements

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
- **THEN** the block references `auth`, and neither `Project X/Login` nor `Project X` exists as a page
