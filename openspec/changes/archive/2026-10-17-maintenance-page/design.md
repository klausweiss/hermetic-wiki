# Design

## Context

See proposal.md (Why) for motivation, and the specs for the required behavior.

Relevant current state:
- `hermetic serve` keeps the current `Build` (`src/serve/mod.rs`). It holds the rendered `SiteFiles`, where attachments are `Content::File`, their fingerprints, and the `EditMap` of editable pages. A rebuild happens under `state.rebuilding`. Writes call `commit` and answer only after the rebuild.
- The graph records every resolved attachment reference (`RefTarget::Attachment`, `Graph::attachment`). This covers inline images and links in block content and property values. Reference-style links and raw HTML are not recorded.
- `assets::attachments` lists the published attachments. It skips dot-files, symbolic links, and files whose path a generated page takes.
- `edit::move_to_trash` moves one note to `.trash/<stamp>/<rel>`, using its own timestamp. It checks the path with `allowed`, which accepts only `journals/`, `pages/`, and the root `index.md`.
- Pages that are not part of the build are already served on demand: `blank_day` renders a journal day that has no file through `site::pages::document`.
- `hermetic serve` injects `<hermetic-editor>` into every page it serves to a browser on the same machine. The editor already adds controls to the page shell, such as "Today" in the header.
- Live reload dispatches `PAGES_CHANGED` on `document` for every rebuild that does not reload the page. A page that is not in the build is never reloaded, so the maintenance pages always get the event.

## Goals / Non-Goals

**Goals:**
- A maintenance gateway that a second action can join by adding a list entry and its own page, with no new plumbing.
- Request paths can never name a file to move. Only membership in the list of unused assets the server computes at that moment counts.
- Rebuilds cost nothing extra. Unused assets are computed only when asked.

**Non-Goals:**
- Reporting broken image links. That is a later action on the same page.
- Restoring from or emptying `.trash/`. Nothing removes files from the trash, as before.
- Recognizing attachments used only by other attachments, such as an HTML file in `pages/` that links an image. The file-name check usually catches these when a note names the HTML file, but not always (see Risks).
- A keyboard shortcut for the maintenance page.

## Decisions

### 1. Served on demand, never built
The server renders `maintenance.html` and `maintenance/unused-assets.html` per request, like `blank_day`, and only when `same_machine` is true. Otherwise it returns the normal 404. Two new functions in `src/site/pages.rs` build them with `document()`, so they get the header, footer, and sidebar:
- `maintenance_page(site_title)` renders the gateway: an `<h1>` and a `<ul class="maintenance-actions">` with one entry per action.
- `unused_assets_page(site_title)` renders an `<h1>` and `<hermetic-unused-assets>`. Inside the element is a `<noscript>`-style fallback text saying that the list needs JavaScript.

The actions are a small static table (`MAINTENANCE_ACTIONS: &[(name, description, path)]`), so a new action is one table entry plus its page.

*Alternative: write them in every build.* Rejected. A built site has no server to act on, and the list of unused files is not something to publish.

### 2. The footer link comes from the editor element
`<hermetic-editor>` is present exactly where editing is available. On setup, it appends `<a href="{root}maintenance.html">Maintenance</a>` to `.site-footer`, after "All pages", unless a link is already there. The sidebar moves the footer node, not a copy, so it does not matter which of the two runs first.

*Alternative: the server rewrites the footer HTML when it injects the elements.* Rejected because it means string surgery on generated HTML. The editor already adds controls to the page shell in the same way ("Today").

### 3. Computing unused assets at request time
`Build` gets one more field, `referenced: HashSet<PathBuf>`. It holds every attachment path the graph resolved, collected in `build_site` from `Graph` (cheap: the graph already has them). A new function `unused_assets(build, notes) -> Vec<(PathBuf, u64)>` (in `src/serve/maintenance.rs`) computes the list:
1. Candidates: the build's `Content::File` entries whose path is not in `referenced`.
2. For each candidate, its file name, and its percent-encoded form when that differs.
3. It reads the text of every note file. The files are the `file` of each entry in `build.editable` that has one, which is the same set the graph loaded. It drops each candidate whose name occurs in any note text.
4. It sorts the remainder by path and adds each file's size from `fs::metadata`. It drops entries whose metadata cannot be read, because the file vanished.

Step 3 runs per request. It reads every note once and checks every remaining name against it. Typical wikis have hundreds of notes and few candidates, and the check runs only when someone opens the page or confirms a move.

*Alternative: compute during every rebuild.* Rejected because it re-reads all notes on every keystroke-driven save to serve a page that is rarely open.

*Alternative: no file-name check.* It is simpler, but it would list files used through raw `<img>` or reference-style links as unused. Trash makes mistakes recoverable, but a cleanup page that suggests deleting used images undermines trust in it.

### 4. Maintenance API: `/_hermetic/unused-assets`
The new constant `http::UNUSED_ASSETS_PATH` gets its own branch in `handle`, next to `SOURCE_PATH` and `ASSET_PATH`. It uses the same `close_after_body` pattern and returns JSON.
- `GET` requires `same_machine`. It answers `{"assets":[{"path":"assets/a.png","size":2048},…]}`.
- `POST` requires `same_machine && from_editor`, which checks `Origin` and `X-Hermetic-Edit`. The server reads the body with a 1 MiB limit and parses `{"paths":[string,…]}` with a minimal parser for exactly that shape. It returns 400 on anything else, and adds no dependency. Then, under `state.rebuilding`:
  1. It computes `unused_assets` afresh, then keeps the set of their paths as `url_path` strings.
  2. Each requested string either equals one of those paths or is skipped (`"not an unused asset"`). A requested string is never joined to the notes path. Traversal, absolute paths, and symbolic links are excluded by construction, because the candidates come from `assets::attachments`, which already skips symbolic links.
  3. It picks one trash folder for the batch and moves each match with `edit::trash_attachment(root, rel, &dir)`. Failures are skipped with the error text.
  4. If anything moved, it calls `commit(state)`. It answers `{"trashed":[…],"trash":".trash/<stamp>","skipped":[{"path":…,"reason":…}]}`.

*Alternative: `DELETE` with a body.* Rejected because bodies on `DELETE` are unusual, and some clients drop them.

### 5. One trash folder per batch
`move_to_trash` is split in two:
- `trash_dir(root) -> PathBuf` picks `.trash/<stamp>` or `.trash/<stamp>-<n>`, whichever does not exist yet.
- `move_into_trash(root, path, dir, allowed_fn)` does the existing checks and the rename. The checks are that `.trash` is not a link out of the notes, a canonicalize check of the destination folder, `ignore_trash`, and pruning empty folders under `pages/`.

`move_to_trash` (page deletion) becomes `trash_dir` plus `move_into_trash` with the existing `allowed`. `trash_attachment` uses a new `allowed_attachment`: under `journals/`, `pages/`, or `assets/` with symbolic links resolved, and not markdown. `prune_empty_dirs` already stops at `pages/`. Since it only walks up from the parent under `pages/`, it never removes `assets/` or `journals/`.

### 6. Frontend: a new `@hermetic/maintenance` package
The new package is `static/packages/maintenance`, laid out like `task-count`. It depends on `@hermetic/core`, defines `<hermetic-unused-assets>`, and is registered in `static/packages/app/src/main.ts`. Its parts:
- `api.ts`: `list()` and `trash(paths)`, using `fetch` with `X-Hermetic-Edit: 1`. A fetch function is injected for tests, as in the editor's `api.ts`.
- `selection.ts`: pure functions for the selection, for unit tests. These are:
  - toggling one entry,
  - a Shift+click range from the last clicked index,
  - the select-all state (`none` / `some` / `all`),
  - keeping the selection when the list is replaced.
- `format.ts`: `formatSize(bytes)` (B, KB, MB, GB with one decimal, 1024-based) and the "is previewable" check by extension.
- `element.ts`: renders the summary line, the "Select all" checkbox (`indeterminate` for `some`), and the list. Each row has a checkbox, a preview (`<img loading="lazy">` for png/jpg/jpeg/gif/webp/svg), the path linked to the file, and the size. It also renders the "Move to trash (n)" button. The confirmation is a `<dialog>`, styled and worded like the editor's "Delete page" confirmation and reusing its `.danger` button style. It lists the paths in a scrollable box. After a move, a status line shows the result, and the list reloads. The element listens for `PAGES_CHANGED` to reload too. Reloads keep the selection of paths still present.

SVG previews go through `<img>`, which does not run scripts in the SVG.

### 7. Styles
New rules in `static/style.css`, with no new colors (existing tokens only):
- `.maintenance-actions`: a list of cards, matching `.related ul.shared`.
- `.unused-assets` and its rows: a grid of checkbox, preview (fixed 3rem box, `object-fit: contain`), path, and size.

## Risks / Trade-offs

- [The file-name check keeps an unused file whose name appears in an unrelated note, such as a common name like `image.png`] → This is a false negative, and safe. The file can still be deleted by hand.
- [A note references an attachment in a way that neither the graph nor the name check sees, such as an HTML attachment that loads `pages/x.png` while no note names `x.png`] → The file is listed as unused. It goes to `.trash/`, never deleted, and the confirmation says so.
- [The page is opened, then a note starts using a listed file before the author confirms] → The `POST` recomputes the list under the rebuild lock and skips the file. Live reload usually removes it from the list before that.
- [Reading every note on each `GET` is slow for very large notes folders] → It runs only on demand. If it becomes a problem, the per-note text can be cached in `Build` and keyed by the existing fingerprints.

## Migration Plan

None needed. The change adds pages and an API to `hermetic serve`. `hermetic build` output is unchanged. Rollback means reverting the change. Files already moved stay in `.trash/`.
