# Spec Delta

## Purpose

Lets the reader find any block or page in the wiki by its words, with results ranked for this kind of notes wiki, without new dependencies and without any service beyond static files.

## ADDED Requirements

### Requirement: What is searchable
The search index SHALL contain every page's name and every block's content and property values (except `id`) of every page in the graph, including journal days. Text inside code spans and fenced code SHALL be searchable. Markup syntax (`[[`, `]]`, `#` of tags, `((…))` of block references, markdown punctuation) SHALL NOT be indexed as words; the names inside page links and tags SHALL be. Linked references, related pages, and task lists (which repeat other pages' blocks) SHALL NOT be indexed again on the pages that show them.

#### Scenario: Words in a journal block
- **WHEN** a journal block reads `token refresh shipped for [[auth]]`
- **THEN** searching `refresh` finds that block, and searching `auth` finds it as well as the page `auth`

#### Scenario: No duplicates from linked references
- **WHEN** the block above is shown in `pages/auth.html` as a linked reference
- **THEN** searching `refresh` returns that block once, on its journal page

### Requirement: Normalization
Words SHALL be split on Unicode word boundaries, lowercased, and folded to their base letters, so that diacritics do not affect matching (`żółć`, `Zolc`, and `ZÓŁĆ` match each other; `ł` folds to `l`). English words SHALL also match their Porter2 stem, so different inflections of the same word match (`tested`, `testing`, `tests`). The build and the browser SHALL normalize identically.

#### Scenario: Diacritics
- **WHEN** a block contains `Zażółć gęślą jaźń`
- **THEN** searching `zazolc` or `gesla` finds it

#### Scenario: Inflection
- **WHEN** a block contains `we were testing the parser`
- **THEN** searching `tested` finds it

### Requirement: Matching
A result SHALL contain every word of the query (each word matching exactly, by stem, or as a prefix of a word in the result). Shorter prefix matches SHALL rank below exact and stem matches. When no result contains every word, results containing any of them SHALL be shown, marked as partial matches. When nothing matches at all, for query words of at least four letters, indexed words within one letter edit (an insertion, deletion, substitution, or swap of adjacent letters) after the first two letters SHALL be tried, marked as approximate matches.

#### Scenario: Prefix while typing
- **WHEN** the reader has typed `authen`
- **THEN** blocks containing `authentication` are found

#### Scenario: All words required
- **WHEN** one block contains `redis` and `ttl`, and another only `redis`
- **THEN** searching `redis ttl` returns the first block and not the second

#### Scenario: Typo
- **WHEN** a block contains `kafka` and nothing contains `kafla`
- **THEN** searching `kafla` returns that block, marked as an approximate match

### Requirement: Ranking
Results SHALL be ordered by relevance computed with BM25F: matches in page names SHALL weigh more than matches in block text, and term frequency and text length SHALL be accounted for. A page whose name equals the query (after normalization) SHALL be the first result. Among otherwise similar results, blocks on more recent journal days SHALL rank higher.

#### Scenario: Exact page name first
- **WHEN** the page `kafka` exists and many blocks mention `kafka`
- **THEN** searching `kafka` lists the page `kafka` first

#### Scenario: Recency breaks ties
- **WHEN** two journal blocks from 2025-01-10 and 2026-10-01 have the same text `deploy freeze`
- **THEN** searching `deploy freeze` lists the 2026-10-01 block first

### Requirement: Results
Each block result SHALL show the page title, the block's breadcrumb, and a preview of the block's text with matched words highlighted, and SHALL link to the block's anchor on its page. Each page result SHALL show the page title and link to the page. Results SHALL be ordinary links, so that Shift+click opens them in the sidebar.

#### Scenario: Block result
- **WHEN** searching `refresh` finds a nested journal block under `standup`
- **THEN** the result shows the journal day, the breadcrumb `standup`, and the block's text with `refresh` highlighted, and links to `journals/<day>.html#b<n>`

### Requirement: Search box and results page
When the script runs over `http:` or `https:`, the header SHALL contain a search box. Typing SHALL show the top results (at most 8) under the box, updating as the reader types; the arrow keys and Enter SHALL select a result, and Enter without a selection SHALL open `search.html?q=<query>`, which lists all results (up to 200) and can be bookmarked or shared. Escape SHALL close the results under the box. Without JavaScript or over `file:`, the box SHALL not be shown, and `search.html` SHALL explain that search needs JavaScript and the site to be served over `http://`.

#### Scenario: Results as you type
- **WHEN** the reader types `kaf` into the search box
- **THEN** up to 8 results appear under the box without a page load

#### Scenario: Full results page
- **WHEN** the reader types `kafka` and presses Enter without selecting a result
- **THEN** `search.html?q=kafka` opens and lists all matching results

#### Scenario: Opened from disk
- **WHEN** a page is opened via `file://`
- **THEN** no search box is shown, and opening `search.html` shows the explanation

### Requirement: Index size and loading
The index SHALL be split so that a query of up to three words loads only the index files for those words' first letters plus metadata, never the whole index. For the 5,000-journal benchmark corpus, the files a typical query loads SHALL total less than 300 KB uncompressed, and the whole index SHALL be no larger than the notes' markdown text.

#### Scenario: Loading only what a query needs
- **WHEN** the reader searches `redis ttl` on the 5,000-journal corpus
- **THEN** the browser fetches the metadata and at most the shards for `re` and `tt` plus the result details shown, totalling under 300 KB

### Requirement: Search during serve
In `hermetic serve`, the index SHALL be rebuilt with every rebuild, and an open search box SHALL use the new index after a live-reload notification, without reloading the page.

#### Scenario: New words become searchable
- **WHEN** `hermetic serve` runs and the author saves a block containing `zeppelin`
- **THEN** within the serve rebuild time, searching `zeppelin` in an already open tab finds it

### Requirement: Quality benchmark
The repository SHALL contain a relevance benchmark: a corpus of realistic notes, a list of queries with the results judged relevant for each, and a runner that measures, for both hermetic's search and Pagefind (used only by the benchmark, never shipped), the share of queries whose first relevant result is in the top 5 (success@5) and the mean reciprocal rank (MRR), compared at page level. hermetic's search SHALL score at least as well as Pagefind on both measures, and its query latency on the 5,000-journal corpus SHALL stay under 50 ms in a headless browser after the shards are loaded.

#### Scenario: Benchmark run
- **WHEN** a developer runs `make bench-search`
- **THEN** it prints success@5 and MRR for both engines plus index sizes and latencies, and fails if hermetic scores below Pagefind on either measure or exceeds the latency target
