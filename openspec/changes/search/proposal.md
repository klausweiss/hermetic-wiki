# Proposal

## Why

Finding everything ever written about a topic needs full-text search, not just backlinks. Search is the one part of the output that may grow with the wiki, so it must stay as small on disk and in memory as possible, and still work when the output is hosted as static files only.

## What Changes

- Full-text search over page titles and block content, with block-level results that link to the block anchor and show a **block preview** plus the page/breadcrumb.
- Use Pagefind (static, sharded, loaded on demand, low bandwidth) instead of a hand-written index. It is embedded in the `hermetic` binary as a Rust library together with its browser assets, so building the index needs no separate tool and no network access.
- Search requires JavaScript and `http://` (any static host or `hermetic serve`); over `file://` it is unavailable and the UI says so.
- Ranking prefers title matches and recent journal days.
- Diacritic-insensitive matching (e.g. Polish text).

## Capabilities

### New Capabilities
- `search`: search index generation during build, the search UI, result granularity and previews, ranking, and the http-only constraint.

### Modified Capabilities
<!-- none -->

## Impact

- Depends on `static-build` (indexes the generated HTML / block anchors).
- New dependency: the `pagefind` crate (its `api` module). To verify in design: that it embeds its JS/WASM assets, its async runtime requirements (tokio), whether block-level sub-results map onto our block anchors, diacritic handling, and binary size impact.
- If verification fails, fall back to a custom sharded index format with a small vanilla JS reader.
