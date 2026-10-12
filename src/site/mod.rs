//! Building the static HTML site from a notes directory.

mod assets;
mod html;
mod pages;
mod paths;
mod render;

/// Percent-encodes one path segment the way generated links do.
pub use paths::encode as encode_url_segment;
/// Output path of a page's HTML file, relative to the site root.
pub use paths::output_path as page_output_path;

use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::diag::Diagnostic;
use crate::graph::Graph;
use crate::layout;

/// Name of the file marking a directory as hermetic output.
pub const MARKER: &str = ".hermetic-site";

#[derive(Debug)]
pub struct BuildReport {
    /// Absolute path of the output directory.
    pub out: PathBuf,
    /// Number of page HTML files (excluding site-wide and continuation files).
    pub pages: usize,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub enum BuildError {
    NotADirectory(PathBuf),
    UnsafeOutput { out: PathBuf, reason: &'static str },
    Io { context: String, error: io::Error },
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::NotADirectory(p) => write!(f, "{} is not a directory", p.display()),
            BuildError::UnsafeOutput { out, reason } => {
                write!(f, "refusing to write to {}: {reason}", out.display())
            }
            BuildError::Io { context, error } => write!(f, "{context}: {error}"),
        }
    }
}

impl std::error::Error for BuildError {}

fn io_err(context: impl Into<String>) -> impl FnOnce(io::Error) -> BuildError {
    let context = context.into();
    move |error| BuildError::Io { context, error }
}

/// The site title: the notes directory's name.
pub fn title_for(notes: &Path) -> String {
    notes
        .file_name()
        .map_or_else(|| "notes".to_owned(), |n| n.to_string_lossy().into_owned())
}

/// Builds the notes directory `notes` into the output directory `out`.
pub fn build(notes: &Path, out: &Path) -> Result<BuildReport, BuildError> {
    let notes = match fs::canonicalize(notes) {
        Ok(p) if p.is_dir() => p,
        _ => return Err(BuildError::NotADirectory(notes.to_path_buf())),
    };
    let out = absolute(out).map_err(io_err(format!("cannot resolve {}", out.display())))?;
    check_output(&notes, &out)?;

    let (graph, mut diagnostics) = Graph::load(&notes);
    let title = title_for(&notes);

    let name = out.file_name().unwrap_or_default().to_string_lossy();
    let tmp = out.with_file_name(format!("{name}.hermetic-tmp-{}", std::process::id()));
    if tmp.exists() {
        fs::remove_dir_all(&tmp).map_err(io_err(format!("cannot remove {}", tmp.display())))?;
    }
    let rendered = render(&graph, &title, &notes, &mut diagnostics)
        .map_err(io_err("cannot read attachments"))?;
    write_files(&rendered.files, &tmp).inspect_err(|_| {
        let _ = fs::remove_dir_all(&tmp);
    })?;
    let pages = rendered.pages;
    if out.exists() {
        fs::remove_dir_all(&out).map_err(io_err(format!("cannot replace {}", out.display())))?;
    }
    fs::rename(&tmp, &out).map_err(io_err(format!("cannot move output into {}", out.display())))?;
    Ok(BuildReport {
        out,
        pages,
        diagnostics,
    })
}

/// One file of a rendered site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    /// Generated or embedded content.
    Bytes(Vec<u8>),
    /// An attachment, read from this absolute path when written or served.
    File(PathBuf),
}

/// A rendered site: every output file by its path relative to the site root.
pub type SiteFiles = BTreeMap<PathBuf, Content>;

/// A site rendered in memory.
#[derive(Debug)]
pub struct Rendered {
    pub files: SiteFiles,
    /// Number of page HTML files (excluding site-wide and continuation files).
    pub pages: usize,
}

/// Renders the site for an already loaded graph of the notes at `notes` (absolute).
/// Attachment problems are added to `diagnostics`.
pub fn render(
    graph: &Graph,
    title: &str,
    notes: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> io::Result<Rendered> {
    let mut files = SiteFiles::new();
    let mut add =
        |path: PathBuf, html: String| files.insert(path, Content::Bytes(html.into_bytes()));

    let site = render::Site::new(graph, title);
    let mut pages = 0;
    for (id, page) in graph.pages() {
        // Without `index.md`, the page `index` is shown by the default home page.
        if page.is_implicit() && layout::is_home_name(&page.name) {
            continue;
        }
        for file in pages::page_files(&site, id) {
            add(file.path, file.html);
        }
        pages += 1;
    }
    let mut site_files = vec![pages::journal_index(&site), pages::all_pages(&site)];
    // A root `index.md` is the page `index`, already rendered as `index.html`.
    if !files.contains_key(Path::new("index.html")) {
        site_files.push(pages::default_home(&site));
    }
    site_files.extend(pages::task_files(&site));
    site_files.push(pages::search_page(&site));
    for file in site_files {
        files.insert(file.path, Content::Bytes(file.html.into_bytes()));
    }
    let info = crate::search::index::SiteInfo {
        path: &|p| site.out_path(p).to_path_buf(),
        title: &|p| graph.page(p).name.clone(),
        anchor: &|b| site.anchor(b),
    };
    for (path, bytes) in crate::search::index::build(graph, &info) {
        files.insert(path, Content::Bytes(bytes));
    }
    let generated: HashSet<PathBuf> = files.keys().cloned().collect();
    for (path, bytes) in assets::EMBEDDED {
        files.insert(PathBuf::from(path), Content::Bytes(bytes.to_vec()));
    }
    for rel in assets::attachments(notes, &generated, diagnostics)? {
        let source = notes.join(&rel);
        files.insert(rel, Content::File(source));
    }
    files.insert(
        PathBuf::from(MARKER),
        Content::Bytes(
            b"This directory was generated by hermetic and is replaced on every build.\n".to_vec(),
        ),
    );
    Ok(Rendered { files, pages })
}

/// Writes rendered files into `dir`, which must not exist.
fn write_files(files: &SiteFiles, dir: &Path) -> Result<(), BuildError> {
    fs::create_dir_all(dir).map_err(io_err(format!("cannot create {}", dir.display())))?;
    for (rel, content) in files {
        let target = dir.join(rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(io_err(format!("cannot create {}", parent.display())))?;
        }
        match content {
            Content::Bytes(bytes) => fs::write(&target, bytes).map(|_| ()),
            Content::File(source) => fs::copy(source, &target).map(|_| ()),
        }
        .map_err(io_err(format!("cannot write {}", target.display())))?;
    }
    Ok(())
}

/// Refuses output locations that could destroy notes or foreign files.
fn check_output(notes: &Path, out: &Path) -> Result<(), BuildError> {
    let refuse = |reason| {
        Err(BuildError::UnsafeOutput {
            out: out.to_path_buf(),
            reason,
        })
    };
    if notes.starts_with(out) {
        return refuse("it contains the notes directory");
    }
    for dir in layout::INPUT_DIRS {
        if out.starts_with(notes.join(dir)) {
            return refuse("it is inside a notes directory that is read as input");
        }
    }
    if out.exists() {
        if !out.is_dir() {
            return refuse("it exists and is not a directory");
        }
        let empty = fs::read_dir(out)
            .map_err(io_err(format!("cannot read {}", out.display())))?
            .next()
            .is_none();
        if !empty && !out.join(MARKER).exists() {
            return refuse("it is not empty and was not generated by hermetic");
        }
    }
    Ok(())
}

/// Absolute, normalized form of a path that may not exist yet: `.`/`..` are resolved
/// lexically, then the longest existing prefix is canonicalized and the rest appended.
fn absolute(path: &Path) -> io::Result<PathBuf> {
    let mut lexical = PathBuf::new();
    for c in std::path::absolute(path)?.components() {
        match c {
            Component::ParentDir => {
                lexical.pop();
            }
            Component::CurDir => {}
            c => lexical.push(c),
        }
    }
    let mut existing = lexical.as_path();
    let mut rest = Vec::new();
    while !existing.exists() {
        match (existing.file_name(), existing.parent()) {
            (Some(name), Some(parent)) => {
                rest.push(name.to_owned());
                existing = parent;
            }
            _ => break,
        }
    }
    let mut out = fs::canonicalize(existing)?;
    for name in rest.iter().rev() {
        out.push(name);
    }
    Ok(out)
}
