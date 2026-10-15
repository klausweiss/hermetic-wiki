# Tasks

## 1. Baseline

- [x] 1.1 Build `wiki/` and `tests/fixtures/notes` into the scratchpad with the current binary, so later output can be diffed against it; verify both output trees exist

## 2. Address type

- [x] 2.1 Add `Address` to `src/layout.rs` with `from_source_path`, `from_html_path`, `name`, `source_path` and `html_path`, covering the home page, `pages/index` rename, journal file stems (both separators, invalid-date fallback) and named pages; verify with unit tests in `layout.rs` for each case, including round trips `from_html_path(html_path(a)) == a`
- [x] 2.2 Implement `journal_html_path`, `journal_file_path` and `page_file_path` through `Address` or delete them if no caller remains; verify `cargo build` with no warnings

## 3. Callers

- [x] 3.1 `layout::scan`: name files via `Address::from_source_path`, keeping the `InvalidJournalName` diagnostic; reduce `graph::reserve_home_name` to emitting `ReservedName`; verify `cargo test --test source_layout --test reference_graph`
- [x] 3.2 `graph::resolve_link_path`: replace the final file → name step (root `index.md`, `pages/`, `journals/` branches) with `Address::from_source_path`; verify `cargo test --test reference_graph`
- [x] 3.3 `site::paths::output_path`: use `Address::html_path` for journal days and pages without a file, keeping the file-derived path for file-backed pages; verify `cargo test --test site --test site_whole`
- [x] 3.4 `serve::edit::resolve` and `source_path`: use `Address::from_html_path` and `Address::source_path`, removing the inline round-trip check; verify `cargo test --test edit --test serve`

## 4. Integration

- [x] 4.1 Rebuild both trees from 1.1 and verify `diff -r` reports no differences; verify `cargo test` and `cargo clippy --all-targets` pass with no warnings
