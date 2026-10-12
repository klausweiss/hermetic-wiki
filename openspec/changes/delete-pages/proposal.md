# Proposal

## Why

Browser editing can create pages and journal days but not remove them. Deleting a page means leaving the browser for the file manager or a terminal. Deleting is also the one edit that cannot be fixed by typing again, so it needs its own safety rules.

## What Changes

- In edit mode, a page's header offers "Delete page". It is available for regular pages and journal days that have a file, and for the home page's `index.md`.
- A confirmation names the file and how many blocks on other pages reference the page. The page is deleted only after it is confirmed.
- Deleting moves the file into `.trash/` in the notes folder, at its original relative path with a timestamp. Dot-folders are ignored by the build and the watcher, so the page disappears from the site, and moving the file back restores it. Namespace directories under `pages/` that become empty are removed.
- After deleting, the browser goes to the home page. A page that is still referenced stays as a page without a file, like any referenced name.
- The edit API gains `DELETE /_hermetic/source?path=…`. It has the same authenticity, safe-path, and version rules as `PUT`: it is refused when the file changed since it was loaded, and is followed by an immediate rebuild.

## Capabilities

### New Capabilities

### Modified Capabilities
- `browser-editing`: deleting pages (the control, confirmation, trash, and navigation), and `DELETE` in "Edit API", "Request authenticity", and "Safe writes".
- `serve-mode`: `DELETE` is accepted for the edit API (HTTP behavior); `serve` writes into `.trash/` only through deletion (Serve command).

## Impact

- Rust: `src/serve/edit.rs` (move to trash, prune empty namespace directories), `src/serve/mod.rs` (method routing), the help text (`commands`).
- Frontend: the editor's page header control and confirmation dialog.
- Tests: HTTP tests for `DELETE` (authenticity, stale version, trash path, symlinks, empty directories), browser checks.
