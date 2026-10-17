## ADDED Requirements

### Requirement: Syntax colouring while editing
The block being edited SHALL show its markdown as written, with every character in the same place as in the text field, and with these parts set apart by colour or decoration: page links (`[[name]]`, `#[[name]]`), tags, and block references (`((id))`), with their brackets dimmed; markdown links, images, and web URLs (a tag's or URL's trailing `. , ; : ! ? )` excluded); bold, italic, and strikethrough text with dimmed markers, also nested and combined (`***both***`); inline code and fenced code lines; backslash escapes; a leading `TODO`, `DOING`, or `DONE` in its task colour; property keys (`key::`); and heading, list, and quote markers. Nothing SHALL be set apart inside code or after a backslash. Styling SHALL NOT change the width of any character, so the cursor and selection stay on the characters they belong to, also on wrapped lines. The colouring SHALL follow every change to the text: typing, formatting keys, completion, pasting, and undo. The edited block SHALL NOT have a focus border.

#### Scenario: References and tags
- **WHEN** the author edits a block `see [[auth]] #backend ((ttl-bug)).`
- **THEN** `auth`, `#backend`, and `ttl-bug` are shown in the link colour, their brackets dimmed, and the final `.` is plain

#### Scenario: Bold and italic together
- **WHEN** the author types `***both***` in the block being edited
- **THEN** `both` is shown bold and slanted, and the `***` markers are dimmed

#### Scenario: Nested emphasis
- **WHEN** the block being edited is `**bold _em_ [[x]]**`
- **THEN** all of `bold em x` is shown bold, `em` also slanted, and `x` also in the link colour

#### Scenario: Not inside code
- **WHEN** the block being edited is ``write `[[auth]]` or \[[auth]]``
- **THEN** `` `[[auth]]` `` is shown as code, and the escaped `[[auth]]` is not shown as a link

#### Scenario: Cursor alignment
- **WHEN** the author clicks between two letters of a long, wrapped line containing bold and italic text
- **THEN** the cursor appears between those two letters

### Requirement: Pasting a link over a selection
While a block is being edited, pasting text that is a single `http://` or `https://` URL (surrounding whitespace ignored) while text on one line is selected SHALL replace the selection with a markdown link `[selection](url)`, leaving whitespace at the edges of the selection outside the link and placing the cursor after it. `[` and `]` in the selection SHALL be escaped; a URL containing `(`, `)`, `<`, or `>` SHALL be written in angle brackets. When nothing is selected, the selection spans lines or is itself a URL, or the pasted text is anything else, pasting SHALL work as before. One undo SHALL restore the selected text.

#### Scenario: Link a selection
- **WHEN** the author selects `the docs` in `see the docs here` and pastes `https://x.dev/a`
- **THEN** the text is `see [the docs](https://x.dev/a) here` with the cursor after the link

#### Scenario: Parentheses in the URL
- **WHEN** the author selects `wiki` and pastes `https://w.org/A_(b)`
- **THEN** the text is `[wiki](<https://w.org/A_(b)>)`

#### Scenario: Ordinary paste
- **WHEN** the author pastes `https://x.dev` with no text selected, or pastes `two words` over a selection
- **THEN** the pasted text is inserted as is

### Requirement: Undo and redo
Where editing is available, Ctrl+Z SHALL undo and Ctrl+Shift+Z and Ctrl+Y SHALL redo (Cmd instead of Ctrl on macOS, where Cmd+Shift+Z redoes), both while a block is being edited and while none is (when no other text field has the focus and no dialog is open); the browser's Edit menu undo and redo in the text field SHALL do the same. The history SHALL cover every change the author makes to the page in the tab: typing, formatting keys, completion, pasting (text, links, and images), outliner keys, task label clicks on the page's own blocks, and the page properties dialog. Undo SHALL go back one step at a time across blocks, also into blocks no longer being edited; a run of typing or deleting SHALL be one step, and a new step SHALL start when the cursor is moved or text is selected, after a pause of about 1.5 seconds, when switching between typing and deleting, and with any other change. Undoing or redoing SHALL show the page as it was, edit the block where the change was made with its cursor or selection as it was before the change (or end editing when the change was not made while editing), and save the result like any other change. A new change SHALL discard the steps that could be redone. Saving SHALL NOT affect the history. When the page is replaced by a version changed elsewhere (another tab, another program, or a save conflict), the history SHALL be cleared. The shortcuts dialog SHALL list undo and redo.

#### Scenario: Undo after a save
- **WHEN** the author types `hello`, waits until the page is saved, and presses Ctrl+Z
- **THEN** `hello` is removed and the change is saved

#### Scenario: Deselecting starts a new step
- **WHEN** the author types `abc`, selects `b`, clicks to clear the selection at the end, types `d`, and presses Ctrl+Z
- **THEN** the text is `abc`, and a second Ctrl+Z gives the text before `abc`

#### Scenario: Across blocks
- **WHEN** the author types `one` in block A, presses Enter, types `two` in the new block B, presses Escape, and presses Ctrl+Z three times
- **THEN** `two` is removed with block B being edited, then block B is removed with block A being edited and the cursor after `one`, then `one` is removed

#### Scenario: Outliner operations
- **WHEN** the author indents a block with Tab and presses Ctrl+Z
- **THEN** the block is back at its previous level, being edited with the cursor where it was

#### Scenario: Redo
- **WHEN** the author undoes a change and presses Ctrl+Shift+Z
- **THEN** the change is made again

#### Scenario: Changed elsewhere
- **WHEN** the page's file is changed by another program and the page updates, and the author presses Ctrl+Z
- **THEN** nothing changes

## MODIFIED Requirements

### Requirement: Formatting keys
While a block is being edited, these keys (Cmd instead of Ctrl on macOS) SHALL toggle inline markdown formatting: Ctrl+B bold (`**`), Ctrl+I italic (`*`), Ctrl+Shift+S strikethrough (`~~`), and Ctrl+E inline code (`` ` ``). With text selected, the key SHALL wrap the selection in the markers and keep it selected; when the selection is already wrapped in those markers (just outside it, or as its first and last characters), the key SHALL remove them instead. Without a selection, the key SHALL insert a pair of markers with the cursor between them, or remove an empty pair the cursor sits in. Bold and italic SHALL be told apart by the run of `*` characters, so italic never removes half of a bold marker. Undo (see "Undo and redo") SHALL revert each change as one step. The keys SHALL take precedence over the browser's own bindings for them, and the shortcuts dialog SHALL list them.

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
