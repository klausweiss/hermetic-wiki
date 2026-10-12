# Tasks

## 1. Corner controls

- [x] 1.1 Vendor the Lucide shapes used (sun, moon, sun-moon, panel-right-open, panel-right-close) in `core/src/icons.ts` and ship `static/icons/LICENSE-Lucide.txt` as `_hermetic/LICENSE-Lucide.txt`; verify the file-list and asset tests
- [x] 1.2 Theme toggle and sidebar control as fixed corner icon buttons with accessible names and a pane count; the header (on narrow screens) and the sidebar's bar keep the corner free; verify `Corner controls` with geometry checks at 360, 860, 870, 1000, and 1440px and the reader suite's toggle checks
- [x] 1.3 Rebuild `static/dist/app.js`; run `make lint`, `make test`, and the reader and editing suites in Chromium and Firefox; record results here

  Results (2026-10-12):
  - `make lint` is clean; 214 Rust tests pass.
  - Reader 62/62 and editing 56/56 in Chromium and Firefox.
  - Geometry at 360, 860, 870, 1000, and 1440px: no overlap with header controls, no horizontal scroll, and the controls stay in place when the sidebar is toggled. The phone check now reads the accessible name and the pane count.
