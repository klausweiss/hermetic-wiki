//! Whole pages: a note's page (with its sections and continuation files) and the
//! site-wide journal index, all-pages list, and open-tasks list.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::html::{document, esc};
use super::paths::{href, refs_continuation_path, tasks_continuation_path};
use super::render::{Mode, Site, task_badge};
use crate::graph::{BlockId, LinkedRef, PageId};
use crate::layout::Date;

/// Linked references shown per file.
pub const REFS_PER_FILE: usize = 100;
/// Open tasks shown per file of the global list.
pub const TASKS_PER_FILE: usize = 100;

/// A generated file: path relative to the output root, and its contents.
pub struct OutFile {
    pub path: PathBuf,
    pub html: String,
}

/// Renders a page's HTML file and its linked-reference continuation files.
pub fn page_files(site: &Site, page: PageId) -> Vec<OutFile> {
    let path = site.out_path(page).to_path_buf();
    let title = site.page_title(page);

    // Linked references, flattened so they can be paginated.
    let refs: Vec<(PageId, LinkedRef)> = site
        .g
        .linked_references(page)
        .into_iter()
        .flat_map(|group| group.refs.into_iter().map(move |r| (group.page, r)))
        .collect();
    let chunks: Vec<&[(PageId, LinkedRef)]> = refs.chunks(REFS_PER_FILE).collect();
    let paths = file_paths(&path, chunks.len(), |n| refs_continuation_path(&path, n));

    let mut main = String::from("<article class=\"page\">\n");
    main.push_str(&page_header(site, &path, page));
    if path == Path::new("index.html") {
        main.push_str(&home_warning(site, &path));
    }
    main.push_str(&page_body(site, &path, page));
    main.push_str(&namespace_children(site, &path, page));
    main.push_str(&open_tasks(site, &path, page));
    main.push_str(&related(site, &path, page));
    main.push_str(&format!(
        "<section class=\"linked-refs\">\n<h2>Linked references <span class=\"count\">{}</span></h2>\n",
        refs.len()
    ));
    if let Some(first) = chunks.first() {
        main.push_str(&ref_groups(site, &path, first));
        main.push_str(&pager(&path, 1, &paths));
    } else {
        main.push_str("<p class=\"empty\">No other page references this one yet.</p>\n");
    }
    main.push_str("</section>\n</article>\n");

    let mut files = vec![OutFile {
        html: document(&site.title, &path, &title, &main),
        path: path.clone(),
    }];
    for (i, chunk) in chunks.iter().enumerate().skip(1) {
        let n = i + 1;
        let file = paths[i].clone();
        let main = format!(
            "<article class=\"page\">\n<header class=\"page-header\"><h1><a href=\"{}\">{}</a> <span class=\"muted\">references {}–{} of {}</span></h1></header>\n<section class=\"linked-refs\">\n{}{}</section>\n</article>\n",
            esc(&href(&file, &path, None)),
            esc(&title),
            i * REFS_PER_FILE + 1,
            i * REFS_PER_FILE + chunk.len(),
            refs.len(),
            ref_groups(site, &file, chunk),
            pager(&file, n, &paths),
        );
        files.push(OutFile {
            html: document(
                &site.title,
                &file,
                &format!("{title} — references {n}"),
                &main,
            ),
            path: file,
        });
    }
    files
}

fn page_header(site: &Site, from: &Path, page: PageId) -> String {
    let p = site.g.page(page);
    let mut out = String::from("<header class=\"page-header\">\n");
    // Namespace ancestors, outermost first.
    let mut ancestors = Vec::new();
    let mut cur = p.ns_parent;
    while let Some(a) = cur {
        ancestors.push(a);
        cur = site.g.page(a).ns_parent;
    }
    if !ancestors.is_empty() {
        out.push_str("<nav class=\"namespace\">");
        for &a in ancestors.iter().rev() {
            out.push_str(&format!(
                "<a href=\"{}\">{}</a> / ",
                esc(&site.page_href(from, a)),
                esc(site.g.page(a).leaf_name())
            ));
        }
        out.push_str("</nav>\n");
    }
    match p.date() {
        Some(date) => {
            out.push_str(&format!(
                "<h1>{} <span class=\"weekday\">{}</span></h1>\n",
                esc(&p.name),
                date.weekday()
            ));
            out.push_str(&day_nav(site, from, date));
        }
        None => out.push_str(&format!("<h1>{}</h1>\n", esc(p.leaf_name()))),
    }
    out.push_str("</header>\n");
    out
}

/// Links to the nearest earlier and later journal days that have a file.
fn day_nav(site: &Site, from: &Path, date: Date) -> String {
    let days = &site.journal_days;
    let prev = days[..days.partition_point(|&(d, _)| d < date)]
        .last()
        .copied();
    let next = days[days.partition_point(|&(d, _)| d <= date)..]
        .first()
        .copied();
    if prev.is_none() && next.is_none() {
        return String::new();
    }
    let mut out = String::from("<nav class=\"day-nav\">");
    if let Some((d, p)) = prev {
        out.push_str(&format!(
            "<a rel=\"prev\" href=\"{}\">← {d}</a>",
            esc(&site.page_href(from, p))
        ));
    }
    if let Some((d, p)) = next {
        out.push_str(&format!(
            "<a rel=\"next\" href=\"{}\">{d} →</a>",
            esc(&site.page_href(from, p))
        ));
    }
    out.push_str("</nav>\n");
    out
}

fn page_body(site: &Site, from: &Path, page: PageId) -> String {
    let p = site.g.page(page);
    if p.is_implicit() {
        let text = if p.is_journal() {
            "There is no journal entry for this day."
        } else {
            "This page has no content of its own yet."
        };
        return format!("<p class=\"empty\">{text}</p>\n");
    }
    format!(
        "<section class=\"content\">\n{}\n</section>\n",
        site.tree(from, &p.roots, true)
    )
}

fn namespace_children(site: &Site, from: &Path, page: PageId) -> String {
    let children = site.g.namespace_children(page);
    if children.is_empty() {
        return String::new();
    }
    let mut out = String::from("<section class=\"children\">\n<h2>Pages under this one</h2>\n<ul>");
    for &c in children {
        out.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>",
            esc(&site.page_href(from, c)),
            esc(site.g.page(c).leaf_name())
        ));
    }
    out.push_str("</ul>\n</section>\n");
    out
}

/// One task line: marker, content, and a link to where it lives.
fn task_item(site: &Site, from: &Path, block: BlockId) -> String {
    let b = site.g.block(block);
    format!(
        "<li>{}<span class=\"task-content\">{}</span> <a class=\"task-source\" href=\"{}\">{}</a></li>",
        b.task.map(task_badge).unwrap_or_default(),
        site.content(from, block, Mode::Expand),
        esc(&site.block_href(from, block)),
        esc(&site.page_title(b.page))
    )
}

fn open_tasks(site: &Site, from: &Path, page: PageId) -> String {
    let tasks = site.g.open_page_tasks(page);
    if tasks.is_empty() {
        return String::new();
    }
    let mut out = format!(
        "<section class=\"open-tasks\">\n<h2>Open tasks <span class=\"count\">{}</span></h2>\n<ul class=\"tasks\">",
        tasks.len()
    );
    for t in tasks {
        out.push_str(&task_item(site, from, t));
    }
    out.push_str("</ul>\n</section>\n");
    out
}

fn related(site: &Site, from: &Path, page: PageId) -> String {
    let related = site.g.related(page);
    if related.is_empty() {
        return String::new();
    }
    // A block mentioning several related pages is listed under each of them.
    let mut bodies: HashMap<BlockId, String> = HashMap::new();
    let mut out = String::from("<section class=\"related\">\n<h2>Related</h2>\n");
    for r in related {
        out.push_str(&format!(
            "<details>\n<summary><a href=\"{}\">{}</a> <span class=\"count\">{}</span></summary>\n<ul class=\"shared\">",
            esc(&site.page_href(from, r.page)),
            esc(&site.page_title(r.page)),
            r.weight()
        ));
        for &b in r.blocks {
            out.push_str(&format!(
                "<li><a class=\"source\" href=\"{}\">{}</a>{}</li>",
                esc(&site.block_href(from, b)),
                esc(&site.page_title(site.g.block(b).page)),
                bodies.entry(b).or_insert_with(|| site.block_body(from, b))
            ));
        }
        out.push_str("</ul>\n</details>\n");
    }
    out.push_str("</section>\n");
    out
}

/// Linked references grouped by source page; a group continuing from the previous file
/// repeats its header.
fn ref_groups(site: &Site, from: &Path, refs: &[(PageId, LinkedRef)]) -> String {
    let mut out = String::new();
    for group in refs.chunk_by(|a, b| a.0 == b.0) {
        let source = group[0].0;
        out.push_str(&format!(
            "<div class=\"ref-group\">\n<h3><a href=\"{}\">{}</a></h3>\n",
            esc(&site.page_href(from, source)),
            esc(&site.page_title(source))
        ));
        for (_, r) in group {
            out.push_str(&linked_ref(site, from, r));
        }
        out.push_str("</div>\n");
    }
    out
}

fn linked_ref(site: &Site, from: &Path, r: &LinkedRef) -> String {
    let mut out = String::from("<div class=\"linked-ref\">");
    if !r.breadcrumb.is_empty() {
        out.push_str("<nav class=\"breadcrumb\">");
        let crumbs: Vec<String> = r
            .breadcrumb
            .iter()
            .map(|&a| {
                format!(
                    "<a href=\"{}\">{}</a>",
                    esc(&site.block_href(from, a)),
                    site.plain(from, a)
                )
            })
            .collect();
        out.push_str(&crumbs.join(" › "));
        out.push_str("</nav>");
    }
    out.push_str(&site.tree(from, &[r.block], false));
    out.push_str("</div>\n");
    out
}

/// Paths of a paginated list's `count` files: `first`, then `continuation(n)` for n >= 2.
fn file_paths(first: &Path, count: usize, continuation: impl Fn(usize) -> PathBuf) -> Vec<PathBuf> {
    std::iter::once(first.to_path_buf())
        .chain((2..=count).map(continuation))
        .collect()
}

/// Previous/next links between the files of a paginated list; `from` is file `n` of `paths`.
fn pager(from: &Path, n: usize, paths: &[PathBuf]) -> String {
    let total = paths.len();
    if total <= 1 {
        return String::new();
    }
    let mut out = String::from("<nav class=\"pager\">");
    if n > 1 {
        out.push_str(&format!(
            "<a rel=\"prev\" href=\"{}\">← Previous</a>",
            esc(&href(from, &paths[n - 2], None))
        ));
    }
    out.push_str(&format!("<span>{n} / {total}</span>"));
    if n < total {
        out.push_str(&format!(
            "<a rel=\"next\" href=\"{}\">Next →</a>",
            esc(&href(from, &paths[n], None))
        ));
    }
    out.push_str("</nav>\n");
    out
}

/// `journals.html`: file-backed journal days, newest first, grouped by month.
pub fn journal_index(site: &Site) -> OutFile {
    journal_list(site, PathBuf::from("journals.html"), "journals", "")
}

/// The default `index.html` when there is no `index.md`: the journal list, with a hint
/// that adding `index.md` replaces it.
/// On the home page: a warning when `pages/index.md` exists, which is not the home page.
fn home_warning(site: &Site, from: &Path) -> String {
    let Some(page) = site.g.find_page(crate::layout::PAGES_INDEX_PAGE) else {
        return String::new();
    };
    let Some(file) = &site.g.page(page).file else {
        return String::new();
    };
    format!(
        "<p class=\"warning\"><code>{}</code> is not the home page: <code>index</code> is reserved for <code>index.md</code> in the notes folder. Its content is shown as <a href=\"{}\">{}</a>; rename the file to avoid confusion.</p>\n",
        esc(&file.to_string_lossy()),
        esc(&site.page_href(from, page)),
        esc(crate::layout::PAGES_INDEX_PAGE)
    )
}

pub fn default_home(site: &Site) -> OutFile {
    journal_list(
        site,
        PathBuf::from("index.html"),
        "home",
        "<p class=\"hint\">Add an <code>index.md</code> to the notes folder to replace this page with your own home page.</p>\n",
    )
}

fn journal_list(site: &Site, path: PathBuf, view: &str, footer: &str) -> OutFile {
    let mut main = format!(
        "<article class=\"page index\" data-view=\"{view}\">\n<header class=\"page-header\"><h1>Journals</h1></header>\n",
    );
    if view == "home" {
        main.push_str(&home_warning(site, &path));
    }
    if site.journal_days.is_empty() {
        main.push_str("<p class=\"empty\">No journal entries yet. Add files like <code>journals/2026-10-12.md</code>.</p>\n");
    }
    // On the journal list, <hermetic-timeline> adds the timeline view around the list;
    // without the script it is inert and the list inside shows as usual.
    let timeline = view == "journals";
    if timeline {
        main.push_str("<hermetic-timeline>\n");
    }
    let months = site
        .journal_days
        .chunk_by(|(a, _), (b, _)| (a.year, a.month) == (b.year, b.month));
    for month in months.rev() {
        let first = month[0].0;
        main.push_str(&format!(
            "<h2>{} {}</h2>\n<ul class=\"days\">",
            first.month_name(),
            first.year
        ));
        for &(date, page) in month.iter().rev() {
            main.push_str(&format!(
                "<li><a href=\"{}\">{date}</a> <span class=\"weekday\">{}</span></li>",
                esc(&site.page_href(&path, page)),
                date.weekday()
            ));
        }
        main.push_str("</ul>\n");
    }
    if timeline {
        main.push_str("</hermetic-timeline>\n");
    }
    main.push_str(footer);
    main.push_str("</article>\n");
    OutFile {
        html: document(&site.title, &path, "Journals", &main),
        path,
    }
}

/// `search.html`: results for `?q=`, filled in by `<hermetic-search-results>`. Without
/// the script (or over `file://`) it explains what search needs.
pub fn search_page(site: &Site) -> OutFile {
    let path = PathBuf::from("search.html");
    let main = "<article class=\"page index\" data-view=\"search\">\n\
        <header class=\"page-header\"><h1>Search</h1></header>\n\
        <hermetic-search-results></hermetic-search-results>\n\
        <p class=\"search-unavailable\">Search needs JavaScript and the site served over <code>http://</code>. \
        Run <code>hermetic serve</code> in the notes folder, or host the site on a web server.</p>\n\
        </article>\n";
    OutFile {
        html: document(&site.title, &path, "Search", main),
        path,
    }
}

/// `pages.html`: every non-journal page, case-insensitively by name.
pub fn all_pages(site: &Site) -> OutFile {
    let path = PathBuf::from("pages.html");
    let mut pages: Vec<PageId> = site
        .g
        .pages()
        .filter(|(_, p)| !p.is_journal())
        .map(|(id, _)| id)
        .collect();
    pages.sort_by_cached_key(|&p| {
        let name = &site.g.page(p).name;
        (name.to_lowercase(), name.clone())
    });
    let mut main = format!(
        "<article class=\"page index\" data-view=\"pages\">\n<header class=\"page-header\"><h1>All pages <span class=\"count\">{}</span></h1></header>\n<ul class=\"all-pages\">",
        pages.len()
    );
    for p in pages {
        let page = site.g.page(p);
        main.push_str(&format!(
            "<li><a href=\"{}\">{}</a>{}</li>",
            esc(&site.page_href(&path, p)),
            esc(&page.name),
            if page.is_implicit() {
                " <span class=\"implicit\" title=\"Referenced, but has no file\">no file</span>"
            } else {
                ""
            }
        ));
    }
    main.push_str("</ul>\n</article>\n");
    OutFile {
        html: document(&site.title, &path, "All pages", &main),
        path,
    }
}

/// `tasks.html` and `tasks/<n>.html`: all open tasks, grouped by page.
pub fn task_files(site: &Site) -> Vec<OutFile> {
    let tasks = site.g.open_tasks();
    let chunks: Vec<&[BlockId]> = tasks.chunks(TASKS_PER_FILE).collect();
    let paths = file_paths(
        Path::new("tasks.html"),
        chunks.len(),
        tasks_continuation_path,
    );
    let mut files = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        let n = i + 1;
        let path = path.clone();
        let mut main = format!(
            "<article class=\"page index\" data-view=\"tasks\">\n<header class=\"page-header\"><h1>Open tasks <span class=\"count\">{}</span></h1></header>\n",
            tasks.len()
        );
        match chunks.get(n - 1) {
            None => main.push_str("<p class=\"empty\">No open tasks.</p>\n"),
            Some(chunk) => {
                let page_of = |t: &BlockId| site.g.block(*t).page;
                for group in chunk.chunk_by(|a, b| page_of(a) == page_of(b)) {
                    let page = page_of(&group[0]);
                    main.push_str(&format!(
                        "<h2><a href=\"{}\">{}</a></h2>\n<ul class=\"tasks\">",
                        esc(&site.page_href(&path, page)),
                        esc(&site.page_title(page))
                    ));
                    for &t in group {
                        main.push_str(&task_item(site, &path, t));
                    }
                    main.push_str("</ul>\n");
                }
            }
        }
        main.push_str(&pager(&path, n, &paths));
        main.push_str("</article>\n");
        files.push(OutFile {
            html: document(&site.title, &path, "Open tasks", &main),
            path,
        });
    }
    files
}
