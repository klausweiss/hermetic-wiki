# Spec Delta

## MODIFIED Requirements

### Requirement: Help reference
`hermetic help` SHALL print a reference to standard output and exit with status 0. The reference SHALL describe only behavior the executable implements, and SHALL consist of these sections, in this order, each with a heading naming its topic:

- `commands`: every command and option, with its defaults, and that pages served by `serve` can be edited in a browser on the same machine (never from other devices), with edits saved to the note files.
- `layout`: how a notes directory is read — `journals/` files named `YYYY-MM-DD.md` as journal days (and what happens to other names there), `pages/` at any depth as regular pages with subdirectories as namespaces, the root `index.md` as the page `index` and the home page (and what the home page shows without it), `assets/` and non-markdown files as attachments, which files are ignored, how page names map to file names (escaping), and that page names match case-insensitively.
- `syntax`: blocks (bullets, nesting by indentation, multi-line blocks, text outside bullets), properties (`key:: value`), block ids (`id::`), task markers (`TODO`, `DOING`, `DONE`), page links (`[[name]]`, including namespaced names and dates), tags (`#name`, `#[[multi word]]`), markdown links to `.md` files, block references (`((id))`), and where references are not recognized (inline code, fenced code, `\` escapes).
- `pages`: what each page shows and how it is collected — implicit pages for names that are referenced but have no file, linked references grouped by source (journals newest first, then pages alphabetically) with breadcrumbs, references inherited by child blocks, a page's tasks including tasks under blocks that reference it, related pages from co-mentions in one block, namespace children, the journal list, all-pages list, open-tasks, and search pages, how search matches, and that problems are reported as warnings without failing the build.

`hermetic help <topic>` SHALL print only the section for that topic and exit with status 0. An unknown topic SHALL print an error naming the valid topics to standard error, print nothing to standard output, and exit with status 2; more than one topic is a wrong number of arguments. Every line of the reference SHALL be at most 80 characters, and the output SHALL contain no terminal control sequences. Every example the reference gives of note text and its effect SHALL hold when notes containing that text are built.

#### Scenario: Full reference
- **WHEN** the user runs `hermetic help`
- **THEN** standard output contains the `commands`, `layout`, `syntax`, and `pages` sections in that order, and the exit status is 0

#### Scenario: How journals and the index are treated
- **WHEN** the user runs `hermetic help layout`
- **THEN** the output explains that `journals/YYYY-MM-DD.md` files are journal days, that the root `index.md` is the page `index` shown as the home page, and that the journal list is the home page when `index.md` is absent, and it contains no other section

#### Scenario: Syntax topic
- **WHEN** the user runs `hermetic help syntax`
- **THEN** the output shows examples of `[[name]]`, `#name`, `#[[multi word]]`, `((id))`, `key:: value`, `id::`, and `TODO`, and contains no other section

#### Scenario: Unknown topic
- **WHEN** the user runs `hermetic help frobnicate`
- **THEN** standard error names `commands`, `layout`, `syntax`, and `pages`, standard output is empty, and the exit status is 2

#### Scenario: Fits a terminal
- **WHEN** the output of `hermetic help` is inspected
- **THEN** no line is longer than 80 characters and it contains no escape characters

#### Scenario: Examples are accurate
- **WHEN** a notes directory is built from the note text of the reference's examples
- **THEN** each example produces the reference, task, property, or page the reference says it does

#### Scenario: Editing is described
- **WHEN** the user runs `hermetic help commands`
- **THEN** the output says that `serve` allows editing in a browser on the same machine only and that edits are saved to the note files
