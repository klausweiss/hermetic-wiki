# Spec Delta

## MODIFIED Requirements

### Requirement: Where editing is available
Editing SHALL be available only on pages served by `hermetic serve` to a browser on the same machine: the request comes from a loopback address and its `Host` header names `localhost`, `127.0.0.1`, or `[::1]`. Only then SHALL the served HTML contain the editor element. Editable pages SHALL be journal days (with or without a file), regular pages (with or without a file), and the home page when the notes root contains `index.md`. The journal list, all-pages list, open-tasks page, search page, and continuation files SHALL NOT be editable. Within an editable page, only the page's own blocks SHALL be editable; linked references, related pages, sidebar panes, and the timeline SHALL stay read-only. The one exception is task labels in the task lists (the open-tasks page and each page's "Open tasks" section), which change the task's state as "Task labels" requires, also on the open-tasks page, which is otherwise not editable. Without JavaScript, and in built sites, pages SHALL be exactly as before.

#### Scenario: Same machine
- **WHEN** the author opens `http://127.0.0.1:8000/pages/auth.html` from `hermetic serve` on the same machine
- **THEN** clicking a block of the page edits it

#### Scenario: Another device
- **WHEN** a phone opens `http://192.168.1.23:8000/pages/auth.html` from `hermetic serve --public`
- **THEN** the page has no editor element and nothing on it is editable

#### Scenario: Built site
- **WHEN** a site produced by `hermetic build` is opened
- **THEN** it contains no editor element and no editing controls

## ADDED Requirements

### Requirement: Task labels
Where editing is available, clicking a task's label (`TODO`, `DOING`, or `DONE`) SHALL move the task to its next state, TODO → DOING → DONE → TODO, and save the change to the task block's own note file, changing nothing else in it. This SHALL work on the page's own blocks (also while another block is being edited) and on the task lists: the open-tasks page with its continuation pages, and each page's "Open tasks" section. The label SHALL show the new state at once. Labels that work SHALL be focusable and react to Enter and Space like a click; labels elsewhere (linked references, related pages, sidebar panes, the timeline, built sites, other devices) SHALL stay plain text. When the task's file changed since the list was built so that the task cannot be found, nothing SHALL be written and the label SHALL keep its state.

#### Scenario: Cycle on the page
- **WHEN** the page `auth` has the block `- TODO rotate signing keys` and the author clicks its `TODO` label twice
- **THEN** `pages/auth.md` contains `- DONE rotate signing keys` and the label shows `DONE`

#### Scenario: Done wraps around
- **WHEN** the author clicks the `DONE` label of a block
- **THEN** the block is `TODO` again

#### Scenario: Tick off from the open-tasks page
- **WHEN** `tasks.html` lists `TODO rotate signing keys` from page `auth` and the author clicks its label
- **THEN** `pages/auth.md` contains `- DOING rotate signing keys`, and nothing else in it changed

#### Scenario: Read-only elsewhere
- **WHEN** a task appears in another page's linked references
- **THEN** its label there is not a control and clicking it changes nothing

#### Scenario: Keyboard
- **WHEN** the author moves focus to a task label on `tasks.html` with Tab and presses Enter
- **THEN** the task moves to its next state
