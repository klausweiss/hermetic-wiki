# Tasks

## 1. Setup

- [ ] 1.1 Create the `hermetic` Cargo package (edition 2024, stable toolchain) with `src/lib.rs`, placeholder `src/main.rs`, and empty `layout`, `parse`, `graph`, `diag` modules; verify `cargo build` and `cargo test` succeed
- [ ] 1.2 Add `unicode-normalization` as the only runtime dependency and `proptest` as a dev-dependency; add `/target` to `.gitignore`; verify `cargo tree --edges normal` lists no other direct dependencies
- [ ] 1.3 Define `Diagnostic { kind, file, line, message }` in `diag`; verify it is constructible and printable in a unit test

## 2. Source layout

- [ ] 2.1 Implement page-name validation (empty, NUL, empty segments, `.`/`..`, leading-dot segments) with table-driven tests covering every invalid case from the `source-layout` spec
- [ ] 2.2 Implement `encode_segment`/`decode_segment` and name<->path mapping; verify the spec scenarios (`C#: a|b?`, `50% done`, `con`, `con.txt`, `etc.`, `a:b.md`) in unit tests and a proptest asserting `decode(encode(n)) == n` for valid names
- [ ] 2.3 Implement `NameKey` (trim, NFC, lowercase); verify case-insensitive and NFD/NFC-equivalent names produce equal keys
- [ ] 2.4 Implement the ISO date parser for journal file names, including leap years; verify `2024-02-29` is valid and `2023-02-29`, `2026-13-40`, `notes` are not
- [ ] 2.5 Implement the directory scan: `journals/` and `pages/**` `.md` files, ignoring dotfiles/dot-dirs, other files, and other directories; deterministic order (journals by date, then pages by path bytes); verify with a fixture directory test, including missing `journals/` or `pages/`
- [ ] 2.6 Implement file reading (UTF-8, CRLF normalization) with diagnostics for unreadable/invalid-UTF-8 files and invalid journal names; verify with fixtures that other files still load

## 3. Block parsing

- [ ] 3.1 Implement the line scanner: bullet detection, tab-to-4 column computation, indentation-stack nesting, continuation lines, relative de-indentation of content; verify the `Children and siblings`, `Tabs and spaces`, `Irregular indentation`, and multi-line scenarios
- [ ] 3.2 Add fence tracking so fenced lines never start blocks or properties; verify the `Bullet inside code fence` scenario
- [ ] 3.3 Turn non-bullet top-level text runs into prose blocks; verify the `Plain prose page` and `Preamble before bullets` scenarios and a test that concatenating all block sources reproduces the original file text exactly
- [ ] 3.4 Parse leading property lines (key charset, case-insensitive keys) and expose display content separately; verify both `Block properties` scenarios
- [ ] 3.5 Extract and validate `id::`; verify valid and invalid id scenarios produce the expected id or diagnostic
- [ ] 3.6 Detect `TODO`/`DOING`/`DONE` markers and strip them from display content; verify the task and non-task scenarios
- [ ] 3.7 Record file and line ranges per block; verify a block's reported line matches its position in a fixture

## 4. Inline references

- [ ] 4.1 Implement the inline scanner for `[[x]]` with code-span and backslash-escape skipping and no nesting/line spanning; verify the link scenarios, including inline code
- [ ] 4.2 Add `#tag` and `#[[multi word]]` recognition with the preceding-whitespace and trailing-punctuation rules; verify `#backend.`, headings, and URL fragments
- [ ] 4.3 Add `((id))` recognition; verify byte ranges of all ref kinds point at the exact source text
- [ ] 4.4 Scan property values for references too; verify a link in a property value is a reference of the block
- [ ] 4.5 Add inline markdown link recognition (`[text](dest)`, `[text](<dest>)`, skipping images and URL schemes); verify ranges and that `![x](a.md)` and `https://` links yield no raw ref

## 5. Graph resolution

- [ ] 5.1 Build the arena `Graph` from parsed files: register file-backed pages, report and resolve colliding files (first by path bytes); verify the `Case collision` scenario
- [ ] 5.2 Resolve page refs and tags by `NameKey`, creating implicit pages (first-mention display name) and implicit journal pages for date names; report invalid names; verify `Page created on demand`, `Link to a day`, `Implicit page display name`, and `Path traversal attempt`
- [ ] 5.3 Create namespace prefix pages and parent/child links; verify the namespace scenarios, including `Project X.md` next to `Project X/`
- [ ] 5.4 Build the id table with duplicate-id diagnostics and resolve `((id))` (block + its page), recording unresolved refs with diagnostics; verify the duplicate and dangling scenarios
- [ ] 5.5 Resolve markdown link destinations (percent-decode, join, lexical normalization, root confinement, file-name decoding, lookup among file-backed pages); verify the `Relative link from a journal`, `Link with encoded characters`, `Link to a missing file`, and `Link escaping the notes root` scenarios
- [ ] 5.6 Compute backlinks per page and inherited references via parent walks; verify the `Child inherits parent's reference` scenario

## 6. Queries

- [ ] 6.1 Implement linked references for a page: exclude self-page blocks, collapse nested references into the ancestor, provide breadcrumb and subtree; verify the `Breadcrumb and subtree`, `Nested references collapse`, and `Self references excluded` scenarios
- [ ] 6.2 Order linked-reference groups (journals newest first, then pages by case-insensitive name; blocks in document order); verify the `Mixed sources` scenario
- [ ] 6.3 Implement page tasks and global open tasks; verify the `Task under a referencing block` and `Global open tasks` scenarios
- [ ] 6.4 Expose namespace children per page; verify the `Listing children` scenario
- [ ] 6.5 Derive co-mentions from each block's deduplicated direct page set and expose per-page related pages with weight, recency, and shared blocks; verify all `Co-mentions` scenarios, including ordering

## 7. Integration

- [ ] 7.1 Add an end-to-end fixture notes directory (journals, namespaced pages, tags, block refs, tasks, unicode names, a broken file) and a test asserting the full graph and diagnostic list; verify two consecutive loads produce identical results
- [ ] 7.2 Add a generated-fixture test of ~5,000 journal files and verify `Graph::load` completes in under one second in release mode
- [ ] 7.3 Run `cargo clippy -- -D warnings`, `cargo fmt --check`, and `cargo test` and verify all pass
