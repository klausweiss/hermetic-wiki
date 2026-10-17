# Proposal

## Why

Pasting images (change `paste-images`) stores every image in `assets/`, and the image stays there after it is removed from the note or the paste is undone. Nothing shows which files are no longer used, so they pile up in the notes folder. `paste-images` deferred this to a follow-up change, `maintenance-page`. Housekeeping like this also needs a home that later actions (for example broken image links) can join.

## What Changes

- A new **maintenance page** at `maintenance.html`. It is a gateway that lists the available maintenance actions, each with a short description and a link to its own page. It is served only by `hermetic serve` to a browser on the same machine, where editing is available. Built sites and other devices never get it.
- The site footer gets a **"Maintenance" link** where editing is available. Built sites and other devices keep their footer unchanged.
- The first action is **Unused assets**, at `maintenance/unused-assets.html`. It lists every attachment that no note links to or shows, with a preview for images, its path, and its size. The author can select several or all of them and move them to the trash after confirming. The list updates itself when the notes change.
- A new **maintenance API** on the server, behind the same request-authenticity checks as the edit API:
  - `GET` lists the unused assets.
  - `POST` moves the chosen ones to `.trash/`.
  
  Before moving each file, the server checks again that it is still unused. The request never chooses a file outside the current list of unused assets.
- To be safe, an attachment does not count as unused if its file name appears anywhere in a note's text. That covers raw HTML `<img>` and reference-style links, which the graph does not track.

## Capabilities

### New Capabilities
- `maintenance`: the maintenance page and its footer link, which attachments count as unused, the unused-assets page (listing, multi-select, confirmation, trash), and the maintenance API.

### Modified Capabilities
- `serve-mode`: HTTP behavior allows `POST` to the maintenance API and serves the maintenance pages, which are not part of the build.
- `reader-ui`: the key menus become `,` for settings (the theme) and `.` for actions the current page offers; `.` then `a` selects all and `.` then `d` moves the selected files to the trash (after confirming) on the unused-assets page. Keys work while a checkbox has the focus.
- `browser-editing`: request authenticity covers the maintenance API. Safe writes allow moving an unused attachment to the trash, which covers files under `assets/` as well as `journals/` and `pages/`.

## Impact

- Rust:
  - `src/serve/mod.rs`: routing for the maintenance pages and the API.
  - `src/serve/edit.rs`: trashing attachments and a shared trash timestamp for one batch.
  - New code that computes the unused attachments from the graph, the attachment list (`src/site/assets.rs`), and the note sources.
  - `src/site`: renders the two maintenance pages with the usual page shell.
- Frontend:
  - A new `static/packages/maintenance` package with the unused-assets element, registered in `static/packages/app`.
  - The editor adds the footer link.
  - New styles in `static/style.css`, and a rebuilt `static/dist/app.js`.
- No changes to `hermetic build` output, the search index, or note files, apart from the files the author chooses to trash.
