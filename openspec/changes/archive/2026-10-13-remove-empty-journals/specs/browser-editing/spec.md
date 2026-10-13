# Spec Delta

## MODIFIED Requirements

### Requirement: Creating journal days and pages
When editing is available, every page SHALL show a "Today" control in the header. It, and the keys `g` then `d`, SHALL open the journal day of the browser's current date without creating its file. Editing SHALL start in its first block only when the day has no content (no block with text); otherwise the day opens without editing. On a browser on the same machine, the server SHALL show a journal day (`journals/YYYY-MM-DD.html` with a valid date) that is not in the site as a day without content, with its links to the nearest earlier and later days, ready to write in; for other requests it SHALL stay 404. Saving a page that has no file, such as one that is only referenced, SHALL create its file at the path given by the `source-layout` name encoding (journal days at `journals/YYYY-MM-DD.md`), creating namespace directories as needed. Existing files SHALL never be replaced by a create. The page `index` is the home page: its file is the root `index.md`, never `pages/index.md`. When the notes root has no `index.md`, the home page SHALL offer "Write a home page" instead of its hint about `index.md`; choosing it SHALL create the root `index.md` with one empty block and start editing it there.

#### Scenario: Today without a file
- **WHEN** the author clicks "Today" on 2026-10-13 and `journals/2026-10-13.md` does not exist
- **THEN** the journal day opens with its first block being edited, and `journals/2026-10-13.md` still does not exist

#### Scenario: First content
- **WHEN** the author then types `standup` into that block
- **THEN** `journals/2026-10-13.md` exists and contains `- standup`

#### Scenario: Today with content
- **WHEN** `journals/2026-10-13.md` contains `- standup notes` and the author presses `g` then `d`
- **THEN** the journal day opens and no block is being edited

#### Scenario: New page from a link
- **WHEN** the author writes `[[Project X/Auth]]` in a journal, follows the link, and types `- first note` on the new page
- **THEN** `pages/Project X/Auth.md` exists and contains `- first note`

#### Scenario: Write a home page
- **WHEN** the notes root has no `index.md` and the author chooses "Write a home page" on `index.html` and types `welcome`
- **THEN** the root `index.md` contains `- welcome`, `index.html` shows it, and no `pages/index.md` exists

## ADDED Requirements

### Requirement: Journal days without content
A journal day SHALL have a file only while it has content. A save of a journal day whose text is empty — only blank lines, or a single bullet without text (`-`) — SHALL delete the day's file (not move it to the trash) when it has one, and SHALL NOT create one otherwise; its response SHALL describe the page without a file. The editor SHALL keep editing the empty block, and the next save with content SHALL create the file again. The editor SHALL treat a page without content and a page without a file as the same, so neither counts as unsaved. Other pages SHALL keep an empty file.

#### Scenario: Emptying a day
- **WHEN** `journals/2026-10-13.md` contains `- standup` and the author deletes the text of that block
- **THEN** `journals/2026-10-13.md` no longer exists, the block is still being edited, and the journal list does not show 2026-10-13

#### Scenario: Writing again
- **WHEN** the author then types `retro` into the block
- **THEN** `journals/2026-10-13.md` contains `- retro`
