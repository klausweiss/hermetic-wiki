# Design

## Context

The editor (`static/packages/editor`) edits one block at a time in a single `<textarea>` that moves between blocks. The page's outline (`Doc`) lives in memory and is saved by serializing it; after each save, and after a rebuild, `resync` replaces the outline with the server's parse and sets the textarea's `value`. Blocks render in Inter, a proportional font. See proposal.md for why each part is needed.

## Goals / Non-Goals

**Goals:**
- Colouring that never moves the caret away from the text, with no editor library.
- Undo that survives saves, re-syncs, and moving between blocks.

**Non-Goals:**
- WYSIWYG editing: markers stay visible and editable as typed.
- Different heading sizes or true bold weight while editing.
- Undo across page loads or tabs, or of changes made on other pages (task labels in task lists, ids added to blocks on other pages for references).

## Decisions

### Mirror behind a transparent textarea, not contenteditable or a library
The textarea keeps native editing (IME, spellcheck, mobile keyboards, selection), and its text is made transparent with a visible caret. A `div` behind it holds the same text split into coloured spans. Both use the same font, size, padding, `white-space: pre-wrap`, and `overflow-wrap`, so lines wrap the same way. The mirror is rebuilt on `input`, and on every programmatic `value` assignment by overriding the instance's `value` setter. That catches every existing call site (completion, image insert, resync, undo) without changing them.
*Alternatives*: OverType (needs a monospace font, its key bindings clash with the outliner keys, and it can only wrap our syntax with a global HTML regex); contenteditable (we would have to rebuild selection, IME, and undo handling).

### Width-preserving styles only
Colour, `text-decoration`, `-webkit-text-stroke` for bold, and synthetic oblique for italic. For the oblique, the editor uses a font family `"Inter Upright"` that declares only the upright Inter file. With no italic face available, `font-style: italic` makes the browser slant the upright glyphs, which keeps their advance widths. Inter's real italic and bold faces have different widths and would make the caret drift.

### Tokenizer
A single left-to-right pass of sticky regex rules per line. Line-level parts (fence, property key, task marker, heading/list/quote marker) come first, then inline rules in priority order: escape, code span, page link/tag-with-brackets, block ref, markdown link, tag, URL, then emphasis. Emphasis rules recurse into their inner text and add their class to each inner run, so nesting works. Triple markers and mixed `***a** b*` / `***a* b**` are matched explicitly. The joined segment texts always equal the input (tested). Tag and URL rules follow `hermetic help syntax` (space before a tag; trailing punctuation excluded).

### Undo by whole-outline snapshots
Each history entry is a `structuredClone` of the `Doc` before a change, plus the cursor before it (block index in document order, selection) and the blocks' rendered elements at that time (so restored blocks show rendered, not raw, until the next build). `commit()` runs on each input, in `changed()`, and at the start of `saveNow()`, which every outline mutation already calls. It compares the serialized outline to the last committed text and records an entry only when it differs. Restoring swaps in the snapshot's `Doc` and edits the recorded block, then saves.
*Alternatives*: inverse operations per op (each op would need an inverse, and typing, completion and images all bypass the ops); the browser's textarea undo (lost on every `value` assignment, and one textarea is shared by all blocks).

### Typing groups
`History.record` joins an input into the previous entry when it is the same kind (`insertText` vs `delete*`, from `beforeinput`), continues at the caret the last input left, in the same block, within 1.5 s. Any other input type, programmatic input, or non-typing change makes its own entry. A commit with no text change (the idle save) ends the group.

### Keys and events
`historyKey` maps Ctrl/Cmd+Z, Ctrl/Cmd+Shift+Z, and Ctrl+Y. In the textarea they are handled in `onKey` before plugins and `preventDefault`ed, and `beforeinput` `historyUndo`/`historyRedo` (Edit menu) is intercepted, so the browser's own stack is never used. A window listener handles them when no block is edited, under the same guards as the Return-to-edit key.

### History lifetime
`resync` clears the history when the payload's text differs from the last committed text (an external change or conflict); a resync after our own save keeps it.

## Risks / Trade-offs

- Textarea and div wrapping differ in some browser, so the caret drifts on wrapped lines → identical wrap properties on both; check Firefox and Chromium by hand (task).
- `-webkit-text-stroke` bold looks fuzzy at some sizes → tune the stroke width; it is purely cosmetic.
- Caret position alone doesn't show that the cursor moved: deselecting at the same spot where typing stopped joins the next typing into the same step → also end the typing group on pointer down in the text field and on navigation keys (task 3.2).
- A clone and a serialize per keystroke → pages are small (tens to hundreds of blocks); limited to 500 entries.
- Image uploads finishing after an undo insert into a block of the replaced outline → rare; the upload's target is located by node and falls back to a new last block.
