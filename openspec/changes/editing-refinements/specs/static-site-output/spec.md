# Spec Delta

## MODIFIED Requirements

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
