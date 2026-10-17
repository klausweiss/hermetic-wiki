# Proposal

## Why

Editing a block shows its markdown as flat, unstyled text, so links, references, and emphasis are hard to spot while writing, and a focus border made the block being edited stand out more than its text. Undo was unreliable: the editor saves and re-reads the page one second after typing stops, which reset the browser's undo history of the text field, and that one text field is reused for every block, so undo never reached back into a block once editing moved on. Pasting a URL over selected text replaced the text instead of linking it, as most editors do.

This change was implemented before it was specified; it records the behavior as built.

## What Changes

- The block being edited shows its markdown with syntax colouring: page links, tags, block references, markdown links and URLs, bold, italic (also nested and combined), strikethrough, code, fenced code, escapes, task markers, property keys, and heading, list and quote markers. The text keeps its exact positions, so the cursor and selection line up with what is shown. The blue focus border around the block is removed.
- Pasting a single web URL over a one-line selection turns the selection into a markdown link to it.
- **BREAKING (behavior)**: Undo and redo become the editor's own, for the whole page: Ctrl/Cmd+Z, Ctrl/Cmd+Shift+Z and Ctrl+Y work while a block is edited and after editing ended, go back across blocks and outliner operations, group runs of typing, and put the cursor back where the change was. They replace the browser's undo of the text field, which the "Formatting keys" requirement relied on.
- The shortcuts dialog and `hermetic help` mention undo, redo, and link pasting.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `browser-editing`: new requirements for syntax colouring of the edited block, pasting a link over a selection, and page-wide undo and redo; "Formatting keys" changes its undo sentence to refer to the editor's undo.

## Impact

- `static/packages/editor/src/highlight.ts` (new): tokenizer and the mirror drawn behind the text field.
- `static/packages/editor/src/history.ts` (new): undo/redo stack with typing groups and the history keys.
- `static/packages/editor/src/element.ts`: wraps the text field with the mirror; records a snapshot before each change; handles history keys, `beforeinput` history events and link pasting; clears history when the page changes elsewhere.
- `static/packages/editor/src/format.ts`: link-over-selection edit.
- `static/style.css`: mirror styles, an upright-only Inter face for the editor so italic is slanted without changing widths, no focus outline.
- `static/packages/shortcuts/src/index.ts`, `src/help/commands.txt`: list undo/redo and link pasting.
- `static/dist/app.js`: rebuilt bundle (committed). No server changes, no new dependencies.
