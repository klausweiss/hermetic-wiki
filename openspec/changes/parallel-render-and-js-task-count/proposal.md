# Proposal

## Why

`hermetic serve` rebuilds the whole site on every change, and on a 5,000-page notes directory that takes about 2.8–3.4 s. The `serve-mode` spec allows 2 s. Profiling showed two easy wins:

- Page rendering runs on one thread and takes about half the time (~1.3 s).
- Every page's header bakes in the number of open tasks, so adding, ticking, or deleting a TODO changes all ~5,100 output files. Every open tab and sidebar pane then reloads.

Measured phases for one rebuild (synthetic corpus: 4,000 pages, 1,000 journal days, 22 cores):

| Phase | Time |
|---|---|
| `Graph::load` (scan, parse, resolve, backlinks, co-mentions) | 241 ms |
| Page files, single-threaded | 1,329 ms |
| `graph.html` (layout, already on up to 8 threads) | 737 ms |
| Search index | 323 ms |
| Task lists and other site-wide files | ~60 ms |
| Diff against the previous build | 25 ms |

## What Changes

- Render page files on several threads, and render the site-wide files (`graph.html` and its layout, the search index, the task lists) at the same time as the pages. The output stays byte-for-byte the same as a single-threaded build. `hermetic build` gains the same speedup. No new dependencies.
- Take the open-tasks count out of the page HTML. hermetic writes the count to a small data file, and the script shows it on the "Open tasks" link in the site header. Under `hermetic serve` the badge updates in place after a rebuild, without reloading the page. Without the script, or when a page is opened from `file://`, the link appears without a count. **BREAKING** for readers of built sites without JavaScript, who lose the count; the link itself is unchanged.

Out of scope: caching parsed notes (worth at most ~90 ms per rebuild), and incremental or on-demand rendering (a "live engine"). The exploration found these are what an interactive app would need, but they are left for a later change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `static-site-output`: site navigation states that the open-tasks count is loaded by the script, is not part of a page's HTML, and is missing without the script.
- `serve-mode`: live updates also refresh the open-tasks count in place, so a change to the number of open tasks alone does not reload pages.

## Impact

- `src/site/mod.rs` (`render`): concurrent rendering with `std::thread::scope`.
- `src/site/html.rs` (`document`), `src/site/render.rs` (`Site::open_tasks`), `src/site/pages.rs` and `src/site/graph_view.rs`: remove the count parameter from the page shell and add the badge element.
- `src/serve/mod.rs` (`Build::open_tasks`, `blank_day`): the count no longer needs to be carried into pages rendered on request.
- New data file in the output (`_hermetic/open-tasks.json`).
- Frontend (`static/packages`): a new element for the count, and an updated `static/dist/app.js`.
- Tests that assert on the badge in page HTML.
