# Design

## Context

- `parse_blocks` already turns property lines at the top of a file (with or without a bullet) into a first block with empty `content` and filled `props` (test `properties_as_first_lines`). Page properties therefore need no parser change.
- `Graph::load` adds file pages, then all blocks, then the block-id table, then resolves raw refs per block. Unknown names become implicit pages in `page_for_name` during resolution.
- `by_key: HashMap<NameKey, PageId>` is the single name → page index. `find_page`, namespace linking (`link_namespaces` indexes `by_key` by prefix), and markdown-link resolution all go through it.
- `hermetic serve` rebuilds the whole graph on every change, so there are no incremental cases to handle.

## Goals / Non-Goals

**Goals:**
- Aliases resolved entirely inside `Graph::load`, so the site, search, the graph view, and tasks get the behavior with no further changes.
- Page properties exposed as one accessor that later page-level features can use.

**Non-Goals:**
- Merging pages and blocks into one entity (as Logseq's database version does).
- YAML front matter.
- Redirect pages or any output at alias URLs.
- Aliases in the editor beyond what search already gives `[[` completion.

## Decisions

### Page properties: an accessor over the first root block
`Graph::page_props(page) -> &[Property]` returns the props of `roots[0]` when that block's `content` is empty and its `props` are not, otherwise `&[]`. No copy is stored on `Page`.
*Alternative:* move the properties onto `Page` and drop the block. That would break "no text is lost" and the editor's block ↔ source mapping. The block stays a normal block.

### Alias table separate from `by_key`
Add `aliases: HashMap<NameKey, PageId>` to `Graph`, plus `Page::aliases: Vec<String>` (accepted names in written order, for search and display). Resolution of `RawRefKind::Page | Tag` checks `aliases` first, then `by_key`, then creates an implicit page. Aliases never name a page with a file, so checking them first only matters for an implicit namespace-prefix page with the alias's name (from `[[login/x]]`): the link still goes to the aliased page, whichever reference is resolved first.
Keeping aliases out of `by_key` means:
- `find_page` (used for URLs and the home page),
- markdown-link resolution,
- and namespace linking

keep meaning "the page with that name", which is what the spec wants (markdown links are not aliased, and no page exists for an alias).
*Alternative:* insert aliases into `by_key`. Fewer lookups, but every `by_key` user would then need to decide whether it wants aliases, and `link_namespaces`/`pages()` would see the same id under two keys.

### Build order
In `load`: after `add_blocks` and `build_id_table`, and before the resolution loop, run `build_aliases(&mut diags)`. At that point only file pages exist (no implicit pages yet), so "a page with a source file" is exactly `by_key`. A name is a namespace prefix of a file page if some page key starts with `<name>/`: precompute the set of all prefixes of file page names once. Pages are visited in `pages` order, which is scan order, so the first declarer wins.
`ensure_namespace_prefixes` is only called for real page names, so an alias like `Project X/Login` creates no `Project X` page.

### Parsing the alias value
Split `value` on `,`, trim each part. A part that is exactly one page link (`[[x]]`) gives `x`; otherwise the part is the bare name. Then `normalize_name` + `validate_name`. Commas inside `[[...]]` are not special-cased: names with commas cannot be aliases. This is documented in the help text.

### Dropping alias-property refs
In `add_blocks`, skip `prop.refs` for the `alias` property of a block that is page-properties-eligible (first root, empty content). Then nothing downstream (backlinks, co-mentions, search ref names) sees them. The renderer special-cases the `alias` key on that block to print `Page::aliases`-style plain names. It reuses the same split, applied to all written parts (including ignored ones) so the display matches the source.

### Diagnostics
One new kind, `DiagnosticKind::InvalidAlias`, for all cases (date, collides with a page, namespace prefix, duplicate, invalid name). Each case gets its own message. The duplicate message names both files, like `DuplicateBlockId`. Line = the alias block's line.

### Search: one extra page document per alias
The client ranks a page first when the query equals its name. It checks this by comparing the name field's word count with the query's (`query.ts`, `lengths.get(doc) === queryWords.length`). Adding aliases to the page document's name field would change that count, so `auth` would stop being an exact match for a page that also has the alias `authentication`.
Instead, `search/index.rs` adds one more page document per accepted alias, right after the page's own document. Each has the same page, an empty anchor, parent 0, and the alias as its name field. The results already show the page's name and path, taken from the chunk's `pages` table, so an alias hit shows and links to the real page. The exact-name rule works for each alias unchanged.
A query can match both the name and an alias (`auth` is a prefix of `authentication`). `resolve` in `static/packages/search/src/results.ts` therefore drops a page result whose path was already returned, keeping the first (best-ranked) one. This can return fewer results than `limit`, which is acceptable. The index format and `VERSION` are unchanged.
*Alternative:* a third field for aliases. That changes the index format and BM25F weights for little gain.

## Risks / Trade-offs

- [An alias written in a later block is silently ignored] → It is still shown as an ordinary property. The help text says aliases go at the top of the page.
- [Renaming a page file to an alias name turns the alias into a diagnostic and moves links to the new file page] → This is what "the page wins" means. The diagnostic points at the stale alias.
- [`[[x]]` displayed via an alias reads differently from the page title] → Intended. It matches Logseq, and the link target makes it clear.
