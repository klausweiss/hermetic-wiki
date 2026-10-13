# Proposal

## Why

"Today" creates the day's journal file with one empty block so the page exists. When nothing gets written that day, the empty file stays, and the journal list and timeline fill up with empty days. Days whose content was cleared stay too.

## What Changes

- "Today" no longer creates the day's file. On the same machine, `hermetic serve` shows a journal day that is not in the site as an empty day to write in; the file appears with the first save that has content.
- Saving a journal day empty (blank, or one empty block) deletes its file instead of writing it, and creates nothing for a day without a file. The editor keeps editing the empty block; typing again recreates the file.
- For files left empty some other way (by another editor, or before this change), `hermetic serve` removes empty journal days: files `journals/YYYY-MM-DD.md` that hold only blank lines, or one empty bullet (`-`) and nothing else.
- It does so when it starts and before every rebuild, for files that have not changed for an hour. The hour leaves time to write in a file just created elsewhere.
- Removed files hold nothing, so they are deleted rather than moved to `.trash/`. Symbolic links, other files under `journals/`, and pages are never touched. Each removal is reported on standard error.
- `hermetic build` is unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `serve-mode`: new requirement "Removing empty journal days".
- `browser-editing`: "Creating journal days and pages" (Today creates no file; the server shows days not in the site); new requirement "Journal days without content".

## Impact

- `src/serve/edit.rs` (`is_empty_note`, `remove_empty_journals`), `src/serve/mod.rs` (called at start and in `rebuild`, which also refreshes the watcher's snapshot so the removal causes no second rebuild). `src/help/commands.txt` (`serve` help).
- `src/site/pages.rs` (`blank_day`), `src/serve/mod.rs` (serving it, empty saves of journal days), `static/packages/editor/src/element.ts` and `serialize.ts` (Today without a file, empty texts alike, keeping the emptied block), `static/dist/app.js`.
