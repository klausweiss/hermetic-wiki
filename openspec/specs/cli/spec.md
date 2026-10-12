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
`hermetic --help` (or no arguments) SHALL print usage information and exit with status 0. `hermetic --version` SHALL print the version and exit with status 0. An unknown command or wrong number of arguments SHALL print usage to standard error and exit with status 2.

#### Scenario: Unknown command
- **WHEN** the user runs `hermetic frobnicate`
- **THEN** usage is printed to standard error and the exit status is 2
