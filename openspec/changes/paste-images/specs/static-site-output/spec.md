# Spec Delta

## ADDED Requirements

### Requirement: Attachment links
An inline markdown image or link (`![alt](dest)` or `[text](dest)`) in block content or a property value SHALL be rendered with its destination rewritten when the destination has no URL scheme, does not start with `#` or `//`, does not point at a note (`.md`), and stays inside the notes folder. A destination starting with `/` SHALL mean the notes root; any other destination SHALL be resolved against the folder of the block's source file, after percent-decoding. The rewritten destination SHALL be the relative href from the page being rendered to that file's path in the output, percent-encoded as other hrefs between generated files are, keeping any `#fragment`. This SHALL apply wherever a block is rendered: its own page, linked references and their continuation pages, block references, related pages, the timeline, and the HTML used by sidebar panes. The destination SHALL be rewritten whether or not the file exists. A destination that leaves the notes folder, reference-style links, and HTML elements written in the note SHALL be left as written.

#### Scenario: Journal image in linked references
- **WHEN** `journals/2026_10_17.md` contains `- [[auth]] ![flow](../assets/flow.png)` and `assets/flow.png` exists
- **THEN** the image's `src` is `../assets/flow.png` on `journals/2026-10-17.html`, and `../assets/flow.png` in the linked references of `pages/auth.html`

#### Scenario: Continuation page
- **WHEN** that block is listed on `refs/pages/auth/2.html`
- **THEN** the image's `src` is `../../../assets/flow.png`

#### Scenario: Namespaced page
- **WHEN** `pages/Project X/Auth.md` contains `![seq](../../assets/seq.png)` and a journal block embeds that block with `((…))`
- **THEN** the image displays on `pages/Project X/Auth.html` and on the journal's page

#### Scenario: Image next to its note
- **WHEN** `pages/auth.md` contains the block `- ![flow](auth-flow.png)` with `id:: flow`, and `journals/2026_10_17.md` contains `- see ((flow))`
- **THEN** the image's `src` is `auth-flow.png` on `pages/auth.html` and `../pages/auth-flow.png` on `journals/2026-10-17.html`

#### Scenario: Root-relative destination
- **WHEN** `pages/auth.md` contains `![logo](/assets/logo.png)`
- **THEN** the image's `src` on `pages/auth.html` is `../assets/logo.png`, which also works when the site is opened via `file://`

#### Scenario: Left as written
- **WHEN** a block contains `![x](https://example.com/x.png)`, `![y](data:image/png;base64,…)`, and `![z](../../../outside.png)` from `pages/auth.md`
- **THEN** all three destinations are rendered exactly as written
