# Proposal

## Why

Reading is the main way the notes pay off: opening a topic and seeing its history, keeping several pages side by side like Logseq's sidebar, and scrolling the journal as one continuous timeline. The plain HTML must look good on its own, and JavaScript should add these richer interactions without becoming a requirement.

## What Changes

- Typography and layout: readable, calm design; always the embedded Inter / JetBrains Mono, for a consistent look on every machine; light and dark themes; works on phone widths.
- Progressive enhancement via a vanilla JS script (no external dependencies, no frameworks, no build toolchain beyond `cargo build`):
  - **Multi-pane**: open additional pages next to the current one (e.g. modifier-click on a link). Panes are fragments in one document, obtained by `fetch()`-ing the target page's HTML and extracting its main content — no iframes, no duplicated content files. Off-screen panes cost little (e.g. `content-visibility: auto`), so many open panes stay cheap.
  - **Merged journal timeline**: the journal view loads consecutive days as you scroll by fetching day pages.
  - Keyboard navigation.
- These enhancements require `http://` (serving via `hermetic serve` or any static host); over `file://` the wiki degrades to plain single-page navigation, which remains fully functional.

## Capabilities

### New Capabilities
- `reader-ui`: visual design, typography, theming, responsive layout, keyboard navigation.
- `multi-pane`: opening, arranging, and closing multiple pages in one document via fetch-based fragments.
- `journal-timeline`: continuous journal view assembled from per-day pages.

### Modified Capabilities
<!-- none -->

## Impact

- Depends on `static-build` (HTML structure must expose a stable main-content element and link markup the script can rely on).
- Adds embedded JS and CSS to the binary.
