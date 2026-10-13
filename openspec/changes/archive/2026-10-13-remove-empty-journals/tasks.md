# Tasks

## 1. Server

- [x] 1.1 `edit::is_empty_note` and `edit::remove_empty_journals` (day files only, regular files only, one-hour grace, plain delete); unit tests `empty_notes`, `empty_journals_removed`
- [x] 1.2 Call it at serve start and at the start of `rebuild`, refreshing the snapshot when something was removed; report each removal on standard error
- [x] 1.3 Mention it in the `serve` section of `src/help/commands.txt`

## 2. Journal days without content

- [x] 2.1 `site::blank_day`; `hermetic serve` serves it on the same machine for a valid journal day not in the site
- [x] 2.2 A `PUT` of empty text to a journal day deletes its file (or creates nothing); test `journal_days_without_content_have_no_file`, `new_pages_and_days` updated
- [x] 2.3 Editor: "Today" only navigates; `isEmpty`/`sameText` in `serialize.ts` for unsaved checks; `resync` keeps the empty block being edited; rebuild `static/dist/app.js`
- [x] 2.4 Update the `serve` section of `src/help/commands.txt`

## 3. Verification

- [x] 3.1 Run `make lint` and `make test`; record results here

  Results (2026-10-13): `make lint` is clean; `make test` passes. In headless Chromium and Firefox (Playwright, outside the repository, against `hermetic serve` on a scratch notes folder), 12 checks pass in each browser: "Today" opens the day with its block being edited and creates no file; the blank day links to the previous day; typing creates `- standup`; deleting the text removes the file while the block stays edited and the status shows "Saved"; typing again recreates `- retro`; emptying and pressing Escape removes it; no conflict notice; reloading shows the day without an unsaved-changes warning.
