# Design

## Context

See proposal.md - Why. The rules being centralized:

- **Home page**: the root `index.md` is the page `index`, served at `index.html`. Without that file, `index` is still the home page.
- **`pages/index.md`**: shown as the page `index (in pages)` with a `ReservedName` warning. `scan` names it `index` first, and `reserve_home_name` renames it afterwards. `resolve_link_path` renames it again for links, and `edit::resolve` refuses the name.
- **Journals**: `journals/YYYY_MM_DD.md` or `journals/YYYY-MM-DD.md` is a journal day, always served at `journals/YYYY-MM-DD.html`. Any other file name under `journals/` becomes a regular page. That fallback is written in both `layout::scan` and `graph::resolve_link_path`.
- **Named pages**: `pages/<percent-encoded segments>.md`; the name is decoded and NFC-normalized.

## Goals / Non-Goals

**Goals:**
- Each rule above is written once, in `layout.rs`.
- Every forward/reverse pair is defined next to each other, so `edit::resolve` no longer needs its round-trip check.

**Non-Goals:**
- Changing the home-page check scattered through rendering (`is_implicit() && is_home_name`, `path == "index.html"`). The review raised it as a separate issue.
- Resolving relative paths in `resolve_link_path`: `..`, links outside the notes, and undecodable destinations stay in `graph.rs`.
- Merging the percent-decoders or the directory walkers.

## Decisions

**An enum over the three forms.**

```rust
pub enum Address { Home, Journal(Date), Page(String) }

impl Address {
    pub fn from_source_path(rel: &Path) -> Option<Address>;
    pub fn from_html_path(rel: &str) -> Option<Address>;
    pub fn name(&self) -> Cow<'_, str>;
    pub fn source_path(&self) -> PathBuf;  // where a new file is written
    pub fn html_path(&self) -> PathBuf;    // canonical HTML path
}
```

The alternative was free functions per direction, which is roughly what exists now. The enum makes the cases exhaustive, so a new form fails to compile in every caller.

**The HTML path of a page with a file comes from that file, not from `Address`.** `output_path` maps an existing file's path to `.html` (`md_to_html`), and the result can differ from the canonical path. For example, `pages/Zu\u{308}rich.md` (decomposed form, NFD) is served at that NFD path while the page name is NFC (`tests/site_whole.rs:64`). A journal file with an invalid date stays under `journals/`. So `Address::html_path` and `Address::source_path` describe a page *without* a file, and `output_path` keeps its `(Some(file), _) => md_to_html(file)` branch, with journal days still taking priority. This matches `edit::resolve`, which looks up file-backed pages in the `EditMap` first and only falls back to the canonical forms.

**`from_source_path` applies the `pages/index` rename itself, and the warning stays in `graph`.** `scan` gets the final name directly, so `reserve_home_name` stops rewriting names. The warning keeps its current trigger: a file under `pages/` whose raw name (`page_name_from_path`) is a home name. It shouldn't test the resulting name, because a file literally named `index (in pages).md` must not get the warning.

**`from_source_path` returns `Address::Page` for invalid journal names.** It doesn't decide whether to warn: `scan` still emits `InvalidJournalName` when a file under `journals/` didn't produce `Address::Journal`.

## Risks / Trade-offs

- [The output differs from today in an edge case the tests don't cover] → Compare `hermetic build` output for `wiki/` and `tests/fixtures/notes` before and after with `diff -r`, in addition to `cargo test`.
- [`from_html_path` loses `resolve`'s exact-path check] → It keeps the same guard: build the canonical path back from the decoded name and require it to equal the input. That check now lives in `layout.rs` next to the forward mapping.
