# Spec Delta

## MODIFIED Requirements

### Requirement: Source directory conventions
The system SHALL treat a given directory as the notes root with no configuration file. Markdown files (`.md`) under `journals/` SHALL be journal pages and markdown files under `pages/` (at any depth) SHALL be regular pages. A file named `index.md` directly in the notes root SHALL be the regular page `index`, the site's home page. The name `index` SHALL be reserved for it: a file under `pages/` whose name would be `index` (such as `pages/index.md`) SHALL instead be the page `index (in pages)`, and a diagnostic SHALL report the conflict. All other files outside these two directories, non-markdown files, and any file or directory whose name begins with `.` SHALL be ignored.

#### Scenario: Journals and pages are discovered
- **WHEN** the root contains `journals/2026-10-12.md` and `pages/auth.md`
- **THEN** the graph contains a journal page for 2026-10-12 and a regular page named `auth`

#### Scenario: Unrelated files are ignored
- **WHEN** the root contains `README.md`, `pages/diagram.png`, and `pages/.drafts/x.md`
- **THEN** none of them become pages and no diagnostic is reported for them

#### Scenario: Empty or missing directories
- **WHEN** the root has no `journals/` or no `pages/` directory
- **THEN** the graph is built from whatever exists without error

#### Scenario: Home page source
- **WHEN** the root contains `index.md`
- **THEN** the graph contains the regular page `index` backed by that file, and it can be linked as `[[index]]`

#### Scenario: Root index and pages index
- **WHEN** the root contains both `index.md` and `pages/index.md`
- **THEN** `index.md` is the page `index`, `pages/index.md` is the page `index (in pages)`, and a diagnostic names `pages/index.md`

#### Scenario: Pages index alone
- **WHEN** the root contains `pages/index.md` and no `index.md`
- **THEN** `pages/index.md` is the page `index (in pages)`, not the home page, and a diagnostic names it
