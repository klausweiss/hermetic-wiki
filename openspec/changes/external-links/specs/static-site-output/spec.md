# Spec Delta

## ADDED Requirements

### Requirement: External links
A bare URL starting with `http://` or `https://` in block content or a property value SHALL render as a link to that URL, showing the URL as written. The URL SHALL end at the first whitespace or `<`. Trailing `.`, `,`, `:`, `;`, `!`, `?`, `*`, `_`, `~`, `'`, and `"` SHALL NOT be part of it, and neither SHALL a trailing `)` that has no matching `(` in the URL. A URL inside inline code, a code block, or an existing link SHALL NOT be linked again. Every link in block content or a property value whose destination starts with `http://` or `https://` (a bare URL, an `<…>` autolink, or a markdown link) SHALL be visually distinct from links to generated pages, by a marker after its text that is shown without JavaScript. External links SHALL open the way plain links do: the page SHALL NOT set a `target` on them.

#### Scenario: Bare URL
- **WHEN** a block contains `see http://127.0.0.1:8000/index.html.`
- **THEN** the HTML links `http://127.0.0.1:8000/index.html` to that URL, and the final `.` is plain text after the link

#### Scenario: Parentheses
- **WHEN** a block contains `(docs: https://en.wikipedia.org/wiki/Rust_(programming_language))`
- **THEN** the link's destination is `https://en.wikipedia.org/wiki/Rust_(programming_language)`, and the last `)` is plain text

#### Scenario: URL in code or an existing link
- **WHEN** a block contains `` `curl https://example.com` `` and `[site](https://example.com)`
- **THEN** the code span holds no link, and the markdown link is a single link with the text `site`

#### Scenario: Distinct from page links
- **WHEN** a block contains `[[redis]]` and `https://redis.io`
- **THEN** the URL is shown with the external-link marker and the page link is not, and neither link has a `target` attribute

#### Scenario: URL in a property value
- **WHEN** a block has the property `source:: https://example.com/report`
- **THEN** the property's value links to `https://example.com/report`
