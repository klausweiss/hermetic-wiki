# Proposal

## Why

hermetic is a writer-first, Logseq-like project memory: notes are written mostly in daily journals that reference pages created on demand, so that a year later every topic page shows everything ever thought about it. Every other feature (static output, search, serving, editing) needs one shared, well-defined model of how a directory of markdown files becomes a graph of pages, blocks, and references. This change establishes that model.

## What Changes

- Define the source directory conventions (zero configuration):
  - `journals/YYYY-MM-DD.md` — one journal page per day (ISO date in filename).
  - `pages/**.md` — regular pages; subdirectories are namespaces (`[[Project X/Auth Flow]]` ↔ `pages/Project X/Auth Flow.md`).
- Define the block model: every line starting with a `- ` bullet is a block, nesting by indentation; continuation lines belong to the block; the block is the unit of backlink context.
- Define block properties as `key:: value` lines inside a block, including `id:: <any string>` (UUIDs not required).
- Define references:
  - `[[Page Name]]` and `#tag` link to pages; links to non-existent pages create implicit pages that consist only of their backlinks.
  - Markdown links to relative `.md` paths (`[text](other.md)`) reference the page stored at that path; links to missing files are reported and rendered broken rather than creating pages.
  - `((id))` references a block by id; it also counts as a reference to the block's page.
  - Ids are graph-wide unique; duplicates produce a warning; dangling refs are reported and rendered visibly broken.
- Define task markers: a block starting with `TODO`, `DOING`, or `DONE` is a task.
- Define reference inheritance ("path refs"): a block inherits the references of its ancestors (used for task views); direct references are tracked separately (used for linked references).
- Define co-mentions: two pages directly referenced by the same block are related by an undirected, weighted relationship (weight = number of such blocks), weaker than a reference, so pages can show which other topics they come up with.
- Define page name resolution: case-insensitive, Unicode NFC-normalized matching; display casing comes from the existing file, or from the first mention for implicit pages.
- Define a reversible, Windows-portable page-name ↔ file-path encoding:
  - Unsafe characters (`<>:"\|?*`, `%`, control chars, trailing dots/spaces, reserved names like `CON`) are escaped as `%XX` and decoded on read.
  - `.` / `..` / empty path segments, NUL, and leading-dot names are rejected as page names.
- Define the ordering of a page's linked references: journal references first (newest date first), then page references alphabetically by page name.

## Capabilities

### New Capabilities
- `source-layout`: directory conventions (journals, pages, namespaces), page-name ↔ file-path encoding, name normalization and case-insensitive resolution.
- `block-model`: parsing markdown files into bullet blocks with nesting, properties (`key:: value`, `id::`), and task markers.
- `reference-graph`: page links, tags, markdown links, block refs, co-mentions, implicit pages, backlinks with direct vs. inherited references, id uniqueness, dangling-ref reporting, and linked-reference ordering.

### Modified Capabilities
<!-- none: this is the first change in the project -->

## Impact

- New Rust crate (stable toolchain only); this change is the library core that all later changes (`static-build`, `search`, `serve-mode`, `browser-editing`) build on.
- Markdown parsing dependency to be selected in design (candidates: comrak, pulldown-cmark).
- No network access, no configuration files.
