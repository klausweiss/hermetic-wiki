# browser-editing Specification

## Purpose

Lets the author write the notes in the browser, in a Logseq-style block outliner on the pages they read, while `hermetic serve` keeps the markdown files as the only source of truth and never lets other websites or devices change them.

## Requirements

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

### Requirement: Editing a block
Clicking anywhere in a block's row (outside its links and controls), also for an empty block, SHALL replace the block's rendered content with a text field holding the block's markdown as written: its first line without the bullet marker, its further lines without the block's indentation, and its property lines. Clicking a link in a block that is not being edited SHALL follow the link. Exactly one block SHALL be edited at a time. Pressing Escape or clicking outside the block SHALL end editing and show the block rendered again, as the next build renders it. An editable page with no blocks SHALL offer one empty block to start writing in. Opening a page SHALL NOT start editing (except as "Creating journal days and pages" says for "Today"); pressing Return while no block is edited and no other control has the focus SHALL start editing the page's last block in document order, with the cursor at its end, or the starter block of an empty page.

#### Scenario: Raw text
- **WHEN** the author clicks the block `- decided to use [[JWT]]` followed by `  status:: accepted`
- **THEN** a text field shows `decided to use [[JWT]]` and `status:: accepted` on two lines

#### Scenario: Links still navigate
- **WHEN** the author clicks the `JWT` link inside a block that is not being edited
- **THEN** the browser opens the page `JWT`

#### Scenario: Empty page
- **WHEN** the author opens the page of a name that is only referenced (it has no file)
- **THEN** the page offers an empty block, and typing in it starts the page

#### Scenario: No edit mode on arrival
- **WHEN** the author opens a page that has blocks
- **THEN** no block is being edited until a block is clicked or Return is pressed

#### Scenario: Return starts editing at the end
- **WHEN** the author opens `journals/2026-10-12.html`, whose last block is `- standup notes`, and presses Return
- **THEN** that block is being edited with the cursor after `notes`

#### Scenario: Empty block
- **WHEN** a page's only block is `- ` and the author leaves editing, then clicks the block's row
- **THEN** the empty block is being edited again

### Requirement: Outliner keys
While a block is being edited:

- Enter SHALL split the block at the cursor. The text after the cursor becomes a new block directly below, as the first child when the block has children, otherwise as the next sibling, and editing moves to it. Inside an unclosed fenced code block, Enter SHALL insert a line break instead.
- Shift+Enter SHALL insert a line break.
- Tab SHALL make the block, with its children, the last child of its previous sibling; without a previous sibling it SHALL do nothing.
- Shift+Tab SHALL make the block, with its children, the next sibling of its parent; its later siblings SHALL stay children of the former parent. For a top-level block it SHALL do nothing.
- Alt+Shift+Up and Alt+Shift+Down SHALL swap the block, with its children, with its previous or next sibling.
- Backspace with the cursor at the start of the block SHALL delete the block if it is empty and has no children. Otherwise, if the previous block in document order has no children, it SHALL append the block's text to that block. In both cases editing continues at the join point in the previous block.
- Up on the first line and Down on the last line SHALL move editing to the previous or next block in document order.
- Ctrl+Enter (Cmd+Enter on macOS) SHALL cycle the block's task marker: none, `TODO`, `DOING`, `DONE`, none.

Text outside bullets keeps its form. New blocks SHALL always be bullet blocks, and Tab and Shift+Tab SHALL do nothing on a block that is not a bullet. When editing is available, the shortcuts dialog (`?`) SHALL also list these keys.

#### Scenario: New sibling
- **WHEN** the author edits `- a` (no children), puts the cursor at the end, presses Enter, and types `b`
- **THEN** the file contains `- a` followed by `- b` at the same indentation

#### Scenario: Split in the middle
- **WHEN** the author edits `- first second` with the cursor before `second` and presses Enter
- **THEN** the blocks `first` and `second` follow each other and editing continues at the start of `second`

#### Scenario: Indent and outdent
- **WHEN** the page is `- a`, `- b`, `- c` and the author presses Tab in `b`, then Shift+Tab in `b`
- **THEN** after Tab `b` is a child of `a`, and after Shift+Tab the page is again `a`, `b`, `c` as siblings

#### Scenario: Outdent keeps later siblings
- **WHEN** `a` has children `b` and `c` and the author presses Shift+Tab in `b`
- **THEN** `b` is a sibling after `a`, and `c` is still a child of `a`

#### Scenario: Enter inside a code fence
- **WHEN** the author edits a block containing an opened but not yet closed ```` ``` ```` fence and presses Enter inside it
- **THEN** a line break is inserted and no new block is created

#### Scenario: Join with previous
- **WHEN** the page is `- one`, `- two` and the author presses Backspace at the start of `two`
- **THEN** the page has one block `onetwo` and editing continues between `one` and `two`

#### Scenario: Cycle task
- **WHEN** the author presses Ctrl+Enter twice in the block `write tests`
- **THEN** the block is `DOING write tests`

### Requirement: Formatting keys
While a block is being edited, these keys (Cmd instead of Ctrl on macOS) SHALL toggle inline markdown formatting: Ctrl+B bold (`**`), Ctrl+I italic (`*`), Ctrl+Shift+S strikethrough (`~~`), and Ctrl+E inline code (`` ` ``). With text selected, the key SHALL wrap the selection in the markers and keep it selected; when the selection is already wrapped in those markers (just outside it, or as its first and last characters), the key SHALL remove them instead. Without a selection, the key SHALL insert a pair of markers with the cursor between them, or remove an empty pair the cursor sits in. Bold and italic SHALL be told apart by the run of `*` characters, so italic never removes half of a bold marker. The browser's own undo SHALL be able to revert each change. The keys SHALL take precedence over the browser's own bindings for them, and the shortcuts dialog SHALL list them.

#### Scenario: Bold a selection
- **WHEN** the author selects `world` in `hello world` and presses Ctrl+B
- **THEN** the text is `hello **world**` and `world` is still selected

#### Scenario: Toggle off
- **WHEN** the author selects `world` in `hello **world**` and presses Ctrl+B
- **THEN** the text is `hello world`

#### Scenario: Italic inside bold
- **WHEN** the author selects `world` in `hello **world**` and presses Ctrl+I
- **THEN** the text is `hello ***world***`

#### Scenario: No selection
- **WHEN** the cursor is at the end of `note ` and the author presses Ctrl+B, types `x`, and presses Ctrl+B again on an empty pair elsewhere
- **THEN** the first press gives `note ****` with the cursor in the middle, typing gives `note **x**`, and Ctrl+B in an empty pair `****` removes it

#### Scenario: Undo
- **WHEN** the author presses Ctrl+B on a selection and then Ctrl+Z
- **THEN** the text is as before the Ctrl+B

### Requirement: Saving
Edits SHALL be saved to the page's markdown file without an explicit save action: when editing of a block ends, after every outliner key that changes the block structure, and 1 second after the last change while typing. Saves of one page SHALL happen one at a time, each including all changes made before it started. The page SHALL show whether all changes are saved, being saved, or not saved. When a save fails because the server cannot be reached, the editor SHALL keep the changes, retry when the server is reachable again, and the browser SHALL warn before leaving the page with unsaved changes. A saved change SHALL be visible on every page it affects within 2 seconds for a notes directory of about 5,000 pages.

#### Scenario: Save while typing
- **WHEN** the author types in a block and stops for 1 second
- **THEN** the file contains the new text and the page shows that everything is saved

#### Scenario: Server stopped
- **WHEN** the server is stopped while the author types, and started again
- **THEN** the page shows the changes as not saved, then saves them after the server is back, without losing text

#### Scenario: Backlinks follow
- **WHEN** the author adds `[[redis]]` to a journal block and editing ends
- **THEN** within 2 seconds `pages/redis.html` lists that block among its linked references

### Requirement: Lossless saving
A save SHALL change only the lines of blocks that were edited, added, moved, or deleted; every other byte of the file SHALL stay as it was. Added and re-indented blocks SHALL use the file's existing child indentation (a tab or a number of spaces, taken from the first indented bullet in the file), or two spaces when the file has none. Continuation lines SHALL be indented to the block's content column. The file's line endings (LF or CRLF) SHALL be kept, and a saved file SHALL end with a line ending. Text that the block rules split into several blocks (for example a line starting with `- ` typed into a block) SHALL become those blocks after the save, and the editor SHALL then show them as such.

#### Scenario: Untouched blocks
- **WHEN** a file has irregular indentation, blank lines between blocks, a preamble paragraph, and CRLF line endings, and the author edits one block's text
- **THEN** the only bytes that differ in the file are in that block's lines

#### Scenario: Tab-indented file
- **WHEN** a file indents children with tabs and the author indents a block with Tab
- **THEN** the block's bullet line is indented with tabs

#### Scenario: Typed bullet
- **WHEN** the author edits `- a`, adds a line break and the text `- b`, and editing ends
- **THEN** the page has blocks `a` and `b`

### Requirement: Creating journal days and pages
When editing is available, every page SHALL show a "Today" control in the header. It, and the keys `g` then `d`, SHALL open the journal day of the browser's current date, first creating `journals/YYYY-MM-DD.md` with one empty block if the day has no file. Editing SHALL start in its first block only when the day has no content (no block with text); otherwise the day opens without editing. Saving a page that has no file, such as one that is only referenced, SHALL create its file at the path given by the `source-layout` name encoding (journal days at `journals/YYYY-MM-DD.md`), creating namespace directories as needed. Existing files SHALL never be replaced by a create. The page `index` is the home page: its file is the root `index.md`, never `pages/index.md`. When the notes root has no `index.md`, the home page SHALL offer "Write a home page" instead of its hint about `index.md`; choosing it SHALL create the root `index.md` with one empty block and start editing it there.

#### Scenario: Today without a file
- **WHEN** the author clicks "Today" on 2026-10-13 and `journals/2026-10-13.md` does not exist
- **THEN** the file is created, the journal day opens, and its first block is being edited

#### Scenario: Today with content
- **WHEN** `journals/2026-10-13.md` contains `- standup notes` and the author presses `g` then `d`
- **THEN** the journal day opens and no block is being edited

#### Scenario: New page from a link
- **WHEN** the author writes `[[Project X/Auth]]` in a journal, follows the link, and types `- first note` on the new page
- **THEN** `pages/Project X/Auth.md` exists and contains `- first note`

#### Scenario: Write a home page
- **WHEN** the notes root has no `index.md` and the author chooses "Write a home page" on `index.html` and types `welcome`
- **THEN** the root `index.md` contains `- welcome`, `index.html` shows it, and no `pages/index.md` exists

### Requirement: Page and block autocomplete
Typing `[[` in a block being edited SHALL show a list of pages matching the text typed after it, ranked by the search engine's page-name matching. The list SHALL also offer a new page with exactly the typed name when no page has that name. Choosing an entry SHALL complete the link as `[[name]]`. Typing `((` SHALL show a list of blocks matching the typed text, ranked by the search engine. Choosing one SHALL insert `((id))` with the block's id, first giving the chosen block a generated id if it has none. Arrow keys SHALL move through the list, Enter or Tab SHALL choose, and Escape SHALL close it and keep the typed text.

#### Scenario: Link to an existing page
- **WHEN** the author types `[[kaf` and the page `kafka` exists
- **THEN** `kafka` is offered first, and choosing it leaves `[[kafka]]` in the text

#### Scenario: New page name
- **WHEN** the author types `[[Zeppelin plan` and no such page exists
- **THEN** a new page `Zeppelin plan` is offered, and choosing it leaves `[[Zeppelin plan]]` in the text

#### Scenario: Block reference by search
- **WHEN** the author types `((rotate keys` and chooses the block `TODO rotate signing keys` on page `auth`, which has no id
- **THEN** that block in `pages/auth.md` gains an `id::` property, and the edited block contains `((<that id>))`

### Requirement: Block references
When editing is available, a block's bullet SHALL offer "Copy reference". It SHALL put `((id))` on the clipboard, first adding an `id::` property line directly after the block's first line (after its other property lines) when the block has no id. A generated id SHALL be 8 characters from `a`–`z` and `0`–`9`. Adding an id SHALL change no other line of the file.

#### Scenario: Block without id
- **WHEN** the author chooses "Copy reference" on `- decided to use JWT`, which has no properties
- **THEN** the file has `id:: ` and 8 such characters on the line after it, indented to the block's content column, and the clipboard holds `((` that id `))`

#### Scenario: Block with id
- **WHEN** the author chooses "Copy reference" on a block with `id:: auth-ttl-bug`
- **THEN** the file is unchanged and the clipboard holds `((auth-ttl-bug))`

### Requirement: Conflicts
A save SHALL succeed only if the file is unchanged since the editor last loaded it. When it changed (for example in another editor or another tab), the save SHALL be refused and the file left as it is. The editor SHALL then load the file's current content, show that the page changed elsewhere, and keep the text of the refused change available to copy. When the page's file changes on disk while no block of it has unsaved changes, the editor SHALL load the new content without notice.

#### Scenario: Changed in $EDITOR
- **WHEN** the author edits a block in the browser, the same file is saved in another editor before the browser saves, and the browser then saves
- **THEN** the file keeps the other editor's content, and the page shows the current content, a notice, and the refused text

#### Scenario: External change while idle
- **WHEN** a page is open in the browser with nothing unsaved and its file is changed in another editor
- **THEN** the page shows the new content and the next edit saves without a conflict

### Requirement: Page updates while editing
A page open for editing SHALL NOT reload when its own content changes. Its rendered blocks, linked references, and other sections SHALL be updated in place from the new build. The block being edited, its text, and the cursor position SHALL be kept.

#### Scenario: Own save
- **WHEN** the author's save of `journals/2026-10-12.md` is rebuilt while they keep typing in a block
- **THEN** the page does not reload, and the text field and cursor stay as they are

### Requirement: Edit API
The server SHALL offer one page's source at `/_hermetic/source?path=<site path>`, where `<site path>` is the page's HTML path relative to the site root (for example `pages/Project%20X/Auth.html`; the home page is `index.html`).

`GET` SHALL return, as JSON, whether the page has a file, a version identifying the file's current bytes, the file's text, and its blocks in document order. Each block SHALL have its line range, depth, whether it is a bullet, and its anchor on the page. A page without a file SHALL have empty text and the version of a missing file.

`PUT` SHALL replace the file with the request body. It SHALL be accepted only when the request names the version it is based on and that version is the file's current one; otherwise the response SHALL be 409 with the current version and text. The response to a successful `PUT` SHALL be sent after the site has been rebuilt with the change, and SHALL give the new version and blocks.

`DELETE` SHALL delete the page's file as "Deleting pages" requires, accepted only when the request names the version it is based on and that version is the file's current one (otherwise 409 with the current version and text); a page without a file SHALL get 404. Its response SHALL be sent after the site has been rebuilt without the page.

The server SHALL answer 404 for a site path that is not an editable page, 400 for a body that is not UTF-8 text, and 413 for a body larger than 8 MiB.

#### Scenario: Read a page
- **WHEN** the editor requests `GET /_hermetic/source?path=pages/auth.html`
- **THEN** the response contains the exact text of `pages/auth.md`, its version, and one entry per block with its anchor (`b1`, `b2`, … or `id-<id>`)

#### Scenario: Stale version
- **WHEN** a `PUT` names a version that is no longer the file's current version
- **THEN** the response is 409 with the current version and text, and the file is unchanged

#### Scenario: Not editable
- **WHEN** a `PUT` targets `path=tasks.html` or `path=refs/pages/kafka/2.html`
- **THEN** the response is 404 and nothing is written

#### Scenario: Delete with a stale version
- **WHEN** a `DELETE` names a version that is no longer the file's current version
- **THEN** the response is 409 and the file is not moved

### Requirement: Request authenticity
The server SHALL answer every request to the edit API with 403, and change nothing, unless all of these hold: it comes from a loopback address; its `Host` header names `localhost`, `127.0.0.1`, or `[::1]` with the server's port; and, for `PUT` and `DELETE`, it carries an `Origin` header equal to `http://` followed by that `Host`, and the header `X-Hermetic-Edit: 1`. This SHALL hold for every binding, including `--host` and `--public`. The edit API SHALL send no CORS headers.

#### Scenario: Another website
- **WHEN** a page from `https://evil.example` in the author's browser sends a `PUT` to `http://127.0.0.1:8000/_hermetic/source?path=pages/auth.html`
- **THEN** the request is refused (by the browser for lack of CORS, or with 403 by the server) and `pages/auth.md` is unchanged

#### Scenario: DNS rebinding
- **WHEN** a request arrives with `Host: evil.example:8000` and a matching `Origin`
- **THEN** the response is 403, also when the server runs with `--public`

#### Scenario: Other device
- **WHEN** a device on the network sends a well-formed `PUT` to a server started with `--public`
- **THEN** the response is 403 and nothing is written

#### Scenario: Cross-site delete
- **WHEN** a page from another website sends `DELETE` to the edit API
- **THEN** the request is refused and no file is moved

### Requirement: Safe writes
The file written for a page SHALL be the page's existing source file or, for a page without one, the path derived from its name by the `source-layout` rules. It SHALL never be a path taken from the request. Before writing, the server SHALL verify that the target, with symbolic links resolved, is inside the notes directory under `journals/` or `pages/` (or is the root `index.md`), and SHALL refuse with 403 otherwise. A deletion SHALL move the file, after the same checks, to `.trash/<timestamp>/<original relative path>` in the notes folder, never outside it, and SHALL remove directories under `pages/` that this leaves empty (never `pages/` or `journals/` themselves). When `.trash/.gitignore` does not exist, the deletion SHALL also create it with content that makes git ignore everything in `.trash/`; an existing `.trash/.gitignore`, whatever its content (even empty), SHALL be left unchanged. Failing to create it SHALL NOT fail the deletion. Nothing SHALL remove files from `.trash/`. Writes SHALL be atomic: a reader SHALL see either the old or the new content, never a partial file. Temporary files SHALL begin with `.`, so the build and the watcher ignore them, and SHALL be removed if the write fails.

#### Scenario: Symbolic link out of the notes
- **WHEN** `pages/out.md` is a symbolic link to a file outside the notes directory and the editor saves the page `out`
- **THEN** the response is 403 and the outside file is unchanged

#### Scenario: Namespaced new page
- **WHEN** the page `Project X/Auth` without a file is saved
- **THEN** `pages/Project X/Auth.md` is created, and no file outside `pages/` is created or changed

#### Scenario: Crash during write
- **WHEN** the server is killed while writing `pages/auth.md`
- **THEN** `pages/auth.md` holds either its old or its new content

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

### Requirement: Deleting pages
Where editing is available, a page that has a file (a regular page, a journal day, or the home page's `index.md`) SHALL offer "Delete page" in a page actions menu: a small menu button in the page header that opens the menu, which closes again on Escape (returning focus to the button), on a click elsewhere, or once an action is chosen. Choosing "Delete page" SHALL ask for confirmation, naming the file and the number of blocks on other pages that reference the page; nothing SHALL change unless the author confirms. A confirmed deletion SHALL move the file to the trash (see "Safe writes"), open the home page, and leave every other file unchanged. A page that is still referenced SHALL remain as a page without a file. Pages without a file SHALL NOT offer the action (nor, while it is the only action, the menu).

#### Scenario: Delete a page
- **WHEN** the author deletes the page `scratch` (`pages/scratch.md`), which nothing references, and confirms
- **THEN** `pages/scratch.md` is gone, a copy is under `.trash/`, the home page opens, and `pages/scratch.html` is no longer served

#### Scenario: Still referenced
- **WHEN** the author deletes `pages/redis.md` while a journal contains `[[redis]]`
- **THEN** the confirmation mentions 1 referencing block, and after deleting `pages/redis.html` shows the page without content and with its linked reference

#### Scenario: Out of the way
- **WHEN** a page with a file is opened in edit mode
- **THEN** its header shows only a small menu button, and "Delete page" appears when that button is chosen

#### Scenario: Cancel
- **WHEN** the author chooses "Delete page" and cancels the confirmation
- **THEN** nothing changes

#### Scenario: Empty namespace directory
- **WHEN** the author deletes `pages/Project X/Auth.md`, the only file in `pages/Project X/`
- **THEN** the directory `pages/Project X/` is removed and `pages/` remains
