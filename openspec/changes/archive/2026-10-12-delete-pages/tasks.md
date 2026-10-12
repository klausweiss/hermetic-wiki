# Tasks

## 1. Edit API (Rust)

- [x] 1.1 `DELETE /_hermetic/source`: routing (405 elsewhere), the same authenticity checks as `PUT`, version check (409), 404 for pages without a file; verify `Delete with a stale version`, `Cross-site delete`, and `Other methods`
- [x] 1.2 Move to `.trash/<timestamp>/<path>` after the safe-path checks (symbolic links resolved, never outside the notes folder), remove emptied directories under `pages/`, rebuild before responding; verify `Empty namespace directory`, a symbolic link out of the notes (Unix), and that the build and watcher ignore `.trash/`
- [x] 1.3 Help text (`commands`): deleting pages and the trash; verify the help tests

## 2. Editor (TypeScript)

- [x] 2.1 "Delete page" in a page actions menu (menu button in the page header, Escape and outside clicks close it) for pages with a file; a confirmation dialog naming the file and the count of referencing blocks (from the page's linked references); on confirm `DELETE`, then open the home page; verify `Delete a page`, `Still referenced`, and `Cancel` in the browser

## 3. Integration

- [x] 3.1 Rebuild `static/dist/app.js`; run `make lint`, `make test`, and the browser suites in Chromium and Firefox; record results here

  Results (2026-10-12):
  - `make lint` is clean; 219 Rust tests (5 new `DELETE` HTTP tests and a timestamp test) and 67 TypeScript tests pass.
  - Browser suites, Chromium and Firefox: editing 63/63, reader 62/62, search 20/20, live reload 8/8. The editing suite now covers `Delete a page`, `Still referenced`, `Cancel`, `Empty namespace directory`, and the missing control on pages without a file.
  - Found along the way:
    - The source payload now names the page's file, so the confirmation can show it.
    - A `.trash` that is a symbolic link out of the notes is refused before anything is created.
    - "Today" and "Write a home page" could be cut off when the write's rebuild reloaded the current page first. Clicking "Today" on the journal list could then fail to navigate at all (intermittent, seen in Firefox). Live reload is now held during those actions; covered by `Today from the journal list`, with Firefox run four times.
  - Follow-up: "Delete page" moved from a header button into a page actions menu (☰). The editing suite checks the menu (closed at first, opens, Escape closes it and returns focus) and is now 64/64 in both engines.
  - Also fixed: at exactly 900px the stylesheet's overlay rule (`max-width: 900px`) disagreed with the sidebar script (`899.98px`), so the empty sidebar covered the page. The stylesheet now uses the script's breakpoint.

