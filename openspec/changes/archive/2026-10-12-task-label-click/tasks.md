# Tasks

## 1. Task labels

- [x] 1.1 Pure `nextState` for a block body (TODO → DOING → DONE → TODO, marker on the first line only, other lines untouched); verify unit tests, including a block with properties and continuation lines
- [x] 1.2 Labels on the page's own blocks: clickable and keyboard-operable, cycle the node, update the label at once, save; also while another block is being edited; verify `Cycle on the page` and `Done wraps around` in the browser
- [x] 1.3 Labels in task lists (`tasks.html`, its continuations, each page's "Open tasks" section): resolve the task's page and anchor from its `.task-source` link; for the page being edited change its outline, otherwise GET the page, change the block, PUT (one retry on conflict, nothing written when the block is gone); verify `Tick off from the open-tasks page` (file otherwise byte-identical), `Keyboard`, and `Read-only elsewhere`
- [x] 1.4 Styles, rebuild `static/dist/app.js`; run `make lint`, `make test`, and the editing browser suite in Chromium and Firefox; record results here

  Results (2026-10-12):
  - `make lint` is clean; 210 Rust tests and 67 TypeScript tests pass.
  - Editing browser suite, Chromium and Firefox: 51/51. It now covers every `Task labels` scenario, a label clicked while another block is edited (focus kept), and a CRLF, tab-indented file changed from `tasks.html` with nothing else touched.

