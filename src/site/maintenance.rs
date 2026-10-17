//! The maintenance pages `hermetic serve` shows to a browser on the same machine (see
//! `maintenance`). They are never part of a build.

use std::path::{Path, PathBuf};

use super::html::{document, esc};
use super::pages::OutFile;
use super::paths::href;

/// The maintenance page, listing [`ACTIONS`].
pub const MAINTENANCE_PAGE: &str = "maintenance.html";
/// The page of the "Unused assets" action.
pub const UNUSED_ASSETS_PAGE: &str = "maintenance/unused-assets.html";

/// A maintenance action: its name, a one-sentence description, and its page.
struct Action {
    name: &'static str,
    description: &'static str,
    path: &'static str,
}

const ACTIONS: &[Action] = &[Action {
    name: "Unused assets",
    description: "Find images and other files that no note links to or shows, and move them to the trash.",
    path: UNUSED_ASSETS_PAGE,
}];

/// `maintenance.html`: every maintenance action, with a link to its page.
pub fn maintenance_page(site_title: &str) -> OutFile {
    let path = PathBuf::from(MAINTENANCE_PAGE);
    let mut main = String::from(
        "<article class=\"page index\" data-view=\"maintenance\">\n\
         <header class=\"page-header\"><h1>Maintenance</h1></header>\n\
         <ul class=\"maintenance-actions\">",
    );
    for a in ACTIONS {
        main.push_str(&format!(
            "<li><a href=\"{}\">{}</a><p>{}</p></li>",
            esc(&href(&path, Path::new(a.path), None)),
            esc(a.name),
            esc(a.description)
        ));
    }
    main.push_str("</ul>\n</article>\n");
    OutFile {
        html: document(site_title, &path, "Maintenance", &main),
        path,
    }
}

/// `maintenance/unused-assets.html`: the list itself is filled in by the script.
pub fn unused_assets_page(site_title: &str) -> OutFile {
    let path = PathBuf::from(UNUSED_ASSETS_PAGE);
    let main = format!(
        "<article class=\"page index\" data-view=\"unused-assets\">\n\
         <header class=\"page-header\"><p class=\"breadcrumb\"><a href=\"{}\">Maintenance</a></p><h1>Unused assets</h1></header>\n\
         <hermetic-unused-assets></hermetic-unused-assets>\n\
         <p class=\"maintenance-unavailable\">The list of unused assets needs JavaScript.</p>\n\
         </article>\n",
        esc(&href(&path, Path::new(MAINTENANCE_PAGE), None))
    );
    OutFile {
        html: document(site_title, &path, "Unused assets", &main),
        path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gateway_lists_the_actions() {
        let page = maintenance_page("Notes");
        assert_eq!(page.path, Path::new("maintenance.html"));
        assert!(page.html.contains("<title>Maintenance · Notes</title>"));
        assert!(
            page.html
                .contains(r#"<meta name="hermetic-root" content=".">"#)
        );
        assert!(page.html.contains(
            r#"<li><a href="maintenance/unused-assets.html">Unused assets</a><p>Find images"#
        ));
    }

    #[test]
    fn unused_assets_page() {
        let page = super::unused_assets_page("Notes");
        assert_eq!(page.path, Path::new("maintenance/unused-assets.html"));
        assert!(
            page.html
                .contains(r#"<meta name="hermetic-root" content="..">"#)
        );
        assert!(
            page.html
                .contains(r#"<a href="../maintenance.html">Maintenance</a>"#)
        );
        assert!(
            page.html
                .contains("<hermetic-unused-assets></hermetic-unused-assets>")
        );
        assert!(
            page.html
                .contains(r#"<link rel="stylesheet" href="../_hermetic/style.css">"#)
        );
    }
}
