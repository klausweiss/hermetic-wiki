# cli Specification

## Purpose

Defines the `hermetic` executable: its commands, zero-configuration behavior, output-directory safety, reporting of diagnostics, and the guarantee that everything it needs is built in.

## Requirements

### Requirement: Single self-contained executable
`hermetic` SHALL be a single executable that contains every asset it writes to its output (stylesheets, fonts, license texts). Building SHALL NOT read any configuration file and SHALL NOT make network requests.

#### Scenario: Build without network access
- **WHEN** `hermetic build notes` runs on a machine with networking disabled
- **THEN** the build completes and produces the same output as with networking enabled

#### Scenario: No configuration
- **WHEN** a notes directory contains only `journals/` and `pages/`
- **THEN** `hermetic build` succeeds without any configuration file or flag

### Requirement: Build command
`hermetic build <dir> [<out>]` SHALL build the notes directory `<dir>` into the output directory `<out>`, which SHALL default to `site` in the current working directory. When `<dir>` does not exist or is not a directory, the command SHALL print an error and exit with status 1 without writing anything.

#### Scenario: Default output directory
- **WHEN** the user runs `hermetic build notes` in `/work`
- **THEN** the site is written to `/work/site`

#### Scenario: Explicit output directory
- **WHEN** the user runs `hermetic build notes /tmp/wiki`
- **THEN** the site is written to `/tmp/wiki`

#### Scenario: Missing notes directory
- **WHEN** the user runs `hermetic build nope` and `nope` does not exist
- **THEN** an error is printed, the exit status is 1, and no output directory is created

### Requirement: Output directory safety
The build SHALL write a marker file `.hermetic-site` into every output directory it creates. It SHALL replace an existing output directory only if that directory is empty or contains the marker file. It SHALL refuse, with exit status 1 and without modifying anything, when the output directory exists, is non-empty, and has no marker; when the output directory is the notes directory or contains it; or when the output directory is inside the notes directory's `journals/`, `pages/`, or `assets/`. A failed build SHALL leave a previously built output directory unchanged.

#### Scenario: Rebuilding replaces the previous output
- **WHEN** `site/` was produced by an earlier build and a page has since been deleted from the notes
- **THEN** after rebuilding, `site/` no longer contains that page's HTML

#### Scenario: Foreign directory is not touched
- **WHEN** `site/` exists, contains `report.pdf`, and has no `.hermetic-site` marker
- **THEN** the build refuses with exit status 1 and `site/report.pdf` still exists

#### Scenario: Output would contain the notes
- **WHEN** the user runs `hermetic build notes .` from the directory that contains `notes/`
- **THEN** the build refuses with exit status 1

#### Scenario: Output inside the notes root but outside note directories
- **WHEN** the user runs `hermetic build .` inside the notes directory
- **THEN** the site is written to `./site`, and later builds do not treat its files as notes

### Requirement: Diagnostics reporting
The build SHALL print every diagnostic from loading the notes to standard error, one per line, including the file and line when known, followed by a one-line summary of pages written and diagnostics reported. Diagnostics SHALL NOT make the build fail: it SHALL exit with status 0 when the site was written.

#### Scenario: Broken link reported, build succeeds
- **WHEN** a note contains `((missing))`
- **THEN** standard error contains a line naming the file, line, and missing id, the site is written, and the exit status is 0

### Requirement: Usage and version
`hermetic --help`, `hermetic -h`, or no arguments SHALL print short usage information, including a line pointing to `hermetic help` for the full reference, and exit with status 0. `hermetic --version` SHALL print the version and exit with status 0. An unknown command or wrong number of arguments SHALL print usage to standard error and exit with status 2.

#### Scenario: Unknown command
- **WHEN** the user runs `hermetic frobnicate`
- **THEN** usage is printed to standard error and the exit status is 2

#### Scenario: Short usage points to the reference
- **WHEN** the user runs `hermetic --help`
- **THEN** the usage lists `hermetic help` and is shorter than the output of `hermetic help`

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
