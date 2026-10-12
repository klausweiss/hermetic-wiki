# Proposal

## Why

Using browser editing day to day showed rough edges:
- Landing on a page that already has content puts you in edit mode.
- There is no key for today's journal.
- An empty block cannot be clicked again once you leave it.
- The journal list opens as a list although the timeline is what you read.
- The home page can only be created outside the browser.
- `[[index]]` silently creates `pages/index.md`, which is not the home page.

## What Changes

- Opening a page never starts editing, except "Today" on a day with no content yet. Return (Enter) starts editing the page's last block, at its end.
- `g` then `d` opens today's journal where editing is available, like the "Today" control.
- Every block row is clickable to edit, including empty blocks.
- The journal list starts in the timeline view; the list view stays one click away and is kept in the URL.
- The name `index` is reserved for the home page. `[[index]]` always links to `index.html`; without `index.md`, the home page offers "Write a home page" in edit mode, which creates the root `index.md`. A file `pages/index.md` is no longer the page `index`. It becomes the page `index (in pages)` with a warning, and the home page shows the conflict with a link to it.
- Deleting pages is **not** part of this change; it is proposed separately as `delete-pages`.

## Capabilities

### New Capabilities

### Modified Capabilities
- `browser-editing`: no edit mode on arrival, Return to start editing, clickable rows, `g d`, and creating the home page.
- `reader-ui`: the `g d` shortcut.
- `journal-timeline`: the timeline is the default view.
- `source-layout`: `index` is reserved for the root `index.md`; `pages/index.md` becomes `index (in pages)`.
- `static-site-output`: the home page warns about `pages/index.md`, and `[[index]]` always links to the home page.

## Impact

- Rust: graph loading (renaming `pages/index.md`), output paths and rendering of the page `index`, the home page warning, the edit API resolving `index.html` to the root `index.md`, and the help text (`layout`).
- Frontend: `editor` (arrival, Return, rows, home page action, Today), `shortcuts` (`g d`), `timeline` (default view), styles; `static/dist/app.js` rebuilt.
- Snapshot and site tests for the `index` rules.
