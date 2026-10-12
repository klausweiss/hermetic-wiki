# Tasks

## 1. The name `index` (Rust)

- [x] 1.1 Graph: a regular file under `pages/` named `index` becomes the page `index (in pages)` with a diagnostic; the page `index` without `index.md` gets output path `index.html` and no page file of its own (the default home stays); verify the `Root index and pages index` and `Pages index alone` scenarios and update the fixture snapshot
- [x] 1.2 Home page warning with a link when `index (in pages)` exists; verify `Conflicting pages/index.md` and `Linking to the home page without index.md` in site tests
- [x] 1.3 Edit API: `index.html` resolves to the page `index` with the root `index.md` as its file, also when neither the file nor the page exists; verify a `PUT` creates the root `index.md` and never `pages/index.md`; update the `layout` help text and its tests

## 2. Editing and reading (TypeScript)

- [x] 2.1 Editor: no edit mode on arrival; "Today" starts editing only when the day has no content; Return starts editing the last block at its end; whole rows (also empty blocks) are clickable; verify `No edit mode on arrival`, `Return starts editing at the end`, `Empty block`, `Today with content`, and `Today without a file` in the browser
- [x] 2.2 "Write a home page" on the default home page in edit mode; verify `Write a home page`
- [x] 2.3 `g d` in `<hermetic-shortcuts>` (only where editing is available, listed in the dialog) through an event the editor handles; verify `Go to today`
- [x] 2.4 Timeline as the default journal view, `view=list` in the URL for the list; verify `Timeline first` and `Reload keeps the view`, and update the reader suite's expectations

## 3. Integration

- [x] 3.1 Rebuild `static/dist/app.js`; run `make lint`, `make test`, and the browser suites (editing, reader, search, live reload) in Chromium and Firefox; record results here

  Results (2026-10-12):
  - `make lint` is clean; 214 Rust tests and 67 TypeScript tests pass.
  - Browser suites, Chromium and Firefox: editing 56/56, reader 62/62, search 20/20, live reload 8/8. The reader suite's timeline checks now expect the timeline first and `view=list` for the list.
  - Firefox found a race: creating the home page or today's journal triggers a rebuild that reloads the tab, in addition to the editor's own navigation. Whichever load came first used up "start editing". The flag is now recorded before the write and honored for 5 seconds.

