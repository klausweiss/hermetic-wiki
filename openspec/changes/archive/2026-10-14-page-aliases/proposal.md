# Proposal

## Why

The same thing often goes by several names (`auth`, `authentication`, `login`), and today each name becomes its own page, so what was written about it is split across them. Aliases let one page own several names. Aliases are a page-level setting, and there are no page properties yet, so this change adds them too.

## What Changes

- **Page properties**: the properties of a page's first top-level block, when that block has no display content (only `key:: value` lines). This is Logseq's "pre-block" convention. The parser already reads property lines at the top of a file as such a block, so there is no new syntax: pages stay pages and blocks stay blocks.
- **Aliases**: the page property `alias:: a, [[b]]` gives the page more names. Values are split on commas and may be written bare or as page links. `[[a]]`, `#a`, and `#[[b]]` then resolve to that page: the link keeps its written text, links to the page's own URL, and the block shows in that page's linked references, tasks, related pages, and the graph view. No page is created for an alias name, and there is no file or URL for it.
- Aliases are ignored, with a diagnostic, when the name is:
  - the name of a page that has a file, or a namespace prefix of one (the page wins)
  - already claimed by an earlier page (the first in scan order keeps it)
  - a journal date
  - invalid as a page name
  
  An alias of the page's own name is ignored silently. Aliases are not followed in chains.
- Links in `alias::` values are not references of the block: they would only point back at the page itself. The value is shown as the plain names.
- Markdown links (`[x](a.md)`) name files, not pages, and are not resolved through aliases.
- Search: each alias is indexed as another name of the page, so searching an alias finds the page (shown and linked under its own name), and a page appears at most once in results. The editor's `[[` completion is built on search, so typing an alias suggests the page.
- `hermetic help syntax` documents page properties and aliases.

## Capabilities

### New Capabilities

None.

### Modified Capabilities
- `block-model`: new requirement "Page properties".
- `reference-graph`: new requirement "Page aliases"; "Implicit pages" (alias names do not create implicit pages).
- `search`: "What is searchable" (aliases are indexed with the page name).
- `static-site-output`: "Reference rendering" (links through an alias go to the page's own URL); "Page content" (how the `alias` property is shown).

## Impact

- `src/graph.rs`: `Page` gets page properties and aliases; an alias table is built after blocks are added and before references are resolved; `alias::` property refs are dropped; new diagnostic kind(s) in `src/diag.rs`.
- `src/site/render.rs`: `alias` property values shown as plain names.
- `src/search/index.rs`: one extra page document per alias; `static/packages/search/src/results.ts`: drop repeated page results; rebuild `static/dist/app.js`.
- `src/help/syntax.txt`; tests in `tests/reference_graph.rs`, `tests/site.rs`, `tests/search_index.rs`.
- No change to the search index format or the file layout.
