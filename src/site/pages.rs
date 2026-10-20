//! Whole pages: a note's page (with its sections and continuation files) and the
//! site-wide journal index, all-pages list, and open-tasks list.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::html::{description, document, esc, unescape};
use super::paths::{href, refs_continuation_path, tasks_continuation_path};
use super::render::{Mode, Site, task_badge};
use crate::graph::{BlockId, LinkedRef, PageId};
use crate::layout::{Address, Date};

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
    let first = chunks
        .first()
        .map(|first| ref_groups(site, &path, first) + &pager(&path, 1, &paths));
    main.push_str(&linked_refs(refs.len(), first.as_deref()));
    main.push_str("</article>\n");

    let mut files = vec![OutFile {
        html: document(
            &site.title,
            &path,
            &title,
            &page_description(site, &path, page),
            &main,
        ),
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
                "",
                &main,
            ),
            path: file,
        });
    }
    files
}

/// The page's description: its `description` property, or else the text of its first
/// block with any, in document order. "" when it has neither.
fn page_description(site: &Site, from: &Path, page: PageId) -> String {
    let props = site.g.page_props(page);
    if let Some(p) = props.iter().find(|p| p.key == "description") {
        return description(&p.value.replace("[[", "").replace("]]", ""));
    }
    let mut stack: Vec<BlockId> = site.g.page(page).roots.iter().rev().copied().collect();
    while let Some(b) = stack.pop() {
        let block = site.g.block(b);
        if !block.holds_page_props() {
            let text = unescape(&site.plain(from, b));
            if !text.trim().is_empty() {
                return description(&text);
            }
        }
        stack.extend(block.children.iter().rev());
    }
    String::new()
}

fn page_header(site: &Site, from: &Path, page: PageId) -> String {
    let p = site.g.page(page);
    let mut out = String::new();
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
        Some(date) => out.push_str(&journal_title(date, &day_nav(site, from, date))),
        None => out.push_str(&format!("<h1>{}</h1>\n", esc(p.leaf_name()))),
    }
    header(&out)
}

// The pieces below are shared with `blank_day`, which renders a journal day that is not
// in the graph. The editor finds its place by this markup (`header.page-header`,
// `article > p.empty`, `section.linked-refs h2 .count`), so both must use them.

fn header(inner: &str) -> String {
    format!("<header class=\"page-header\">\n{inner}</header>\n")
}

/// A journal day's title and its links to the nearest days around it.
fn journal_title(date: Date, nav: &str) -> String {
    format!(
        "<h1>{date} <span class=\"weekday\">{}</span></h1>\n{nav}",
        date.weekday()
    )
}

/// The body of a page without a file.
fn empty_body(journal: bool) -> String {
    let text = if journal {
        "There is no journal entry for this day."
    } else {
        "This page has no content of its own yet."
    };
    format!("<p class=\"empty\">{text}</p>\n")
}

/// The linked-references section: `count` references, of which `shown` is the first
/// file's HTML (`None` when there are none).
fn linked_refs(count: usize, shown: Option<&str>) -> String {
    format!(
        "<section class=\"linked-refs\">\n<h2>Linked references <span class=\"count\">{count}</span></h2>\n{}</section>\n",
        shown.unwrap_or("<p class=\"empty\">No other page references this one yet.</p>\n")
    )
}

/// Links to the nearest earlier and later journal days that have a file.
fn day_nav(site: &Site, from: &Path, date: Date) -> String {
    day_links(
        &site.journal_days,
        date,
        |(d, _)| d,
        |(_, p)| site.page_href(from, p),
    )
}

/// The day navigation from `date` among `days` (sorted by `day`); only the two
/// neighbours' links are built.
fn day_links<T: Copy>(
    days: &[T],
    date: Date,
    day: impl Fn(T) -> Date,
    link: impl Fn(T) -> String,
) -> String {
    let prev = days[..days.partition_point(|&t| day(t) < date)].last();
    let next = days[days.partition_point(|&t| day(t) <= date)..].first();
    if prev.is_none() && next.is_none() {
        return String::new();
    }
    let mut out = String::from("<nav class=\"day-nav\">");
    if let Some(&t) = prev {
        let d = day(t);
        out.push_str(&format!(
            "<a rel=\"prev\" href=\"{}\">← {d}</a>",
            esc(&link(t))
        ));
    }
    if let Some(&t) = next {
        let d = day(t);
        out.push_str(&format!(
            "<a rel=\"next\" href=\"{}\">{d} →</a>",
            esc(&link(t))
        ));
    }
    out.push_str("</nav>\n");
    out
}

/// The page of a journal day that is not in the site (no file, no references), as a day
/// referenced without a file would be shown. `days` are the journal days that have a
/// file, sorted. `hermetic serve` shows it so the day can be written in the browser.
pub fn blank_day(site_title: &str, date: Date, days: &[Date]) -> OutFile {
    let path = Address::Journal(date).html_path();
    let nav = day_links(
        days,
        date,
        |d| d,
        |d| href(&path, &Address::Journal(d).html_path(), None),
    );
    let main = format!(
        "<article class=\"page\">\n{}{}{}</article>\n",
        header(&journal_title(date, &nav)),
        empty_body(true),
        linked_refs(0, None),
    );
    OutFile {
        html: document(
            site_title,
            &path,
            &format!("{date} {}", date.weekday()),
            "",
            &main,
        ),
        path,
    }
}

fn page_body(site: &Site, from: &Path, page: PageId) -> String {
    let p = site.g.page(page);
    if p.is_implicit() {
        return empty_body(p.is_journal());
    }
    format!(
        "<section class=\"content\">\n{}\n</section>\n",
        site.tree(from, &p.roots, true)
    )
}

fn namespace_children(site: &Site, from: &Path, page: PageId) -> String {
    let children = site.g.namespace_listing(page);
    if children.is_empty() {
        return String::new();
    }
    let mut out = String::from("<section class=\"children\">\n<h2>Pages under this one</h2>\n<ul>");
    for (leaf, c) in children {
        out.push_str(&format!(
            "<li><a href=\"{}\">{}</a></li>",
            esc(&site.page_href(from, c)),
            esc(leaf)
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
        html: document(&site.title, &path, "Journals", "", &main),
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
        html: document(&site.title, &path, "Search", "", main),
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
    pages.sort_by_cached_key(|&p| crate::graph::display_key(&site.g.page(p).name));
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
        html: document(&site.title, &path, "All pages", "", &main),
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
            html: document(&site.title, &path, "Open tasks", "", &main),
            path,
        });
    }
    files
}
