# Tasks

## 1. Normalization and stemming (Rust)

- [x] 1.1 Create `tests/fixtures/search/normalize.tsv` (input → expected words) covering Polish, German, French, Nordic, Turkish, digits, punctuation, and mixed scripts; implement `search::normalize` (NFKD, drop marks, fold table, lowercase, letter/digit runs); verify with `cargo test` over the vectors
- [x] 1.2 Add `tests/fixtures/search/porter2.tsv` (about 5,000 word/stem pairs from Snowball's English vocabulary, with its BSD-3 notice) and implement `search::stem` (Porter2); verify every vector matches

## 2. Indexer (Rust)

- [x] 2.1 `search::text`: plain text of a block (links and tags as names, block refs removed, markdown reduced to text and code, properties except `id`), breadcrumb text, and preview; verify `Words in a journal block` and that `((id))`, `[[`, `#`, and markup never produce words
- [x] 2.2 `search::index`: documents (pages and blocks in scan order), terms with stems, sharded postings with field lengths, document chunks, `meta.json` with averages and `journal_docs`; add them and `search.html` to `site::render`; verify the files appear in a build, `No duplicates from linked references`, and determinism (two builds byte-identical)
- [x] 2.3 Measure index size and build time on the 5,000-journal corpus; verify the index is no larger than the markdown text and `serve` still shows a change within 2 seconds (`make test-perf`)

## 3. Search engine (TypeScript)

- [x] 3.1 New workspace package `@hermetic/search` with Makefile and tsconfig; `normalize.ts` and `stem.ts` passing the same `normalize.tsv` and `porter2.tsv` vectors in `bun test`
- [x] 3.2 Index loader (meta, shards by hex key, document chunks, per-version cache, invalidation on `hermetic-pages-changed` paths under `search/`) and pure `query.ts` (candidates, BM25F, AND with OR fallback, typo fallback, exact-name and recency boosts, highlighting); verify `Prefix while typing`, `All words required`, `Typo`, `Diacritics`, `Inflection`, `Exact page name first`, and `Recency breaks ties` with `bun test` over a small in-memory index built by the Rust indexer from a fixture (checked in as JSON)

## 4. UI

- [x] 4.1 `<hermetic-search>` in the header (http/https only): debounced as-you-type results (≤8) with page title, breadcrumb, highlighted preview; arrow keys, Enter, Escape; Enter without selection opens `search.html?q=`; Rust emits the element in the header nav; verify `Results as you type`, `Full results page`, and `Block result` in Chromium and Firefox
- [x] 4.2 `search.html` with `<hermetic-search-results>` (up to 200 results, partial/approximate markers) and the no-JS/file:// explanation; verify `Opened from disk` and the page without JavaScript
- [x] 4.3 `/` and `Ctrl+K`/`Cmd+K` shortcuts focusing the search box via `<hermetic-shortcuts>` and an event; update the help dialog; verify `Focus search`, `Focus search while typing` and `Typing is not intercepted`
- [x] 4.4 Styles for the box, dropdown, results page, and highlights (light and dark), usable at 320px; verify in both browsers and that Shift+click on a result opens the sidebar

## 5. Serve integration

- [x] 5.1 Verify `New words become searchable` against a running `hermetic serve` in Chromium and Firefox (open tab, save a note, search without reloading)

## 6. Benchmark

- [x] 6.1 Create `bench/search/` (Bun project, Makefile, dev dependencies only) with a deterministic corpus generator (project docs as pages + journals, mixed English/Polish) and `queries.json` with about 40 judged queries across the categories in the design
- [x] 6.2 Runner: build the corpus, prepare and index the Pagefind copy with `bunx pagefind@1.5.2`, serve both, run all queries in headless Chromium, compute success@5 and MRR per engine, index sizes, per-query loaded bytes, and latency (also on the 5,000-journal corpus); `make bench-search` at the repository root
- [x] 6.3 Run the benchmark; tune weights only through `query.ts` constants until hermetic ≥ Pagefind on both measures, latency < 50 ms, and per-query loads < 300 KB; record the results table in this file

  Results (2026-10-12, `make bench-search`, headless Chromium; no tuning was needed):

  | | hermetic | Pagefind 1.5.2 |
  |---|---|---|
  | success@5 (40 queries) | 100.0% | 90.0% |
  | MRR | 0.933 | 0.833 |
  | index, relevance corpus (135 KB markdown) | 246 KB | 145 KB |

  Pagefind misses: typo queries (`rebuidl` not found, `sidebr` 4th, `pagefnid` 18th) and Polish diacritics (`zazolc gesla jazn`, `wlasna wyszukiwarka` not found). Hermetic's misses at rank 1: `pagefind` 2, `linked references pagination` 2, `rebuilding` 3, `namesp` 2, `pagefnid` 2.

  5,000 journals (678 KB markdown, 1,351 KB index): per-query loads 28–63 KB, cold 9–19 ms, warm 0.3–11 ms.

## 7. Finish

- [x] 7.1 Update the end-to-end snapshot and whole-site tests for `search.html` and `search/`; run `make lint`, `make test`, `make test-perf`, and the full browser suite in both engines; record results here

  Results (2026-10-12): `make lint` clean; `make test` 174 Rust tests and all TypeScript tests pass; `make test-perf` loads 5,000 journals in 109 ms and serves a change after 0.87 s; browser suites pass in Chromium and Firefox (search 20/20 including `Ctrl+K`, reader 60/60, live reload 7/7).
