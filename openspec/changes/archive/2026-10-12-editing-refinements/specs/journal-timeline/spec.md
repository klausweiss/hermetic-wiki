# Spec Delta

## MODIFIED Requirements

### Requirement: Timeline view
When the script is active on the journal list page, it SHALL offer a toggle between the timeline view (default) and the list. The timeline SHALL show the full content of journal days that have a source file, newest first, each headed by a link to its day page, with the day's own blocks and without its linked references, related pages, or open tasks sections. Choosing the list SHALL be reflected in the URL so that reloading or sharing the URL keeps it.

#### Scenario: Switch to timeline
- **WHEN** the list is shown on `journals.html` and the user selects the timeline view
- **THEN** the newest journal days are shown with their full content, newest first

#### Scenario: Timeline first
- **WHEN** the user opens `journals.html`
- **THEN** the newest journal days are shown with their full content, newest first

#### Scenario: Reload keeps the view
- **WHEN** the list is selected and the page is reloaded
- **THEN** the list is shown
