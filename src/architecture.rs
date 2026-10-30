//! Architecture-beta diagram rendering: nested groups of services,
//! connected by orthogonal edges between named port sides.
//!
//! The layout follows the ports, as mermaid's does: `a:R -- L:b` puts
//! `b` one grid cell right of `a`, `a:B -- T:b` one cell below, and a
//! mixed pair such as `a:R -- T:b` one cell diagonally (right, and
//! below because `b` is entered from the top). A breadth-first walk
//! over the edges turns those steps into a grid ([`place_cells`]);
//! every group's cells are then made contiguous, so no frame overlaps
//! another or swallows a stranger ([`legalize`]); and the grid becomes
//! pixels with each column and row only as wide and tall as what is in
//! it ([`Axis`]). Edges leave and enter along their port normals and
//! are routed orthogonally around every service box and group title
//! ([`Grid`]), so no line passes behind a box.
//!
//! Like `pie`/`journey`/`mindmap`/`gitgraph`, the layout is fully
//! automatic and the result is exposed as a generic [`Scene`] so
//! terminal/GUI hosts can draw it (and pick on it with
//! [`Scene::hit_test`]) without knowing the source syntax.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap, HashSet, VecDeque};

use crate::layout::{text_width, LABEL_BOX_H, LABEL_PAD_W, TEXT_CALIBRATION};
use crate::model::{ArchSide, Architecture, NodeStyle, Shape};
use crate::parser::normalize_breaks;
use crate::scene::{
    escape, svg_label_box, svg_open_a11y, A11y, Scene, SceneCluster, SceneEdge, SceneNode,
    SvgOptions, EDGE_COLOR, TEXT_COLOR,
};
use crate::style::accent;

/// Canvas margin around the whole diagram.
pub const PAD: f64 = 24.0;
/// Smallest service box. A box grows taller for a wrapped label, and
/// wider only for a single word too long to wrap.
pub const SERVICE_W: f64 = 150.0;
pub const SERVICE_H: f64 = 70.0;
/// Padding inside a group box.
pub const GROUP_PAD: f64 = 24.0;
/// Height reserved for a group title.
pub const GROUP_TITLE_H: f64 = 26.0;
/// Space between a group's frame and its siblings.
pub const GROUP_GAP: f64 = 50.0;
/// Space between two children of one group that no edge joins.
pub const CHILD_GAP_X: f64 = 32.0;
pub const CHILD_GAP_Y: f64 = 32.0;
/// Font size for service and group labels (the flowchart node size).
pub const FONT: u32 = 14;
/// Font size for icon glyphs inside service boxes.
pub const ICON_FONT: u32 = 24;

/// Space between two siblings an edge joins: both ports' stubs plus
/// an arrowhead with a visible shaft between them.
const EDGE_GAP: f64 = 56.0;
/// Width of the placeholder that holds an empty group's frame open.
const EMPTY_GROUP_W: f64 = 80.0;
/// Diameter of a junction's dot (and of its layout box).
const JUNCTION_D: f64 = 8.0;
/// Line advance of a wrapped service label.
const LINE_H: f64 = 18.0;
/// Room kept between a label and its box's left and right sides.
const LABEL_PAD_X: f64 = 12.0;
/// Room kept above and below a box's icon + label block.
const LABEL_PAD_Y: f64 = 11.0;
/// Space between a box's icon and its label.
const ICON_GAP: f64 = 6.0;
/// Centre of a group title below the frame's top edge (inside the
/// `GROUP_PAD + GROUP_TITLE_H` band above the first child).
const TITLE_CY: f64 = 20.0;
/// How far a route runs straight out of its port before it may turn.
const STUB: f64 = 16.0;
/// Clearance a route keeps from every service box it passes.
const CLEAR: f64 = 10.0;
/// Clearance a route keeps from a group title.
const TITLE_CLEAR: f64 = 4.0;
/// How far outside the drawing the outermost routing channel runs.
const OUTER: f64 = 12.0;
/// Corner radius of a drawn route (never more than half a leg).
const CORNER_R: f64 = 8.0;
/// Samples per rounded corner in an edge's waypoints: dense enough
/// that a host splining through them draws the corner the SVG does.
const CORNER_STEPS: usize = 6;
/// Route cost of one bend, in px of length.
const BEND_COST: f64 = 40.0;
/// Route cost of crossing an edge routed earlier, in px of length.
const CROSS_COST: f64 = 60.0;
/// Extra route cost per px run on top of an edge routed earlier: two
/// edges share a line only where they must (into a shared port).
const OVERLAP_COST: f64 = 3.0;
/// Cost per px of running along a box's clearance line rather than
/// down the middle of a gap, so routes take the middle when they can.
const HUG_RATE: f64 = 1.15;

/// Map a Mermaid-style icon name to an emoji glyph.
///
/// flowmaid is zero-dependency and has no icon font, so we render a
/// single glyph as a lightweight visual hint. An iconify-style name
/// (`mdi:database`) is looked up by the part after the pack prefix.
/// Unknown icon names are silently ignored (only the title is drawn).
///
/// Glyphs whose default presentation is text (☁ ✉ ⚙ 🗄 🖥 🛢 …) carry
/// VARIATION SELECTOR-16 (U+FE0F): without it a browser draws them in
/// the monochrome text font while their neighbours come out in colour.
pub fn icon_glyph(name: &str) -> Option<&'static str> {
    let name = name.trim().to_ascii_lowercase();
    let key = name.rsplit(':').next().unwrap_or_default();
    Some(match key {
        "cloud" => "\u{2601}\u{FE0F}",
        // An oil drum: the usual emoji stand-in for a database
        // cylinder (the file cabinet reads as a server rack instead).
        "database" | "db" => "\u{1F6E2}\u{FE0F}",
        "server" => "\u{1F5C4}\u{FE0F}",
        "disk" | "storage" => "\u{1F4BE}",
        "internet" | "web" | "globe" => "\u{1F310}",
        "user" => "\u{1F464}",
        "users" | "group" => "\u{1F465}",
        "lock" | "secure" => "\u{1F512}",
        "key" => "\u{1F511}",
        "mail" | "email" | "envelope" => "\u{2709}\u{FE0F}",
        "phone" | "mobile" => "\u{1F4F1}",
        "desktop" => "\u{1F5A5}\u{FE0F}",
        "laptop" | "code" => "\u{1F4BB}",
        "file" => "\u{1F4C4}",
        "folder" => "\u{1F4C1}",
        "bug" => "\u{1F41B}",
        "search" => "\u{1F50D}",
        "chart" | "graph" => "\u{1F4CA}",
        "cpu" | "processor" | "settings" | "config" | "gear" => "\u{2699}\u{FE0F}",
        "memory" | "ram" => "\u{1F9EE}",
        "firewall" => "\u{1F9F1}",
        "network" | "router" | "switch" => "\u{1F310}",
        "queue" | "message" => "\u{1F4EC}",
        "cache" => "\u{26A1}\u{FE0F}",
        "container" | "docker" | "package" => "\u{1F4E6}",
        "monitoring" | "metrics" => "\u{1F4C8}",
        "log" | "logs" => "\u{1F4DC}",
        "alert" | "bell" | "notification" => "\u{1F514}",
        "api" | "plug" => "\u{1F50C}",
        _ => return None,
    })
}

/// Positioned geometry for an architecture diagram.
#[derive(Debug, Clone)]
pub struct ArchScene {
    /// Services and junctions (index-parallel with
    /// `Architecture::services`; a junction is a small `Circle` with an
    /// empty label), group frames, and one edge per
    /// `Architecture::edges` entry in order. An edge's `waypoints` are
    /// its routed, corner-rounded path, densely sampled at the corners.
    pub scene: Scene,
    /// Optional icon glyph for each service node, index-parallel with `scene.nodes`.
    pub icons: Vec<Option<String>>,
    /// Whether each edge (index-parallel with `scene.edges`) has an
    /// arrowhead pointing at its `from` / `to` endpoint.
    pub arrow_starts: Vec<bool>,
    pub arrow_ends: Vec<bool>,
}

type Pt = (f64, f64);

/// Compute the scene for an architecture diagram.
pub fn scene(d: &Architecture) -> ArchScene {
    let cards: Vec<Card> = d
        .services
        .iter()
        .map(|s| {
            if s.junction {
                Card {
                    lines: Vec::new(),
                    w: JUNCTION_D,
                    h: JUNCTION_D,
                }
            } else {
                card(&s.title, s.icon.as_deref().and_then(icon_glyph).is_some())
            }
        })
        .collect();
    let sizes: Vec<Pt> = cards.iter().map(|c| (c.w, c.h)).collect();
    let t = Tree::new(d, &sizes);
    let cell = place_cells(d, &t);
    let titles: Vec<String> = d
        .groups
        .iter()
        .map(|g| match g.icon.as_deref().and_then(icon_glyph) {
            Some(glyph) if g.title.is_empty() => glyph.to_string(),
            Some(glyph) => format!("{} {}", glyph, g.title),
            None => g.title.clone(),
        })
        .collect();
    let title_w: Vec<f64> = titles.iter().map(|s| title_px(s)).collect();
    let (x, y) = solve(d, &t, &cell, &title_w);

    let na = t.na();
    let frame = |g: usize| Rect {
        x0: x.lo(na + g),
        y0: y.lo(na + g),
        x1: x.hi(na + g),
        y1: y.hi(na + g),
    };
    let frames: Vec<Rect> = (0..d.groups.len()).map(frame).collect();
    let (mut right, mut bottom) = (PAD, PAD);
    for &it in &t.kids[t.root] {
        right = right.max(x.hi(it));
        bottom = bottom.max(y.hi(it));
    }

    let clusters: Vec<SceneCluster> = d
        .groups
        .iter()
        .enumerate()
        .map(|(g, grp)| {
            let f = frames[g];
            SceneCluster {
                id: grp.id.clone(),
                x: f.x0,
                y: f.y0,
                w: f.x1 - f.x0,
                h: f.y1 - f.y0,
                title: titles[g].clone(),
                depth: t.depth[g],
            }
        })
        .collect();

    // Services take the accent of their top-level group, so colour
    // tells which box belongs where; a service outside every group gets
    // the next accent after the groups'.
    let top_groups: Vec<usize> = t.kids[t.root]
        .iter()
        .filter(|&&it| t.is_group(it))
        .map(|&it| it - na)
        .collect();
    let mut loose = top_groups.len();
    let mut nodes = Vec::with_capacity(d.services.len());
    let mut icons: Vec<Option<String>> = Vec::with_capacity(d.services.len());
    for (i, s) in d.services.iter().enumerate() {
        let top = *t.chain[i]
            .last()
            .expect("an atom's chain starts with itself");
        let tint = if t.is_group(top) {
            top_groups
                .iter()
                .position(|&g| g == top - na)
                .unwrap_or_default()
        } else {
            loose += 1;
            loose - 1
        };
        let (cx, cy) = (x.at[i], y.at[i]);
        let c = &cards[i];
        if s.junction {
            icons.push(None);
            nodes.push(SceneNode {
                id: s.id.clone(),
                x: cx,
                y: cy,
                w: c.w,
                h: c.h,
                shape: Shape::Circle,
                label: String::new(),
                style: NodeStyle {
                    fill: Some(EDGE_COLOR.to_string()),
                    stroke: Some(EDGE_COLOR.to_string()),
                    stroke_width: Some(0.0),
                    color: None,
                },
                link: None,
            });
            continue;
        }
        icons.push(
            s.icon
                .as_deref()
                .and_then(icon_glyph)
                .map(|g| g.to_string()),
        );
        nodes.push(SceneNode {
            id: s.id.clone(),
            x: cx,
            y: cy,
            w: c.w,
            h: c.h,
            shape: Shape::Rounded,
            label: c.lines.join("\n"),
            style: NodeStyle {
                fill: Some("#ffffff".to_string()),
                stroke: Some(accent(tint).to_string()),
                stroke_width: Some(2.0),
                color: None,
            },
            link: None,
        });
    }

    // Edge ends: a port on a box side, a junction's centre, or — for
    // a `{group}` end — the point where the port's normal leaves the
    // service's group frame.
    let ends: Vec<(Pt, Pt)> = d
        .edges
        .iter()
        .map(|e| {
            let end = |s: usize, side: ArchSide, group_end: bool| -> Pt {
                let n = &nodes[s];
                let p = port(n, side, d.services[s].junction);
                match d.services[s].group.filter(|_| group_end) {
                    Some(g) if g < frames.len() => {
                        let f = frames[g];
                        match side {
                            ArchSide::T => (p.0, f.y0),
                            ArchSide::B => (p.0, f.y1),
                            ArchSide::L => (f.x0, p.1),
                            ArchSide::R => (f.x1, p.1),
                        }
                    }
                    _ => p,
                }
            };
            (
                end(e.from, e.from_side, e.from_group),
                end(e.to, e.to_side, e.to_group),
            )
        })
        .collect();

    // Obstacles: every service box (junctions are bare points) and
    // every group title, which edges must never strike.
    let boxes: Vec<Rect> = nodes
        .iter()
        .zip(&d.services)
        .filter(|(_, s)| !s.junction)
        .map(|(n, _)| Rect::around(n))
        .collect();
    let mut obstacles: Vec<(Rect, bool)> = boxes.iter().map(|b| (b.grow(CLEAR), true)).collect();
    for (g, f) in frames.iter().enumerate() {
        if title_w[g] > 0.0 {
            let tr = Rect {
                x0: f.x0 + GROUP_PAD,
                y0: f.y0 + TITLE_CY - FONT as f64 * 0.6,
                x1: f.x0 + GROUP_PAD + title_w[g],
                y1: f.y0 + TITLE_CY + FONT as f64 * 0.6,
            };
            obstacles.push((tr.grow(TITLE_CLEAR), true));
        }
    }
    let mut plain: Vec<Rect> = boxes.clone();
    plain.extend(frames.iter().copied());
    let outer = Rect {
        x0: PAD - OUTER,
        y0: PAD - OUTER,
        x1: right + OUTER,
        y1: bottom + OUTER,
    };
    let mut grid = Grid::new(&obstacles, &plain, outer, d, &ends);

    // The canvas keeps PAD around the boxes and frames; a route on the
    // outermost channel (OUTER beyond them) keeps OUTER more, and a
    // label a little.
    let (mut width, mut height) = (right + PAD, bottom + PAD);
    let mut edges: Vec<SceneEdge> = Vec::with_capacity(d.edges.len());
    let mut arrow_starts: Vec<bool> = Vec::with_capacity(d.edges.len());
    let mut arrow_ends: Vec<bool> = Vec::with_capacity(d.edges.len());
    for (e, &(p0, p1)) in d.edges.iter().zip(&ends) {
        let corners = grid.route(p0, e.from_side, p1, e.to_side);
        let label = e.title.as_ref().map(|text| {
            let w = text_px(text) + LABEL_PAD_W;
            (text.clone(), label_spot(&corners, w), w)
        });
        if let Some((_, c, w)) = &label {
            width = width.max(c.0 + w / 2.0 + OUTER);
            height = height.max(c.1 + LABEL_BOX_H / 2.0 + OUTER);
        }
        let waypoints = rounded(&corners);
        for p in &waypoints {
            width = width.max(p.0 + OUTER);
            height = height.max(p.1 + OUTER);
        }
        edges.push(SceneEdge {
            from: d.services[e.from].id.clone(),
            to: d.services[e.to].id.clone(),
            bezier: [p0, p0, p1, p1],
            waypoints,
            kind: if e.arrow_start || e.arrow_end {
                crate::model::EdgeKind::Arrow
            } else {
                crate::model::EdgeKind::Open
            },
            // The arrow ends ride `arrow_starts` / `arrow_ends`.
            ends: Default::default(),
            meta: Default::default(),
            label,
        });
        arrow_starts.push(e.arrow_start);
        arrow_ends.push(e.arrow_end);
    }

    ArchScene {
        scene: Scene {
            nodes,
            edges,
            clusters,
            width,
            height,
        },
        icons,
        arrow_starts,
        arrow_ends,
    }
}

/// The attach point of `side` on a node: the middle of that box side,
/// or a junction's centre (every side of a junction is its centre).
fn port(n: &SceneNode, side: ArchSide, junction: bool) -> Pt {
    if junction {
        return (n.x, n.y);
    }
    let (hw, hh) = (n.w / 2.0, n.h / 2.0);
    match side {
        ArchSide::T => (n.x, n.y - hh),
        ArchSide::B => (n.x, n.y + hh),
        ArchSide::L => (n.x - hw, n.y),
        ArchSide::R => (n.x + hw, n.y),
    }
}

// ------------------------------------------------------------------
// Boxes and labels
// ------------------------------------------------------------------

/// A service box's wrapped label and size.
struct Card {
    lines: Vec<String>,
    w: f64,
    h: f64,
}

/// Estimated width of `s` at [`FONT`].
fn text_px(s: &str) -> f64 {
    text_width(s) * FONT as f64 / TEXT_CALIBRATION
}

/// Estimated width of a bold group title at [`FONT`].
fn title_px(s: &str) -> f64 {
    text_px(s) * 1.06
}

fn card(title: &str, icon: bool) -> Card {
    let lines = wrap_label(title, SERVICE_W - 2.0 * LABEL_PAD_X);
    let text_w = lines.iter().map(|l| text_px(l)).fold(0.0, f64::max);
    Card {
        w: SERVICE_W.max((text_w + 2.0 * LABEL_PAD_X).ceil()),
        h: SERVICE_H.max(block_h(icon, lines.len()) + 2.0 * LABEL_PAD_Y),
        lines,
    }
}

/// Height of a box's icon + label block.
fn block_h(icon: bool, lines: usize) -> f64 {
    let icon_h = match (icon, lines) {
        (false, _) => 0.0,
        (true, 0) => ICON_FONT as f64,
        (true, _) => ICON_FONT as f64 + ICON_GAP,
    };
    icon_h + lines as f64 * LINE_H
}

/// Break a label into lines no wider than `max` px: at `<br/>`, and
/// between words wherever a line would overflow. The words are spread
/// over the fewest lines that fit so that the widest line is as narrow
/// as it can be, rather than filling lines greedily and leaving a
/// stub. A single word wider than `max` keeps a line to itself (the
/// box widens for it).
fn wrap_label(title: &str, max: f64) -> Vec<String> {
    let mut out = Vec::new();
    for para in normalize_breaks(title).split('\n') {
        let words: Vec<&str> = para.split_whitespace().collect();
        if !words.is_empty() {
            out.extend(balance(&words, max));
        }
    }
    out
}

fn balance(words: &[&str], max: f64) -> Vec<String> {
    let n = words.len();
    let width = |i: usize, j: usize| text_px(&words[i..j].join(" "));
    // Greedy filling needs the fewest lines any wrap at `max` can.
    let (mut lines, mut start) = (1, 0);
    for k in 1..n {
        if width(start, k + 1) > max {
            lines += 1;
            start = k;
        }
    }
    if lines == 1 {
        return vec![words.join(" ")];
    }
    // best[l][j]: the narrowest widest line over words[..j] in l lines.
    let mut best = vec![vec![f64::INFINITY; n + 1]; lines + 1];
    let mut cut = vec![vec![0usize; n + 1]; lines + 1];
    best[0][0] = 0.0;
    for l in 1..=lines {
        for j in l..=n {
            for i in (l - 1)..j {
                let c = best[l - 1][i].max(width(i, j));
                if c < best[l][j] - 1e-9 {
                    best[l][j] = c;
                    cut[l][j] = i;
                }
            }
        }
    }
    let mut out = Vec::with_capacity(lines);
    let mut j = n;
    for l in (1..=lines).rev() {
        let i = cut[l][j];
        out.push(words[i..j].join(" "));
        j = i;
    }
    out.reverse();
    out
}

// ------------------------------------------------------------------
// The containment tree
// ------------------------------------------------------------------

/// One leaf of the layout grid: a service, a junction, or the
/// placeholder that keeps an empty group's frame open.
struct Atom {
    /// The container: a group index, or [`Tree::root`] for none.
    parent: usize,
    w: f64,
    h: f64,
}

/// The containment tree the layout works on. Items are numbered atoms
/// first (`0..na()`, the services in declaration order, then one
/// placeholder per empty group), then groups (`na() + g`). Containers
/// are the groups `0..root` and the root itself.
struct Tree {
    atoms: Vec<Atom>,
    root: usize,
    /// Children of each container, as item ids: its groups, then its
    /// atoms, each in declaration order.
    kids: Vec<Vec<usize>>,
    /// The atoms under each item (just itself, for an atom).
    under: Vec<Vec<usize>>,
    /// The items from each atom up to its top-level ancestor, the atom
    /// first.
    chain: Vec<Vec<usize>>,
    /// Nesting depth of each group (0 = top level).
    depth: Vec<usize>,
}

impl Tree {
    fn new(d: &Architecture, sizes: &[Pt]) -> Tree {
        let ng = d.groups.len();
        let root = ng;
        // The parser only lets a group sit in one declared before it;
        // a hand-built model that breaks that (a cycle, an index out of
        // range) hangs the group off the root instead of looping.
        let group_parent: Vec<usize> = d
            .groups
            .iter()
            .enumerate()
            .map(|(g, grp)| grp.parent.filter(|&p| p < g).unwrap_or(root))
            .collect();
        let mut atoms: Vec<Atom> = d
            .services
            .iter()
            .zip(sizes)
            .map(|(s, &(w, h))| Atom {
                parent: s.group.filter(|&g| g < ng).unwrap_or(root),
                w,
                h,
            })
            .collect();
        let mut filled = vec![false; ng + 1];
        for a in &atoms {
            filled[a.parent] = true;
        }
        for &p in &group_parent {
            filled[p] = true;
        }
        for (g, &full) in filled.iter().enumerate().take(ng) {
            if !full {
                atoms.push(Atom {
                    parent: g,
                    w: EMPTY_GROUP_W,
                    h: 0.0,
                });
            }
        }
        let na = atoms.len();
        let mut kids = vec![Vec::new(); ng + 1];
        for (g, &p) in group_parent.iter().enumerate() {
            kids[p].push(na + g);
        }
        for (a, atom) in atoms.iter().enumerate() {
            kids[atom.parent].push(a);
        }
        let mut depth = vec![0; ng];
        for g in 0..ng {
            let p = group_parent[g];
            depth[g] = if p == root { 0 } else { depth[p] + 1 };
        }
        let mut under: Vec<Vec<usize>> = (0..na).map(|a| vec![a]).collect();
        under.resize(na + ng, Vec::new());
        let mut chain = Vec::with_capacity(na);
        for (a, atom) in atoms.iter().enumerate() {
            let mut ch = vec![a];
            let mut c = atom.parent;
            while c != root {
                under[na + c].push(a);
                ch.push(na + c);
                c = group_parent[c];
            }
            chain.push(ch);
        }
        Tree {
            atoms,
            root,
            kids,
            under,
            chain,
            depth,
        }
    }

    fn na(&self) -> usize {
        self.atoms.len()
    }

    fn is_group(&self, item: usize) -> bool {
        item >= self.na()
    }

    /// Whether atom `a` sits (at any depth) in container `c`.
    fn holds(&self, c: usize, a: usize) -> bool {
        c == self.root || self.chain[a].contains(&(self.na() + c))
    }
}

// ------------------------------------------------------------------
// Grid cells
// ------------------------------------------------------------------

/// A rectangle of grid cells, bounds inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CellRect {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl CellRect {
    fn around(cell: &[(i32, i32)], atoms: &[usize]) -> CellRect {
        let mut r = CellRect {
            x0: i32::MAX,
            y0: i32::MAX,
            x1: i32::MIN,
            y1: i32::MIN,
        };
        for &a in atoms {
            let (x, y) = cell[a];
            r.x0 = r.x0.min(x);
            r.y0 = r.y0.min(y);
            r.x1 = r.x1.max(x);
            r.y1 = r.y1.max(y);
        }
        r
    }

    fn shifted(self, (dx, dy): (i32, i32)) -> CellRect {
        CellRect {
            x0: self.x0 + dx,
            y0: self.y0 + dy,
            x1: self.x1 + dx,
            y1: self.y1 + dy,
        }
    }

    fn overlaps(&self, o: &CellRect) -> bool {
        self.x0 <= o.x1 && o.x0 <= self.x1 && self.y0 <= o.y1 && o.y0 <= self.y1
    }

    fn w(&self) -> i32 {
        self.x1 - self.x0 + 1
    }

    fn h(&self) -> i32 {
        self.y1 - self.y0 + 1
    }
}

fn horizontal(side: ArchSide) -> bool {
    matches!(side, ArchSide::L | ArchSide::R)
}

/// The grid step from a service to its neighbour across an edge that
/// leaves the service by side `from` and enters the neighbour by side
/// `to` — mermaid's spatial map: `R→L` puts the neighbour one column
/// right, `B→T` one row down, a mixed pair one cell diagonally (`R→T`:
/// right, and down because it is entered from the top). Two ports on
/// the same side (`R→R`) stack the pair across that side, so the edge
/// is a short U beside them.
fn cell_step(from: ArchSide, to: ArchSide) -> (i32, i32) {
    let sx = |s: ArchSide| if s == ArchSide::R { 1 } else { -1 };
    let sy = |s: ArchSide| if s == ArchSide::B { 1 } else { -1 };
    match (horizontal(from), horizontal(to)) {
        (true, true) if from == to => (0, 1),
        (true, true) => (sx(from), 0),
        (false, false) if from == to => (1, 0),
        (false, false) => (0, sy(from)),
        (true, false) => (sx(from), -sy(to)),
        (false, true) => (-sx(to), sy(from)),
    }
}

/// Grid cell of every atom. A breadth-first walk over the edges steps
/// from each placed service to its neighbours by [`cell_step`]; a cell
/// already taken pushes the newcomer on across the step, so two
/// services feeding one port stack in a column instead of landing on
/// each other. Each
/// connected component is walked on its own grid; [`legalize`] then
/// keeps every group's cells together and packs the components.
fn place_cells(d: &Architecture, t: &Tree) -> Vec<(i32, i32)> {
    let na = t.na();
    let ns = d.services.len();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); na];
    for (k, e) in d.edges.iter().enumerate() {
        if e.from != e.to && e.from < ns && e.to < ns {
            adj[e.from].push(k);
            adj[e.to].push(k);
        }
    }
    let mut cell = vec![(0, 0); na];
    let mut comp = vec![usize::MAX; na];
    let mut next = 0;
    for start in 0..na {
        if comp[start] != usize::MAX {
            continue;
        }
        comp[start] = next;
        let mut taken: HashSet<(i32, i32)> = HashSet::new();
        taken.insert((0, 0));
        let mut queue = VecDeque::from([start]);
        while let Some(u) = queue.pop_front() {
            for &k in &adj[u] {
                let e = &d.edges[k];
                let step = cell_step(e.from_side, e.to_side);
                let (v, step) = if e.from == u {
                    (e.to, step)
                } else {
                    (e.from, (-step.0, -step.1))
                };
                if comp[v] != usize::MAX {
                    continue;
                }
                let want = (cell[u].0 + step.0, cell[u].1 + step.1);
                let c = free_cell(&taken, want, step);
                taken.insert(c);
                cell[v] = c;
                comp[v] = next;
                queue.push_back(v);
            }
        }
        next += 1;
    }
    legalize(d, t, &mut cell, &comp);
    cell
}

/// `want`, or the first free cell past it across the step that led
/// there: below it for a sideways (or diagonal) step, right of it for a
/// vertical one. Always the same way, so the services feeding one port
/// stack in declaration order.
fn free_cell(taken: &HashSet<(i32, i32)>, want: (i32, i32), step: (i32, i32)) -> (i32, i32) {
    let (dx, dy) = if step.0 == 0 { (1, 0) } else { (0, 1) };
    let mut c = want;
    while taken.contains(&c) {
        c = (c.0 + dx, c.1 + dy);
    }
    c
}

fn shift(cell: &mut [(i32, i32)], atoms: &[usize], (dx, dy): (i32, i32)) {
    for &a in atoms {
        cell[a].0 += dx;
        cell[a].1 += dy;
    }
}

/// Make every group's cells contiguous: inside each container, deepest
/// first, no two children's cell rectangles may overlap — so no frame
/// covers a service or frame that is not its own. Once a container is
/// done its children move only together, so the pass never undoes a
/// deeper one. Children of one connected component keep their walked
/// positions, the later of two that collide moving to the nearest free
/// offset; unconnected components are packed in rows beside the first.
fn legalize(d: &Architecture, t: &Tree, cell: &mut [(i32, i32)], comp: &[usize]) {
    let mut order: Vec<usize> = (0..t.root).collect();
    order.sort_by_key(|&g| Reverse(t.depth[g]));
    order.push(t.root);
    for c in order {
        let kids = &t.kids[c];
        let comps: Vec<Vec<usize>> = kids
            .iter()
            .map(|&it| {
                let mut v: Vec<usize> = t.under[it].iter().map(|&a| comp[a]).collect();
                v.sort_unstable();
                v.dedup();
                v
            })
            .collect();
        let mut cluster_of: Vec<usize> = (0..kids.len()).collect();
        for i in 0..kids.len() {
            for j in 0..i {
                if comps[i].iter().any(|x| comps[j].binary_search(x).is_ok()) {
                    let (keep, drop) = (
                        cluster_of[i].min(cluster_of[j]),
                        cluster_of[i].max(cluster_of[j]),
                    );
                    for x in cluster_of.iter_mut().filter(|x| **x == drop) {
                        *x = keep;
                    }
                }
            }
        }
        let mut ids: Vec<usize> = Vec::new();
        let mut clusters: Vec<Vec<usize>> = Vec::new();
        for (i, &id) in cluster_of.iter().enumerate() {
            match ids.iter().position(|&x| x == id) {
                Some(p) => clusters[p].push(i),
                None => {
                    ids.push(id);
                    clusters.push(vec![i]);
                }
            }
        }
        for cl in &clusters {
            let mut placed: Vec<CellRect> = Vec::new();
            for &i in cl {
                let atoms = &t.under[kids[i]];
                let r = CellRect::around(cell, atoms);
                let off = nearest_free(r, &placed);
                shift(cell, atoms, off);
                placed.push(r.shifted(off));
            }
        }
        if clusters.len() > 1 {
            pack(d, t, c, cell, &clusters);
        }
    }
}

/// The smallest offset that moves `r` clear of every placed rectangle:
/// none if it is clear already, then straight moves before diagonal
/// ones — down, right, up, left — so a block that collides steps below
/// or beside what is there.
fn nearest_free(r: CellRect, placed: &[CellRect]) -> (i32, i32) {
    let clear = |off: (i32, i32)| placed.iter().all(|p| !r.shifted(off).overlaps(p));
    if clear((0, 0)) {
        return (0, 0);
    }
    let mut k = 1;
    loop {
        for off in [(0, k), (k, 0), (0, -k), (-k, 0)] {
            if clear(off) {
                return off;
            }
        }
        for a in 1..k {
            let b = k - a;
            for off in [(a, b), (a, -b), (-a, b), (-a, -b)] {
                if clear(off) {
                    return off;
                }
            }
        }
        k += 1;
    }
}

/// Lay a container's unconnected clusters out in rows, the first where
/// it is. Rows hold about √n clusters, so a set of unrelated groups (or
/// edge-less services) forms a block instead of a strip — unless the
/// edges leaving the container favour one side: left/right ports stack
/// the clusters in a column, keeping those sides free, and top/bottom
/// ports line them up in one row.
fn pack(d: &Architecture, t: &Tree, c: usize, cell: &mut [(i32, i32)], clusters: &[Vec<usize>]) {
    let kids = &t.kids[c];
    let members: Vec<Vec<usize>> = clusters
        .iter()
        .map(|cl| {
            cl.iter()
                .flat_map(|&i| t.under[kids[i]].iter().copied())
                .collect()
        })
        .collect();
    let rects: Vec<CellRect> = members.iter().map(|m| CellRect::around(cell, m)).collect();
    let (mut sideways, mut upright) = (0, 0);
    for e in &d.edges {
        for (s, side, other) in [(e.from, e.from_side, e.to), (e.to, e.to_side, e.from)] {
            if s < t.na() && other < t.na() && t.holds(c, s) && !t.holds(c, other) {
                if horizontal(side) {
                    sideways += 1;
                } else {
                    upright += 1;
                }
            }
        }
    }
    let widest = rects.iter().map(CellRect::w).max().unwrap_or(1);
    let row_w = if sideways > upright {
        widest
    } else if upright > sideways {
        i32::MAX
    } else {
        let n = rects.len() as i32;
        let cols = (rects.len() as f64).sqrt().ceil() as i32;
        let total: i32 = rects.iter().map(CellRect::w).sum();
        widest.max((total * cols + n - 1) / n)
    };
    let (ox, oy) = (rects[0].x0, rects[0].y0);
    let (mut x, mut y, mut row_h) = (ox, oy, 0);
    for (m, r) in members.iter().zip(&rects) {
        if x > ox && (x - ox).saturating_add(r.w()) > row_w {
            y += row_h;
            x = ox;
            row_h = 0;
        }
        shift(cell, m, (x - r.x0, y - r.y0));
        x += r.w();
        row_h = row_h.max(r.h());
    }
}

// ------------------------------------------------------------------
// Cells to pixels
// ------------------------------------------------------------------

/// One axis of the pixel solve. Atoms are swept in cell order, each
/// class of aligned atoms placed as far up/left as every constraint
/// allows: the canvas margin plus the frames around it, and — for
/// every sibling pair that lies apart on this axis in cells — the far
/// side of the earlier sibling's frame (or box) plus a gap. A constraint
/// always points from a lower cell coordinate to a higher one, so one
/// pass in cell order settles everything, and a column (row) is only as
/// wide (tall) as what actually sits in it there.
struct Axis<'a> {
    t: &'a Tree,
    /// Cell coordinate of each atom along this axis.
    coord: Vec<i32>,
    /// Extent of each atom along this axis.
    size: Vec<f64>,
    /// Room a frame keeps before its first child (its title band too,
    /// on y) and after its last.
    pad_lo: f64,
    pad_hi: f64,
    /// Smallest extent of each group's frame (its title, on x).
    min_len: Vec<f64>,
    /// For each item, the siblings that lie before it on this axis,
    /// each with the gap to keep.
    pred: Vec<Vec<(usize, f64)>>,
    /// Centre of each atom, once placed.
    at: Vec<f64>,
}

impl Axis<'_> {
    /// Near edge of an item: an atom's box, a group's frame.
    fn lo(&self, it: usize) -> f64 {
        let na = self.t.na();
        if it < na {
            return self.at[it] - self.size[it] / 2.0;
        }
        self.t.kids[it - na]
            .iter()
            .map(|&k| self.lo(k))
            .fold(f64::INFINITY, f64::min)
            - self.pad_lo
    }

    /// Far edge of an item; a frame is at least its title long.
    fn hi(&self, it: usize) -> f64 {
        let na = self.t.na();
        if it < na {
            return self.at[it] + self.size[it] / 2.0;
        }
        let inner = self.t.kids[it - na]
            .iter()
            .map(|&k| self.hi(k))
            .fold(f64::NEG_INFINITY, f64::max)
            + self.pad_hi;
        inner.max(self.lo(it) + self.min_len[it - na])
    }

    /// Lowest near edge atom `m`'s box may take.
    fn bound(&self, m: usize) -> f64 {
        let chain = &self.t.chain[m];
        let mut b = PAD + self.pad_lo * (chain.len() - 1) as f64;
        for (j, &it) in chain.iter().enumerate() {
            for &(p, gap) in &self.pred[it] {
                b = b.max(self.hi(p) + gap + self.pad_lo * j as f64);
            }
        }
        b
    }

    fn sweep(&mut self, class_of: &[usize]) {
        let mut classes: BTreeMap<(i32, usize), Vec<usize>> = BTreeMap::new();
        for a in 0..self.t.na() {
            classes
                .entry((self.coord[a], class_of[a]))
                .or_default()
                .push(a);
        }
        for atoms in classes.values() {
            let c = atoms
                .iter()
                .map(|&m| self.bound(m) + self.size[m] / 2.0)
                .fold(f64::NEG_INFINITY, f64::max);
            for &m in atoms {
                self.at[m] = c;
            }
        }
    }
}

fn find(uf: &mut [usize], mut a: usize) -> usize {
    while uf[a] != a {
        uf[a] = uf[uf[a]];
        a = uf[a];
    }
    a
}

fn union(uf: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (find(uf, a), find(uf, b));
    if ra != rb {
        uf[ra.max(rb)] = ra.min(rb);
    }
}

/// Pixel centres of every atom (x axis, y axis) from its cell.
fn solve<'a>(
    d: &Architecture,
    t: &'a Tree,
    cell: &[(i32, i32)],
    title_w: &[f64],
) -> (Axis<'a>, Axis<'a>) {
    let na = t.na();
    let ns = d.services.len();
    let items = na + t.root;
    // Atoms an edge joins, with the widest label among those edges.
    let mut joins: HashMap<(usize, usize), f64> = HashMap::new();
    for e in &d.edges {
        if e.from != e.to && e.from < ns && e.to < ns {
            let w = e.title.as_deref().map_or(0.0, text_px);
            let slot = joins
                .entry((e.from.min(e.to), e.from.max(e.to)))
                .or_insert(0.0);
            *slot = slot.max(w);
        }
    }
    let link = |p: usize, q: usize| -> Option<f64> {
        let mut best: Option<f64> = None;
        for &a in &t.under[p] {
            for &b in &t.under[q] {
                if let Some(&w) = joins.get(&(a.min(b), a.max(b))) {
                    best = Some(best.map_or(w, |x: f64| x.max(w)));
                }
            }
        }
        best
    };
    // A labelled edge needs its stubs plus the label between the boxes.
    let label_room = |w: f64, on_x: bool| {
        if w <= 0.0 {
            0.0
        } else if on_x {
            w + LABEL_PAD_W + 2.0 * STUB + 8.0
        } else {
            LABEL_BOX_H + 2.0 * STUB + 8.0
        }
    };
    let gap = |p: usize, q: usize, on_x: bool| -> f64 {
        let lw = link(p, q);
        if t.is_group(p) || t.is_group(q) {
            let room = lw.map_or(0.0, |w| label_room(w, on_x) - 2.0 * GROUP_PAD);
            GROUP_GAP.max(room)
        } else if let Some(w) = lw {
            EDGE_GAP.max(label_room(w, on_x))
        } else if on_x {
            CHILD_GAP_X
        } else {
            CHILD_GAP_Y
        }
    };
    let rect: Vec<CellRect> = (0..items)
        .map(|it| CellRect::around(cell, &t.under[it]))
        .collect();
    let mut xpred = vec![Vec::new(); items];
    let mut ypred = vec![Vec::new(); items];
    for kids in &t.kids {
        for (i, &p) in kids.iter().enumerate() {
            for &q in &kids[i + 1..] {
                let (a, b) = (rect[p], rect[q]);
                let rows = a.y0 <= b.y1 && b.y0 <= a.y1;
                let cols = a.x0 <= b.x1 && b.x0 <= a.x1;
                debug_assert!(!(rows && cols), "siblings overlap after legalize");
                // Apart on x (side by side, or diagonal): the later one
                // keeps clear of the earlier one's far side; diagonal
                // pairs keep clear on both axes, so the diagonal holds.
                if !cols {
                    let (f, s) = if a.x1 < b.x0 { (p, q) } else { (q, p) };
                    xpred[s].push((f, gap(f, s, true)));
                }
                if !rows {
                    let (f, s) = if a.y1 < b.y0 { (p, q) } else { (q, p) };
                    ypred[s].push((f, gap(f, s, false)));
                }
            }
        }
    }
    // Two boxes an edge joins port-to-port across the gap between them
    // (`a:R -- L:b` with `b` in a later column) keep an edge's gap even
    // when they are not siblings — services of two groups that are
    // stacked rather than side by side — so the edge keeps a shaft.
    for e in &d.edges {
        if e.from == e.to || e.from >= ns || e.to >= ns || e.from_side == e.to_side {
            continue;
        }
        let w = e.title.as_deref().map_or(0.0, text_px);
        let (a, b) = (e.from, e.to);
        match (e.from_side, horizontal(e.to_side)) {
            (ArchSide::R, true) if cell[a].0 < cell[b].0 => {
                xpred[b].push((a, EDGE_GAP.max(label_room(w, true))))
            }
            (ArchSide::L, true) if cell[b].0 < cell[a].0 => {
                xpred[a].push((b, EDGE_GAP.max(label_room(w, true))))
            }
            (ArchSide::B, false) if cell[a].1 < cell[b].1 => {
                ypred[b].push((a, EDGE_GAP.max(label_room(w, false))))
            }
            (ArchSide::T, false) if cell[b].1 < cell[a].1 => {
                ypred[a].push((b, EDGE_GAP.max(label_room(w, false))))
            }
            _ => {}
        }
    }
    // Aligned atoms share a centre line: the members of one container
    // that share a column (row), and the two ends of a vertical
    // (horizontal) edge that the grid put in one column (row), so that
    // edge is a straight segment.
    let mut xs: Vec<usize> = (0..na).collect();
    let mut ys: Vec<usize> = (0..na).collect();
    for kids in &t.kids {
        let atoms: Vec<usize> = kids.iter().copied().filter(|&k| k < na).collect();
        for (i, &a) in atoms.iter().enumerate() {
            for &b in &atoms[..i] {
                if cell[a].0 == cell[b].0 {
                    union(&mut xs, a, b);
                }
                if cell[a].1 == cell[b].1 {
                    union(&mut ys, a, b);
                }
            }
        }
    }
    for e in &d.edges {
        if e.from == e.to || e.from >= ns || e.to >= ns {
            continue;
        }
        let (h0, h1) = (horizontal(e.from_side), horizontal(e.to_side));
        if !h0 && !h1 && cell[e.from].0 == cell[e.to].0 {
            union(&mut xs, e.from, e.to);
        }
        if h0 && h1 && cell[e.from].1 == cell[e.to].1 {
            union(&mut ys, e.from, e.to);
        }
    }
    let x_class: Vec<usize> = (0..na).map(|a| find(&mut xs, a)).collect();
    let y_class: Vec<usize> = (0..na).map(|a| find(&mut ys, a)).collect();

    let mut x = Axis {
        t,
        coord: cell.iter().map(|c| c.0).collect(),
        size: t.atoms.iter().map(|a| a.w).collect(),
        pad_lo: GROUP_PAD,
        pad_hi: GROUP_PAD,
        min_len: title_w.iter().map(|w| w + 2.0 * GROUP_PAD).collect(),
        pred: xpred,
        at: vec![0.0; na],
    };
    x.sweep(&x_class);
    let mut y = Axis {
        t,
        coord: cell.iter().map(|c| c.1).collect(),
        size: t.atoms.iter().map(|a| a.h).collect(),
        pad_lo: GROUP_PAD + GROUP_TITLE_H,
        pad_hi: GROUP_PAD,
        min_len: vec![0.0; t.root],
        pred: ypred,
        at: vec![0.0; na],
    };
    y.sweep(&y_class);
    (x, y)
}

// ------------------------------------------------------------------
// Edge routing
// ------------------------------------------------------------------

/// An axis-aligned box `(x0, y0)–(x1, y1)`.
#[derive(Debug, Clone, Copy)]
struct Rect {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Rect {
    fn around(n: &SceneNode) -> Rect {
        Rect {
            x0: n.x - n.w / 2.0,
            y0: n.y - n.h / 2.0,
            x1: n.x + n.w / 2.0,
            y1: n.y + n.h / 2.0,
        }
    }

    fn grow(self, m: f64) -> Rect {
        Rect {
            x0: self.x0 - m,
            y0: self.y0 - m,
            x1: self.x1 + m,
            y1: self.y1 + m,
        }
    }
}

/// Tolerance for coordinates that should coincide.
const EPS: f64 = 1e-6;

/// Direction indices: east, south, west, north (screen y points down).
const DIRS: [(f64, f64); 4] = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];

/// The outward direction of a port side, as a [`DIRS`] index.
fn dir_of(side: ArchSide) -> usize {
    match side {
        ArchSide::R => 0,
        ArchSide::B => 1,
        ArchSide::L => 2,
        ArchSide::T => 3,
    }
}

/// The fewest turns an orthogonal path heading `d` needs to reach a
/// point `(dx, dy)` away in an empty plane: none if it already heads
/// straight at it, one if it heads along one of the two legs (or
/// across the only one), two otherwise.
fn min_turns(dx: f64, dy: f64, d: usize) -> u32 {
    let need = |dir: usize| match dir {
        0 => dx > EPS,
        1 => dy > EPS,
        2 => dx < -EPS,
        _ => dy < -EPS,
    };
    let legs = (0..4).filter(|&k| need(k)).count();
    match (legs, need(d)) {
        (0, _) | (1, true) => 0,
        (2, true) => 1,
        (1, false) if !need((d + 2) % 4) => 1,
        _ => 2,
    }
}

/// The orthogonal routing lattice: lines along every box's clearance
/// boundary and title, down the middle of every gap between boxes and
/// frames, through every port and its stub point, and just outside the
/// drawing. A route is a cheapest path along the lines — length, plus a
/// price per bend, per crossing of an edge routed earlier and per px
/// run on top of one — that never enters a box's clearance.
struct Grid {
    xs: Vec<f64>,
    ys: Vec<f64>,
    /// Cost per px of running along each vertical (`x_rate`) or
    /// horizontal (`y_rate`) line.
    x_rate: Vec<f64>,
    y_rate: Vec<f64>,
    /// Lattice points inside a clearance, indexed `j * nx + i`.
    blocked: Vec<bool>,
    /// Whether the step east (`h_open`) / south (`v_open`) of a point
    /// stays out of every clearance.
    h_open: Vec<bool>,
    v_open: Vec<bool>,
    /// The routed edges using each lattice step, at `2 * point` for
    /// the step east of a point and `2 * point + 1` for the step south
    /// of it.
    runs: Vec<Vec<usize>>,
    /// The routed edges through each point, each with whether it runs
    /// east-west there.
    marks: Vec<Vec<(usize, bool)>>,
    /// The two end points of each routed edge. Edges that share an end
    /// (a fan-in to one port) may run together without paying for it,
    /// so they merge into one trunk in the gap, as a bus would.
    ports: Vec<[Pt; 2]>,
    /// Search buffers, per (point, heading) state, kept between
    /// searches: a state's entries are current only when its `seen`
    /// stamp is the running search's.
    dist_to: Vec<f64>,
    prev: Vec<usize>,
    seen: Vec<u32>,
    done: Vec<u32>,
    stamp: u32,
}

/// Sort and de-duplicate lattice lines, a coincident pair keeping the
/// cheaper rate.
fn dedup_lines(mut v: Vec<(f64, f64)>) -> (Vec<f64>, Vec<f64>) {
    v.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (mut at, mut rate): (Vec<f64>, Vec<f64>) = (Vec::new(), Vec::new());
    for (x, r) in v {
        match at.last() {
            Some(&last) if (x - last).abs() < EPS => {
                let k = rate.len() - 1;
                rate[k] = rate[k].min(r);
            }
            _ => {
                at.push(x);
                rate.push(r);
            }
        }
    }
    (at, rate)
}

/// Middle of the gap between each rect and the nearest edge to its
/// right (`on_x`) or below among the rects level with it: a
/// neighbour's near side, or the far side of a frame around it — so a
/// gap between two frames gets its own middle line, and a box's line
/// stops at its own frame rather than at the next frame over.
fn midlines(rects: &[Rect], on_x: bool) -> Vec<f64> {
    let span = |r: &Rect| {
        if on_x {
            (r.x0, r.x1, r.y0, r.y1)
        } else {
            (r.y0, r.y1, r.x0, r.x1)
        }
    };
    let mut out = Vec::new();
    for a in rects {
        let (_, a1, a_lo, a_hi) = span(a);
        let near = rects
            .iter()
            .filter(|b| {
                let (_, _, b_lo, b_hi) = span(b);
                a_lo < b_hi && b_lo < a_hi
            })
            .flat_map(|b| {
                let (b0, b1, _, _) = span(b);
                [b0, b1]
            })
            .filter(|&c| c > a1 + EPS)
            .fold(f64::INFINITY, f64::min);
        if near.is_finite() {
            out.push((a1 + near) / 2.0);
        }
    }
    out
}

impl Grid {
    /// `obstacles` are clearances routes may not enter (with whether
    /// their boundary lines cost [`HUG_RATE`]); `plain` the boxes and
    /// frames whose gaps get a middle line; `outer` the outermost lines.
    fn new(
        obstacles: &[(Rect, bool)],
        plain: &[Rect],
        outer: Rect,
        d: &Architecture,
        ends: &[(Pt, Pt)],
    ) -> Grid {
        // Lattice lines as (coordinate, rate).
        let mut xl: Vec<(f64, f64)> = Vec::new();
        let mut yl: Vec<(f64, f64)> = Vec::new();
        for &(o, hug) in obstacles {
            let rate = if hug { HUG_RATE } else { 1.0 };
            xl.extend([(o.x0, rate), (o.x1, rate)]);
            yl.extend([(o.y0, rate), (o.y1, rate)]);
        }
        xl.extend(midlines(plain, true).into_iter().map(|x| (x, 1.0)));
        yl.extend(midlines(plain, false).into_iter().map(|y| (y, 1.0)));
        xl.extend([(outer.x0, 1.0), (outer.x1, 1.0)]);
        yl.extend([(outer.y0, 1.0), (outer.y1, 1.0)]);
        // A search runs between stub ends, so each stub end needs both
        // its lines: the port's own line along the normal, and the line
        // across it, which costs HUG_RATE so a route turns there only
        // when it is about to enter the port.
        for (e, &(p0, p1)) in d.edges.iter().zip(ends) {
            for (p, side) in [(p0, e.from_side), (p1, e.to_side)] {
                let n = DIRS[dir_of(side)];
                let s = (p.0 + n.0 * STUB, p.1 + n.1 * STUB);
                if horizontal(side) {
                    xl.push((s.0, HUG_RATE));
                    yl.push((p.1, 1.0));
                } else {
                    yl.push((s.1, HUG_RATE));
                    xl.push((p.0, 1.0));
                }
            }
        }
        let (xs, x_rate) = dedup_lines(xl);
        let (ys, y_rate) = dedup_lines(yl);
        let (nx, ny) = (xs.len(), ys.len());
        let inside = |lo: f64, v: f64, hi: f64| lo + EPS < v && v < hi - EPS;
        let mut blocked = vec![false; nx * ny];
        let mut h_open = vec![false; nx * ny];
        let mut v_open = vec![false; nx * ny];
        for j in 0..ny {
            for i in 0..nx {
                let (x, y) = (xs[i], ys[j]);
                let v = j * nx + i;
                blocked[v] = obstacles
                    .iter()
                    .any(|(o, _)| inside(o.x0, x, o.x1) && inside(o.y0, y, o.y1));
                if i + 1 < nx {
                    let x2 = xs[i + 1];
                    h_open[v] = !obstacles
                        .iter()
                        .any(|(o, _)| inside(o.y0, y, o.y1) && x < o.x1 - EPS && x2 > o.x0 + EPS);
                }
                if j + 1 < ny {
                    let y2 = ys[j + 1];
                    v_open[v] = !obstacles
                        .iter()
                        .any(|(o, _)| inside(o.x0, x, o.x1) && y < o.y1 - EPS && y2 > o.y0 + EPS);
                }
            }
        }
        Grid {
            xs,
            ys,
            x_rate,
            y_rate,
            blocked,
            h_open,
            v_open,
            runs: vec![Vec::new(); 2 * nx * ny],
            marks: vec![Vec::new(); nx * ny],
            ports: Vec::new(),
            dist_to: vec![0.0; 4 * nx * ny],
            prev: vec![0; 4 * nx * ny],
            seen: vec![0; 4 * nx * ny],
            done: vec![0; 4 * nx * ny],
            stamp: 0,
        }
    }

    fn point(&self, v: usize) -> Pt {
        let nx = self.xs.len();
        (self.xs[v % nx], self.ys[v / nx])
    }

    fn node_at(&self, p: Pt) -> Option<usize> {
        let find = |v: &[f64], x: f64| v.iter().position(|&a| (a - x).abs() < EPS);
        Some(find(&self.ys, p.1)? * self.xs.len() + find(&self.xs, p.0)?)
    }

    /// The neighbour of `v` in direction `d`, with the step's length,
    /// its rate, and its key in [`Grid::runs`].
    fn step(&self, v: usize, d: usize) -> Option<(usize, f64, f64, usize)> {
        let (nx, ny) = (self.xs.len(), self.ys.len());
        let (i, j) = (v % nx, v / nx);
        match d {
            0 => (i + 1 < nx && self.h_open[v])
                .then(|| (v + 1, self.xs[i + 1] - self.xs[i], self.y_rate[j], 2 * v)),
            2 => (i > 0 && self.h_open[v - 1]).then(|| {
                (
                    v - 1,
                    self.xs[i] - self.xs[i - 1],
                    self.y_rate[j],
                    2 * (v - 1),
                )
            }),
            1 => (j + 1 < ny && self.v_open[v]).then(|| {
                (
                    v + nx,
                    self.ys[j + 1] - self.ys[j],
                    self.x_rate[i],
                    2 * v + 1,
                )
            }),
            _ => (j > 0 && self.v_open[v - nx]).then(|| {
                let w = v - nx;
                (w, self.ys[j] - self.ys[j - 1], self.x_rate[i], 2 * w + 1)
            }),
        }
    }

    /// Cheapest lattice path from `start` (leaving in direction `d0`)
    /// to `goal` (where the route turns into the port moving `d_in`),
    /// by A* over (point, heading), for an edge whose ends are `mine`.
    /// The estimate is the Manhattan distance plus a bend for every
    /// turn an empty plane would still need ([`min_turns`]), so it
    /// never overestimates. Ties go to the lower state index, so the
    /// result is deterministic.
    fn search(
        &mut self,
        start: usize,
        d0: usize,
        goal: usize,
        d_in: usize,
        mine: [Pt; 2],
    ) -> Option<Vec<usize>> {
        if start == goal {
            return Some(vec![start]);
        }
        self.stamp += 1;
        let stamp = self.stamp;
        let g_pt = self.point(goal);
        let est = |grid: &Grid, v: usize, d: usize| {
            let p = grid.point(v);
            let (dx, dy) = (g_pt.0 - p.0, g_pt.1 - p.1);
            dx.abs() + dy.abs() + BEND_COST * min_turns(dx, dy, d) as f64
        };
        let foreign = |grid: &Grid, r: usize| {
            !grid.ports[r]
                .iter()
                .any(|q| mine.iter().any(|m| dist(*q, *m) < EPS))
        };
        let mut heap = BinaryHeap::new();
        let s0 = start * 4 + d0;
        self.dist_to[s0] = 0.0;
        self.prev[s0] = usize::MAX;
        self.seen[s0] = stamp;
        heap.push(Reverse((est(self, start, d0).to_bits(), s0)));
        while let Some(Reverse((_, s))) = heap.pop() {
            if self.done[s] == stamp {
                continue;
            }
            self.done[s] = stamp;
            let (v, d) = (s / 4, s % 4);
            if v == goal {
                let mut path = vec![v];
                let mut at = s;
                while self.prev[at] != usize::MAX {
                    at = self.prev[at];
                    path.push(at / 4);
                }
                path.reverse();
                return Some(path);
            }
            for nd in 0..4 {
                if nd == (d + 2) % 4 {
                    continue;
                }
                let Some((w, len, rate, key)) = self.step(v, nd) else {
                    continue;
                };
                if self.blocked[w] && w != goal {
                    continue;
                }
                let shared = self.runs[key].iter().filter(|&&r| foreign(self, r)).count();
                let mut c = self.dist_to[s] + len * rate + OVERLAP_COST * len * shared as f64;
                let east_west = nd % 2 == 0;
                let across = self.marks[w]
                    .iter()
                    .any(|&(r, ew)| ew != east_west && foreign(self, r));
                if across && w != goal {
                    c += CROSS_COST;
                }
                if nd != d {
                    c += BEND_COST;
                }
                if w == goal {
                    c += if nd == d_in {
                        0.0
                    } else if nd == (d_in + 2) % 4 {
                        2.0 * BEND_COST
                    } else {
                        BEND_COST
                    };
                }
                let ws = w * 4 + nd;
                if self.seen[ws] != stamp || c < self.dist_to[ws] - EPS {
                    self.seen[ws] = stamp;
                    self.dist_to[ws] = c;
                    self.prev[ws] = s;
                    heap.push(Reverse(((c + est(self, w, nd)).to_bits(), ws)));
                }
            }
        }
        None
    }

    /// Mark a routed path so later edges pay to cross or share it.
    fn record(&mut self, path: &[usize], ends: [Pt; 2]) {
        let r = self.ports.len();
        self.ports.push(ends);
        let nx = self.xs.len();
        for w in path.windows(2) {
            let (a, b) = (w[0].min(w[1]), w[0].max(w[1]));
            let east_west = a / nx == b / nx;
            let key = 2 * a + usize::from(!east_west);
            self.runs[key].push(r);
            for p in [a, b] {
                self.marks[p].push((r, east_west));
            }
        }
    }

    /// The corner points of the route from port `p0` (side `side0`) to
    /// port `p1` (side `side1`): straight out of `p0` along its normal
    /// for [`STUB`], along the lattice, and straight into `p1`.
    fn route(&mut self, p0: Pt, side0: ArchSide, p1: Pt, side1: ArchSide) -> Vec<Pt> {
        let (d0, d1) = (dir_of(side0), dir_of(side1));
        let (n0, n1) = (DIRS[d0], DIRS[d1]);
        let s0 = (p0.0 + n0.0 * STUB, p0.1 + n0.1 * STUB);
        let s1 = (p1.0 + n1.0 * STUB, p1.1 + n1.1 * STUB);
        let path = match (self.node_at(s0), self.node_at(s1)) {
            (Some(a), Some(b)) => self.search(a, d0, b, (d1 + 2) % 4, [p0, p1]),
            _ => None,
        };
        let mut corners = vec![p0];
        match path {
            Some(path) => {
                self.record(&path, [p0, p1]);
                corners.extend(path.iter().map(|&v| self.point(v)));
            }
            // Unreachable on a lattice that runs around the drawing;
            // kept so a degenerate model still draws something.
            None => corners.extend([s0, (s1.0, s0.1), s1]),
        }
        corners.push(p1);
        simplify(&corners)
    }
}

/// Drop repeated points and the middle of three collinear ones.
fn simplify(pts: &[Pt]) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(pts.len());
    for &p in pts {
        if out.last().is_some_and(|q| dist(*q, p) < EPS) {
            continue;
        }
        if out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            let cross = (b.0 - a.0) * (p.1 - b.1) - (b.1 - a.1) * (p.0 - b.0);
            let forward = (b.0 - a.0) * (p.0 - b.0) + (b.1 - a.1) * (p.1 - b.1) > 0.0;
            if cross.abs() < EPS && forward {
                out.pop();
            }
        }
        out.push(p);
    }
    out
}

fn dist(a: Pt, b: Pt) -> f64 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

/// The corner path with every corner rounded (radius [`CORNER_R`], at
/// most half of either leg), each corner sampled [`CORNER_STEPS`]
/// times: drawn as a polyline it reads as a smooth bend, and a host
/// splining through the points (see [`SceneEdge::waypoints`]) draws the
/// same shape.
fn rounded(corners: &[Pt]) -> Vec<Pt> {
    let n = corners.len();
    if n < 3 {
        return corners.to_vec();
    }
    let mut out = vec![corners[0]];
    for k in 1..n - 1 {
        let (a, c, b) = (corners[k - 1], corners[k], corners[k + 1]);
        let (l1, l2) = (dist(a, c), dist(c, b));
        let r = CORNER_R.min(l1 / 2.0).min(l2 / 2.0);
        if r < 0.5 {
            out.push(c);
            continue;
        }
        let p = (c.0 + (a.0 - c.0) / l1 * r, c.1 + (a.1 - c.1) / l1 * r);
        let q = (c.0 + (b.0 - c.0) / l2 * r, c.1 + (b.1 - c.1) / l2 * r);
        for s in 0..=CORNER_STEPS {
            let u = s as f64 / CORNER_STEPS as f64;
            let (w0, w1, w2) = ((1.0 - u) * (1.0 - u), 2.0 * u * (1.0 - u), u * u);
            out.push((
                w0 * p.0 + w1 * c.0 + w2 * q.0,
                w0 * p.1 + w1 * c.1 + w2 * q.1,
            ));
        }
    }
    out.push(corners[n - 1]);
    out.dedup_by(|a, b| dist(*a, *b) < EPS);
    out
}

/// Centre for an edge label of width `w`: the middle of the route's
/// longest horizontal run that holds it, else of its longest run.
fn label_spot(corners: &[Pt], w: f64) -> Pt {
    let mid = |s: &[Pt]| ((s[0].0 + s[1].0) / 2.0, (s[0].1 + s[1].1) / 2.0);
    let flat = corners
        .windows(2)
        .filter(|s| (s[0].1 - s[1].1).abs() < EPS && dist(s[0], s[1]) >= w + 8.0)
        .max_by(|a, b| dist(a[0], a[1]).total_cmp(&dist(b[0], b[1])));
    let any = corners
        .windows(2)
        .max_by(|a, b| dist(a[0], a[1]).total_cmp(&dist(b[0], b[1])));
    flat.or(any).map_or(corners[0], mid)
}

// ------------------------------------------------------------------
// SVG
// ------------------------------------------------------------------

/// Serialise an architecture scene to SVG.
pub fn to_svg(as_: &ArchScene) -> String {
    to_svg_with(as_, &SvgOptions::default())
}

/// [`to_svg`] with explicit viewport options (see [`SvgOptions`]).
pub fn to_svg_with(as_: &ArchScene, opts: &SvgOptions) -> String {
    to_svg_a11y_with(as_, &A11y::default(), opts)
}

/// [`to_svg_with`] plus mermaid's `accTitle` / `accDescr` (see
/// [`A11y`]).
pub(crate) fn to_svg_a11y_with(as_: &ArchScene, a11y: &A11y, opts: &SvgOptions) -> String {
    let (title, desc) = a11y.resolve("Architecture diagram");
    let mut s = String::new();
    svg_open_a11y(
        &mut s,
        as_.scene.width,
        as_.scene.height,
        FONT,
        &title,
        desc,
        opts,
    );

    s.push_str(&format!(
        "<defs>\
         <marker id='flowmaid-arch-arrow-end' markerWidth='10' markerHeight='10' \
         refX='9' refY='5' orient='auto' markerUnits='userSpaceOnUse'>\
         <path d='M 0,0 L 10,5 L 0,10 L 1,5 z' fill='{0}'/>\
         </marker>\
         <marker id='flowmaid-arch-arrow-start' markerWidth='10' markerHeight='10' \
         refX='1' refY='5' orient='auto' markerUnits='userSpaceOnUse'>\
         <path d='M 10,0 L 0,5 L 10,10 L 9,5 z' fill='{0}'/>\
         </marker>\
         </defs>\n",
        EDGE_COLOR
    ));

    // Group boxes, outermost first.
    for c in &as_.scene.clusters {
        s.push_str(&format!(
            "<rect x='{:.1}' y='{:.1}' width='{:.1}' height='{:.1}' \
             rx='8' fill='#f7f8fc' stroke='#d5d9ec' stroke-width='2'/>\n",
            c.x, c.y, c.w, c.h
        ));
        if !c.title.is_empty() {
            s.push_str(&format!(
                "<text x='{:.1}' y='{:.1}' dy='0.33em' font-weight='bold' \
                 fill='{}'>{}</text>\n",
                c.x + GROUP_PAD,
                c.y + TITLE_CY,
                TEXT_COLOR,
                escape(&c.title)
            ));
        }
    }

    // Service boxes. Edges come after them: a route never runs under a
    // box, so drawing on top hides nothing and shows any mistake.
    for (i, n) in as_.scene.nodes.iter().enumerate() {
        if n.shape == Shape::Circle {
            continue; // a junction: its dot goes over the edges below
        }
        let fill = crate::scene::style_attr(n.style.fill.as_deref(), "#ffffff");
        let stroke = crate::scene::style_attr(n.style.stroke.as_deref(), EDGE_COLOR);
        s.push_str(&format!(
            "<rect x='{:.1}' y='{:.1}' width='{:.1}' height='{:.1}' \
             rx='8' fill='{}' stroke='{}' stroke-width='{:.1}'/>\n",
            n.x - n.w / 2.0,
            n.y - n.h / 2.0,
            n.w,
            n.h,
            fill,
            stroke,
            n.style.stroke_width.unwrap_or(2.0)
        ));
        // Icon above the label lines, the whole block centred.
        let icon = as_.icons.get(i).and_then(|g| g.as_deref());
        let lines: Vec<&str> = if n.label.is_empty() {
            Vec::new()
        } else {
            n.label.split('\n').collect()
        };
        let mut top = n.y - block_h(icon.is_some(), lines.len()) / 2.0;
        if let Some(icon) = icon {
            s.push_str(&format!(
                "<text x='{:.1}' y='{:.1}' dy='0.33em' text-anchor='middle' \
                 font-size='{}'>{}</text>\n",
                n.x,
                top + ICON_FONT as f64 / 2.0,
                ICON_FONT,
                escape(icon)
            ));
            top += ICON_FONT as f64 + ICON_GAP;
        }
        for (k, line) in lines.iter().enumerate() {
            s.push_str(&format!(
                "<text x='{:.1}' y='{:.1}' dy='0.33em' text-anchor='middle' \
                 fill='{}'>{}</text>\n",
                n.x,
                top + LINE_H * (k as f64 + 0.5),
                TEXT_COLOR,
                escape(line)
            ));
        }
    }

    // Edges.
    for (i, e) in as_.scene.edges.iter().enumerate() {
        let points = e
            .waypoints
            .iter()
            .map(|(x, y)| format!("{:.1},{:.1}", x, y))
            .collect::<Vec<_>>()
            .join(" ");
        let mut markers = String::new();
        // A single auto-oriented marker can only point one way, so the two
        // arrowheads need distinct marker instances with explicit orientation.
        if as_.arrow_ends.get(i).copied().unwrap_or(false) {
            markers.push_str(" marker-end='url(#flowmaid-arch-arrow-end)'");
        }
        if as_.arrow_starts.get(i).copied().unwrap_or(false) {
            markers.push_str(" marker-start='url(#flowmaid-arch-arrow-start)'");
        }
        s.push_str(&format!(
            "<polyline points='{}' fill='none' stroke='{}' stroke-width='2' \
             stroke-linejoin='round'{} />\n",
            points, EDGE_COLOR, markers
        ));
    }
    for e in &as_.scene.edges {
        if let Some((text, c, w)) = &e.label {
            svg_label_box(&mut s, text, *c, *w);
        }
    }

    // Junction dots, over the edges they join.
    for n in as_.scene.nodes.iter().filter(|n| n.shape == Shape::Circle) {
        s.push_str(&format!(
            "<circle cx='{:.1}' cy='{:.1}' r='{:.1}' fill='{}'/>\n",
            n.x,
            n.y,
            n.w / 2.0,
            crate::scene::style_attr(n.style.fill.as_deref(), EDGE_COLOR)
        ));
    }

    s.push_str("</svg>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_document;

    fn arch(src: &str) -> Architecture {
        match parse_document(src).unwrap() {
            crate::model::Document::Architecture(a) => a,
            other => panic!("expected architecture, got {:?}", other),
        }
    }

    #[test]
    fn basic_group_and_service() {
        let a = arch(
            "architecture-beta\n\
             group infra[Infrastructure]\n\
             service db(Database)[Database] in infra\n\
             service api(API)[API] in infra",
        );
        assert_eq!(a.groups.len(), 1);
        assert_eq!(a.services.len(), 2);
        let sc = scene(&a);
        assert_eq!(sc.scene.nodes.len(), 2);
        assert_eq!(sc.scene.clusters.len(), 1);
    }

    #[test]
    fn edge_between_services() {
        let a = arch(
            "architecture-beta\n\
             group g[Group]\n\
             service a(A)[A] in g\n\
             service b(B)[B] in g\n\
             a:R -- L:b",
        );
        let sc = scene(&a);
        assert_eq!(sc.scene.edges.len(), 1);
        assert!(!sc.scene.edges[0].waypoints.is_empty());
    }

    #[test]
    fn directed_edge() {
        let a = arch(
            "architecture-beta\n\
             group g[Group]\n\
             service a(A)[A] in g\n\
             service b(B)[B] in g\n\
             a:R --> L:b",
        );
        let e = &a.edges[0];
        assert!(!e.arrow_start);
        assert!(e.arrow_end);
        let sc = scene(&a);
        assert_eq!(sc.scene.edges[0].kind, crate::model::EdgeKind::Arrow);
    }

    #[test]
    fn bidirectional_edge() {
        let a = arch(
            "architecture-beta\n\
             group g[Group]\n\
             service a(A)[A] in g\n\
             service b(B)[B] in g\n\
             a:R <--> L:b",
        );
        let e = &a.edges[0];
        assert!(e.arrow_start);
        assert!(e.arrow_end);
    }

    #[test]
    fn reverse_arrow_edge() {
        let a = arch(
            "architecture-beta\n\
             group g[Group]\n\
             service a(A)[A] in g\n\
             service b(B)[B] in g\n\
             a:R <-- L:b",
        );
        let e = &a.edges[0];
        assert!(e.arrow_start);
        assert!(!e.arrow_end);
    }

    #[test]
    fn svg_markers_respect_per_end_arrows() {
        let a = arch(
            "architecture-beta\n\
             group g[Group]\n\
             service a(A)[A] in g\n\
             service b(B)[B] in g\n\
             service c(C)[C] in g\n\
             service d(D)[D] in g\n\
             a:R --> L:b\n\
             c:R <-- L:d\n\
             b:R -- L:c",
        );
        let svg = to_svg(&scene(&a));

        // Only the directed `-->` polyline gets a marker-end; only the
        // `<--` polyline gets a marker-start; the bare `--` gets neither.
        // Each edge renders exactly one polyline, in declaration order.
        let ends = svg
            .matches("marker-end='url(#flowmaid-arch-arrow-end)'")
            .count();
        let starts = svg
            .matches("marker-start='url(#flowmaid-arch-arrow-start)'")
            .count();
        assert_eq!(ends, 1, "only the --> edge has an end arrowhead");
        assert_eq!(starts, 1, "only the <-- edge has a start arrowhead");
    }

    #[test]
    fn bidirectional_svg_has_both_markers() {
        let a = arch(
            "architecture-beta\n\
             group g[Group]\n\
             service a(A)[A] in g\n\
             service b(B)[B] in g\n\
             a:R <--> L:b",
        );
        let svg = to_svg(&scene(&a));
        assert!(svg.contains("marker-end='url(#flowmaid-arch-arrow-end)'"));
        assert!(svg.contains("marker-start='url(#flowmaid-arch-arrow-start)'"));
    }

    #[test]
    fn group_qualifier_on_edge() {
        let a = arch(
            "architecture-beta\n\
             group g1[One]\n\
             group g2[Two]\n\
             service a(A)[A] in g1\n\
             service b(B)[B] in g2\n\
             a{g1}:R --> L:b{g2}",
        );
        assert_eq!(a.edges.len(), 1);
        let sc = scene(&a);
        assert_eq!(sc.scene.edges[0].kind, crate::model::EdgeKind::Arrow);
    }

    #[test]
    fn group_qualifier_mismatch_is_rejected() {
        let err = parse_document(
            "architecture-beta\n\
             group g1[One]\n\
             group g2[Two]\n\
             service a(A)[A] in g1\n\
             service b(B)[B] in g2\n\
             a{g2}:R --> L:b",
        )
        .unwrap_err();
        assert!(err.message.contains("not inside group"));
    }

    #[test]
    fn icon_is_rendered_to_svg() {
        let a = arch(
            "architecture-beta\n\
             group infra(cloud)[Infra]\n\
             service db(database)[Database] in infra",
        );
        let sc = scene(&a);
        let svg = to_svg(&sc);
        assert!(svg.contains("\u{1F6E2}\u{FE0F}"), "database: an oil drum");
        assert!(svg.contains("\u{2601}\u{FE0F}"), "cloud, in colour");
    }

    #[test]
    fn nested_groups_place_services_inside() {
        let a = arch(
            "architecture-beta\n\
             group outer[Outer]\n\
             group inner[Inner] in outer\n\
             service s(S)[Service] in inner",
        );
        assert_eq!(a.groups.len(), 2);
        assert_eq!(a.services.len(), 1);
        let sc = scene(&a);
        assert_eq!(sc.scene.clusters.len(), 2);
        let inner_cluster = &sc.scene.clusters[1];
        let service_node = &sc.scene.nodes[0];
        // Service centre must sit inside the inner cluster box.
        assert!(service_node.x > inner_cluster.x);
        assert!(service_node.x < inner_cluster.x + inner_cluster.w);
        assert!(service_node.y > inner_cluster.y);
        assert!(service_node.y < inner_cluster.y + inner_cluster.h);
    }

    #[test]
    fn duplicate_service_id_is_rejected() {
        let err = parse_document(
            "architecture-beta\n\
             group g[Group]\n\
             service a(A)[A] in g\n\
             service a(B)[B] in g",
        )
        .unwrap_err();
        assert!(err.message.contains("duplicate service id"));
    }

    #[test]
    fn a_service_colour_is_escaped_on_its_way_into_the_attribute() {
        // architecture-beta writes its rects with SINGLE-quoted
        // attributes, so this call site is the reason `escape` has to
        // neutralise both quote forms, not just the double one.
        let d = match crate::parser::parse_document(
            "architecture-beta\n  group g[G]\n  service a(cloud)[A] in g\n",
        )
        .unwrap()
        {
            crate::model::Document::Architecture(a) => a,
            _ => panic!("not an architecture diagram"),
        };
        let mut as_ = scene(&d);
        as_.scene.nodes[0].style.fill = Some("x' onload=1".to_string());
        as_.scene.nodes[0].style.stroke = Some("y\" onload=1".to_string());
        let svg = to_svg(&as_);
        assert!(
            !svg.contains("' onload") && !svg.contains("\" onload"),
            "a service colour escaped its attribute:\n{svg}"
        );
        assert!(svg.contains("&#39;"), "{svg}");
    }

    /// The comparison corpus diagram (report P0 #12).
    const CORPUS: &str = "architecture-beta
    group frontend(cloud)[Frontend]
    service web(web)[Web App] in frontend
    service mobile(mobile)[Mobile App] in frontend

    group backend(server)[Backend]
    service api(server)[API Gateway] in backend
    service svc(cpu)[Business Logic] in backend

    group data(database)[Data]
    service db(database)[PostgreSQL] in data
    service cache(cache)[Redis] in data

    web:R -- L:api
    mobile:R -- L:api
    api:R --> L:svc
    svc:R <--> L:db
    svc:B -- T:cache";

    /// More shapes of diagram: ungrouped services, nesting, fan-in and
    /// fan-out, junctions, labels, same-side ports, a self-loop.
    const MORE: &[&str] = &[
        "architecture-beta
  group g(cloud)[G]
  service a(server)[A] in g
  service b(database)[Free]
  service c(disk)[Also free]
  a:R -- L:b
  b:B -- T:c",
        "architecture-beta
    group cloud(cloud)[AWS Cloud]
    group vpc(network)[VPC] in cloud
    group priv(lock)[Private subnet] in vpc
    service lb(internet)[Load Balancer] in vpc
    service app1(server)[App 1] in priv
    service app2(server)[App 2] in priv
    service rds(database)[RDS] in cloud
    service user(user)[User]
    user:R --> L:lb
    lb:R --> L:app1
    lb:R --> L:app2
    app1:R --> L:rds
    app2:R --> L:rds",
        "architecture-beta
    group edge(internet)[Edge]
    service gw(internet)[Gateway] in edge
    group svcs(server)[Services]
    service s1(server)[Users] in svcs
    service s2(server)[Orders] in svcs
    service s3(server)[Payments] in svcs
    service q(queue)[Queue]
    gw:R --> L:s1
    gw:R --> L:s2
    gw:R --> L:s3
    s1:B --> T:q
    s3:R --> L:q",
        "architecture-beta
    service left_disk(disk)[Disk]
    service top_disk(disk)[Disk]
    service bottom_disk(disk)[Disk]
    service top_gateway(internet)[Gateway]
    service bottom_gateway(internet)[Gateway]
    junction junctionCenter
    junction junctionRight
    left_disk:R -- L:junctionCenter
    top_disk:B -- T:junctionCenter
    bottom_disk:T -- B:junctionCenter
    junctionCenter:R -- L:junctionRight
    top_gateway:B -- T:junctionRight
    bottom_gateway:T -- B:junctionRight",
        "architecture-beta
    group api(cloud)[API]
    service db(database)[Database] in api
    service disk1(disk)[Storage] in api
    service disk2(disk)[Storage] in api
    service server(server)[Server] in api
    db:L -[reads]- R:server
    disk1:T -- B:server
    disk2:T -- B:db",
        "architecture-beta
    group g[G]
    service a(server)[A] in g
    service b(server)[B] in g
    service c(server)[C] in g
    a:R -- R:b
    b:T -- T:c
    c:B -- T:a
    a:R -- L:a
    c:L -- R:b",
        "architecture-beta
    group g1[One]
    group g2[Two]
    service a(server)[A] in g1
    service b(server)[B] in g2
    service c(server)[C] in g2
    a{group}:R --> L:b{group}
    a:B -- T:c",
    ];

    /// Whether the segment `a`–`b` enters the interior of `r` (1px in
    /// from its outline, so a route ending ON a box side does not).
    fn enters(a: Pt, b: Pt, r: &Rect) -> bool {
        let n = (dist(a, b) / 0.5).ceil().max(1.0) as usize;
        (0..=n).any(|k| {
            let u = k as f64 / n as f64;
            let (x, y) = (a.0 + (b.0 - a.0) * u, a.1 + (b.1 - a.1) * u);
            x > r.x0 + 1.0 && x < r.x1 - 1.0 && y > r.y0 + 1.0 && y < r.y1 - 1.0
        })
    }

    /// No drawn edge passes through any service box — neither a
    /// stranger's nor (after leaving it) one of its own ends'.
    fn assert_no_edge_through_a_box(src: &str) {
        let sc = scene(&arch(src)).scene;
        for e in &sc.edges {
            for n in sc.nodes.iter().filter(|n| n.shape != Shape::Circle) {
                let r = Rect::around(n);
                for w in e.waypoints.windows(2) {
                    assert!(
                        !enters(w[0], w[1], &r),
                        "{src}\nedge {}->{} runs through {}: {:?}",
                        e.from,
                        e.to,
                        n.id,
                        e.waypoints
                    );
                }
            }
        }
    }

    /// Boxes never overlap; every service sits inside its own frames
    /// and outside every other; frames that do not nest never overlap.
    fn assert_layout_is_clean(src: &str) {
        let a = arch(src);
        let sc = scene(&a).scene;
        let inside = |n: &SceneNode, c: &SceneCluster| {
            n.x - n.w / 2.0 >= c.x
                && n.x + n.w / 2.0 <= c.x + c.w
                && n.y - n.h / 2.0 >= c.y
                && n.y + n.h / 2.0 <= c.y + c.h
        };
        let apart = |a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)| {
            a.2 <= b.0 || b.2 <= a.0 || a.3 <= b.1 || b.3 <= a.1
        };
        let nb = |n: &SceneNode| {
            (
                n.x - n.w / 2.0,
                n.y - n.h / 2.0,
                n.x + n.w / 2.0,
                n.y + n.h / 2.0,
            )
        };
        let cb = |c: &SceneCluster| (c.x, c.y, c.x + c.w, c.y + c.h);
        let ancestors = |mut g: Option<usize>| {
            let mut v = Vec::new();
            while let Some(i) = g {
                v.push(i);
                g = a.groups[i].parent;
            }
            v
        };
        for (i, n) in sc.nodes.iter().enumerate() {
            for m in &sc.nodes[i + 1..] {
                assert!(apart(nb(n), nb(m)), "{src}\n{} overlaps {}", n.id, m.id);
            }
            let own = ancestors(a.services[i].group);
            for (g, c) in sc.clusters.iter().enumerate() {
                if own.contains(&g) {
                    assert!(inside(n, c), "{src}\n{} sticks out of {}", n.id, c.id);
                } else {
                    assert!(apart(nb(n), cb(c)), "{src}\n{} is under {}", n.id, c.id);
                }
            }
        }
        for g in 0..sc.clusters.len() {
            for h in g + 1..sc.clusters.len() {
                if !ancestors(Some(h)).contains(&g) && !ancestors(Some(g)).contains(&h) {
                    assert!(
                        apart(cb(&sc.clusters[g]), cb(&sc.clusters[h])),
                        "{src}\nframes {} and {} overlap",
                        sc.clusters[g].id,
                        sc.clusters[h].id
                    );
                }
            }
        }
    }

    fn seg_dist(p: Pt, a: Pt, b: Pt) -> f64 {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let l2 = dx * dx + dy * dy;
        let u = if l2 < EPS {
            0.0
        } else {
            (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
        };
        dist(p, (a.0 + dx * u, a.1 + dy * u))
    }

    fn node<'a>(sc: &'a Scene, id: &str) -> &'a SceneNode {
        sc.nodes.iter().find(|n| n.id == id).unwrap()
    }

    #[test]
    fn no_edge_passes_behind_or_through_a_box() {
        // Report P0 #12: web->api ran behind Mobile App and svc->cache
        // through PostgreSQL, hidden under their fills.
        assert_no_edge_through_a_box(CORPUS);
        for src in MORE {
            assert_no_edge_through_a_box(src);
            assert_layout_is_clean(src);
        }
        assert_layout_is_clean(CORPUS);
    }

    #[test]
    fn declaration_order_does_not_change_the_topology_drawn() {
        // The old grid followed source order, so swapping two lines drew
        // a different false link. Every order of the corpus edges, with
        // db/cache swapped too, stays clean.
        let lines: Vec<&str> = CORPUS.lines().collect();
        let (head, edges) = lines.split_at(lines.len() - 5);
        let mut order: Vec<usize> = (0..5).collect();
        // Heap's algorithm over the five edge lines.
        let mut c = [0usize; 5];
        let mut perms = vec![order.clone()];
        let mut i = 0;
        while i < 5 {
            if c[i] < i {
                let j = if i % 2 == 0 { 0 } else { c[i] };
                order.swap(j, i);
                perms.push(order.clone());
                c[i] += 1;
                i = 0;
            } else {
                c[i] = 0;
                i += 1;
            }
        }
        assert_eq!(perms.len(), 120);
        for (k, p) in perms.iter().enumerate() {
            let mut src: Vec<String> = head.iter().map(|l| l.to_string()).collect();
            if k % 2 == 1 {
                let (db, cache) = (
                    src.iter().position(|l| l.contains("service db")).unwrap(),
                    src.iter()
                        .position(|l| l.contains("service cache"))
                        .unwrap(),
                );
                src.swap(db, cache);
            }
            src.extend(p.iter().map(|&e| edges[e].to_string()));
            let src = src.join("\n");
            assert_no_edge_through_a_box(&src);
            assert_layout_is_clean(&src);
        }
    }

    #[test]
    fn ports_place_the_corpus_like_mermaid() {
        // B5: cache sits right under svc (svc:B -- T:cache), web and
        // mobile share a column (both feed api:L), db is right of svc.
        let sc = scene(&arch(CORPUS)).scene;
        let (svc, cache) = (node(&sc, "svc"), node(&sc, "cache"));
        assert!((cache.x - svc.x).abs() < 0.5, "cache under svc");
        assert!(cache.y > svc.y);
        let (web, mobile) = (node(&sc, "web"), node(&sc, "mobile"));
        assert!((web.x - mobile.x).abs() < 0.5, "one column");
        assert!(mobile.y > web.y, "declaration order, top-down");
        let api = node(&sc, "api");
        assert!((web.y - api.y).abs() < 0.5, "web -> api is straight");
        assert!(node(&sc, "db").x > svc.x + svc.w);
        // Q18: no longer a 6.6:1 strip.
        assert!(sc.width / sc.height <= 2.0, "{}x{}", sc.width, sc.height);
    }

    #[test]
    fn a_port_pair_sets_the_direction_of_placement() {
        let pos = |edge: &str| {
            let sc = scene(&arch(&format!(
                "architecture-beta\nservice a(server)[A]\nservice b(server)[B]\n{edge}"
            )))
            .scene;
            let (a, b) = (node(&sc, "a"), node(&sc, "b"));
            let sign = |v: f64| (v > 0.5) as i32 - (v < -0.5) as i32;
            (sign(b.x - a.x), sign(b.y - a.y))
        };
        assert_eq!(pos("a:R -- L:b"), (1, 0));
        assert_eq!(pos("a:L -- R:b"), (-1, 0));
        assert_eq!(pos("a:B -- T:b"), (0, 1));
        assert_eq!(pos("a:T -- B:b"), (0, -1));
        assert_eq!(pos("a:R -- T:b"), (1, 1), "mixed pair: diagonal");
        assert_eq!(pos("a:B -- R:b"), (-1, 1));
        assert_eq!(pos("b:L -- R:a"), (1, 0), "the declared direction reverses");
    }

    #[test]
    fn edges_leave_and_enter_along_their_port_normals() {
        for src in std::iter::once(&CORPUS).chain(MORE) {
            let a = arch(src);
            let sc = scene(&a).scene;
            for (e, se) in a.edges.iter().zip(&sc.edges) {
                let w = &se.waypoints;
                let unit = |p: Pt, q: Pt| {
                    let l = dist(p, q);
                    ((q.0 - p.0) / l, (q.1 - p.1) / l)
                };
                let out = unit(w[0], w[1]);
                let n0 = DIRS[dir_of(e.from_side)];
                assert!(
                    dist(out, n0) < 1e-6,
                    "{src}\n{}->{} leaves {:?} along {out:?}",
                    se.from,
                    se.to,
                    e.from_side
                );
                let inn = unit(w[w.len() - 2], w[w.len() - 1]);
                let n1 = DIRS[dir_of(e.to_side)];
                assert!(
                    dist(inn, (-n1.0, -n1.1)) < 1e-6,
                    "{src}\n{}->{} enters {:?} along {inn:?}",
                    se.from,
                    se.to,
                    e.to_side
                );
            }
        }
    }

    #[test]
    fn services_outside_any_group_are_laid_out_not_stacked_at_the_origin() {
        let sc = scene(&arch(MORE[0])).scene;
        let (b, c) = (node(&sc, "b"), node(&sc, "c"));
        assert!(b.x - b.w / 2.0 >= PAD && b.y - b.h / 2.0 >= PAD);
        assert!((b.x - c.x).abs() < 0.5 && c.y > b.y, "b:B -- T:c");
        assert!(b.x > node(&sc, "a").x, "a:R -- L:b");
        // Without any edge they are packed in rows, not piled up.
        let sc = scene(&arch(
            "architecture-beta\nservice a(server)[A]\nservice b(server)[B]\nservice c(server)[C]",
        ))
        .scene;
        assert_layout_is_clean(
            "architecture-beta\nservice a(server)[A]\nservice b(server)[B]\nservice c(server)[C]",
        );
        assert_eq!(sc.nodes.len(), 3);
    }

    #[test]
    fn unrelated_groups_wrap_into_a_block() {
        // Q18: top-level groups with nothing between them no longer run
        // off in one row.
        let src = "architecture-beta
    group a[A]
    service a1(server)[A one] in a
    group b[B]
    service b1(server)[B one] in b
    group c[C]
    service c1(server)[C one] in c
    group d[D]
    service d1(server)[D one] in d";
        let sc = scene(&arch(src)).scene;
        assert_layout_is_clean(src);
        assert!(sc.width / sc.height <= 2.0, "{}x{}", sc.width, sc.height);
        // Edge-less services in a group keep the √n grid, in source
        // order.
        let sc = scene(&arch(
            "architecture-beta\ngroup g[G]\nservice a[A] in g\nservice b[B] in g\nservice c[C] in g\nservice d[D] in g",
        ))
        .scene;
        let (a, b, c, d) = (
            node(&sc, "a"),
            node(&sc, "b"),
            node(&sc, "c"),
            node(&sc, "d"),
        );
        assert!((a.y - b.y).abs() < 0.5 && b.x > a.x);
        assert!((a.x - c.x).abs() < 0.5 && c.y > a.y);
        assert!((c.y - d.y).abs() < 0.5 && (b.x - d.x).abs() < 0.5);
    }

    #[test]
    fn an_empty_group_keeps_a_frame() {
        let sc = scene(&arch("architecture-beta\ngroup e[Empty]")).scene;
        let c = &sc.clusters[0];
        assert!(c.w >= GROUP_PAD * 2.0 + EMPTY_GROUP_W - 0.5);
        assert!(c.h >= GROUP_PAD * 2.0 + GROUP_TITLE_H - 0.5);
        assert!(sc.nodes.is_empty());
    }

    #[test]
    fn junctions_parse_and_edges_meet_at_their_centre() {
        let a = arch(MORE[3]);
        let j = a
            .services
            .iter()
            .position(|s| s.id == "junctionCenter")
            .unwrap();
        assert!(a.services[j].junction && a.services[j].title.is_empty());
        let a2 =
            arch("architecture-beta\ngroup g[G]\njunction j in g\nservice s[S] in g\ns:R -- L:j");
        assert!(a2.services[0].junction);
        assert_eq!(a2.services[0].group, Some(0));
        let sc = scene(&a).scene;
        let jn = &sc.nodes[j];
        assert_eq!(jn.shape, Shape::Circle);
        assert!(jn.label.is_empty());
        let mut meeting = 0;
        for e in &sc.edges {
            for p in [e.waypoints[0], *e.waypoints.last().unwrap()] {
                if dist(p, (jn.x, jn.y)) < 1e-6 {
                    meeting += 1;
                }
            }
        }
        assert_eq!(meeting, 4, "three disks and the other junction");
        let svg = to_svg(&scene(&a));
        assert_eq!(svg.matches("<circle").count(), 2, "one dot per junction");
        // A junction id is taken like a service id.
        let err = parse_document("architecture-beta\nservice j[J]\njunction j").unwrap_err();
        assert!(err.message.contains("duplicate junction id"));
    }

    #[test]
    fn the_group_modifier_attaches_the_edge_to_the_frame() {
        let a = arch(MORE[6]);
        assert!(a.edges[0].from_group && a.edges[0].to_group);
        assert!(!a.edges[1].from_group && !a.edges[1].to_group);
        let sc = scene(&a).scene;
        let (g1, g2) = (&sc.clusters[0], &sc.clusters[1]);
        let w = &sc.edges[0].waypoints;
        assert!(
            (w[0].0 - (g1.x + g1.w)).abs() < 1e-6,
            "leaves One's right side"
        );
        assert!(
            (w[w.len() - 1].0 - g2.x).abs() < 1e-6,
            "enters Two's left side"
        );
        // `{group}` on a service outside every group: its own box.
        let a = arch("architecture-beta\nservice a[A]\nservice b[B]\na{group}:R -- L:b");
        assert!(!a.edges[0].from_group);
    }

    #[test]
    fn labelled_edges_parse_and_draw_a_chip() {
        let a = arch(MORE[4]);
        assert_eq!(a.edges[0].title.as_deref(), Some("reads"));
        assert_eq!(a.edges[1].title, None);
        let a2 =
            arch("architecture-beta\nservice a[A]\nservice b[B]\na:R <-[\"calls, async\"]-> L:b");
        assert_eq!(a2.edges[0].title.as_deref(), Some("calls, async"));
        assert!(a2.edges[0].arrow_start && a2.edges[0].arrow_end);
        let sc = scene(&a);
        let (text, c, w) = sc.scene.edges[0].label.clone().unwrap();
        assert_eq!(text, "reads");
        // On the route, clear of both boxes.
        let (srv, db) = (node(&sc.scene, "server"), node(&sc.scene, "db"));
        assert!((c.1 - srv.y).abs() < 0.5);
        assert!(c.0 - w / 2.0 >= srv.x + srv.w / 2.0 && c.0 + w / 2.0 <= db.x - db.w / 2.0);
        assert!(to_svg(&sc).contains(">reads</text>"));
    }

    #[test]
    fn long_labels_wrap_inside_the_box() {
        let sc = scene(&arch(
            "architecture-beta\ngroup g[G]\nservice a(server)[Authentication and Authorization Service] in g\nservice b(server)[Supercalifragilisticexpialidocious] in g",
        ))
        .scene;
        let a = node(&sc, "a");
        assert_eq!(a.w, SERVICE_W);
        let lines: Vec<&str> = a.label.split('\n').collect();
        assert!(lines.len() >= 2, "{:?}", a.label);
        for l in &lines {
            assert!(text_px(l) <= a.w - 2.0 * LABEL_PAD_X, "{l} overflows");
        }
        assert!(a.h >= block_h(true, lines.len()) + 2.0 * LABEL_PAD_Y);
        // One word too long to break widens its box instead.
        let b = node(&sc, "b");
        assert!(b.w >= text_px(&b.label) + 2.0 * LABEL_PAD_X);
        // The frame holds both.
        assert_layout_is_clean(
            "architecture-beta\ngroup g[G]\nservice a(server)[Authentication and Authorization Service] in g\nservice b(server)[Supercalifragilisticexpialidocious] in g",
        );
        // `<br/>` breaks a line.
        let sc = scene(&arch("architecture-beta\nservice a[One<br/>Two]")).scene;
        assert_eq!(sc.nodes[0].label, "One\nTwo");
    }

    #[test]
    fn wrapping_balances_lines_instead_of_leaving_a_stub() {
        // Greedy filling at this width gives "aaaa bbbb cccc" / "dd";
        // the balanced wrap splits it 2 + 2.
        let max = text_px("aaaa bbbb cccc");
        let lines = wrap_label("aaaa bbbb cccc dd", max);
        assert_eq!(lines, ["aaaa bbbb", "cccc dd"]);
        assert_eq!(wrap_label("short", 100.0), ["short"]);
        assert!(wrap_label("", 100.0).is_empty());
    }

    #[test]
    fn text_default_emoji_carry_vs16() {
        // Q18: without U+FE0F ☁ 🗄 🖥 ✉ draw monochrome next to coloured
        // neighbours.
        for name in [
            "cloud", "database", "server", "mail", "desktop", "cpu", "cache",
        ] {
            let g = icon_glyph(name).unwrap();
            assert!(g.ends_with('\u{FE0F}'), "{name}: {g:?}");
        }
        for name in [
            "cloud",
            "database",
            "db",
            "server",
            "disk",
            "internet",
            "user",
            "users",
            "lock",
            "key",
            "mail",
            "phone",
            "desktop",
            "laptop",
            "file",
            "folder",
            "code",
            "bug",
            "search",
            "chart",
            "cpu",
            "memory",
            "network",
            "firewall",
            "queue",
            "cache",
            "container",
            "settings",
            "monitoring",
            "log",
            "alert",
            "api",
        ] {
            let g = icon_glyph(name).unwrap();
            let first = g.chars().next().unwrap();
            if (first as u32) < 0x1F000 {
                assert!(g.ends_with('\u{FE0F}'), "{name}: BMP symbol without VS16");
            }
        }
        assert_eq!(icon_glyph("mdi:database"), icon_glyph("database"));
        assert_eq!(icon_glyph("nope"), None);
    }

    #[test]
    fn icon_and_label_are_centred_in_the_box() {
        let sc = scene(&arch(CORPUS));
        let svg = to_svg(&sc);
        let web = node(&sc.scene, "web");
        let y_of = |marker: &str| -> f64 {
            let at = svg.find(marker).unwrap();
            let head = &svg[..at];
            let open = head.rfind("<text").unwrap();
            let tag = &svg[open..at];
            let y = tag.split("y='").nth(1).unwrap();
            y[..y.find('\'').unwrap()].parse().unwrap()
        };
        let icon_y = y_of(">\u{1F310}</text>");
        let label_y = y_of(">Web App</text>");
        // Block: icon (ICON_FONT) + gap + one line; its middle is the
        // box centre.
        let top = icon_y - ICON_FONT as f64 / 2.0;
        let bottom = label_y + LINE_H / 2.0;
        assert!(
            ((top + bottom) / 2.0 - web.y).abs() < 0.11,
            "{top} {bottom} {}",
            web.y
        );
        // The label is its own element: no trailing newline chunk that
        // a browser would centre along with it.
        assert!(svg.contains("fill='#232840'>Web App</text>"));
    }

    #[test]
    fn services_take_the_accent_of_their_top_level_group() {
        let sc = scene(&arch(CORPUS)).scene;
        let stroke = |id: &str| node(&sc, id).style.stroke.clone().unwrap();
        assert_eq!(stroke("web"), stroke("mobile"));
        assert_eq!(stroke("db"), stroke("cache"));
        assert_ne!(stroke("web"), stroke("api"));
        assert_ne!(stroke("api"), stroke("db"));
        assert_eq!(stroke("web"), accent(0));
    }

    #[test]
    fn picking_matches_the_drawn_edges() {
        // Hosts spline through the waypoints (and `Scene` picks on that
        // spline): it must stay on the polyline the SVG draws.
        for src in std::iter::once(&CORPUS).chain(MORE) {
            let sc = scene(&arch(src)).scene;
            for e in &sc.edges {
                for q in crate::scene::edge_polyline(e) {
                    let near = e
                        .waypoints
                        .windows(2)
                        .map(|w| seg_dist(q, w[0], w[1]))
                        .fold(f64::INFINITY, f64::min);
                    assert!(
                        near < 1.5,
                        "{}->{}: spline leaves the route by {near}",
                        e.from,
                        e.to
                    );
                }
            }
        }
        let sc = scene(&arch(CORPUS)).scene;
        // api --> svc runs alone between the two boxes.
        let (api, svc) = (node(&sc, "api"), node(&sc, "svc"));
        let mid = ((api.x + api.w / 2.0 + svc.x - svc.w / 2.0) / 2.0, api.y);
        assert_eq!(sc.edge_at(mid.0, mid.1, 4.0), Some(2));
        assert!(matches!(
            sc.hit_test(svc.x, svc.y, 4.0),
            Some(crate::scene::Hit::Node(3))
        ));
    }

    #[test]
    fn edges_are_drawn_over_the_boxes_and_titles_are_never_struck() {
        let sc = scene(&arch(CORPUS));
        let svg = to_svg(&sc);
        let last_box = svg.rfind("fill='#ffffff' stroke=").unwrap();
        let first_edge = svg.find("<polyline").unwrap();
        assert!(first_edge > last_box, "a hidden edge would hide a bug");
        for src in std::iter::once(&CORPUS).chain(MORE) {
            let sc = scene(&arch(src)).scene;
            for c in sc.clusters.iter().filter(|c| !c.title.is_empty()) {
                let r = Rect {
                    x0: c.x + GROUP_PAD,
                    y0: c.y + TITLE_CY - FONT as f64 * 0.6,
                    x1: c.x + GROUP_PAD + title_px(&c.title),
                    y1: c.y + TITLE_CY + FONT as f64 * 0.6,
                };
                for e in &sc.edges {
                    for w in e.waypoints.windows(2) {
                        assert!(
                            !enters(w[0], w[1], &r),
                            "{src}\n{}->{} strikes {}",
                            e.from,
                            e.to,
                            c.id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn mermaid_syntax_that_used_to_fail_now_parses() {
        // Optional titles, quoted titles, `align`, accessibility.
        let a = arch(
            "architecture-beta
  accTitle: Shop
  accDescr: Services of the shop
  group g(cloud)
  service a(server) in g
  service b(server)[\"B, quoted\"] in g
  align row a b
  a:R -- L:b",
        );
        assert!(a.groups[0].title.is_empty() && a.services[0].title.is_empty());
        assert_eq!(a.services[1].title, "B, quoted");
        assert_eq!(a.acc_title.as_deref(), Some("Shop"));
        let svg = crate::render::render_architecture(&a);
        assert!(svg.contains("aria-label=\"Shop\""));
        assert!(svg.contains("Services of the shop</desc>"));
        assert!(parse_document("architecture-beta\nservice a[A]\nalign row a nope").is_err());
        assert!(parse_document("architecture-beta\nservice a[A]\nalign diagonal a a").is_err());
    }

    #[test]
    fn output_is_deterministic() {
        for src in std::iter::once(&CORPUS).chain(MORE) {
            let a = arch(src);
            assert_eq!(to_svg(&scene(&a)), to_svg(&scene(&a)));
        }
    }
}
