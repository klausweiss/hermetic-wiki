# journal-timeline Specification

## Purpose

Lets a reader scroll through journal days as one continuous timeline, assembled in the browser from the per-day pages, as a separate view next to the journal list.

## Requirements

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

### Requirement: Loading as you scroll
The timeline SHALL initially load a small batch of days and load the next older batch when the reader scrolls near its end, until all days are shown. Each day SHALL be fetched from its own page; no other file listing all journal content SHALL be needed.

#### Scenario: Scrolling loads older days
- **WHEN** the site has 400 journal days and the user scrolls to the end of the timeline
- **THEN** older days are appended without a page reload, and the first load fetched no more than 10 day pages

#### Scenario: End of the journal
- **WHEN** the oldest day has been loaded
- **THEN** no further requests are made and the timeline shows that it has reached the beginning

### Requirement: Timeline links and panes
Links and Shift+click inside the timeline SHALL behave as on any page: plain clicks navigate, Shift+click opens a sidebar pane.

#### Scenario: Open a topic from the timeline
- **WHEN** the user Shift+clicks `[[auth]]` inside a day in the timeline
- **THEN** the `auth` page opens in the sidebar and the timeline stays in place

### Requirement: Journal calendar
The timeline view SHALL start with a calendar of the months that have journal days with a source file, and the list view SHALL NOT show it. Each month SHALL be a small grid of its days, weeks starting on Monday, where days with an entry are highlighted links to their day pages and today is marked. Months without entries between two months with entries SHALL NOT be drawn; each run of them SHALL be shown as one gap marker stating how long it lasted. The calendar SHALL stay one row of months high however many months the journal spans, oldest on the left, scrolling sideways, and SHALL initially show the newest month.

#### Scenario: Navigate from the calendar
- **WHEN** the user clicks a highlighted day in the calendar
- **THEN** that day's page opens

#### Scenario: Gap between months
- **WHEN** the journal has entries in June 2025 and August 2026 and none in between
- **THEN** the calendar shows June 2025, then a marker for 1 year 1 month without entries, then August 2026

#### Scenario: Long-running journal
- **WHEN** the journal spans ten years
- **THEN** the calendar is as tall as for one month, and shows the newest month without scrolling
