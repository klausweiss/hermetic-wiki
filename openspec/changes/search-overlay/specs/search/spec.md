# Spec Delta

## MODIFIED Requirements

### Requirement: Search box and results page
When the script runs over `http:` or `https:`, the header SHALL contain a search button: an icon showing a magnifier, with an accessible name naming search and its shortcut. Activating it SHALL open a search dialog over the page, placed in the upper part of the window and no wider than about 40rem, holding a search field with focus and, below it, the top results (at most 8), updating as the reader types. The arrow keys SHALL move through the results and, after the last one, the link to all results, and Enter SHALL open the selected one; Enter without a selection SHALL open `search.html?q=<query>`, which lists all results (up to 200) and can be bookmarked or shared. Opening a result, pressing Escape, or clicking outside the dialog SHALL close it and return focus to where it was before the dialog opened; the rest of the page SHALL NOT be reachable with the keyboard while the dialog is open. On `search.html`, the field SHALL start with the page's query. The dialog SHALL fit windows from 320px wide, using the full width on narrow screens. Without JavaScript or over `file:`, neither the button nor the dialog SHALL be shown, and `search.html` SHALL explain that search needs JavaScript and the site to be served over `http://`.

#### Scenario: Search button in the header
- **WHEN** a page is served over `http://` with the script running
- **THEN** the header shows a magnifier button where the search box was, and no text field

#### Scenario: Results as you type
- **WHEN** the reader opens the search dialog and types `kaf`
- **THEN** up to 8 results appear under the field without a page load

#### Scenario: All results with the keyboard
- **WHEN** the dialog shows 8 results and the reader presses the down arrow 9 times, then Enter
- **THEN** the ninth press selects the link to all results, and Enter opens `search.html?q=<query>`

#### Scenario: Full results page
- **WHEN** the reader types `kafka` in the search dialog and presses Enter without selecting a result
- **THEN** `search.html?q=kafka` opens and lists all matching results

#### Scenario: Closing the dialog
- **WHEN** the search dialog is open and the reader presses Escape or clicks outside it
- **THEN** the dialog closes and focus returns to the element that had it before

#### Scenario: Refining a search
- **WHEN** `search.html?q=kafka` is shown and the reader opens the search dialog
- **THEN** the field contains `kafka`, selected

#### Scenario: Phone width
- **WHEN** the search dialog is opened in a 360px-wide window
- **THEN** it spans the window width without horizontal scrolling and its results scroll within it

#### Scenario: Opened from disk
- **WHEN** a page is opened via `file://`
- **THEN** no search button is shown, and opening `search.html` shows the explanation
