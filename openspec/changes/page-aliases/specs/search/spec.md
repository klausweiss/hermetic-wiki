# Spec Delta

## MODIFIED Requirements

### Requirement: What is searchable
The search index SHALL contain every page's name and aliases and every block's content and property values (except `id`) of every page in the graph, including journal days. A page's aliases SHALL be searchable as names of the page: a query equal to an alias SHALL rank the page as an exact page-name match, and a page SHALL appear at most once among the results. Text inside code spans and fenced code SHALL be searchable. Markup syntax (`[[`, `]]`, `#` of tags, `((…))` of block references, markdown punctuation) SHALL NOT be indexed as words; the names inside page links and tags SHALL be. Linked references, related pages, and task lists (which repeat other pages' blocks) SHALL NOT be indexed again on the pages that show them.

#### Scenario: Words in a journal block
- **WHEN** a journal block reads `token refresh shipped for [[auth]]`
- **THEN** searching `refresh` finds that block, and searching `auth` finds it as well as the page `auth`

#### Scenario: No duplicates from linked references
- **WHEN** the block above is shown in `pages/auth.html` as a linked reference
- **THEN** searching `refresh` returns that block once, on its journal page

#### Scenario: Searching an alias
- **WHEN** the page `auth` declares `alias:: authentication`
- **THEN** searching `authentication` lists the page `auth`, shown with its own name and linking to `pages/auth.html`
