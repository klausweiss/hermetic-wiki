#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use hermetic::graph::{BlockId, PageId};
use hermetic::site::{self, BuildReport};
use hermetic::{Diagnostic, DiagnosticKind, Graph};

/// The shared fixture notes directory.
pub const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/notes");

/// A temporary notes directory, removed on drop.
pub struct Notes {
    pub root: PathBuf,
}

impl Notes {
    pub fn new(files: &[(&str, &str)]) -> Notes {
        let notes = Notes::empty();
        for (path, content) in files {
            notes.write(path, content.as_bytes());
        }
        notes
    }

    pub fn empty() -> Notes {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "hermetic-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Notes { root }
    }

    pub fn write(&self, path: &str, content: &[u8]) {
        let full = self.root.join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, content).unwrap();
    }

    pub fn load(&self) -> (Graph, Vec<Diagnostic>) {
        Graph::load(&self.root)
    }

    pub fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for Notes {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Builds `notes` into `site/` inside a fresh temporary directory, which is returned
/// so it lives as long as the caller needs the output.
pub fn build_site(notes: &Path) -> (Notes, PathBuf, BuildReport) {
    let holder = Notes::empty();
    let out = holder.path().join("site");
    let report = site::build(notes, &out, None).expect("build succeeds");
    (holder, out, report)
}

pub fn page(g: &Graph, name: &str) -> PageId {
    g.find_page(name)
        .unwrap_or_else(|| panic!("no page named {name:?}"))
}

pub fn names(g: &Graph, pages: impl IntoIterator<Item = PageId>) -> Vec<String> {
    pages.into_iter().map(|p| g.page(p).name.clone()).collect()
}

/// The block whose content is exactly `content`.
pub fn block(g: &Graph, content: &str) -> BlockId {
    g.blocks()
        .find(|(_, b)| b.content == content)
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("no block with content {content:?}"))
}

pub fn contents(g: &Graph, blocks: impl IntoIterator<Item = BlockId>) -> Vec<String> {
    blocks
        .into_iter()
        .map(|b| g.block(b).content.clone())
        .collect()
}

pub fn kinds(diags: &[Diagnostic]) -> Vec<DiagnosticKind> {
    diags.iter().map(|d| d.kind).collect()
}

/// Every file under `dir` with its bytes, keyed by path relative to `dir`.
pub fn tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    fs::read(&path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}
