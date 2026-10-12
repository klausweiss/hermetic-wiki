# Design

## Context

`static-build` is implemented and archived: `site::build` renders one HTML file per page through `site/render.rs` (`Site`, block trees) and `site/pages.rs` (page sections, `journal_index`, `all_pages`, `task_files`), wrapped by `html::document`, which emits one `<main>` with an `<article class="page">`. Assets are embedded via `site/assets.rs` (`EMBEDDED`). There is no JavaScript. The graph (`graph.rs`) loads `journals/` and `pages/` only.

New constraints from the user: TypeScript compiled with Bun, as few JS dependencies as possible, a separate `static/Makefile` for the frontend. See `specs/` for behavior; this covers how.

## Goals / Non-Goals

**Goals:**
- One small script (`_hermetic/app.js`) with zero runtime dependencies, strict TypeScript, unit tests for its pure logic.
- `cargo build` keeps working without Bun installed.
- The no-JS / `file://` experience is unchanged.

**Non-Goals:**
- Search UI (`search`), editing (`browser-editing`), live reload (`serve-mode`).
- A component framework, virtual DOM, or client-side router.

## Decisions

### The home page is the ordinary page `index`
`layout::scan` also picks up a root-level `index.md` as a regular source file named `index` (scan order: it sorts with the other non-journal files by path bytes, i.e. before `pages/…`). Everything else follows from existing rules: it is resolved, linked, and rendered like any page; its output path is `index.md` → `index.html` through the existing `.md` → `.html` mapping; a `pages/index.md` collides with it under the existing colliding-files rule and loses because `index.md` sorts first. `graph::resolve_link_path` additionally accepts the single root file `index.md`. No new page kind is needed.

*Alternative considered (and rejected by the user):* a home page outside the graph that never appears in linked references, related pages, or tasks.

### Site-wide pages
`journal_index` renders the list into `journals.html`. When the page `index` is file-backed, its own page file is `index.html`; otherwise `index.html` is the same list plus a `<p class="hint">` mentioning `index.md`. (An implicit page `index`, created by a `[[index]]` link without an `index.md`, keeps its normal path `pages/index.html`.) The header's title links to `index.html` and "Journals" to `journals.html`. `html::document` adds `<meta name="hermetic-root" content="…">` (`.` at the root) and `<script src="{root}/_hermetic/app.js"></script>` in `<head>` (not `defer`: see theming).

### Frontend tree and toolchain
```
 static/
   Makefile            build | check | test | verify | clean
   package.json        private; devDependencies: { "typescript": "…" }; "packageManager": "bun@1.4.2"
   bun.lock
   tsconfig.json       strict, noEmit, lib ["dom", "es2022"], noUncheckedIndexedAccess
   ts/
     main.ts           entry: theme first, then the rest on DOMContentLoaded
     env.ts            site root from <meta>, protocol check (http/https only)
     links.ts          classify a link: page / block-in-page / other   (pure)
     state.ts          pane list <-> URL query (?p=…, ~ prefix = collapsed) (pure)
     fetchpage.ts      fetch + DOMParser, extract main article, absolutize href/src, strip ids
     sidebar.ts        pane stack UI, collapse/close, overlay on narrow screens
     timeline.ts       list/timeline toggle, batches of 5 days, IntersectionObserver
     keys.ts           g-prefixed shortcuts, Escape, \, ?, help dialog
     theme.ts          system/light/dark cycle stored in localStorage
     *.test.ts         bun test for links, state, URL rewriting (no DOM library)
   dist/app.js         committed build output, embedded by assets.rs
```
- `make -C static build` runs `bun build ts/main.ts --target browser --minify --outfile dist/app.js`.
- `make -C static check` runs `bunx tsc --noEmit`.
- `make -C static test` runs `bun test`.
- `make -C static verify` rebuilds into a temp file and fails if it differs from `dist/app.js`.

The top-level `make lint` and `make test` call `check`/`verify` and `test` respectively.

The compiled file is committed so the Rust build stays independent of Bun; `verify` keeps it honest.

### Banner on the compiled script
`bun build` is run with `--banner` holding a block comment, so `dist/app.js` starts with an explanation that survives minification:

```
/*
 * GENERATED FILE - DO NOT EDIT.
 *
 * This is hermetic's browser script, compiled from the TypeScript sources in
 * static/ts/ (entry point: static/ts/main.ts) with Bun <version>:
 *
 *     make -C static build
 *     (bun build ts/main.ts --target browser --minify --banner ... --outfile dist/app.js)
 *
 * It is committed to the repository on purpose: the Rust binary embeds this file
 * (src/site/assets.rs) and writes it to _hermetic/app.js in every site, and
 * committing it keeps `cargo build` working on machines without Bun.
 * `make -C static verify` (run by `make lint`) rebuilds it and fails if this
 * copy is out of date. To change the script, edit static/ts/ and run
 * `make -C static build`.
 */
```

The banner text lives in `static/banner.txt` (with the Bun version filled in by the Makefile from `bun --version`), so it is reviewed like any source file. `verify` also checks that the output starts with the banner. DOM-heavy code is kept thin and is verified manually in a browser; logic that can be pure (URL state, link classification, path rewriting) is tested with `bun test`, which needs no extra dependency.

*Alternatives:* a `build.rs` invoking Bun (makes `cargo build` depend on Bun); `happy-dom` for DOM tests (adds a dependency the user wants to avoid).

### Theming without a flash
CSS keeps `@media (prefers-color-scheme: dark)` for the no-JS case, scoped as `:root:not([data-theme="light"])`, plus explicit `:root[data-theme="dark"]` / `[data-theme="light"]` blocks. The script is loaded synchronously in `<head>` and its first statement applies the stored theme to `<html data-theme>` before first paint; everything else waits for `DOMContentLoaded`. The theme toggle is created by the script, so it never appears without it.

### Sidebar mechanics
- **Trigger:** a delegated `click` listener on `document` handles Shift+click on links classified as pages. Pages are same-origin `.html` paths under the site root, not under `_hermetic/`.
- **Fetching a pane:** the target is fetched and `main > article` is extracted. Every `href`/`src` is rewritten to an absolute URL against the target's URL so relative links keep working. `id` attributes are removed so main-page anchors stay unique; when the link carried a fragment, the matching element is located first and scrolled into view.
- **Navigation:** plain clicks on page links (main or pane) are intercepted only to carry the current `?p=` query into the next URL; navigation itself is a normal page load. Pane changes use `history.replaceState`, so the URL always reflects the panes without polluting history.
- **Layout:** the layout becomes a two-column grid when the sidebar is open. Each pane has `content-visibility: auto; contain-intrinsic-size: auto 30rem`. Below 900px the sidebar is a fixed overlay with hide/show buttons.

### Timeline mechanics
On `journals.html` (detected via `data-view="journals"` on the article), the script inserts a List/Timeline toggle reflected as `?view=timeline`. Day URLs come from the list already in the page (`ul.days a`), newest first, so no extra index file is needed. Days load in batches of 5; a sentinel element observed by `IntersectionObserver` triggers the next batch. Each day shows its `section.content` under a link heading. An end marker appears after the oldest day.

### HTML hooks
Rust adds small, stable hooks the script relies on:
- `data-view` on the article of generated site-wide pages (`journals`, `pages`, `tasks`, and `home` for the default home page; a home page from `index.md` is an ordinary page);
- `class="content"` and `class="linked-refs"` sections (already present);
- the `hermetic-root` meta.

These are covered by Rust tests so the contract cannot drift silently.

## Risks / Trade-offs

- [Committed `dist/app.js` can drift from sources] → `make lint` runs `verify`; reviewers see both diffs.
- [Bun output may differ across Bun versions, making `verify` noisy] → `packageManager` pins the version; `verify` failures name the version used.
- [Synchronous script in `<head>` delays first paint slightly] → the script is small and cached; it avoids a theme flash.
- [DOM behavior has no automated tests] → pure logic is unit-tested; a manual browser checklist is part of the tasks.
- [A root `index.md` and `pages/index.md` collide] → reported by the existing colliding-files diagnostic; the root file wins.
- [Fetching many panes at once on load from a long URL] → panes load lazily as they approach the viewport; collapsed panes load when expanded.
