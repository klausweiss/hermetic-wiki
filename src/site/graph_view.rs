//! `graph.html`: every non-journal page and the connections between them, laid out at
//! build time and drawn as inline SVG whose nodes are plain links.

use std::fmt::Write;
use std::path::{Path, PathBuf};

use super::graph_layout::{self, Node};
use super::html::{document, esc};
use super::pages::OutFile;
use super::render::Site;

/// Smallest and largest node radius, in layout units.
const R_MIN: f64 = 6.0;
const R_MAX: f64 = 30.0;
/// Radius added per square root of a node's mentions.
const R_STEP: f64 = 2.5;
/// Space around the drawing, so labels at the edge are not cut off.
const MARGIN: f64 = 40.0;
/// Distance from a node's circle to the top of its label, which hangs below it.
const LABEL_GAP: f64 = 3.0;
/// Edge weight buckets: the first weight of buckets 2 and 3.
const WEIGHT_BUCKETS: [u32; 2] = [2, 4];

pub fn graph_page(site: &Site) -> OutFile {
    let path = PathBuf::from("graph.html");
    let o = site.g.overview();
    let mut main = format!(
        "<article class=\"page graph-view\" data-view=\"graph\">\n<header class=\"page-header\"><h1>Graph <span class=\"count\">{}</span></h1></header>\n",
        o.nodes.len()
    );
    if o.nodes.is_empty() {
        main.push_str(
            "<p class=\"graph-empty\">There are no pages to show yet. \
             The graph shows every page other than journal days, and how they link to each other.</p>\n",
        );
    } else {
        main.push_str(&svg(site, &path, &o));
    }
    main.push_str("</article>\n");
    OutFile {
        html: document(&site.title, site.open_tasks, &path, "Graph", &main),
        path,
    }
}

fn radius(mentions: u32) -> f64 {
    (R_MIN + R_STEP * f64::from(mentions).sqrt()).min(R_MAX)
}

/// Label tier of the node ranked `rank` (0 = biggest) among `n`: tier 0 labels are
/// always shown, higher tiers as the reader zooms in.
fn tier(rank: usize, n: usize) -> usize {
    if rank < 30.max(n / 20) {
        0
    } else if rank < 60.max(n / 8) {
        1
    } else if rank < n / 3 {
        2
    } else {
        3
    }
}

fn svg(site: &Site, from: &Path, o: &crate::graph::Overview) -> String {
    let n = o.nodes.len();
    let nodes: Vec<Node> = o
        .nodes
        .iter()
        .zip(&o.mentions)
        .map(|(&p, &m)| Node {
            seed: graph_layout::seed(site.g.page(p).key.as_str().as_bytes()),
            radius: radius(m),
        })
        .collect();
    let pos: Vec<(f64, f64)> = graph_layout::layout(&nodes, &o.edges)
        .into_iter()
        .map(|(x, y)| (round(x), round(y)))
        .collect();

    let mut ranked: Vec<usize> = (0..n).collect();
    ranked.sort_by_cached_key(|&i| {
        let name = &site.g.page(o.nodes[i]).name;
        (
            std::cmp::Reverse(o.mentions[i]),
            crate::graph::display_key(name),
        )
    });
    let mut tiers = vec![0; n];
    for (rank, &i) in ranked.iter().enumerate() {
        tiers[i] = tier(rank, n);
    }
    let mut neighbours = vec![Vec::new(); n];
    for &(a, b, _) in &o.edges {
        neighbours[a as usize].push(b);
        neighbours[b as usize].push(a);
    }

    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for (&(x, y), node) in pos.iter().zip(&nodes) {
        x0 = x0.min(x - node.radius);
        y0 = y0.min(y - node.radius);
        x1 = x1.max(x + node.radius);
        y1 = y1.max(y + node.radius);
    }
    let (x0, y0) = (x0 - MARGIN, y0 - MARGIN);
    let (w, h) = (x1 - x0 + MARGIN, y1 - y0 + MARGIN);

    let mut out = String::new();
    let _ = writeln!(
        out,
        "<hermetic-graph>\n<svg class=\"graph\" viewBox=\"{} {} {w} {h}\" width=\"{w}\" height=\"{h}\" aria-label=\"Graph of all pages\">\n<g class=\"graph-world\">",
        num(x0),
        num(y0),
        w = num(w),
        h = num(h)
    );
    out.push_str("<g class=\"graph-edges\">");
    let mut buckets = [String::new(), String::new(), String::new()];
    for &(a, b, weight) in &o.edges {
        let bucket = WEIGHT_BUCKETS.iter().filter(|&&min| weight >= min).count();
        let ((ax, ay), (bx, by)) = (pos[a as usize], pos[b as usize]);
        let _ = write!(
            buckets[bucket],
            "M{} {}L{} {}",
            num(ax),
            num(ay),
            num(bx),
            num(by)
        );
    }
    for (k, d) in buckets.iter().enumerate() {
        if !d.is_empty() {
            let _ = write!(out, "<path class=\"w{}\" d=\"{d}\"/>", k + 1);
        }
    }
    out.push_str("</g>\n<g class=\"graph-nodes\">\n");
    for i in 0..n {
        let p = o.nodes[i];
        let page = site.g.page(p);
        let (x, y) = pos[i];
        let r = nodes[i].radius;
        let name = esc(&page.name);
        let near: Vec<String> = neighbours[i].iter().map(u32::to_string).collect();
        let _ = writeln!(
            out,
            "<a href=\"{}\" class=\"t{}{}\" data-i=\"{i}\" data-n=\"{}\"><title>{name}</title><circle cx=\"{}\" cy=\"{}\" r=\"{}\"/><text x=\"{}\" y=\"{}\">{name}</text></a>",
            esc(&site.page_href(from, p)),
            tiers[i],
            if page.is_implicit() { " implicit" } else { "" },
            near.join(" "),
            num(x),
            num(y),
            num(r),
            num(x),
            num(round(y + r + LABEL_GAP)),
        );
    }
    out.push_str("</g>\n</g>\n</svg>\n</hermetic-graph>\n");
    out
}

/// Rounds to one decimal, the precision written to the file.
fn round(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// A coordinate as written: one decimal at most, no negative zero.
fn num(v: f64) -> String {
    let v = round(v);
    if v == 0.0 {
        "0".to_owned()
    } else if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        assert_eq!(num(-0.04), "0");
        assert_eq!(num(12.0), "12");
        assert_eq!(num(-3.24), "-3.2");
        assert_eq!(num(7.06), "7.1");
    }

    #[test]
    fn radius_grows_and_is_capped() {
        assert_eq!(radius(0), R_MIN);
        assert!(radius(1) < radius(10));
        assert_eq!(radius(10_000), R_MAX);
    }

    #[test]
    fn tiers_start_with_the_biggest() {
        assert_eq!(tier(0, 10), 0);
        assert_eq!(tier(29, 1000), 0);
        assert_eq!(tier(49, 1000), 0);
        assert_eq!(tier(50, 1000), 1);
        assert_eq!(tier(200, 1000), 2);
        assert_eq!(tier(999, 1000), 3);
    }
}
