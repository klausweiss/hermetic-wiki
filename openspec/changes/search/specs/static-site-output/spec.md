# Spec Delta

## MODIFIED Requirements

### Requirement: Output layout
The output SHALL contain one HTML file per page in the graph, including implicit pages. A page backed by a file SHALL be written at its source path with `.md` replaced by `.html`; an implicit regular page at the encoded path of its name under `pages/`; an implicit journal page at `journals/YYYY-MM-DD.html`. Site-wide pages SHALL be `index.html` (home page), `journals.html` (journal list), `pages.html` (all pages), `tasks.html` (open tasks), and `search.html` (search results). Files belonging to hermetic itself SHALL be under `_hermetic/`. Continuation files SHALL be under `refs/` and `tasks/`. The search index SHALL be under `search/`.

#### Scenario: Pages, journals, and implicit pages
- **WHEN** the notes contain `journals/2026-10-12.md` linking to `[[kafka]]` and `[[2026-10-01]]`, and `pages/Project X/Auth Flow.md`
- **THEN** the output contains `journals/2026-10-12.html`, `journals/2026-10-01.html`, `pages/kafka.html`, `pages/Project X/Auth Flow.html`, and the implicit namespace page `pages/Project X.html`

#### Scenario: Encoded page names
- **WHEN** the notes contain `pages/C#%3A a.md`
- **THEN** the output contains `pages/C#%3A a.html`, and links to it use the href `C%23%253A%20a.html` relative to `pages/`

#### Scenario: Site-wide pages
- **WHEN** any notes directory is built
- **THEN** the output contains `index.html`, `journals.html`, `pages.html`, `tasks.html`, `search.html`, and `search/meta.json`

### Requirement: Bounded file sizes
Apart from the journal index and the all-pages list, which list only names, and the search index under `search/`, whose size is governed by the `search` capability, the size of every generated file SHALL depend only on the content of one page, its linked references and related blocks up to the pagination limit, or one page of tasks, and SHALL NOT grow with the total size of the wiki.

#### Scenario: Unrelated growth
- **WHEN** 1,000 journal days unrelated to page `auth` are added
- **THEN** the size of `pages/auth.html` does not change
