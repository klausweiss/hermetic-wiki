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

## MODIFIED Requirements

### Requirement: Works offline and without JavaScript
Every generated page SHALL be fully usable when opened directly from disk (`file://`) with JavaScript disabled: all content, navigation, linked references, related pages, and tasks SHALL be present in the HTML. Pages SHALL NOT load any resource from outside the output directory, and all links between generated files SHALL be relative. Pages MAY load hermetic's own bundled script from `_hermetic/`; nothing on a page SHALL depend on it to be shown or reached.

#### Scenario: Opened from disk
- **WHEN** a generated page is opened via `file://` in a browser with JavaScript disabled
- **THEN** its content, linked references, and navigation links work, and stylesheets and fonts load

#### Scenario: No external resources
- **WHEN** any generated HTML or CSS file is inspected
- **THEN** it references no URL with a scheme or a protocol-relative URL, except links written by the author in note content, the site footer's link to hermetic's project page, and `data:` URLs, which carry their content inline

#### Scenario: Only hermetic's own script
- **WHEN** any generated HTML file is inspected
- **THEN** its only `<script>` element loads `_hermetic/app.js` via a relative URL, and it has no inline scripts
