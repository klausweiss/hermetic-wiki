# Proposal

## Why

A URL written into a note, like `see http://127.0.0.1:8000/index.html`, renders as plain text. Only `<http://…>` and `[text](http://…)` become links, because the markdown renderer only supports CommonMark autolinks, not GitHub-style bare URLs. Notes collect pasted URLs all the time, so they should be clickable. Once they are, a reader should also be able to tell a link that leaves the wiki from a link to another page.

## What Changes

- Bare `http://` and `https://` URLs in block content and property values render as links, following GitHub's rules for where a URL ends (trailing punctuation and an unbalanced `)` are not part of it).
- URLs inside inline code, code blocks, and existing links are left alone.
- Links to external URLs in note content (bare URLs, `<…>` autolinks, and `[text](https://…)`) look different from links to wiki pages: they get a small "external link" marker after the text. Page links keep their `[[ ]]` look, and tags keep their colour.
- External links open in the same tab, like every other link. The page adds no `target` attribute.
- `hermetic help syntax` mentions bare URLs.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `static-site-output`: adds a requirement for rendering external links (linking bare URLs and making external links visually distinct).

## Impact

- `src/site/render.rs`: the markdown step gets a pass over the parser's events that turns bare URLs in text into links.
- `static/style.css`: styling for external links in block content and property values.
- `src/help/syntax.txt`: documents bare URLs.
- No new dependencies. Reference scanning (`src/parse.rs`) is unchanged: URLs are already ignored there, including `#fragment`s.
