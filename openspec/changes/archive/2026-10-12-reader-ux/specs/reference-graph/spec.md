# Spec Delta

## MODIFIED Requirements

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
