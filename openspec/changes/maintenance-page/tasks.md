# Tasks

## 1. Trash for attachments

- [ ] 1.1 In `src/serve/edit.rs`, split `move_to_trash` into `trash_dir(root)` and `move_into_trash(root, path, dir, allowed)`. Keep page deletion behavior identical. Verify with the existing `tests/edit.rs` and serve deletion tests, which pass unchanged.
- [ ] 1.2 Add `allowed_attachment`: the target, with symbolic links resolved, is under `journals/`, `pages/`, or `assets/`, and is not markdown. Add `trash_attachment(root, rel, dir)`. Verify with unit tests:
  - `assets/a.png` and `pages/sub/b.png` land in the same dir, and `pages/sub/` is pruned when it is left empty.
  - `assets/` is kept when it is left empty.
  - A markdown file is refused.
  - A file behind a symbolic link out of the notes is refused.

## 2. Computing unused assets

- [ ] 2.1 Add `referenced: HashSet<PathBuf>` to `Build`, filled in `build_site` from the graph's resolved attachment references. Verify with a unit test: a build of notes with `![x](../assets/x.png)` has `assets/x.png` in `referenced`.
- [ ] 2.2 Create `src/serve/maintenance.rs` with `unused_assets(build, notes)`, as described in design decision 3: unreferenced attachments, minus file names (as written or percent-encoded) found in any note's text, sorted by path, with sizes. Verify with unit tests for each `maintenance` "Unused assets" scenario: image removed, still shown, linked with `#page=2`, raw HTML, name with spaces, and skipped symbolic link.

## 3. Maintenance API

- [ ] 3.1 Add `http::UNUSED_ASSETS_PATH` (`/_hermetic/unused-assets`) and route it in `handle`:
  - `GET` lists the assets as JSON and requires a browser on the same machine.
  - `POST` requires `same_machine && from_editor`, takes a body of at most 1 MiB, parses `{"paths":[…]}` (400 otherwise), and matches each path against a fresh `unused_assets` under the rebuild lock.
  - `POST` moves matches into one trash dir, commits, and answers `{trashed, trash, skipped}`.
  - Other methods get 405.
  
  Verify with `tests/serve.rs` integration tests for the API scenarios:
  - list,
  - not unused,
  - path tricks (`assets/../pages/auth.md`, `/etc/passwd`, a symbolic link),
  - one trash folder per request,
  - 400 on a malformed body, 413 on an oversized body.
- [ ] 3.2 Verify the authenticity rules with integration tests:
  - a `POST` without `Origin`/`X-Hermetic-Edit` gets 403 and nothing is moved,
  - a `GET` and `POST` from a non-loopback `Host` get 403,
  - a `DELETE /_hermetic/unused-assets` gets 405.

## 4. Maintenance pages

- [ ] 4.1 In `src/site/pages.rs`, add the `MAINTENANCE_ACTIONS` table and render `maintenance_page` and `unused_assets_page` with `document()`. The unused-assets page holds `<hermetic-unused-assets>` with fallback text for when JavaScript is unavailable. Verify with unit tests: the gateway lists "Unused assets" with its description and a link to `maintenance/unused-assets.html`, and both pages carry the correct `hermetic-root`.
- [ ] 4.2 Serve `/maintenance.html` and `/maintenance/unused-assets.html` from `handle` only when `same_machine`, with elements injected as for other pages, and 404 otherwise. Verify with `tests/serve.rs`:
  - both pages return 200 to a same-machine request and 404 to a `--public` request from another address,
  - `hermetic build` output contains neither, which is checked in `tests/site.rs`.

## 5. Footer link

- [ ] 5.1 In `static/packages/editor/src/element.ts`, add the "Maintenance" link to `.site-footer` after "All pages" on setup, without duplicating it. Verify by running `make serve-test`: the link appears in the sidebar footer and opens the gateway, and it is absent from a page built with `make site`.

## 6. Unused-assets element

- [ ] 6.1 Scaffold `static/packages/maintenance` like `task-count`: `package.json`, `Makefile`, `tsconfig.json`, the `@hermetic/core` dependency, and `define()`. Register it in `static/packages/app/src/main.ts`. Verify that `make -C static check` passes.
- [ ] 6.2 Add `format.ts` (`formatSize`, previewable extensions) and `selection.ts` (toggle, Shift range, select-all state, keeping the selection across reloads). Verify with `format.test.ts` and `selection.test.ts`, including the scenarios "Select all (12)" and "Range selection (2–6)".
- [ ] 6.3 Add `api.ts` (`list`, `trash`, with an injectable fetch and `X-Hermetic-Edit: 1`). Verify with `api.test.ts`: request method, headers, and body, and parsing of the `{trashed, trash, skipped}` response.
- [ ] 6.4 Add `element.ts` (`<hermetic-unused-assets>`). It renders:
  - the summary and the empty state,
  - "Select all" with the indeterminate state,
  - rows with checkbox, preview, linked path, and size,
  - "Move to trash (n)", disabled at 0,
  - the confirmation `<dialog>` with count, total size, paths, and the `.trash/` note, where Cancel changes nothing,
  - the result status with skipped files.
  
  It reloads on `PAGES_CHANGED`. Verify in the browser with `make serve-test`:
  - paste an image, remove it from the block, and see it listed,
  - select it and confirm, and see it under `.trash/<stamp>/assets/` with the list updated,
  - re-use a listed file in another tab, and see it disappear.
- [ ] 6.5 Add styles for `.maintenance-actions` and the unused-assets list and dialog in `static/style.css`, using existing tokens. Verify in light and dark themes and at phone width in the browser. Rebuild `static/dist/app.js` with `make -C static`.

## 7. Documentation and integration

- [ ] 7.1 Mention the maintenance page in the `serve` section of `src/help/commands.txt` and add it to the project wiki's [[edit mode]] page. Verify that `cargo test --test help` passes and `hermetic help commands` shows the text.
- [ ] 7.2 Run `make test` and `make lint`, and verify that both pass.
