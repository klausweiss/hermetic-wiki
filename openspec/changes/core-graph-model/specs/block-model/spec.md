# Spec Delta

## Purpose

Defines how a page's markdown text is split into a tree of blocks, the unit of backlink context, together with block properties, block ids, and task markers.

## ADDED Requirements

### Requirement: Bullet blocks
A line whose first non-whitespace characters are `- ` (or a line consisting only of `-`) SHALL start a new block. The block's content SHALL be the text after the bullet marker plus all following lines up to the next bullet line or the end of the file. A block's content SHALL be preserved exactly as written, with indentation relative to the block's content column removed.

#### Scenario: Single-line blocks
- **WHEN** a page contains `- first` and `- second` on consecutive lines
- **THEN** the page has two top-level blocks with contents `first` and `second`

#### Scenario: Multi-line block
- **WHEN** a page contains `- first line` followed by an indented line `  more text`
- **THEN** the page has one block whose content is `first line` and `more text` on two lines

### Requirement: Block nesting
Nesting SHALL be determined by the indentation column of the bullet marker, with a tab counting as advancing to the next multiple of 4 columns. A bullet indented further than the previous bullet SHALL be a child of it; otherwise it SHALL be a child of the nearest preceding bullet with a smaller indentation, or top-level if none exists.

#### Scenario: Children and siblings
- **WHEN** a page contains `- a`, `  - b`, `    - c`, `  - d`, `- e`
- **THEN** `a` and `e` are top-level, `b` and `d` are children of `a`, and `c` is a child of `b`

#### Scenario: Tabs and spaces
- **WHEN** one page indents children with tabs and another with 2 spaces
- **THEN** both produce the same block tree

#### Scenario: Irregular indentation
- **WHEN** a page contains `- a`, `      - b`, `  - c`
- **THEN** `b` and `c` are both children of `a`

### Requirement: Code fences are opaque
Lines inside a fenced code block (opened by a line of three or more backticks or tildes within a block's content and closed by a matching fence) SHALL belong to the enclosing block's content and SHALL NOT start blocks, define properties, or contain references.

#### Scenario: Bullet inside code fence
- **WHEN** a block contains a fenced code block whose lines include `- not a block` and `[[not a link]]`
- **THEN** no new block is created and no reference is recorded from the fence

### Requirement: Content outside bullets is never lost
Non-blank text that is not part of any bullet block (text before the first bullet, or a non-indented non-bullet line after a block) SHALL become a top-level block of its own, a run of consecutive such lines forming one block. No text of a source file SHALL be dropped by parsing.

#### Scenario: Plain prose page
- **WHEN** a page contains two paragraphs of prose with no bullets
- **THEN** the page has two top-level blocks, one per paragraph

#### Scenario: Preamble before bullets
- **WHEN** a page starts with a prose line followed by `- a`
- **THEN** the page has a top-level block for the prose line followed by the block `a`

### Requirement: Block properties
Lines of the form `key:: value` that immediately follow a block's first line, or form the block's first lines, SHALL be properties of that block. A key SHALL consist of ASCII letters, digits, `-`, and `_`, and SHALL be matched case-insensitively. Property lines SHALL remain part of the block's preserved source but SHALL be exposed separately from the block's display content. References inside property values SHALL count as references of the block.

#### Scenario: Property after first line
- **WHEN** a block is `- decided to use JWT` followed by `  status:: accepted`
- **THEN** the block has the property `status` = `accepted` and display content `decided to use JWT`

#### Scenario: Property-looking line later in the block
- **WHEN** a block has a first line, a plain text line, and then a line `note:: x`
- **THEN** `note:: x` is ordinary content, not a property

### Requirement: Block ids
A block's `id` property SHALL give the block a graph-wide id. An id SHALL be any non-empty string without whitespace, `(`, or `)`, compared case-sensitively. An `id` value that does not meet these rules SHALL be reported as a diagnostic and ignored.

#### Scenario: Human-readable id
- **WHEN** a block has the property `id:: auth-ttl-bug`
- **THEN** the block can be referenced as `((auth-ttl-bug))`

#### Scenario: Invalid id
- **WHEN** a block has the property `id:: two words`
- **THEN** a diagnostic is reported and the block has no id

### Requirement: Task markers
A block whose display content starts with `TODO `, `DOING `, or `DONE ` (uppercase, followed by a space or end of line) SHALL be a task in that state. `TODO` and `DOING` SHALL be open states; `DONE` SHALL be a closed state. The marker SHALL be exposed separately from the remaining content.

#### Scenario: Open task
- **WHEN** a block is `- TODO rotate signing keys`
- **THEN** the block is a task in state TODO with content `rotate signing keys`

#### Scenario: Not a task
- **WHEN** a block is `- todo later` or `- TODOS for today`
- **THEN** the block is not a task

### Requirement: Source positions
Every block SHALL record the file and line range it was parsed from, so that diagnostics and later features can point to it and the source text can be reproduced exactly.

#### Scenario: Diagnostic location
- **WHEN** a block on line 12 of `journals/2026-10-12.md` contains a dangling block ref
- **THEN** the diagnostic names that file and line 12
