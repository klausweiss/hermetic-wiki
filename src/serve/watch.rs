//! Change detection by polling: snapshots of the notes' inputs, compared over time.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime};

use crate::layout;

/// Size and modification time of every input file, by path relative to the notes root.
pub type Snapshot = BTreeMap<PathBuf, (u64, Option<SystemTime>)>;

/// Records every file that can affect the site: everything under the input directories
/// (notes and attachments) and the root `index.md`. Dot-entries are ignored, like the
/// build does.
pub fn snapshot(notes: &Path) -> Snapshot {
    let mut out = Snapshot::new();
    for dir in layout::INPUT_DIRS {
        walk(notes, Path::new(dir), &mut out);
    }
    record(notes, Path::new(layout::ROOT_INDEX), &mut out);
    out
}

fn record(notes: &Path, rel: &Path, out: &mut Snapshot) {
    if let Ok(meta) = fs::metadata(notes.join(rel))
        && meta.is_file()
    {
        out.insert(rel.to_path_buf(), (meta.len(), meta.modified().ok()));
    }
}

fn walk(notes: &Path, rel: &Path, out: &mut Snapshot) {
    let Ok(entries) = fs::read_dir(notes.join(rel)) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let child = rel.join(&name);
        match fs::metadata(notes.join(&child)) {
            Ok(meta) if meta.is_dir() => walk(notes, &child, out),
            Ok(meta) if meta.is_file() => {
                out.insert(child, (meta.len(), meta.modified().ok()));
            }
            _ => {}
        }
    }
}

/// How often the notes are scanned, and how long they must stay unchanged after a
/// change before a rebuild starts (so a burst of saves causes one rebuild).
#[derive(Debug, Clone, Copy)]
pub struct Timing {
    pub poll: Duration,
    pub quiet: Duration,
}

impl Default for Timing {
    fn default() -> Self {
        Timing {
            poll: Duration::from_millis(250),
            quiet: Duration::from_millis(150),
        }
    }
}

/// Blocks until the inputs differ from `current` (which saves from the editor update
/// while this waits), then until they have stayed the same for the quiet period.
/// Returns the settled snapshot.
pub fn wait_for_change(notes: &Path, current: &Mutex<Snapshot>, timing: Timing) -> Snapshot {
    let mut latest = loop {
        thread::sleep(timing.poll);
        let now = snapshot(notes);
        if now != *current.lock().expect("snapshot lock") {
            break now;
        }
    };
    loop {
        thread::sleep(timing.quiet);
        let now = snapshot(notes);
        if now == latest {
            return latest;
        }
        latest = now;
    }
}
