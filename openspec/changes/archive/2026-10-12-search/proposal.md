# Proposal

## Why

Finding everything ever written about a topic needs full-text search, not just backlinks. The goal is the best possible results for this kind of wiki, with no new dependencies and everything inside the single `hermetic` binary. Pagefind was evaluated and rejected: its crate pulls in 183 crates and tokio and grows the stripped binary from about 2.6 MB to about 7.2 MB, and the alternatives (reimplementing its private index format, or embedding its executable) are fragile. An engine built for this wiki can match Pagefind on relevance and beat it where it matters here: block-level results, page-name and recency signals, prefix matching on every word, and diacritic folding.

## What Changes

- **Index at build time.** `hermetic build` and `hermetic serve` write a search index under `search/`: page titles, block content (including properties), and tags, tokenized with Unicode word boundaries, diacritic-folded (`żółć` ↔ `zolc`), and stemmed with English Porter2. The index is sharded by word prefix so a query loads only what it needs.
- **Ranking.** BM25F with field weights (page name above block text), exact page-name matches first, all query words required (falling back to any word when nothing matches all), prefix matching on every query word (`auth` finds `authentication`), a small boost for recent journal days, and tolerance of single-letter typos when nothing matches exactly.
- **Results are blocks.** Each result is a block with its page, breadcrumb, and a preview that highlights the matched words, linking to the block's anchor; pages whose name matches appear as page results.
- **UI.** A `<hermetic-search>` box in the header shows the top results as you type; Enter opens `search.html?q=…` with the full list (shareable). `/` and `Ctrl+K` (`Cmd+K` on macOS) focus the box. Results are ordinary links, so Shift+click opens them in the sidebar.
- **Where it works.** Search needs JavaScript and `http://` (any static host or `hermetic serve`). Over `file://` and without JavaScript the box is not shown, and `search.html` explains why. In `hermetic serve` the index is rebuilt with the site, and open search boxes drop stale shards after live reload.
- **Proof of quality.** A relevance benchmark (judged queries over a realistic corpus) compares our results with Pagefind's (used only as a test-time tool, never shipped), alongside index size and query latency targets.

## Capabilities

### New Capabilities
- `search`: index contents and format guarantees, tokenization and normalization, ranking, results and previews, the search box and results page, availability (http/JS), and the quality benchmark.

### Modified Capabilities
- `static-site-output`: the output gains `search.html` and the `search/` index; the bounded-file-size rule exempts the search index.
- `reader-ui`: the `/` and `Ctrl+K`/`Cmd+K` shortcuts focus the search box.

## Impact

- New Rust module `src/search/` (tokenizer and folding, Porter2 stemmer, indexer writing into `SiteFiles`); `site::render` adds the index and `search.html`.
- New web component package `@hermetic/search` (`<hermetic-search>`, `<hermetic-search-results>`) with a TypeScript Porter2 stemmer and folding that must match Rust exactly (shared test vectors).
- Rust emits `<hermetic-search>` in the header; `app.js` grows by the search component.
- A separate benchmark project (`bench/search/`) with development-only tools (Pagefind CLI via `bunx`, headless browser) run by `make bench-search`; nothing from it ships.
- No new Rust crates; no new runtime JavaScript dependencies.
