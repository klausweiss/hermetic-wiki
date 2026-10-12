//! Diagnostics: problems found while loading a notes directory.
//!
//! Diagnostics never abort loading; they are collected and returned next to the graph.

use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticKind {
    UnreadableFile,
    InvalidUtf8,
    InvalidJournalName,
    CollidingFiles,
    InvalidPageName,
    InvalidBlockId,
    DuplicateBlockId,
    DanglingBlockRef,
    BrokenLink,
    LinkOutsideRoot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    /// Path relative to the notes root.
    pub file: PathBuf,
    /// 1-based line number, when the problem is tied to a block.
    pub line: Option<u32>,
    pub message: String,
}

impl Diagnostic {
    pub fn new(
        kind: DiagnosticKind,
        file: impl Into<PathBuf>,
        line: Option<u32>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            file: file.into(),
            line,
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.file.display())?;
        if let Some(line) = self.line {
            write!(f, ":{line}")?;
        }
        write!(f, ": {}", self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_file_line_and_message() {
        let d = Diagnostic::new(
            DiagnosticKind::DanglingBlockRef,
            "journals/2026-10-12.md",
            Some(12),
            "unknown block id `x`",
        );
        assert_eq!(
            d.to_string(),
            "journals/2026-10-12.md:12: unknown block id `x`"
        );

        let d = Diagnostic::new(DiagnosticKind::InvalidUtf8, "pages/a.md", None, "not UTF-8");
        assert_eq!(d.to_string(), "pages/a.md: not UTF-8");
    }
}
