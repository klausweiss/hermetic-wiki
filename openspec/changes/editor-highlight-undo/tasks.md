# Tasks

## 1. Syntax colouring

- [x] 1.1 Add `highlight(body)` in `static/packages/editor/src/highlight.ts` covering every part listed in "Syntax colouring while editing", with nested and combined emphasis. Verify with `highlight.test.ts` (segments join back to the input; references, tags, URLs, emphasis, code, escapes, tasks, properties, fences) and `make -C static/packages/editor test`
- [x] 1.2 Add `highlighted(area)` (mirror element, `value` setter override, `input` listener) and place the wrapper instead of the textarea in `element.ts`. Verify by typing, completing a `[[` link, and pressing Ctrl+B in `hermetic serve`: the colouring follows each change
- [x] 1.3 Add the mirror styles, the `"Inter Upright"` face, and synthetic italic in `static/style.css`; remove the focus outline. Verify in the browser that `***both***` is bold and slanted and that the edited block has no blue border
- [ ] 1.4 Check caret alignment by hand in Firefox and Chromium: click between letters on long wrapped lines with bold, italic, links, and code, and select across them. Fix any wrapping property that differs

## 2. Pasting a link over a selection

- [x] 2.1 Add `linkSelection` in `format.ts` (single http(s) URL, one-line non-URL selection, edge whitespace outside, escaped brackets, `<url>` for parentheses). Verify with the "pasting a link over a selection" cases in `format.test.ts`
- [x] 2.2 Handle it in `onPaste` through the shared `replace()` helper (browser text insertion). Verify in the browser that pasting a URL over a word links it and Ctrl+Z restores the word

## 3. Undo and redo

- [x] 3.1 Add `History` and `historyKey` in `history.ts`. Verify with `history.test.ts` (typing runs, cursor move / pause / kind switch, other changes, redo dropped after a new change)
- [x] 3.2 End the typing group on pointer down in the text field and on navigation keys (arrows, Home, End, PageUp, PageDown), so that deselecting at the caret starts a new step. Verify with the "Deselecting starts a new step" scenario in the browser
- [x] 3.3 Snapshot and commit in `element.ts` (`commit` from input, `changed`, `saveNow`; cursor noted on keydown, `beforeinput`, paste, image insert, and task label clicks), restore with the cursor, and the textarea, `beforeinput`, and window key handling. Verify the "Undo after a save", "Across blocks", "Outliner operations", and "Redo" scenarios in `hermetic serve`
- [x] 3.4 Clear the history in `resync` when the page text changed elsewhere. Verify the "Changed elsewhere" scenario by editing the file in another program
- [x] 3.5 List "Undo / redo" (`MOD Z` / `MOD Shift Z`) and "Paste a URL over selected text: link it" in the editing section of `static/packages/shortcuts/src/index.ts`. Verify by opening the shortcuts dialog with `?` in edit mode
- [x] 3.6 Mention undo across blocks and link pasting in the edit-mode paragraph of `src/help/commands.txt`. Verify with `cargo test --test help`

## 4. Integration check

- [ ] 4.1 Run `make -C static test check build` and `cargo test`, then commit `static/dist/app.js` with the sources. Run `openspec validate editor-highlight-undo --strict`
