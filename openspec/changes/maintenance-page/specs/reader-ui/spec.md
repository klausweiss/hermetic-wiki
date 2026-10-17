# Spec Delta

## MODIFIED Requirements

### Requirement: Keyboard shortcuts
When the script runs, it SHALL provide these shortcuts, ignored while typing in a text field (a checkbox or button having the focus is not typing): `g` then `h` opens the home page, `g` then `j` the journal list, `g` then `p` all pages, `g` then `t` open tasks, `g` then `g` the graph view; `g` then `d` today's journal, where editing is available (see `browser-editing`); `/` opens the search dialog when search is available (see `search`); `Escape` closes the topmost sidebar pane; `\` shows or hides the sidebar; `?` shows a list of the shortcuts; `,` then `t` switches between the dark and light theme (the settings menu); `.` opens the menu of the actions the page offers, such as `.` then `a` and `.` then `d` on the unused-assets page (see `maintenance`), and does nothing on a page that offers none. A key menu's key followed by a pause SHALL show the menu with its actions and their keys. In addition, `Ctrl+K` (`Cmd+K` on macOS) SHALL open the search dialog when search is available, also while typing in a text field, and close it when it is open. On the graph page, `Ctrl+F` (`Cmd+F` on macOS) SHALL open the graph search (see `graph-view`).

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
- **THEN** the search dialog opens with focus in its field, and the `/` is not typed into it

#### Scenario: Focus search while typing
- **WHEN** the page is served over `http://`, focus is in a text field, and the user presses `Ctrl+K` (`Cmd+K` on macOS)
- **THEN** the search dialog opens with focus in its field and the browser's own binding for the key does not run

#### Scenario: Ctrl+K closes search
- **WHEN** the search dialog is open and the user presses `Ctrl+K` (`Cmd+K` on macOS)
- **THEN** the dialog closes

#### Scenario: Graph search in the shortcut list
- **WHEN** the user presses `?` on the graph page
- **THEN** the list includes `Ctrl+F` (`⌘ F` on macOS) for searching the graph, and on other pages it does not

#### Scenario: Go to today
- **WHEN** editing is available and the user presses `g` then `d`
- **THEN** today's journal day opens

#### Scenario: Settings menu
- **WHEN** the user presses `,` then `t` on a page shown in the light theme
- **THEN** the page switches to the dark theme

#### Scenario: Page menu
- **WHEN** the user presses `.` and waits on a page that offers no actions
- **THEN** no menu opens
