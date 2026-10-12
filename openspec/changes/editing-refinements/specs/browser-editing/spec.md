# Spec Delta

## MODIFIED Requirements

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
