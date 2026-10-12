//! hermetic: turns a directory of markdown notes into a graph of pages, blocks and references.

pub mod diag;
pub mod graph;
pub mod layout;
pub mod parse;
pub mod serve;
pub mod site;

pub use diag::{Diagnostic, DiagnosticKind};
pub use graph::Graph;
