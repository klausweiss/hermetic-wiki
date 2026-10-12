# Design

## Context

`site::render` produces a `SiteFiles` map that `build` writes and `serve` keeps in memory, so anything added to the map ships automatically in both. Blocks carry their markdown `content`, properties, and resolved `refs` (with byte ranges); pages are in scan order, with journals first by ascending date, so block ids in scan order correlate with journal dates. The frontend is a Bun workspace of light-DOM web components, and `<hermetic-live-reload>` dispatches `hermetic-pages-changed` with the changed site paths. Behavior is in `specs/search` (plus small changes to `static-site-output` and `reader-ui`).

Measured alternatives: the `pagefind` crate needs 183 crates and tokio, and its stripped binary is about 7.2 MB against hermetic's 2.6 MB today. That is why we build our own engine, and why Pagefind appears only in the benchmark.

## Goals / Non-Goals

**Goals:**
- Relevance at least as good as Pagefind on a judged benchmark; block-level results.
- No new crates; identical normalization in Rust and TypeScript.
- A query fetches only the shards its words need.

**Non-Goals:**
- Phrase queries, boolean operators, filters, field syntax (`tag:`); possible later.
- Stemmers for languages other than English; Polish relies on folding and prefix matching.
- CJK word segmentation.
- Search without JavaScript or over `file://`.

## Decisions

### Documents
- **Block documents:** one per block. The text field is the block's *plain text*: content and property values (except `id`) with page links and tags replaced by their names, block references removed, and markdown reduced to its text and code via `pulldown-cmark` events.
- **Page documents:** one per page. The name field is the page's name.

Only a page's own blocks are documents, so linked references and related sections are never indexed twice. Document ids follow scan order, which puts journal blocks first and in date order.

### Normalization (Rust `search::normalize`, TS `normalize.ts`)
1. NFKD.
2. Drop combining marks.
3. Fold letters NFKD does not decompose: `ł→l`, `đ→d`, `ø→o`, `ß→ss`, `æ→ae`, `œ→oe`, `ı→i`, `þ→th`.
4. Lowercase.
5. Split into maximal runs of letters and digits.

A shared vector file (`tests/fixtures/search/normalize.tsv`, about 50 hand-checked lines covering Polish, German, French, Nordic, Turkish, digits, and mixed scripts) is asserted by both `cargo test` and `bun test`.

### Porter2 stemmer in both languages
A direct port of the Snowball English (Porter2) algorithm in Rust (`search::stem`) and TypeScript (`stem.ts`), each about 300 lines. Both are tested against the same vector file: a subset of Snowball's published `voc.txt`/`output.txt` (BSD-3, attributed) of about 5,000 words, at `tests/fixtures/search/porter2.tsv`. Stems are computed on folded words.

### Index format (`search/`)
```
 search/meta.json         { version, n_docs, avg_len: {name, text}, k1, b,
                            journal_docs: [first, last], shards: [keys], doc_chunk }
 search/t/<hex key>.json  terms sharing a 2-character prefix, sorted:
                            [[term, stem | "", "<packed postings>"], ...]
 search/d/<n>.json         documents n*256 .. n*256+255:
                            { pages: [[path, name], ...],
                              docs: [[page_idx, anchor | "", parent_delta, preview], ...] }
```
- **Shard key:** the first two characters of the folded term, or the whole term if shorter. It is hex-encoded in the file name, so any script yields a safe path. A query word reads the shard of its first two characters.
- **Postings:** packed into a string of variable-length numbers, five bits per character from a 64-character alphabet. Each posting is `doc_delta, tf·2+field, field_length`. Carrying the field length means BM25 needs no global length table, and folding the field into `tf` saves a number. This halves the size compared with JSON number arrays.
- **Breadcrumbs:** a block document stores `parent_delta`, the distance back to its parent block's document, instead of the breadcrumb text. The client builds breadcrumbs from the parents' previews, which usually sit in the same chunk. Titles are page names; the client adds weekdays for journal dates.
- **Recency:** comes from the document id's position within `journal_docs`. That is monotonic in date and needs no per-document date table.
- **Previews:** at most 240 characters of plain text. Breadcrumbs are ancestor texts truncated to 40 characters each, joined with ` › `.
- **Encoding:** JSON throughout, which is readable, compressed by every static host and by browsers, and parsed natively.

### Query evaluation (`query.ts`, pure, no DOM)
1. Normalize the query into words. The last word counts as "still typing" in the header box.
2. Each word's shard is fetched once per index version and cached.
3. **Candidate terms per word:**
   - exact term: weight 1.0;
   - same stem: 0.9;
   - prefix: 0.5 + 0.4·|q|/|term| (words of two or more characters);
   - typo fallback (only when step 5 finds nothing, words of four or more characters): terms within Damerau distance 1 beyond the first two characters, weight 0.4.
4. **Per document, per query word:** take the best BM25F contribution, `idf · tf'·(k1+1)/(tf' + k1·(1−b+b·len/avg))` with `tf' = tf·w_field` (name 3.0, text 1.0), times the candidate weight. k1 = 1.2, b = 0.75.
5. **Combine:** documents matching every word sum their contributions. If there are none, documents matching any word are used, flagged as partial.
6. **Boosts:** a page document whose normalized name equals the normalized query goes first. Journal documents get ×(1 + 0.15·recency).
7. **Fetch details:** only the document chunks for the top results shown (8 in the box, 200 on the page). Matched words are highlighted by normalizing each preview word and comparing it against the matched terms.

### Components (`@hermetic/search`)
- **`<hermetic-search>`** sits in the header nav. It renders only over `http:`/`https:`, debounces typing at 80 ms, and keeps results in a listbox with arrow-key and Enter handling. Enter without a selection goes to `search.html?q=`. Pressing `/`, or `Ctrl+K`/`Cmd+K` even in a text field (handled by `<hermetic-shortcuts>`), dispatches a `hermetic-search-focus` event.
- **`<hermetic-search-results>`** sits in `search.html` and renders up to 200 results for `?q=`.
- **Shared index loader:** it drops cached shards and metadata when a `hermetic-pages-changed` event lists anything under `search/`.

### Rust side
- **`src/search/`** contains `normalize.rs`, `stem.rs`, `text.rs` (plain text of blocks), and `index.rs` (builds the shards and chunks into `SiteFiles`).
- **`site::render`** calls the indexer after rendering pages and adds `search.html`. That page has `data-view="search"`, a `<hermetic-search-results>`, and a paragraph explaining the requirements, which the component hides when it runs.

### Benchmark (`bench/search/`, development only)
A separate Bun project with its own Makefile. Its dev dependencies are `playwright-core`, and Pagefind via `bunx pagefind@1.5.2`.

Corpus and queries:
- **Realistic corpus (`bench/search/corpus/`):** this project's own design notes and specs turned into pages of bullet blocks, plus journals that reference them. Mixed English and Polish blocks, generated deterministically by a script.
- **Queries (`queries.json`):** about 40 queries with hand-judged relevant pages, covering exact names, inflections, diacritic-free typing, prefixes, multi-word queries, and typos.

The runner:
1. builds the corpus with `hermetic build`;
2. makes a copy marked with `data-pagefind-body` on each page's own content and title, so Pagefind indexes the same text without linked-reference duplicates, and indexes it with Pagefind;
3. serves both copies locally;
4. runs every query through both engines in headless Chromium;
5. compares success@5 and MRR at page level, index sizes, and latencies, including on the 5,000-journal corpus.

Downloads happen only when a developer runs `make bench-search`, never in `make test`.

## Risks / Trade-offs

- [Rust and TypeScript normalization or stemming drift apart] → shared vector files asserted on both sides; any mismatch fails `make test`.
- [Prefix shards for common prefixes (e.g. `th`) grow large] → the 300 KB per-query budget is checked by the benchmark; if exceeded, hot shards can split on a third character without changing the format's idea.
- [Typo tolerance cannot fix the first two letters] → stated in the spec; those typos usually come from the first keystrokes and are noticed immediately.
- [Recency by document position is approximate] → it is monotonic in date, which is all a tie-breaker needs.
- [Indexing adds build time] → measured; `serve` must still show a change within 2 seconds on 5,000 journals.
- [Previews make the index larger than the notes] → measured at about 1.8× the markdown uncompressed and 0.14× gzipped on the 5,000-journal corpus. The spec allows up to 2× (¼ gzipped), relying on transfer compression rather than our own compressor.
- [The benchmark corpus is our own text, so relevance judgments are ours] → queries and judgments are reviewable files; you can add your own notes and queries.
