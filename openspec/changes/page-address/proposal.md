# Proposal

## Why

Converting between a page's name, its source file and its HTML file follows one set of rules, but no module owns them. They are restated in five functions, so changing a rule (the home page, `pages/index.md`, journal file names) means finding every copy. `serve::edit::resolve` even checks its reverse mapping against the forward one because the two can drift.

## What Changes

- Add a page address type to `layout.rs` with three forms: the home page, a journal day, and a named page. It owns both directions of the mapping: source path ↔ address and HTML path ↔ address, plus where a page without a file would be stored and served.
- Make these callers use it instead of their own rules:
  - `layout::scan` and `graph::reserve_home_name` (file → name)
  - `site::paths::output_path` (page → HTML path)
  - `serve::edit::resolve` (HTML path → page)
  - `serve::edit::source_path` (page → file to create)
  - the final file → name step of `graph::resolve_link_path`
- No behavior change: names, output paths, diagnostics and the edit API stay exactly as they are.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. This is a refactor, so the change sets `skip_specs: true`. The rules it centralizes are already specified in `source-layout`, `static-site-output` and `browser-editing`, and their tests stay unchanged.

## Impact

- Code: `src/layout.rs`, `src/graph.rs`, `src/site/paths.rs`, `src/serve/edit.rs`.
- No change to the CLI, the generated site, the edit API or the on-disk layout.
