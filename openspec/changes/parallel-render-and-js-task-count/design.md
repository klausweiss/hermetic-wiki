# Design

## Context

See proposal.md for the motivation and measured phases.

- `site::render` (`src/site/mod.rs`) renders each page with `pages::page_files` in a single loop. It then renders the site-wide files one after another (`journal_index`, `all_pages`, `default_home`, `task_files`, `search_page`, `graph_view::graph_page`), builds the search index, and adds embedded assets and attachments. The results go into a `BTreeMap`, so insertion order doesn't affect the output.
- `render::Site` holds only `&Graph`, `String`s and `Vec`s. Nothing in `graph`, `site` or `search` uses `Rc`, `RefCell` or `Cell`, so `&Site` and `&Graph` can be shared across threads as they are.
- `graph_layout` already spreads each iteration over up to 8 scoped threads, and its result doesn't depend on how many threads it uses.
- `html::document` takes `open_tasks: usize` and writes `<span class="count">N</span>` into the "Open tasks" link. That is the only site-wide value in the page shell. It reaches every page through `Site::open_tasks`, and reaches `blank_day` through `serve::Build::open_tasks`.
- The frontend loads data over HTTP only: `readEnv().interactive` is false on `file://`, and search hides itself there. Components hear about rebuilds through the `PAGES_CHANGED` event, which carries `{changed, removed}` site paths. The search index invalidates itself when a `search/` path changes.

## Goals / Non-Goals

**Goals:**
- Render the page files and the site-wide files concurrently, with output byte-identical to today's (`static-site-output` "Deterministic output").
- No new crate dependencies; use `std::thread::scope` as `graph_layout` does.
- Remove the open-tasks count from every HTML file except the open-tasks list itself.

**Non-Goals:**
- Making `Graph::load` parallel or incremental.
- Speeding up the graph layout. It measured 737 ms on the synthetic corpus, against a `graph-view` budget of 300 ms for 5,000 nodes and 20,000 edges; that corpus has far more edges, so the comparison is not exact. Tracked separately (see Open Questions).
- Writing output files in parallel in `hermetic build`.

## Decisions

### D1: Worker threads pull chunks of pages from a shared counter

`render` collects the ids of the pages to render (skipping the implicit home page, as today), then runs scoped workers. Each worker repeatedly takes the next chunk of 32 page ids from an `AtomicUsize` and renders them into its own `Vec<OutFile>`. Afterwards the main thread inserts every file into the `BTreeMap`. The page count is the number of ids.

- Pages vary a lot in cost: a hub page with thousands of linked references is far slower than a leaf. Pulling chunks keeps all threads busy, which fixed per-thread slices would not.
- Each worker writes only to its own buffer, so there is no locking on the output map.
- Worker count: `available_parallelism()`, falling back to 1. With one thread, the same code path runs serially, so tests on single-core CI still exercise it.
- Alternative considered: `rayon`. Rejected because it adds a dependency for a single parallel loop.

### D2: Site-wide files run as their own scoped tasks, alongside the page workers

Inside the same `thread::scope`, spawn one task each for `graph_view::graph_page` (the layout) and the search index build, plus one task for the remaining cheap site-wide files (`journal_index`, `all_pages`, `task_files`, `search_page`). The page workers start next to them.

- The graph layout (~740 ms) and search index (~320 ms) are the longest single pieces of work. Started first, they overlap with page rendering (~1.3 s ÷ cores) instead of running after it.
- `default_home` depends on whether a page rendered to `index.html`. That is known before rendering (the page `index` exists and is not implicit), so the check moves ahead of the scope instead of looking at `files` afterwards.
- The graph layout's own up to 8 threads will compete with the page workers. That is acceptable: the OS shares the cores, and the layout has the longest single chain, so it should be started first.
- Embedded assets, attachments (I/O), and the marker file stay on the main thread after the scope; together they take about 3 ms.

### D3: The count goes in a data file, `_hermetic/open-tasks.json`

`render` writes `{"count":N}` at `_hermetic/open-tasks.json`. `document()` loses its `open_tasks` parameter and instead writes an empty `<hermetic-task-count></hermetic-task-count>` inside the "Open tasks" link. `Site::open_tasks`, `Build::open_tasks` and `blank_day`'s count argument are removed.

- Under `_hermetic/`, since `static-site-output` puts "files belonging to hermetic itself" there and the live-reload endpoint already lives there. A name of its own (rather than `tasks/…`) avoids confusion with the task-list continuation files.
- JSON loaded with `fetch`, the same way as `search/meta.json`. Alternative considered: a generated `<script src>` file that sets a global, which would also work on `file://`. Rejected because the spec allows only `_hermetic/app.js` as a script (`static-site-output` "Only hermetic's own script"), and because a script tag is awkward to reload after a rebuild.
- `tasks.html` keeps its `<h1>Open tasks <span class="count">N</span></h1>`. That page changes with the count anyway.

### D4: A new frontend package, `@hermetic/task-count`

It provides the `<hermetic-task-count>` element, following the one-package-per-element layout (`package.json`, `Makefile` that includes `package.mk`, and `src/index.ts` exporting `define`). When it connects:

1. If `!readEnv().interactive`, render nothing.
2. Fetch `_hermetic/open-tasks.json` relative to `env.root`, with `cache: "no-store"` so a stale count isn't served. If the count is above zero, render ` <span class="count">N</span>`, the same markup the server writes today, so the existing styling applies. Render nothing on zero, an error, or malformed JSON.
3. Listen for `PAGES_CHANGED`. When `_hermetic/open-tasks.json` is in `changed`, fetch it again and re-render.

All instances on a page share one in-flight request, since a sidebar pane may contain another header copy; panes fetch only `main > article`, so in practice there is one. It is registered in `packages/app/src/main.ts` and added to `PACKAGES` in `static/Makefile` before `app`. `static/dist/app.js` is rebuilt and committed.

Live reload needs no server change. The data file changes only when the count changes, so `serve::diff` already lists it, and every page now receives the event without reloading.

## Risks / Trade-offs

- [Parallel output differs from serial output through hidden shared state] → Nothing in the render path has interior mutability (checked). Add a test that renders a fixture with 1 worker and with N workers and compares the `SiteFiles` maps, alongside the existing deterministic-output tests.
- [Peak memory rises because every worker holds its rendered pages until the merge] → The total is the same as today: all HTML ends up in the map anyway. Only the per-chunk temporaries add up, and they are small.
- [Too many threads on many-core machines: page workers plus layout threads] → Cap the page workers at `available_parallelism()`. Scoped threads end before `render` returns.
- [The count briefly shows nothing before the fetch finishes] → It is a small, non-essential badge in the header and the page layout doesn't depend on it. Accepted.
- [Readers of built sites without JavaScript lose the count] → Accepted by the author; the link and `tasks.html` still give the count.

## Open Questions

- Whether the graph layout meets its 300 ms `graph-view` budget at realistic edge counts. That is for a separate investigation; it changes neither this design nor its tasks.
