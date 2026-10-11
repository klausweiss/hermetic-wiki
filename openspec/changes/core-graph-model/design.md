# Design

## Context

The repository has no code yet; this change creates the Rust crate and its graph core. Constraints carried over from exploration: stable Rust only, a single self-contained binary eventually, no configuration, no network access, minimal dependencies. Later changes depend on this one: `static-build` renders blocks and refs, `search` indexes them, `browser-editing` writes blocks back to disk, so the model must preserve source text exactly and expose where every reference is located. See `specs/` for the behavior contract.

## Goals / Non-Goals

**Goals:**
- A library API: `Graph::load(root) -> (Graph, Vec<Diagnostic>)` that is deterministic and never fails on content.
- Lossless block model: every source byte is attributable to exactly one block, so editing can round-trip.
- Reference locations as byte ranges inside block content, so the renderer can substitute links without re-parsing.
- Efficient enough to rebuild a graph of several thousand files in well under a second.

**Non-Goals:**
- Markdown rendering and the choice of markdown renderer (belongs to `static-build`).
- Co-mentions through inherited references (parent-block context); only direct references count for now.
- Page-level properties, aliases, block embeds, queries, `NOW`/`LATER`/`CANCELED` task states.
- Incremental graph updates (belongs to `serve-mode`; the model must just not preclude it).
- Stable block anchors for HTML (belongs to `static-build`).
- A CLI; this change ships a library and its tests only.

## Decisions

### Crate layout: one package, library + binary
A single Cargo package `hermetic` (edition 2024) with `src/lib.rs` and a placeholder `src/main.rs` for later changes. Modules:

```
 layout   scan the root, journal/page classification, name <-> path encoding, name keys
 parse    line scanner -> block tree, properties, task markers, inline ref scanner
 graph    pages, blocks, resolution, backlinks, inherited refs, task queries
 diag     Diagnostic { kind, file, line, message }
```

*Alternative:* a Cargo workspace with separate crates. Rejected for now: one binary, one consumer; splitting later is cheap.

### Own line scanner and inline scanner instead of a markdown parser
Block structure (bullets, indentation, properties, fences) is line-oriented, and the reference syntax (`[[x]]`, `#x`, `#[[x]]`, `((x))`) is not CommonMark. Neither pulldown-cmark nor comrak models these, and getting the block tree out of a CommonMark list AST loses Logseq-style semantics (e.g. a non-indented paragraph after a list item, lossless spans). So parsing is two small hand-written passes:

1. **Line scanner** — walks lines, tracks fence state, detects bullet lines and their columns (tab → next multiple of 4), builds the tree with an indentation stack, groups non-bullet top-level runs into prose blocks, and splits off leading property lines.
2. **Inline scanner** — runs over each block's display content, skips inline code spans (CommonMark backtick-run matching) and backslash escapes, and yields `RawRef { kind: Page | Tag | Block | MdLink, text, range }`. For markdown links it recognizes only the inline form `[text](dest)` / `[text](<dest>)` with balanced brackets, skips images (`![`), and does not handle reference-style links (`[text][ref]`).

The markdown renderer chosen in `static-build` will receive block content plus these ranges.

*Alternative:* comrak with its wikilinks extension. Rejected: still needs custom handling for tags, block refs, and properties, and an AST does not give the lossless line-level mapping editing needs.

### Data model: arenas with integer ids
```
 Graph
   pages:  Vec<Page>          PageId(u32)
   blocks: Vec<Block>         BlockId(u32)
   by_key: HashMap<NameKey, PageId>
   by_id:  HashMap<String, BlockId>
   backlinks: Vec<Vec<BlockId>>          indexed by PageId (direct refs to page or its blocks)

 Page   { name, key, kind: Journal(Date) | Regular, file: Option<PathBuf>,
          roots: Vec<BlockId>, ns_parent: Option<PageId>, ns_children: Vec<PageId> }
 Block  { page, parent: Option<BlockId>, children: Vec<BlockId>,
          lines: Range<u32>, source: String, content_offset,
          props: Vec<(key, value)>, id: Option<String>, task: Option<TaskState>,
          refs: Vec<Ref> }
 Ref    { range, target: Page(PageId) | Block(BlockId) | Unresolved(String) }
```

Inherited references are not stored; they are computed by walking `parent` links (trees are shallow). Page tasks are computed by walking each task block's ancestors once, then indexing by page.

*Alternative:* `Rc<RefCell<…>>` tree. Rejected: arenas are simpler, `Send`, cheap to clone/rebuild, and serialize easily for later incremental work.

### Two-phase load
1. Scan + parse every file (sorted for determinism: journals by date, then pages by path bytes), collecting raw refs and ids.
2. Resolve: register file-backed pages, then create implicit pages (and namespace prefixes) for unresolved names in scan order, which gives the "first mention" display name for free; resolve block refs against the id table; build backlinks.

### Markdown link resolution goes through the file system layout, not names
A markdown link names a *file*, so it is resolved as a path: percent-decode the destination once, join it with the linking file's directory (or the root for `/…`), normalize `.`/`..` lexically (no symlink following, no file system access needed), and check the result is under `root/journals` or `root/pages`. The relative path is then turned into a page name with the same `decode_segment` used by the scanner, and looked up by `NameKey` among file-backed pages only. Missing targets become `Ref::Unresolved` with a diagnostic, never implicit pages.

*Alternative:* treat the link text or file stem as a page name, like `[[x]]`. Rejected: it would ignore directories and create pages from typos.

### Co-mentions are derived, not parsed
After resolution, each block's direct references are reduced to a deduplicated, sorted set of target `PageId`s (block refs contribute their target's page; the block's own page is removed). Every unordered pair `(a, b)` with `a < b` appends the block to `comentions: HashMap<(PageId, PageId), Vec<BlockId>>`. A per-page adjacency list is then built and sorted per the spec. Recency is the max journal date among the edge's blocks, computed on demand.

Storing blocks rather than a bare counter keeps the rendering option of expanding a related page into the shared blocks, at the cost of one `u32` per pair occurrence.

*Alternative:* include inherited references. Deferred: it relates a parent topic to everything nested under it, and the noise cannot be filtered out afterwards.

### Names and encoding
- `NameKey` = NFC(trimmed name) lowercased with `str::to_lowercase`. Full Unicode case folding is not needed for this use.
- Encoding is a pure function pair `encode_segment` / `decode_segment` with property tests asserting `decode(encode(n)) == n` over arbitrary valid names.
- Dates are validated by a small hand-written ISO parser (leap years included); no date crate.

### Dependencies
Only `unicode-normalization` (NFC). Directory walking uses `std::fs`; tests use `proptest` as a dev-dependency for the encoding round trip. Everything else is std.

## Risks / Trade-offs

- [Hand-written scanners diverge from CommonMark edge cases (e.g. backtick runs, nested fences)] → keep scanners small and focused only on what affects references; cover with table-driven tests; the renderer remains the authority for display.
- [`to_lowercase` vs full case folding (e.g. `ß`/`SS`)] → acceptable for page names; documented; key function is one place to change.
- [Tab width of 4 differs from some editors] → only relative indentation matters, so mixed files still nest sensibly; documented.
- [Prose-run blocks differ from Logseq, which would treat them differently] → deliberate: no content is ever dropped.
- [Implicit-page display name depends on scan order] → deterministic by spec; renaming a file-less page means editing its first mention.
- [Blocks listing many pages produce n·(n−1)/2 co-mention pairs] → negligible at normal sizes; a per-block cap can be added later without changing the spec for typical blocks.
- [Double decoding of markdown link destinations (URL, then file name) is surprising] → it matches what a browser does when following the same link to the encoded file on disk; covered by tests.
- [Decoding hand-created files with literal `%XX` sequences] → a file literally named `100%25.md` reads as `100%`; acceptable and documented.
