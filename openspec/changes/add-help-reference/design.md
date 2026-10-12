# Design

## Context

`src/main.rs` matches arguments by hand (no argument-parsing crate) and holds a short `USAGE` constant; `help` is currently matched together with `--help`/`-h`. The rules the reference must explain live in the `source-layout`, `block-model`, `reference-graph`, `static-site-output`, `serve-mode`, and `cli` specs. The `search` change is in flight and not implemented; per the spec the reference describes only implemented behavior.

## Goals / Non-Goals

**Goals:**
- One place in the source tree holds the reference text, readable and editable as prose.
- The reference's examples are checked by tests, so the text cannot quietly drift from the parser and graph.

**Non-Goals:**
- Man pages, shell completions, `--help` per subcommand, or a pager.
- Rendering the reference into the generated site.
- Localization.

## Decisions

**Text in plain files, embedded with `include_str!`.** Each topic is a file `src/help/<topic>.txt` (`commands`, `layout`, `syntax`, `pages`), and a small `src/help.rs` module exposes the topic list and a lookup by name. Prose is far easier to edit as a text file than as an escaped Rust string, and `include_str!` keeps the binary self-contained. *Alternative:* one large `const` in `main.rs` like `USAGE` — rejected; it would be several hundred lines of string literal and hard to review. The module lives in the library crate so integration tests can use it directly.

**Section headings.** Each file starts with an uppercase heading line (`COMMANDS`, `NOTES LAYOUT`, `SYNTAX`, `PAGES AND REFERENCES`) followed by its body; the full reference is the four files joined by a blank line, preceded by a one-line title and followed by nothing else. `hermetic help <topic>` prints one file unchanged. Topic names are matched exactly (lowercase); there are no aliases.

**A worked example that tests build.** The `pages` section ends with a small example notes directory — a journal day, `pages/auth.md`, and `index.md` — showing what pages it produces (an implicit page, linked references with a breadcrumb, an inherited task, a co-mention). The example files are stored as real files under `src/help/example/` and spliced into the section text, indented, when the section is printed, so the text the user reads and the files the tests build are the same bytes. A test loads `src/help/example/` as a notes directory and asserts every claim the section makes about it. *Alternative:* parse code examples out of the prose — rejected as brittle. The short one-line examples in `syntax` (e.g. `#[[Project X]]` references `Project X`) are covered by a table-driven test that feeds each example line to the parser and checks the stated reference; the test keeps that table next to a comment pointing at `syntax.txt`.

**Width and control-sequence checks are tests, not runtime wrapping.** The text is hand-wrapped at ≤80 columns; a test asserts every line of `hermetic help` fits and contains no `\x1b`. Runtime wrapping to the terminal width would need terminal detection and add code for little gain.

**Dispatch.** `main.rs` gains `["help"]` → full reference and `["help", topic]` → one topic or exit 2 with `error: unknown help topic `<x>` (topics: commands, layout, syntax, pages)`. `help` with more arguments falls into the existing wrong-arguments path, so it prints usage with status 2. `USAGE` gains `hermetic help [<topic>]   Show the full reference (syntax, layout, pages)`. The `commands` section repeats and expands on `USAGE` (defaults, port fallback, `--public` warning, output-directory safety rules) rather than including it, since its purpose is different.

## Risks / Trade-offs

- [Reference drifts as features change (e.g. when `search` lands)] → The examples are tested; for the rest, add a checklist item to future changes' tasks when they change user-visible syntax or layout. The `search` change's tasks should gain "update `hermetic help`" when it is applied.
- [`hermetic help` stops showing the short usage] → Low impact: the full reference begins with the `commands` section, so the usage information is still the first thing printed.
- [Long output scrolls off the terminal] → Topics give short entry points; users can pipe to a pager.
