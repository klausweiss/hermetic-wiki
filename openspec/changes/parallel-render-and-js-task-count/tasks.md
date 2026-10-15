# Tasks

## 1. Parallel rendering

- [ ] 1.1 In `src/site/mod.rs` `render`, decide before rendering whether `default_home` is needed (the page `index` exists and is not implicit), rather than checking `files` afterwards. Verify with `cargo test --test site`; the home-page tests still pass.
- [ ] 1.2 Render page files with scoped workers that pull chunks of 32 page ids from an `AtomicUsize` into per-worker `Vec<OutFile>`s, then merge them into `SiteFiles` (design D1). Choose the worker count with a small internal helper that takes an explicit thread count, so tests can force 1 or N. Verify with `cargo test`; all existing tests pass unchanged.
- [ ] 1.3 Inside the same scope, spawn the graph page, the search index, and the remaining site-wide files (`journal_index`, `all_pages`, `task_files`, `search_page`) as tasks next to the page workers (design D2). Verify with `cargo test`.
- [ ] 1.4 Add a unit test in `src/site/mod.rs` that renders a fixture with 1 worker and with 4 workers and asserts the two `SiteFiles` maps are equal. Use enough pages (for example 200 with cross-links and tasks) that several chunks are spread across workers. Verify with `cargo test site::`.
- [ ] 1.5 In release mode, run `cargo test --release --test serve -- --ignored change_served_within_two_seconds_at_5000_pages` and time `hermetic build` on the scratchpad's synthetic 5,000-page corpus (or an equivalent generated one). Record before and after times in the change's PR description, and verify the serve test passes.

## 2. Open-tasks count out of the page HTML (Rust side)

- [ ] 2.1 Remove the `open_tasks` parameter from `html::document`, and write `<hermetic-task-count></hermetic-task-count>` inside the "Open tasks" link instead. Update every caller in `pages.rs` and `graph_view.rs`, remove `Site::open_tasks`, and keep the `<h1>` count on `tasks.html`. Verify with `cargo test`, and that `cargo build` reports no unused-field warnings.
- [ ] 2.2 Write `_hermetic/open-tasks.json` (`{"count":N}`) in `render` (design D3). Add a test in `tests/site.rs` that a fixture with 3 open tasks produces `{"count":3}`, and that no HTML file other than the `tasks*.html` files contains the count markup. Verify with `cargo test --test site`.
- [ ] 2.3 Remove `Build::open_tasks` from `src/serve/mod.rs`, and the count argument from `site::blank_day`/`pages::blank_day`. Verify with `cargo test --test serve`; the blank-day tests pass.
- [ ] 2.4 Add a test for the "Ticking off a task changes only the pages that show it" scenario. Render a fixture, change a `TODO` in `pages/auth.md` that references no page to `DONE`, render again, and assert that the only changed `.html` paths are `pages/auth.html` and `tasks*.html`. Verify with `cargo test --test site`.
- [ ] 2.5 Add a test to `tests/serve.rs` modeled on `events_list_exactly_the_affected_pages`: adding a `TODO` to an unrelated note sends an event listing `_hermetic/open-tasks.json` and the note's own page, but not other open pages. Verify with `cargo test --test serve`.

## 3. `<hermetic-task-count>` element (frontend)

- [ ] 3.1 Create `static/packages/task-count` (`package.json`, a `Makefile` that includes `../../package.mk`, `tsconfig.json`, `src/index.ts` and `src/register.ts`), modeled on `theme-toggle`. Add it to `PACKAGES` in `static/Makefile` before `app`. Verify with `make -C static check`.
- [ ] 3.2 Implement the element (design D4): render nothing when not interactive; fetch `_hermetic/open-tasks.json` with `no-store` and render ` <span class="count">N</span>` when N > 0; re-fetch on `PAGES_CHANGED` when that path is listed; render nothing on errors, malformed JSON, or zero. Put parsing in a pure function with unit tests for valid, zero, malformed, and missing input. Verify with `make -C static test`.
- [ ] 3.3 Register the element in `packages/app/src/main.ts`, rebuild, and commit `static/dist/app.js`. Verify with `make -C static verify` and `cargo test` (assets are embedded).

## 4. Integration checks

- [ ] 4.1 Run `hermetic serve` on a notes directory with tasks. Open a page, then add a `TODO` to an unrelated note in another editor. Check that the badge goes from N to N+1 without the tab reloading, and that ticking a task off from `tasks.html` lowers it.
- [ ] 4.2 Open a built site via `file://` with JavaScript enabled, and with JavaScript disabled. Check that the "Open tasks" link shows no count, works, and that the browser console shows no errors from the element.
- [ ] 4.3 Run `make test lint` and `make test-perf`, and verify they pass. Together they cover the CI checks in `.gitea/workflows/ci.yml` plus the release-mode performance tests.
