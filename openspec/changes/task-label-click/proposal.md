# Proposal

## Why

Moving a task along (TODO → DOING → DONE) is the most frequent edit in project notes. Today it means opening the block's text and retyping the marker, or pressing Ctrl+Enter inside it, and it cannot be done from the open-tasks list at all. That list is where tasks are reviewed and ticked off.

## What Changes

- Where editing is available, clicking a task's label (`TODO`, `DOING`, `DONE`) moves the task to its next state: TODO → DOING → DONE → TODO. The label never disappears this way; Ctrl+Enter while editing still removes it.
- This works on the page's own blocks, on the open-tasks page (`tasks.html` and its continuation pages), and in each page's "Open tasks" section. The change is saved to the task's own note file, which may be another page's file.
- The labels become keyboard-operable controls (focusable; Enter or Space acts like a click) only where they work.
- Linked references, related pages, sidebar panes, and the timeline stay read-only.

## Capabilities

### New Capabilities

### Modified Capabilities
- `browser-editing`: "Where editing is available" names task labels in the task lists as the one editable element outside a page's own blocks; a new requirement "Task labels" defines the click behavior.

## Impact

- Frontend only: `static/packages/editor` (click handling, a pure next-state function, and saving to another page's file through the existing edit API); styles for clickable labels; `static/dist/app.js` rebuilt.
- No server change: the existing `GET`/`PUT /_hermetic/source` and anchors in the task lists' links are enough.
