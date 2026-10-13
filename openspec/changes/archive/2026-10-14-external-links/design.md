# Design

## Context

`render_text` (`src/site/render.rs`) replaces references with placeholders (`\u{E000}<n>\u{E001}`) and passes the text to `markdown()`. That function runs pulldown-cmark 0.13 with tables, strikethrough, task lists, and footnotes turned on, then fills the placeholders back in. pulldown-cmark has no option for linking bare URLs: `ENABLE_GFM` only adds blockquote callouts. Block content and property values both go through `render_text`, so a change in `markdown()` covers both. The editor renders nothing on the client side; the server renders everything.

## Goals / Non-Goals

**Goals:**
- Link bare `http(s)://` URLs, ending them the way GitHub does.
- Mark external links with CSS only, so the marker also shows without JavaScript and when a page is opened from a file.

**Non-Goals:**
- Linking `www.example.com` without a scheme, or email addresses (GitHub does both; add them later if wanted).
- Opening links in a new tab.
- Treating external URLs as references in the graph or the search index.

## Decisions

**Filter pulldown-cmark's events instead of rewriting the source.** Linking URLs in the markdown source before parsing would have to skip code spans, fences, link destinations and HTML by itself, which the parser already does. Instead, `markdown()` wraps the parser's events:
- Join neighbouring `Event::Text`s first. The parser splits text at characters like `_` and `*` that could start emphasis, so one URL can arrive as several pieces.
- Track depth inside `Tag::Link`, `Tag::Image`, and `Tag::CodeBlock`. Also track inline HTML `<a …>` / `</a>`, so text inside an author-written `<a>` isn't linked again. Only text outside all of these is scanned. Inline code arrives as `Event::Code` and is never scanned.
- Split each text into plain text and URLs, and emit each URL as `Start(Link)`, `Text`, `End(Link)` with `LinkType::Autolink`. pulldown's HTML writer then escapes the `href` and the text the same way it does for `<…>` autolinks.
- Alternative considered: switch to `comrak`, which supports GitHub's autolinks. Rejected because it would mean changing the whole renderer and its output for one feature.

**URL scanning.**
- A URL starts at `http://` or `https://`. If it's in the middle of a word (the character before it is alphanumeric), it isn't linked, as on GitHub.
- It runs until whitespace, `<`, or a placeholder character (`\u{E000}`/`\u{E001}`). Stopping at placeholders means a URL written right up against a `[[page]]` can never swallow it.
- Then trailing punctuation is trimmed repeatedly, as listed in the spec, and a trailing `)` is dropped while the URL has more `)` than `(`.
- A URL with nothing after the scheme isn't linked.
- This is a small pure function with unit tests.

**Styling by `href`, not by a class.** One CSS rule covers bare URLs, `<…>` autolinks, and markdown links:

```css
:is(.block-content, .props dd) a:is([href^="http://"], [href^="https://"])
```

Links to wiki pages always have relative `href`s (spec: links between generated files are relative), so the rule never matches them. The marker is the Lucide `external-link` icon, drawn with `::after` and a `mask-image` data URI coloured `currentColor`. A data URI is not a resource from outside the output directory, so the "no external resources" scenario still holds. The ↗ character is ruled out because block references already use it for "go to block".

## Risks / Trade-offs

- [Emphasis that the parser does recognize splits a URL, e.g. `https://a.com/*x*` links only up to `/`] → GitHub has the same ambiguity. Joining text pieces handles the common case, URLs containing `_`.
- [Pages store a link to every URL in the HTML, so the HTML gets a little larger] → Negligible.
- [Pages opened with an old cached stylesheet show external links without the marker] → Cosmetic only; it goes away when the stylesheet reloads.
