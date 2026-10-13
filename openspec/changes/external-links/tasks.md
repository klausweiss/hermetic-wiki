# Tasks

## 1. Linking bare URLs

- [x] 1.1 Add a URL scanner to `src/site/render.rs` that finds `http(s)://` URLs in a string, as described in design.md (word boundary, ends at whitespace, `<` or a placeholder character, trailing punctuation and unbalanced `)` trimmed). Verify with unit tests covering the spec scenarios plus: a URL in the middle of a word, a bare `https://`, and a URL followed by a placeholder.
- [x] 1.2 Wrap the parser's events in `markdown()`: join neighbouring text pieces, skip text inside links, images, code blocks and inline `<a>` HTML, and emit autolinks for the URLs found. Verify with unit tests: `http://127.0.0.1:8000/index.html` becomes an `<a href>`; a URL containing `_` stays one link; inline code, `[site](https://…)`, and `<https://…>` are unchanged; and existing `markdown_extensions` still passes.
- [x] 1.3 Add a test in `tests/site.rs` for the property-value scenario and the page-link scenario (a URL next to `[[redis]]` renders both links, and nothing has `target=`). Verify with `cargo test`.

## 2. Styling

- [x] 2.1 In `static/style.css`, add an external-link marker for links in `.block-content` and `.props dd` whose `href` starts with `http://` or `https://`, drawn with a `currentColor` mask of Lucide's `external-link` icon. Verify that a build of `wiki/` shows the marker on the URLs in `wiki/index.md` in light and dark themes, but not on page links, tags, or the footer's link.
- [x] 2.2 Confirm the "No external resources" scenario still holds: no page refers to a URL with a scheme outside note content and the footer (a data URI doesn't count). Verify with the existing `tests/site_whole.rs` / `tests/end_to_end.rs` checks.

## 3. Documentation

- [x] 3.1 In `src/help/syntax.txt`, note that bare `http://`/`https://` URLs become links, keeping lines within 80 characters. Verify with `cargo test --test help`.
