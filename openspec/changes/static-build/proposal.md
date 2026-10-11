# Proposal

## Why

The notes must stay readable years from now without any tooling beyond a browser: `hermetic build <dir>` should turn the graph into a plain directory of HTML pages that works with JavaScript disabled, can be opened from disk, and can be hosted on any static file host.

## What Changes

- Add a `hermetic build <dir> [<out>]` command; no configuration files, no flags required.
- hermetic is a single executable that embeds everything needed to produce the output (CSS, JS, fonts, search tooling). Building makes **no network requests**.
- Output is a directory of HTML files, one per page and one per journal day; HTML is the only content format and there is a single markdown renderer (Rust).
- The whole wiki works without JavaScript: every page includes its own content, its linked references (each referencing block with its subtree and an ancestor breadcrumb), its open tasks, and a "Related" section listing co-mentioned pages with their weight, each expandable (e.g. `<details>`) to the shared blocks.
- Markdown links between notes are rewritten to the corresponding generated pages; unresolved ones are rendered visibly broken.
- No output file grows with the size of the whole wiki, except the search index (see `search`). Global views are split so each file stays bounded (e.g. journal index split by period; heavily referenced pages may paginate their references).
- Generated pages:
  - page and implicit-page HTML,
  - journal day HTML with previous/next-day navigation,
  - a journal index,
  - a global open-tasks page (`TODO`/`DOING`) — tasks count toward a page when the block or any ancestor references it.
- Block anchors in HTML so blocks and block refs are linkable.
- Hrefs are percent-encoded per path segment and work identically over `file://` and `http://`.
- Fonts: Inter and JetBrains Mono (SIL OFL 1.1) embedded and emitted with their license texts. They are always used (no preference for locally installed fonts), so the wiki looks the same on every machine.

## Capabilities

### New Capabilities
- `cli`: the `hermetic` executable, its commands and zero-configuration behavior, the no-network / fully-embedded guarantee.
- `static-site-output`: output directory layout, per-page and per-day HTML, linked references and task rendering, bounded file sizes, no-JS operation, embedded assets and font licensing.

### Modified Capabilities
<!-- none -->

## Impact

- Depends on `core-graph-model`.
- Embedded assets (CSS, fonts, license texts) via `include_bytes!`/`include_str!`.
- Fonts vendored into the repository with their OFL license files (verify Reserved Font Names before subsetting/converting).
