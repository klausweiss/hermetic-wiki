//! Whole pages: a note's page (with its sections and continuation files) and the
//! site-wide journal index, all-pages list, and open-tasks list.

use std::path::{Path, PathBuf};

use super::html::{document, esc};
use super::paths::{href, refs_continuation_path, tasks_continuation_path};
use super::render::{Mode, Site, task_badge};
use crate::graph::{BlockId, LinkedRef, PageId};

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
    let chunk_path = |n: usize| {
        if n == 1 {
            path.clone()
        } else {
            refs_continuation_path(&path, n)
        }
    };

    let mut main = String::from("<article class=\"page\">\n");
    main.push_str(&page_header(site, &path, page));
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
        main.push_str(&pager(&path, 1, chunks.len(), &chunk_path));
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
        let file = chunk_path(n);
        let main = format!(
            "<article class=\"page\">\n<header class=\"page-header\"><h1><a href=\"{}\">{}</a> <span class=\"muted\">references {}–{} of {}</span></h1></header>\n<section class=\"linked-refs\">\n{}{}</section>\n</article>\n",
            esc(&href(&file, &path, None)),
            esc(&title),
            i * REFS_PER_FILE + 1,
            i * REFS_PER_FILE + chunk.len(),
            refs.len(),
            ref_groups(site, &file, chunk),
            pager(&file, n, chunks.len(), &chunk_path),
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
            out.push_str(&day_nav(site, from, page));
        }
        None => out.push_str(&format!("<h1>{}</h1>\n", esc(p.leaf_name()))),
    }
    out.push_str("</header>\n");
    out
}

/// Links to the nearest earlier and later journal days that have a file.
fn day_nav(site: &Site, from: &Path, page: PageId) -> String {
    let Some(date) = site.g.page(page).date() else {
        return String::new();
    };
    let days = &site.journal_days;
    let pos = days.partition_point(|&(d, _)| d < date);
    let prev = pos.checked_sub(1).map(|i| days[i]);
    let next_pos = if days.get(pos).is_some_and(|&(d, _)| d == date) {
        pos + 1
    } else {
        pos
    };
    let next = days.get(next_pos).copied();
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
                site.block_body(from, b)
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
    let mut current: Option<PageId> = None;
    for (source, r) in refs {
        if current != Some(*source) {
            if current.is_some() {
                out.push_str("</div>\n");
            }
            out.push_str(&format!(
                "<div class=\"ref-group\">\n<h3><a href=\"{}\">{}</a></h3>\n",
                esc(&site.page_href(from, *source)),
                esc(&site.page_title(*source))
            ));
            current = Some(*source);
        }
        out.push_str("<div class=\"linked-ref\">");
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
    }
    if current.is_some() {
        out.push_str("</div>\n");
    }
    out
}

/// Previous/next links between the files of a paginated list.
fn pager(from: &Path, n: usize, total: usize, path_of: &dyn Fn(usize) -> PathBuf) -> String {
    if total <= 1 {
        return String::new();
    }
    let mut out = String::from("<nav class=\"pager\">");
    if n > 1 {
        out.push_str(&format!(
            "<a rel=\"prev\" href=\"{}\">← Previous</a>",
            esc(&href(from, &path_of(n - 1), None))
        ));
    }
    out.push_str(&format!("<span>{n} / {total}</span>"));
    if n < total {
        out.push_str(&format!(
            "<a rel=\"next\" href=\"{}\">Next →</a>",
            esc(&href(from, &path_of(n + 1), None))
        ));
    }
    out.push_str("</nav>\n");
    out
}

/// `index.html`: file-backed journal days, newest first, grouped by month.
pub fn journal_index(site: &Site) -> OutFile {
    let path = PathBuf::from("index.html");
    let mut main = String::from(
        "<article class=\"page index\">\n<header class=\"page-header\"><h1>Journals</h1></header>\n",
    );
    if site.journal_days.is_empty() {
        main.push_str("<p class=\"empty\">No journal entries yet. Add files like <code>journals/2026-10-12.md</code>.</p>\n");
    }
    let mut month: Option<(u16, u8)> = None;
    for &(date, page) in site.journal_days.iter().rev() {
        if month != Some((date.year, date.month)) {
            if month.is_some() {
                main.push_str("</ul>\n");
            }
            main.push_str(&format!(
                "<h2>{} {}</h2>\n<ul class=\"days\">",
                date.month_name(),
                date.year
            ));
            month = Some((date.year, date.month));
        }
        main.push_str(&format!(
            "<li><a href=\"{}\">{date}</a> <span class=\"weekday\">{}</span></li>",
            esc(&site.page_href(&path, page)),
            date.weekday()
        ));
    }
    if month.is_some() {
        main.push_str("</ul>\n");
    }
    main.push_str("</article>\n");
    OutFile {
        html: document(&site.title, &path, "Journals", &main),
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
        "<article class=\"page index\">\n<header class=\"page-header\"><h1>All pages <span class=\"count\">{}</span></h1></header>\n<ul class=\"all-pages\">",
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
    let total = chunks.len().max(1);
    let path_of = |n: usize| {
        if n == 1 {
            PathBuf::from("tasks.html")
        } else {
            tasks_continuation_path(n)
        }
    };
    let mut files = Vec::new();
    for n in 1..=total {
        let path = path_of(n);
        let mut main = format!(
            "<article class=\"page index\">\n<header class=\"page-header\"><h1>Open tasks <span class=\"count\">{}</span></h1></header>\n",
            tasks.len()
        );
        match chunks.get(n - 1) {
            None => main.push_str("<p class=\"empty\">No open tasks.</p>\n"),
            Some(chunk) => {
                let mut current: Option<PageId> = None;
                for &t in chunk.iter() {
                    let page = site.g.block(t).page;
                    if current != Some(page) {
                        if current.is_some() {
                            main.push_str("</ul>\n");
                        }
                        main.push_str(&format!(
                            "<h2><a href=\"{}\">{}</a></h2>\n<ul class=\"tasks\">",
                            esc(&site.page_href(&path, page)),
                            esc(&site.page_title(page))
                        ));
                        current = Some(page);
                    }
                    main.push_str(&task_item(site, &path, t));
                }
                main.push_str("</ul>\n");
            }
        }
        main.push_str(&pager(&path, n, total, &path_of));
        main.push_str("</article>\n");
        files.push(OutFile {
            html: document(&site.title, &path, "Open tasks", &main),
            path,
        });
    }
    files
}
