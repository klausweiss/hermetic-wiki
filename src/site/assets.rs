//! hermetic's own embedded files, and copying the notes' attachments.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::diag::{Diagnostic, DiagnosticKind};
use crate::layout;

/// Files written under `_hermetic/` in every build.
pub const EMBEDDED: &[(&str, &[u8])] = &[
    (
        "_hermetic/style.css",
        include_bytes!("../../static/style.css"),
    ),
    // Compiled from static/ts/ by `make -C static build`; committed so building
    // hermetic needs no JavaScript tooling.
    (
        "_hermetic/app.js",
        include_bytes!("../../static/dist/app.js"),
    ),
    (
        "_hermetic/fonts/InterVariable.woff2",
        include_bytes!("../../static/fonts/InterVariable.woff2"),
    ),
    (
        "_hermetic/fonts/InterVariable-Italic.woff2",
        include_bytes!("../../static/fonts/InterVariable-Italic.woff2"),
    ),
    (
        "_hermetic/fonts/JetBrainsMono-Regular.woff2",
        include_bytes!("../../static/fonts/JetBrainsMono-Regular.woff2"),
    ),
    (
        "_hermetic/fonts/JetBrainsMono-Italic.woff2",
        include_bytes!("../../static/fonts/JetBrainsMono-Italic.woff2"),
    ),
    (
        "_hermetic/fonts/JetBrainsMono-Bold.woff2",
        include_bytes!("../../static/fonts/JetBrainsMono-Bold.woff2"),
    ),
    (
        "_hermetic/fonts/LICENSE-Inter.txt",
        include_bytes!("../../static/fonts/LICENSE-Inter.txt"),
    ),
    (
        "_hermetic/fonts/LICENSE-JetBrainsMono.txt",
        include_bytes!("../../static/fonts/LICENSE-JetBrainsMono.txt"),
    ),
];

/// Lists attachments (relative paths) to publish, skipping symlinks and files whose
/// output path is already taken by a generated page, with a diagnostic for each.
pub fn attachments(
    notes: &Path,
    generated: &HashSet<PathBuf>,
    diags: &mut Vec<Diagnostic>,
) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    // Every input directory: their non-markdown files are attachments.
    for dir in layout::INPUT_DIRS {
        collect(notes, Path::new(dir), &mut files, diags)?;
    }
    files.sort();
    let mut kept = Vec::with_capacity(files.len());
    for rel in files {
        if generated.contains(&rel) {
            diags.push(Diagnostic::new(
                DiagnosticKind::AttachmentConflict,
                rel.clone(),
                None,
                "attachment has the same path as a generated page; skipped",
            ));
        } else {
            kept.push(rel);
        }
    }
    Ok(kept)
}

fn collect(
    notes: &Path,
    rel: &Path,
    out: &mut Vec<PathBuf>,
    diags: &mut Vec<Diagnostic>,
) -> io::Result<()> {
    let entries = match fs::read_dir(notes.join(rel)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.starts_with('.') {
            continue;
        }
        let child = rel.join(name);
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            if !layout::is_markdown(name) {
                diags.push(Diagnostic::new(
                    DiagnosticKind::SkippedSymlink,
                    child,
                    None,
                    "symbolic links are not copied into the site",
                ));
            }
        } else if file_type.is_dir() {
            collect(notes, &child, out, diags)?;
        } else if file_type.is_file() && !layout::is_markdown(name) {
            out.push(child);
        }
    }
    Ok(())
}
