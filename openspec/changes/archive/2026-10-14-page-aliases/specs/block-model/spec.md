# Spec Delta

## ADDED Requirements

### Requirement: Page properties
When a page's first top-level block has properties and no display content (it consists only of property lines), its properties SHALL be the page's properties. A page whose first top-level block has display content, and an implicit page, SHALL have no page properties. The block SHALL remain an ordinary block of the page.

#### Scenario: Property lines at the top of a file
- **WHEN** `pages/auth.md` starts with `alias:: authentication` and `tags:: security`, followed by a blank line and `- first note`
- **THEN** the page `auth` has the page properties `alias` = `authentication` and `tags` = `security`

#### Scenario: Properties-only first bullet
- **WHEN** a page starts with `- alias:: authentication` followed by `- first note`
- **THEN** the page has the page property `alias` = `authentication`

#### Scenario: First block with content
- **WHEN** a page starts with `- decided to use JWT` followed by `  status:: accepted`
- **THEN** the page has no page properties and the block has the property `status`
