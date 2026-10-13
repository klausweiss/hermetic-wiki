//! Force-directed layout of the graph view, computed at build time.
//!
//! Fruchterman–Reingold style: edges pull their ends together, all nodes push each
//! other apart (approximated with a Barnes–Hut quadtree), and a weak gravity toward
//! the origin keeps unconnected nodes nearby. A fixed number of iterations with a
//! cooling step limit makes the running time predictable, and a final pass pushes
//! overlapping circles apart. Everything is plain `f64` arithmetic in a fixed
//! order (repulsion is spread over threads, but each body's force is computed alone),
//! so the same input always gives the same positions.

use std::collections::HashMap;

/// A node to place.
#[derive(Debug, Clone, Copy)]
pub struct Node {
    /// Decides the starting position; see [`seed`].
    pub seed: u64,
    pub radius: f64,
}

/// Distance at which two connected nodes of the smallest size settle.
pub const SPACING: f64 = 60.0;
const ITERATIONS: usize = 150;
/// Barnes–Hut opening angle: cells smaller than this fraction of their distance are
/// treated as one body.
const THETA: f64 = 1.2;
const GRAVITY: f64 = 2.0;
/// Nodes this far apart (beyond their radii) do not count as overlapping.
const GAP: f64 = 4.0;
const OVERLAP_PASSES: usize = 100;
/// Graphs at least this big compute repulsion on several threads.
const PARALLEL_FROM: usize = 1000;
/// Deepest quadtree level; nodes still sharing a cell there are handled pairwise.
const MAX_DEPTH: u32 = 24;

/// A stable 64-bit seed for a name (FNV-1a, never a randomly keyed hasher).
pub fn seed(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    // FNV's low bits mix poorly for short inputs; finish with a splitmix64 round.
    h = (h ^ (h >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    h ^ (h >> 31)
}

/// Positions of `nodes` given `edges` as `(node, node, weight)`.
pub fn layout(nodes: &[Node], edges: &[(u32, u32, u32)]) -> Vec<(f64, f64)> {
    let n = nodes.len();
    let mut pos: Vec<(f64, f64)> = nodes.iter().map(|node| start(node.seed, n)).collect();
    if n < 2 {
        return pos.into_iter().map(|_| (0.0, 0.0)).collect();
    }
    let min_radius = nodes.iter().map(|n| n.radius).fold(f64::INFINITY, f64::min);
    // Bigger nodes push harder, so hubs get room around them.
    let charge: Vec<f64> = nodes.iter().map(|n| n.radius / min_radius).collect();
    let strength: Vec<f64> = edges.iter().map(|e| 1.0 + f64::from(e.2).ln()).collect();

    let start_step = SPACING * (n as f64).sqrt() * 0.1 + SPACING;
    let end_step = SPACING * 0.02;
    let mut disp = vec![(0.0, 0.0); n];
    // Bodies in quadtree order, kept from one iteration to the next.
    let mut bodies: Vec<Body> = (0..n as u32)
        .map(|i| Body {
            x: 0.0,
            y: 0.0,
            charge: charge[i as usize],
            index: i,
        })
        .collect();
    let mut scratch = bodies.clone();
    let mut tree = Vec::new();
    // More threads than this only add overhead at the sizes the graph view sees.
    let threads = std::thread::available_parallelism().map_or(1, |t| t.get().min(8));
    for iteration in 0..ITERATIONS {
        let t = iteration as f64 / (ITERATIONS - 1) as f64;
        let step = start_step + (end_step - start_step) * t;

        for b in &mut bodies {
            (b.x, b.y) = pos[b.index as usize];
        }
        build_tree(&mut bodies, &mut scratch, &mut tree);
        let (bodies, tree) = (&bodies, &tree);
        if n < PARALLEL_FROM {
            let mut stack = Vec::new();
            for b in bodies {
                disp[b.index as usize] = field(b, bodies, tree, &mut stack);
            }
        } else {
            // Each body's force depends only on the previous positions, so splitting
            // the work across threads gives the same result as doing it in one.
            let forces: Vec<Vec<(f64, f64)>> = std::thread::scope(|s| {
                let handles: Vec<_> = bodies
                    .chunks(n.div_ceil(threads))
                    .map(|chunk| {
                        s.spawn(move || {
                            let mut stack = Vec::new();
                            chunk
                                .iter()
                                .map(|b| field(b, bodies, tree, &mut stack))
                                .collect()
                        })
                    })
                    .collect();
                handles.into_iter().map(|h| h.join().unwrap()).collect()
            });
            for (b, f) in bodies.iter().zip(forces.into_iter().flatten()) {
                disp[b.index as usize] = f;
            }
        }
        for (&(a, b, _), &s) in edges.iter().zip(&strength) {
            // Linear in the distance (as in ForceAtlas2): long edges do not crush
            // everything into a ball the way a quadratic spring does.
            let (a, b) = (a as usize, b as usize);
            let (dx, dy) = (pos[a].0 - pos[b].0, pos[a].1 - pos[b].1);
            disp[a].0 -= dx * s;
            disp[a].1 -= dy * s;
            disp[b].0 += dx * s;
            disp[b].1 += dy * s;
        }
        for i in 0..n {
            let (dx, dy) = disp[i];
            let d = (dx * dx + dy * dy).sqrt();
            if d > 0.0 {
                let m = d.min(step) / d;
                pos[i].0 += dx * m;
                pos[i].1 += dy * m;
            }
        }
    }
    separate(nodes, &mut pos);
    pos
}

/// A starting point on a disc that grows with the number of nodes.
fn start(seed: u64, n: usize) -> (f64, f64) {
    let u = (seed & 0xffff_ffff) as f64 / 4_294_967_296.0;
    let v = (seed >> 32) as f64 / 4_294_967_296.0;
    let r = SPACING * (n as f64).sqrt() * v.sqrt();
    let a = u * std::f64::consts::TAU;
    (r * a.cos(), r * a.sin())
}

/// A node during one iteration.
#[derive(Debug, Clone, Copy)]
struct Body {
    x: f64,
    y: f64,
    charge: f64,
    index: u32,
}

/// A quadtree cell over `bodies[start..end]`.
#[derive(Debug, Clone, Copy)]
struct Cell {
    /// Centre of charge.
    x: f64,
    y: f64,
    charge: f64,
    /// Side length.
    size: f64,
    start: u32,
    end: u32,
    /// First child, or 0 for a leaf; children are `first..first + 4`, empty ones
    /// having `start == end`.
    first: u32,
}

/// Sorts `bodies` into quadtree order and builds the tree over them.
fn build_tree(bodies: &mut [Body], scratch: &mut [Body], tree: &mut Vec<Cell>) {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for b in bodies.iter() {
        x0 = x0.min(b.x);
        y0 = y0.min(b.y);
        x1 = x1.max(b.x);
        y1 = y1.max(b.y);
    }
    let size = (x1 - x0).max(y1 - y0).max(1e-9);
    tree.clear();
    tree.push(Cell {
        x: 0.0,
        y: 0.0,
        charge: 0.0,
        size,
        start: 0,
        end: bodies.len() as u32,
        first: 0,
    });
    split(0, x0, y0, 0, bodies, scratch, tree);
}

/// Splits `tree[cell]` into quadrants, recursively, and sets its centre of charge.
fn split(
    cell: usize,
    x0: f64,
    y0: f64,
    depth: u32,
    all: &mut [Body],
    scratch: &mut [Body],
    tree: &mut Vec<Cell>,
) {
    let Cell {
        size, start, end, ..
    } = tree[cell];
    let (start, end) = (start as usize, end as usize);
    if end - start < 2 || depth == MAX_DEPTH {
        let (mut q, mut x, mut y) = (0.0, 0.0, 0.0);
        for b in &all[start..end] {
            q += b.charge;
            x += b.x * b.charge;
            y += b.y * b.charge;
        }
        (tree[cell].charge, tree[cell].x, tree[cell].y) = (q, x / q, y / q);
        return;
    }
    let half = size / 2.0;
    let (mx, my) = (x0 + half, y0 + half);
    let quadrant = |b: &Body| usize::from(b.x >= mx) + 2 * usize::from(b.y >= my);
    // A stable counting sort by quadrant keeps the order deterministic.
    let mut counts = [0usize; 4];
    for b in &all[start..end] {
        counts[quadrant(b)] += 1;
    }
    let mut next = [
        0,
        counts[0],
        counts[0] + counts[1],
        counts[0] + counts[1] + counts[2],
    ];
    let sorted = &mut scratch[start..end];
    for b in &all[start..end] {
        let q = quadrant(b);
        sorted[next[q]] = *b;
        next[q] += 1;
    }
    all[start..end].copy_from_slice(sorted);
    let first = tree.len();
    tree[cell].first = first as u32;
    let mut at = start as u32;
    for count in counts {
        tree.push(Cell {
            x: 0.0,
            y: 0.0,
            charge: 0.0,
            size: half,
            start: at,
            end: at + count as u32,
            first: 0,
        });
        at += count as u32;
    }
    let (mut q, mut x, mut y) = (0.0, 0.0, 0.0);
    for (k, count) in counts.into_iter().enumerate() {
        if count > 0 {
            let cx = if k & 1 == 1 { mx } else { x0 };
            let cy = if k & 2 == 2 { my } else { y0 };
            split(first + k, cx, cy, depth + 1, all, scratch, tree);
            let child = tree[first + k];
            q += child.charge;
            x += child.x * child.charge;
            y += child.y * child.charge;
        }
    }
    (tree[cell].charge, tree[cell].x, tree[cell].y) = (q, x / q, y / q);
}

/// The forces on `body` not along edges: repulsion from every other body (magnitude
/// k²·qᵢ·qⱼ/d) and gravity toward the origin.
fn field(body: &Body, bodies: &[Body], tree: &[Cell], stack: &mut Vec<usize>) -> (f64, f64) {
    let Body { x, y, index: i, .. } = *body;
    let qi = body.charge * SPACING * SPACING;
    let (mut fx, mut fy) = (-GRAVITY * x, -GRAVITY * y);
    stack.clear();
    stack.push(0);
    while let Some(c) = stack.pop() {
        let cell = &tree[c];
        if cell.start == cell.end {
            continue;
        }
        let (dx, dy) = (x - cell.x, y - cell.y);
        let d2 = dx * dx + dy * dy;
        if cell.first == 0 {
            for other in &bodies[cell.start as usize..cell.end as usize] {
                let j = other.index;
                if j == i {
                    continue;
                }
                let (mut dx, mut dy) = (x - other.x, y - other.y);
                if dx == 0.0 && dy == 0.0 {
                    // Coincident nodes: separate them in a direction fixed by the pair.
                    let a = f64::from(i.wrapping_mul(2_654_435_761) ^ j) * 0.618_034;
                    (dx, dy) = (a.cos() * 1e-3, a.sin() * 1e-3);
                }
                let f = qi * other.charge / (dx * dx + dy * dy);
                fx += dx * f;
                fy += dy * f;
            }
        } else if cell.size * cell.size < THETA * THETA * d2 {
            let f = qi * cell.charge / d2;
            fx += dx * f;
            fy += dy * f;
        } else {
            stack.extend(cell.first as usize..cell.first as usize + 4);
        }
    }
    (fx, fy)
}

/// Pushes overlapping circles apart until none overlap (or the pass limit is hit).
fn separate(nodes: &[Node], pos: &mut [(f64, f64)]) {
    let max_radius = nodes.iter().map(|n| n.radius).fold(0.0, f64::max);
    let cell = 2.0 * max_radius + GAP;
    let key = |(x, y): (f64, f64)| ((x / cell).floor() as i64, (y / cell).floor() as i64);
    let mut grid: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
    for _ in 0..OVERLAP_PASSES {
        grid.clear();
        for (i, &p) in pos.iter().enumerate() {
            grid.entry(key(p)).or_default().push(i as u32);
        }
        let mut moved = false;
        for i in 0..pos.len() {
            let (gx, gy) = key(pos[i]);
            for nx in gx - 1..=gx + 1 {
                for ny in gy - 1..=gy + 1 {
                    let Some(others) = grid.get(&(nx, ny)) else {
                        continue;
                    };
                    for &j in others {
                        let j = j as usize;
                        if j <= i {
                            continue;
                        }
                        let min = nodes[i].radius + nodes[j].radius + GAP;
                        let (mut dx, mut dy) = (pos[j].0 - pos[i].0, pos[j].1 - pos[i].1);
                        let mut d = (dx * dx + dy * dy).sqrt();
                        if d >= min {
                            continue;
                        }
                        if d == 0.0 {
                            let a = (i * 7 + j) as f64;
                            (dx, dy, d) = (a.cos(), a.sin(), 1.0);
                        }
                        // Move both halfway, a little further so the gap does not
                        // reappear from rounding.
                        let push = (min - d) / d * 0.5 * 1.01;
                        pos[i].0 -= dx * push;
                        pos[i].1 -= dy * push;
                        pos[j].0 += dx * push;
                        pos[j].1 += dy * push;
                        moved = true;
                    }
                }
            }
        }
        if !moved {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    fn nodes(n: usize) -> Vec<Node> {
        (0..n)
            .map(|i| Node {
                seed: seed(format!("page {i}").as_bytes()),
                radius: 4.0 + (i % 5) as f64,
            })
            .collect()
    }

    fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
        ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
    }

    fn assert_no_overlap(nodes: &[Node], pos: &[(f64, f64)]) {
        for i in 0..pos.len() {
            for j in i + 1..pos.len() {
                let d = dist(pos[i], pos[j]);
                assert!(
                    d >= nodes[i].radius + nodes[j].radius,
                    "nodes {i} and {j} overlap: distance {d}"
                );
            }
        }
    }

    #[test]
    fn seeds_are_stable() {
        assert_eq!(seed(b"auth"), seed(b"auth"));
        assert_ne!(seed(b"auth"), seed(b"autg"));
        assert_eq!(seed(b""), seed(b""));
    }

    #[test]
    fn empty_and_single() {
        assert!(layout(&[], &[]).is_empty());
        assert_eq!(layout(&nodes(1), &[]), [(0.0, 0.0)]);
    }

    #[test]
    fn deterministic() {
        let ns = nodes(200);
        let edges: Vec<_> = (0..300u32)
            .map(|i| (i % 200, (i * 7 + 3) % 200, 1 + i % 3))
            .filter(|e| e.0 != e.1)
            .collect();
        let a = layout(&ns, &edges);
        let b = layout(&ns, &edges);
        assert_eq!(a, b);
        assert!(a.iter().all(|p| p.0.is_finite() && p.1.is_finite()));
    }

    #[test]
    fn connected_nodes_are_near() {
        // Two triangles with no edge between them.
        let ns = nodes(6);
        let edges = [
            (0, 1, 1),
            (1, 2, 1),
            (0, 2, 1),
            (3, 4, 1),
            (4, 5, 1),
            (3, 5, 1),
        ];
        let pos = layout(&ns, &edges);
        let group = |i: usize| i / 3;
        for i in 0..6 {
            let own = (0..6)
                .filter(|&j| j != i && group(j) == group(i))
                .map(|j| dist(pos[i], pos[j]))
                .fold(0.0, f64::max);
            let other = (0..6)
                .filter(|&j| group(j) != group(i))
                .map(|j| dist(pos[i], pos[j]))
                .fold(f64::INFINITY, f64::min);
            assert!(
                own < other,
                "node {i}: own group up to {own}, other group from {other}"
            );
        }
    }

    #[test]
    fn edge_length_is_about_the_spacing() {
        let pos = layout(&nodes(2), &[(0, 1, 1)]);
        let d = dist(pos[0], pos[1]);
        assert!(d > SPACING * 0.3 && d < SPACING * 2.0, "{d}");
    }

    #[test]
    fn no_overlaps() {
        // A star: many nodes pulled toward one hub, with big radii.
        let ns: Vec<Node> = (0..80)
            .map(|i| Node {
                seed: seed(format!("n{i}").as_bytes()),
                radius: if i == 0 { 24.0 } else { 10.0 },
            })
            .collect();
        let edges: Vec<_> = (1..80u32).map(|i| (0, i, 5)).collect();
        let pos = layout(&ns, &edges);
        assert_no_overlap(&ns, &pos);
    }

    #[test]
    fn coincident_seeds() {
        let ns = vec![
            Node {
                seed: 42,
                radius: 5.0
            };
            10
        ];
        let pos = layout(&ns, &[]);
        assert_no_overlap(&ns, &pos);
    }

    /// A wiki-like graph: `n` nodes in communities of 50 grown by preferential
    /// attachment (so each has a few hubs), plus a few links between communities.
    /// Radii grow with degree, as in the graph view.
    fn wiki_like(n: u32, edges_per_node: u32) -> (Vec<Node>, Vec<(u32, u32, u32)>) {
        let mut rng = 0x2545_f491_4f6c_dd1du64;
        let mut next = |m: u32| {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            (rng % u64::from(m)) as u32
        };
        let mut edges = Vec::new();
        let mut ends: Vec<u32> = Vec::new();
        for i in 0..n {
            let base = i - i % 50;
            if i == base {
                ends.clear();
                continue;
            }
            for k in 0..edges_per_node {
                let target = if k == 0 && next(10) == 0 {
                    next(n)
                } else if ends.is_empty() || next(4) == 0 {
                    base + next(i - base)
                } else {
                    ends[next(ends.len() as u32) as usize]
                };
                if target != i {
                    edges.push((i.min(target), i.max(target), 1 + next(3) / 2));
                    ends.push(target);
                }
            }
            ends.push(i);
        }
        edges.sort_unstable();
        edges.dedup_by_key(|e| (e.0, e.1));
        let mut degree = vec![0u32; n as usize];
        for &(a, b, _) in &edges {
            degree[a as usize] += 1;
            degree[b as usize] += 1;
        }
        let nodes = (0..n)
            .map(|i| Node {
                seed: seed(format!("page {i}").as_bytes()),
                radius: (4.0 + 1.5 * f64::from(degree[i as usize]).sqrt()).min(24.0),
            })
            .collect();
        (nodes, edges)
    }

    /// Writes an SVG of a laid-out wiki-like graph to `$HERMETIC_LAYOUT_PREVIEW`, for
    /// tuning the layout by eye: `HERMETIC_LAYOUT_PREVIEW=/tmp/l.svg cargo test
    /// --release layout_preview -- --ignored`. Set `HERMETIC_LAYOUT_NODES` for size.
    #[test]
    #[ignore = "development aid"]
    fn layout_preview() {
        let Ok(out) = std::env::var("HERMETIC_LAYOUT_PREVIEW") else {
            return;
        };
        let n = std::env::var("HERMETIC_LAYOUT_NODES").map_or(500, |v| v.parse().unwrap());
        let (ns, edges) = wiki_like(n, 4);
        let t = Instant::now();
        let pos = layout(&ns, &edges);
        eprintln!("{n} nodes, {} edges: {:?}", edges.len(), t.elapsed());
        let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for (p, node) in pos.iter().zip(&ns) {
            x0 = x0.min(p.0 - node.radius);
            y0 = y0.min(p.1 - node.radius);
            x1 = x1.max(p.0 + node.radius);
            y1 = y1.max(p.1 + node.radius);
        }
        let mut svg = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='{x0} {y0} {} {}' width='2000' height='2000'><rect x='{x0}' y='{y0}' width='100%' height='100%' fill='white'/><path stroke='#999' stroke-width='1' fill='none' d='",
            x1 - x0,
            y1 - y0
        );
        for &(a, b, _) in &edges {
            let (p, q) = (pos[a as usize], pos[b as usize]);
            svg += &format!("M{:.1} {:.1}L{:.1} {:.1}", p.0, p.1, q.0, q.1);
        }
        svg += "'/>";
        for (i, (p, node)) in pos.iter().zip(&ns).enumerate() {
            // Colour by community, to see whether communities stay together.
            let hue = (i / 50) * 37 % 360;
            svg += &format!(
                "<circle cx='{:.1}' cy='{:.1}' r='{}' fill='hsl({hue},70%,50%)'/>",
                p.0, p.1, node.radius
            );
        }
        svg += "</svg>";
        std::fs::write(out, svg).unwrap();
    }

    /// Run with `make test-perf` (release mode).
    #[test]
    #[ignore = "performance check; run in release mode"]
    fn lays_out_five_thousand_nodes_quickly() {
        let (ns, edges) = wiki_like(5000, 6);
        assert!(edges.len() >= 20_000, "{}", edges.len());
        let t = Instant::now();
        let pos = layout(&ns, &edges);
        let elapsed = t.elapsed();
        eprintln!(
            "layout of {} nodes, {} edges: {elapsed:?}",
            ns.len(),
            edges.len()
        );
        assert_eq!(pos.len(), ns.len());
        assert!(elapsed.as_millis() <= 300, "{elapsed:?}");
    }
}
