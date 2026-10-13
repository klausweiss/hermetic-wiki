# Spec Delta

## MODIFIED Requirements

### Requirement: Keyboard shortcuts
When the script runs, it SHALL provide these shortcuts, ignored while typing in a text field: `g` then `h` opens the home page, `g` then `j` the journal list, `g` then `p` all pages, `g` then `t` open tasks, `g` then `g` the graph view; `g` then `d` today's journal, where editing is available (see `browser-editing`); `/` focuses the search box when it is shown; `Escape` closes the topmost sidebar pane; `\` shows or hides the sidebar; `?` shows a list of the shortcuts. In addition, `Ctrl+K` (`Cmd+K` on macOS) SHALL focus the search box when it is shown, also while typing in a text field.

#### Scenario: Go to journals
- **WHEN** the user presses `g` then `j` on any page
- **THEN** the journal list page opens

#### Scenario: Go to the graph
- **WHEN** the user presses `g` then `g` on any page
- **THEN** `graph.html` opens, and the shortcut list shown by `?` includes `g g`

#### Scenario: Typing is not intercepted
- **WHEN** focus is in a text input and the user types `g j`
- **THEN** the characters are typed and no navigation happens

#### Scenario: Focus search
- **WHEN** the page is served over `http://` and the user presses `/`
- **THEN** the search box receives focus and the `/` is not typed into it

#### Scenario: Focus search while typing
- **WHEN** the page is served over `http://`, focus is in a text field, and the user presses `Ctrl+K` (`Cmd+K` on macOS)
- **THEN** the search box receives focus and the browser's own binding for the key does not run

#### Scenario: Go to today
- **WHEN** editing is available and the user presses `g` then `d`
- **THEN** today's journal day opens
