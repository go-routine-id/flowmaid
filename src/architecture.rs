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
//! are routed orthogonally around every service box, junction and
//! group title ([`Grid`]), so no line passes behind a box or strikes a
//! title, and no edge turns on (or runs along) an unrelated one, which
//! would draw a junction that is not there. Edge label chips go last,
//! each on (or just beside) a stretch of its own line where it covers no
//! box, title or other chip ([`place_chips`]).
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
/// wider only for a single word too long to wrap. (0.32 and earlier:
/// every box was exactly this size.)
pub const SERVICE_W: f64 = 150.0;
/// Smallest service box height; see [`SERVICE_W`].
pub const SERVICE_H: f64 = 70.0;
/// Padding inside a group box.
pub const GROUP_PAD: f64 = 24.0;
/// Height reserved for a group title.
pub const GROUP_TITLE_H: f64 = 26.0;
/// Space between a group's frame and its siblings, nested groups
/// included (more when an edge label between them needs it). (0.32 and
/// earlier: only between top-level groups.)
pub const GROUP_GAP: f64 = 50.0;
/// Horizontal space between two children of one group that no edge
/// joins and whose ports do not face each other; two that an edge joins
/// sit further apart. (0.32 and earlier: 28px between every two
/// children, joined or not.)
pub const CHILD_GAP_X: f64 = 32.0;
/// [`CHILD_GAP_X`] on the vertical axis.
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
/// Clearance a route keeps from either end of a group title. The
/// title's width is only an estimate ([`title_px`] errs wide), and a
/// line beside a word reads as striking it well before it touches.
const TITLE_CLEAR: f64 = 8.0;
/// Clearance a route keeps above and below a group title, whose
/// height ([`TITLE_HALF_H`]) already spans an emoji's full box.
const TITLE_CLEAR_Y: f64 = 4.0;
/// Half the height of a group title's ink: a 14px colour emoji runs
/// about 0.6em above and below the title's centre line.
const TITLE_HALF_H: f64 = FONT as f64 * 0.6;
/// How far right of an edge that rises through a title band into a
/// port the title starts when it is moved off that edge's line.
const TITLE_SHIFT: f64 = TITLE_CLEAR + 4.0;
/// Margin the canvas keeps around every route and label chip.
const OUTER: f64 = 12.0;
/// Routing channels run around the outside of the drawing ...
const OUTER_CHANNELS: usize = 3;
/// ... this far apart, and the first this far beyond every clearance
/// and stub end (so it never runs a hair from one of those lines).
const CHANNEL_GAP: f64 = 8.0;
/// Corner radius of a drawn route (never more than half a leg).
const CORNER_R: f64 = 8.0;
/// Samples per rounded corner in an edge's waypoints: dense enough
/// that a host splining through them draws the corner the SVG does.
const CORNER_STEPS: usize = 6;
/// Route cost of one bend, in px of length.
const BEND_COST: f64 = 40.0;
/// Route cost of crossing an edge routed earlier, in px of length.
const CROSS_COST: f64 = 60.0;
/// Route cost of turning on a line an unrelated edge (one that shares
/// no port) has taken, or of running through the point where it turns.
/// Either draws a T on that edge — a junction that is not there, which
/// misreads the topology far worse than a crossing or a longer route.
const TOUCH_COST: f64 = 200.0;
/// Extra route cost per px run on top of an edge routed earlier: two
/// edges share a line only where they must (into a shared port).
const OVERLAP_COST: f64 = 3.0;
/// States the routes of one diagram may settle before later searches
/// weight their estimate (see [`Grid::search`]). Real diagrams settle a
/// small fraction of it (fifteen groups of eight services wired as
/// grids, 170 edges, settle ~60k); a large tangle of crossing edges
/// would otherwise search the whole lattice for every edge, taking
/// seconds.
const SEARCH_BUDGET: usize = 300_000;
/// Two parallel lines closer than this draw as one: edges on them
/// count as sharing a line.
const NEAR: f64 = 5.0;
/// A route running closer than this alongside a group frame's side
/// reads as part of the frame's outline ...
const FRAME_NEAR: f64 = 8.0;
/// ... so a px of it costs this much more than one down the gap.
const FRAME_HUG: f64 = 1.3;
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
///
/// `#[non_exhaustive]`: future versions may add fields without a
/// breaking release; field access stays ordinary.
#[derive(Debug, Clone)]
#[non_exhaustive]
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
    /// Where each group title is drawn, index-parallel with
    /// `scene.clusters`: the left end of its text and the centre line
    /// it sits on. Usually [`GROUP_PAD`] in from the frame's left side;
    /// further right when an edge rises through the title band into a
    /// port below it, so that the edge runs beside the title instead
    /// of striking it. Edges are routed clear of the title drawn here.
    pub title_at: Vec<(f64, f64)>,
}

type Pt = (f64, f64);

/// Compute the scene for an architecture diagram.
pub fn scene(d: &Architecture) -> ArchScene {
    build(d).0
}

/// The scene, and each edge's route as its corner points (before the
/// corners are rounded into `waypoints`).
fn build(d: &Architecture) -> (ArchScene, Vec<Vec<Pt>>) {
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

    // Lay out, then move every title off the edges that rise through
    // its band into a port below it. A title that moves may need a
    // wider frame: lay out again with that width (the frame's own
    // contents keep their places, only what lies right of it moves),
    // and a title that still does not fit stays put for the router to
    // go around.
    let na = t.na();
    let ns = d.services.len();
    let ng = d.groups.len();
    let mut min_w: Vec<f64> = title_w.iter().map(|w| w + 2.0 * GROUP_PAD).collect();
    let mut pass = 0;
    let (boxes, frames, ends, title_x) = loop {
        let (x, y) = solve(d, &t, &cell, &min_w);
        let boxes: Vec<Rect> = (0..ns)
            .map(|i| Rect {
                x0: x.at[i] - sizes[i].0 / 2.0,
                y0: y.at[i] - sizes[i].1 / 2.0,
                x1: x.at[i] + sizes[i].0 / 2.0,
                y1: y.at[i] + sizes[i].1 / 2.0,
            })
            .collect();
        let frames: Vec<Rect> = (0..ng)
            .map(|g| Rect {
                x0: x.lo(na + g),
                y0: y.lo(na + g),
                x1: x.hi(na + g),
                y1: y.hi(na + g),
            })
            .collect();
        let ends = edge_ends(d, &boxes, &frames);
        let mut title_x = title_starts(d, &t, &boxes, &frames, &ends, &title_w);
        let mut grown = false;
        for g in 0..ng {
            let need = title_x[g] + title_w[g] + GROUP_PAD - frames[g].x0;
            if need > frames[g].x1 - frames[g].x0 + EPS {
                if pass < 2 {
                    min_w[g] = min_w[g].max(need);
                    grown = true;
                } else {
                    title_x[g] = frames[g].x0 + GROUP_PAD;
                }
            }
        }
        pass += 1;
        if !grown {
            break (boxes, frames, ends, title_x);
        }
    };
    let (mut right, mut bottom) = (PAD, PAD);
    for r in boxes.iter().chain(&frames) {
        right = right.max(r.x1);
        bottom = bottom.max(r.y1);
    }

    let mut clusters: Vec<SceneCluster> = d
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
    let mut nodes = Vec::with_capacity(ns);
    let mut icons: Vec<Option<String>> = Vec::with_capacity(ns);
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
        let (cx, cy) = (
            (boxes[i].x0 + boxes[i].x1) / 2.0,
            (boxes[i].y0 + boxes[i].y1) / 2.0,
        );
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

    // Obstacles: every service box, every junction (a route through a
    // junction's dot would read as joining it) and every group title,
    // which edges must never strike.
    let mut obstacles: Vec<(Rect, bool)> = boxes.iter().map(|b| (b.grow(CLEAR), true)).collect();
    for g in 0..ng {
        if title_w[g] > 0.0 {
            let tr = title_rect(frames[g], title_x[g], title_w[g]);
            obstacles.push((tr.grow_xy(TITLE_CLEAR, TITLE_CLEAR_Y), true));
        }
    }
    let mut plain: Vec<Rect> = boxes
        .iter()
        .zip(&d.services)
        .filter(|(_, s)| !s.junction)
        .map(|(b, _)| *b)
        .collect();
    plain.extend(frames.iter().copied());
    let mut grid = Grid::new(&obstacles, &plain, &frames, d, &ends);
    let mut routes: Vec<Vec<Pt>> = d
        .edges
        .iter()
        .zip(&ends)
        .enumerate()
        .map(|(k, (e, &(p0, p1)))| grid.route(k, p0, e.from_side, p1, e.to_side))
        .collect();

    // Label chips go where they cover no box, title, junction or other
    // chip, and (wherever there is room) no line but their own.
    let widths: Vec<Option<f64>> = d
        .edges
        .iter()
        .map(|e| e.title.as_deref().map(|t| text_px(t) + LABEL_PAD_W))
        .collect();
    let mut blocks: Vec<Rect> = boxes.iter().map(|b| b.grow(CHIP_GAP)).collect();
    for g in 0..ng {
        if title_w[g] > 0.0 {
            blocks.push(title_rect(frames[g], title_x[g], title_w[g]).grow(CHIP_GAP));
        }
    }
    let mut bounds = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: right + PAD,
        y1: bottom + PAD,
    };
    for &p in routes.iter().flatten() {
        bounds.x0 = bounds.x0.min(p.0 - OUTER);
        bounds.y0 = bounds.y0.min(p.1 - OUTER);
        bounds.x1 = bounds.x1.max(p.0 + OUTER);
        bounds.y1 = bounds.y1.max(p.1 + OUTER);
    }
    let field = ChipField::new(&routes, &blocks, &frames, bounds);
    let chips = place_chips(&field, &widths);

    let mut edges: Vec<SceneEdge> = Vec::with_capacity(d.edges.len());
    let mut arrow_starts: Vec<bool> = Vec::with_capacity(d.edges.len());
    let mut arrow_ends: Vec<bool> = Vec::with_capacity(d.edges.len());
    for (k, (e, &(p0, p1))) in d.edges.iter().zip(&ends).enumerate() {
        let label = match (&e.title, chips[k], widths[k]) {
            (Some(text), Some(c), Some(w)) => Some((text.clone(), c, w)),
            _ => None,
        };
        edges.push(SceneEdge {
            from: d.services[e.from].id.clone(),
            to: d.services[e.to].id.clone(),
            bezier: [p0, p0, p1, p1],
            waypoints: rounded(&routes[k]),
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

    // The canvas keeps PAD around the boxes and frames, and OUTER
    // around every route and label chip — on all four sides: a route
    // over the top row (on a top port's stub line, above the outermost
    // channel) or its label chip may reach above or left of the
    // margin, and then the whole drawing moves down or right instead
    // of the chip being cut off.
    let (mut lo_x, mut lo_y) = (0.0f64, 0.0f64);
    let (mut width, mut height) = (right + PAD, bottom + PAD);
    for e in &edges {
        for p in &e.waypoints {
            lo_x = lo_x.min(p.0 - OUTER);
            lo_y = lo_y.min(p.1 - OUTER);
            width = width.max(p.0 + OUTER);
            height = height.max(p.1 + OUTER);
        }
        if let Some((_, c, w)) = &e.label {
            lo_x = lo_x.min(c.0 - w / 2.0 - OUTER);
            lo_y = lo_y.min(c.1 - LABEL_BOX_H / 2.0 - OUTER);
            width = width.max(c.0 + w / 2.0 + OUTER);
            height = height.max(c.1 + LABEL_BOX_H / 2.0 + OUTER);
        }
    }
    let (dx, dy) = (-lo_x, -lo_y);
    let mv = |p: &mut Pt| {
        p.0 += dx;
        p.1 += dy;
    };
    let mut title_at: Vec<Pt> = (0..ng)
        .map(|g| (title_x[g], frames[g].y0 + TITLE_CY))
        .collect();
    if dx > 0.0 || dy > 0.0 {
        for n in &mut nodes {
            n.x += dx;
            n.y += dy;
        }
        for c in &mut clusters {
            c.x += dx;
            c.y += dy;
        }
        for e in &mut edges {
            e.waypoints.iter_mut().for_each(mv);
            e.bezier.iter_mut().for_each(mv);
            if let Some((_, c, _)) = &mut e.label {
                mv(c);
            }
        }
        title_at.iter_mut().for_each(mv);
        routes.iter_mut().flatten().for_each(mv);
        width += dx;
        height += dy;
    }

    let arch = ArchScene {
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
        title_at,
    };
    (arch, routes)
}

/// The attach point of `side` on a box: the middle of that side, or a
/// junction's centre (every side of a junction is its centre).
fn port(r: &Rect, side: ArchSide, junction: bool) -> Pt {
    let (cx, cy) = ((r.x0 + r.x1) / 2.0, (r.y0 + r.y1) / 2.0);
    if junction {
        return (cx, cy);
    }
    match side {
        ArchSide::T => (cx, r.y0),
        ArchSide::B => (cx, r.y1),
        ArchSide::L => (r.x0, cy),
        ArchSide::R => (r.x1, cy),
    }
}

/// Both ends of every edge: a port on a box side, a junction's centre,
/// or — for a `{group}` end — the point where the port's normal leaves
/// the service's group frame.
fn edge_ends(d: &Architecture, boxes: &[Rect], frames: &[Rect]) -> Vec<(Pt, Pt)> {
    let end = |s: usize, side: ArchSide, group_end: bool| -> Pt {
        let p = port(&boxes[s], side, d.services[s].junction);
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
    d.edges
        .iter()
        .map(|e| {
            (
                end(e.from, e.from_side, e.from_group),
                end(e.to, e.to_side, e.to_group),
            )
        })
        .collect()
}

/// The ink box of a group title of width `w` that starts at `x`.
fn title_rect(f: Rect, x: f64, w: f64) -> Rect {
    Rect {
        x0: x,
        y0: f.y0 + TITLE_CY - TITLE_HALF_H,
        x1: x + w,
        y1: f.y0 + TITLE_CY + TITLE_HALF_H,
    }
}

/// Where each group title starts: [`GROUP_PAD`] in from its frame,
/// or right of every edge that rises through its band into a port
/// below. Such an edge leaves (or enters) a top port straight up, and
/// when nothing stands between the port and the band it either runs
/// straight through the band or — if the title sat on its line —
/// would have to wiggle round the title just above the port, with no
/// room to do it cleanly. The edge's line is kept for it when the port
/// is that close below the band, or when the edge's far end lies above
/// the band (so the edge comes straight down to it).
fn title_starts(
    d: &Architecture,
    t: &Tree,
    boxes: &[Rect],
    frames: &[Rect],
    ends: &[(Pt, Pt)],
    title_w: &[f64],
) -> Vec<f64> {
    let na = t.na();
    let mut cols: Vec<Vec<f64>> = vec![Vec::new(); frames.len()];
    for (e, &(p0, p1)) in d.edges.iter().zip(ends) {
        for (s, side, group_end, p, q) in [
            (e.from, e.from_side, e.from_group, p0, p1),
            (e.to, e.to_side, e.to_group, p1, p0),
        ] {
            if side != ArchSide::T {
                continue;
            }
            // The frames around the port: around the service, or —
            // for a `{group}` end, which sits on its group's frame —
            // around that frame.
            let skip = 1 + usize::from(group_end && t.chain[s].len() > 1);
            for &it in t.chain[s].iter().skip(skip) {
                let g = it - na;
                if title_w[g] <= 0.0 {
                    continue;
                }
                let f = frames[g];
                let (top, bottom) = (
                    f.y0 + TITLE_CY - TITLE_HALF_H,
                    f.y0 + TITLE_CY + TITLE_HALF_H,
                );
                let between = boxes.iter().enumerate().any(|(b, r)| {
                    b != s
                        && !d.services[b].junction
                        && r.x0 - CLEAR < p.0
                        && p.0 < r.x1 + CLEAR
                        && r.y1 > bottom
                        && r.y0 < p.1
                });
                if between {
                    // The frames further out are behind that box too.
                    break;
                }
                let tight = p.1 - STUB < bottom + TITLE_CLEAR_Y + 2.0 * CORNER_R;
                if tight || q.1 < top {
                    cols[g].push(p.0);
                }
            }
        }
    }
    cols.iter_mut()
        .enumerate()
        .map(|(g, c)| {
            c.sort_by(f64::total_cmp);
            let mut x = frames[g].x0 + GROUP_PAD;
            for &r in c.iter() {
                if r > x - TITLE_SHIFT && r < x + title_w[g] + TITLE_SHIFT {
                    x = r + TITLE_SHIFT;
                }
            }
            x
        })
        .collect()
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

/// Width a bold group title may take at [`FONT`], erring wide: edges
/// keep clear of it, and a title measured short gets struck. Bold
/// Helvetica/Arial runs up to ~12% wider than the regular-weight
/// estimate of [`text_width`], and a colour emoji draws about 1.25em
/// wide where [`text_width`] counts 1em.
fn title_px(s: &str) -> f64 {
    let emoji = |c: char| c >= '\u{2600}' && c != '\u{FE0F}' && c != '\u{200D}';
    let n = s.chars().filter(|&c| emoji(c)).count();
    let rest: String = s.chars().filter(|&c| !emoji(c)).collect();
    n as f64 * 1.3 * FONT as f64 + text_px(&rest) * 1.12
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
/// already taken by a service of the same container pushes the
/// newcomer on across the step, so two services feeding one port stack
/// in a column instead of landing on each other.
///
/// The walk finishes a container before it leaves it. Edges inside a
/// container are followed first, breadth-first; an edge into another
/// container waits until nothing inside is left, and the first such
/// edge to come up places its service — the anchor — whose own
/// container is then walked in full before the next waiting edge is
/// looked at (and edges found on the way wait behind all of those). So
/// every group is laid out by its own edges from a single anchor, and
/// services of other groups never push its cells around — [`legalize`]
/// then moves whole groups apart where they collide, and packs the
/// connected components.
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
        // Cells taken in each container, and the walk's queues of
        // (service, the cell it wants, the step that led there): inside
        // the container being walked, into another container now, and
        // into another container once those are done.
        let mut taken: HashSet<(usize, i32, i32)> = HashSet::new();
        type Item = (usize, (i32, i32), (i32, i32));
        let mut inside: VecDeque<Item> = VecDeque::new();
        let mut queue: VecDeque<Item> = VecDeque::from([(start, (0, 0), (0, 0))]);
        let mut later: VecDeque<Item> = VecDeque::new();
        loop {
            if inside.is_empty() && queue.is_empty() {
                std::mem::swap(&mut queue, &mut later);
            }
            let Some((u, want, step)) = inside.pop_front().or_else(|| queue.pop_front()) else {
                break;
            };
            if comp[u] != usize::MAX {
                continue;
            }
            let parent = t.atoms[u].parent;
            let c = free_cell(&taken, parent, want, step);
            taken.insert((parent, c.0, c.1));
            cell[u] = c;
            comp[u] = next;
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
                let item = (v, (c.0 + step.0, c.1 + step.1), step);
                if t.atoms[v].parent == parent {
                    inside.push_back(item);
                } else {
                    later.push_back(item);
                }
            }
        }
        next += 1;
    }
    legalize(d, t, &mut cell, &comp);
    cell
}

/// `want`, or the first cell past it that no service of `container`
/// has taken, across the step that led there: below it for a sideways
/// (or diagonal) step, right of it for a vertical one. Always the same
/// way, so the services feeding one port stack in declaration order.
fn free_cell(
    taken: &HashSet<(usize, i32, i32)>,
    container: usize,
    want: (i32, i32),
    step: (i32, i32),
) -> (i32, i32) {
    let (dx, dy) = if step.0 == 0 { (1, 0) } else { (0, 1) };
    let mut c = want;
    while taken.contains(&(container, c.0, c.1)) {
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

/// Pixel centres of every atom (x axis, y axis) from its cell; each
/// group's frame is at least `min_w` wide (its title and padding).
fn solve<'a>(
    d: &Architecture,
    t: &'a Tree,
    cell: &[(i32, i32)],
    min_w: &[f64],
) -> (Axis<'a>, Axis<'a>) {
    let na = t.na();
    let ns = d.services.len();
    let items = na + t.root;
    // Atoms an edge joins, with the room (on x, on y) the labels of the
    // edges between them need: their stubs, plus every chip between the
    // boxes — side by side along a flat line, stacked along an upright
    // one — so two labelled edges on one line (a request and its reply)
    // each get a chip's room, not one between them.
    let mut chips_between: HashMap<(usize, usize), Vec<f64>> = HashMap::new();
    for e in &d.edges {
        if e.from != e.to && e.from < ns && e.to < ns {
            let slot = chips_between
                .entry((e.from.min(e.to), e.from.max(e.to)))
                .or_default();
            if let Some(t) = e.title.as_deref() {
                slot.push(text_px(t) + LABEL_PAD_W);
            }
        }
    }
    let joins: HashMap<(usize, usize), Pt> = chips_between
        .into_iter()
        .map(|(pair, ws)| {
            let n = ws.len() as f64;
            let room = if ws.is_empty() {
                (0.0, 0.0)
            } else {
                let stubs = 2.0 * STUB + 8.0 + CHIP_APART * (n - 1.0);
                (ws.iter().sum::<f64>() + stubs, n * LABEL_BOX_H + stubs)
            };
            (pair, room)
        })
        .collect();
    let link = |p: usize, q: usize| -> Option<Pt> {
        let mut best: Option<Pt> = None;
        for &a in &t.under[p] {
            for &b in &t.under[q] {
                if let Some(&r) = joins.get(&(a.min(b), a.max(b))) {
                    best = Some(best.map_or(r, |x: Pt| (x.0.max(r.0), x.1.max(r.1))));
                }
            }
        }
        best
    };
    // What the labels between two atoms need on one axis.
    let label_room = |r: Pt, on_x: bool| if on_x { r.0 } else { r.1 };
    // The box sides (as DIRS indices) that carry a port, the stub
    // reaching STUB out of it.
    let mut ported = vec![[false; 4]; na];
    for e in &d.edges {
        if e.from < ns && e.to < ns {
            for (s, side, group_end) in [
                (e.from, e.from_side, e.from_group),
                (e.to, e.to_side, e.to_group),
            ] {
                if !(group_end && d.services[s].group.is_some()) {
                    ported[s][dir_of(side)] = true;
                }
            }
        }
    }
    // `p` lies before `q` on the axis. Two boxes that face each other
    // with a port each keep an edge's gap even when no edge joins them:
    // at a child's gap (2 × STUB) their stubs would meet and draw a link
    // between them that is not there.
    let gap = |p: usize, q: usize, on_x: bool| -> f64 {
        let lw = link(p, q);
        let (far, near) = if on_x { (0, 2) } else { (1, 3) };
        if t.is_group(p) || t.is_group(q) {
            let room = lw.map_or(0.0, |r| label_room(r, on_x) - 2.0 * GROUP_PAD);
            GROUP_GAP.max(room)
        } else if let Some(r) = lw {
            EDGE_GAP.max(label_room(r, on_x))
        } else if ported[p][far] && ported[q][near] {
            EDGE_GAP
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
        let (a, b) = (e.from, e.to);
        let r = joins
            .get(&(a.min(b), a.max(b)))
            .copied()
            .unwrap_or_default();
        match (e.from_side, horizontal(e.to_side)) {
            (ArchSide::R, true) if cell[a].0 < cell[b].0 => {
                xpred[b].push((a, EDGE_GAP.max(label_room(r, true))))
            }
            (ArchSide::L, true) if cell[b].0 < cell[a].0 => {
                xpred[a].push((b, EDGE_GAP.max(label_room(r, true))))
            }
            (ArchSide::B, false) if cell[a].1 < cell[b].1 => {
                ypred[b].push((a, EDGE_GAP.max(label_room(r, false))))
            }
            (ArchSide::T, false) if cell[b].1 < cell[a].1 => {
                ypred[a].push((b, EDGE_GAP.max(label_room(r, false))))
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
        min_len: min_w.to_vec(),
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
    #[cfg(test)]
    fn around(n: &SceneNode) -> Rect {
        Rect {
            x0: n.x - n.w / 2.0,
            y0: n.y - n.h / 2.0,
            x1: n.x + n.w / 2.0,
            y1: n.y + n.h / 2.0,
        }
    }

    fn grow(self, m: f64) -> Rect {
        self.grow_xy(m, m)
    }

    fn grow_xy(self, mx: f64, my: f64) -> Rect {
        Rect {
            x0: self.x0 - mx,
            y0: self.y0 - my,
            x1: self.x1 + mx,
            y1: self.y1 + my,
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
/// frames, through every port and its stub point, and in a few
/// channels around the drawing. A route is a cheapest path along the
/// lines — length, plus a price per bend, per crossing of another
/// edge, per px run on top of (or beside) one or alongside a frame,
/// and a steep one for touching an unrelated edge — that never enters
/// a box's clearance.
struct Grid {
    xs: Vec<f64>,
    ys: Vec<f64>,
    /// Cost per px of running along each vertical (`x_rate`) or
    /// horizontal (`y_rate`) line.
    x_rate: Vec<f64>,
    y_rate: Vec<f64>,
    /// Whether the step with each [`Grid::runs`] key runs alongside a
    /// group frame's side, within [`FRAME_NEAR`].
    framed: Vec<bool>,
    /// For each vertical (`near_x`) or horizontal (`near_y`) line, the
    /// lines within [`NEAR`] of it ([`near_lines`]): an edge on one of
    /// them is as good as on this one.
    near_x: Vec<(usize, usize)>,
    near_y: Vec<(usize, usize)>,
    /// Lattice points inside a clearance, indexed `j * nx + i`.
    blocked: Vec<bool>,
    /// Whether the step east (`h_open`) / south (`v_open`) of a point
    /// stays out of every clearance.
    h_open: Vec<bool>,
    v_open: Vec<bool>,
    /// The edges using each lattice step, at `2 * point` for the step
    /// east of a point and `2 * point + 1` for the step south of it:
    /// every edge's stubs from the start, each route once it is found.
    runs: Lists<usize>,
    /// The edges through each point, each with whether it runs
    /// east-west there.
    marks: Lists<(usize, bool)>,
    /// The two end points of every edge, by edge index. Edges that
    /// share an end (a fan-in to one port) may run together without
    /// paying for it, so they merge into one trunk in the gap, as a bus
    /// would; any other edge is foreign to them.
    ports: Vec<[Pt; 2]>,
    /// The lattice points on each edge's stubs between its ports and
    /// its stub ends. Its route may not use them: it would cross its
    /// own stub, or come back along it into the port.
    stub_pts: Vec<Vec<usize>>,
    /// The edges whose stub (stub end included) covers each point. A
    /// route through a foreign stub passes right by that edge's port
    /// and reads as joining it there, so it costs [`TOUCH_COST`].
    stubs_at: Lists<usize>,
    /// Search buffers, per (point, heading) state, kept between
    /// searches: a state's entries are current only when its `seen`
    /// stamp is the running search's.
    dist_to: Vec<f64>,
    prev: Vec<usize>,
    seen: Vec<u32>,
    done: Vec<u32>,
    stamp: u32,
    /// States settled by every search so far (see [`SEARCH_BUDGET`]).
    spent: usize,
}

/// Many short lists kept in one arena, list `k` threaded from
/// `head[k]` through `items` (newest entry first): the lattice's
/// per-point and per-step edge lists, nearly all of them empty, without
/// a heap allocation each.
struct Lists<T> {
    head: Vec<u32>,
    items: Vec<(T, u32)>,
}

impl<T: Copy> Lists<T> {
    const NONE: u32 = u32::MAX;

    fn new(n: usize) -> Self {
        Lists {
            head: vec![Self::NONE; n],
            items: Vec::new(),
        }
    }

    fn push(&mut self, k: usize, t: T) {
        let next = self.head[k];
        self.head[k] = self.items.len() as u32;
        self.items.push((t, next));
    }

    fn iter(&self, k: usize) -> impl Iterator<Item = T> + '_ {
        let mut at = self.head[k];
        std::iter::from_fn(move || {
            let (t, next) = *self.items.get(at as usize)?;
            at = next;
            Some(t)
        })
    }
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

/// For each line, the range of lines (indices, inclusive) within
/// [`NEAR`] of it — itself, and any neighbour close enough that the
/// two draw as one.
fn near_lines(at: &[f64]) -> Vec<(usize, usize)> {
    (0..at.len())
        .map(|i| {
            let (mut lo, mut hi) = (i, i);
            while lo > 0 && at[i] - at[lo - 1] < NEAR {
                lo -= 1;
            }
            while hi + 1 < at.len() && at[hi + 1] - at[i] < NEAR {
                hi += 1;
            }
            (lo, hi)
        })
        .collect()
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
    /// frames whose gaps get a middle line; `frames` the group frames,
    /// along whose sides a route pays [`FRAME_HUG`]. [`OUTER_CHANNELS`]
    /// lines run beyond all of them on every side.
    fn new(
        obstacles: &[(Rect, bool)],
        plain: &[Rect],
        frames: &[Rect],
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
        for l in [&mut xl, &mut yl] {
            let lo = l.iter().map(|a| a.0).fold(f64::INFINITY, f64::min);
            let hi = l.iter().map(|a| a.0).fold(f64::NEG_INFINITY, f64::max);
            for k in 1..=OUTER_CHANNELS {
                let off = CHANNEL_GAP * k as f64;
                l.extend([(lo - off, 1.0), (hi + off, 1.0)]);
            }
        }
        xl.extend(midlines(plain, true).into_iter().map(|x| (x, 1.0)));
        yl.extend(midlines(plain, false).into_iter().map(|y| (y, 1.0)));
        let (xs, x_rate) = dedup_lines(xl);
        let (ys, y_rate) = dedup_lines(yl);
        let (nx, ny) = (xs.len(), ys.len());
        // A point is blocked strictly inside a clearance, and a step is
        // closed where it runs strictly inside one or across it. Each
        // clearance marks only the lines it covers (found by bisection),
        // so building the lattice costs what the clearances cover rather
        // than every point times every clearance.
        let mut blocked = vec![false; nx * ny];
        let mut h_open = vec![true; nx * ny];
        let mut v_open = vec![true; nx * ny];
        for j in 0..ny {
            h_open[j * nx + nx - 1] = false;
        }
        for i in 0..nx {
            v_open[(ny - 1) * nx + i] = false;
        }
        let strictly = |v: &[f64], lo: f64, hi: f64| {
            (
                v.partition_point(|&a| a <= lo + EPS),
                v.partition_point(|&a| a < hi - EPS),
            )
        };
        for (o, _) in obstacles {
            let (i0, i1) = strictly(&xs, o.x0, o.x1);
            let (j0, j1) = strictly(&ys, o.y0, o.y1);
            for j in j0..j1 {
                for i in i0..i1 {
                    blocked[j * nx + i] = true;
                }
                for i in i0.saturating_sub(1)..i1.min(nx - 1) {
                    h_open[j * nx + i] = false;
                }
            }
            for j in j0.saturating_sub(1)..j1.min(ny - 1) {
                for i in i0..i1 {
                    v_open[j * nx + i] = false;
                }
            }
        }
        // Steps that run alongside a frame side, within FRAME_NEAR.
        let mut framed = vec![false; 2 * nx * ny];
        let within = |v: &[f64], c: f64, m: f64| {
            (
                v.partition_point(|&a| a <= c - m),
                v.partition_point(|&a| a < c + m),
            )
        };
        for f in frames {
            let (i0, i1) = within(&xs, (f.x0 + f.x1) / 2.0, (f.x1 - f.x0) / 2.0 + EPS);
            for y in [f.y0, f.y1] {
                let (j0, j1) = within(&ys, y, FRAME_NEAR);
                for j in j0..j1 {
                    for i in i0..i1.saturating_sub(1) {
                        framed[2 * (j * nx + i)] = true;
                    }
                }
            }
            let (j0, j1) = within(&ys, (f.y0 + f.y1) / 2.0, (f.y1 - f.y0) / 2.0 + EPS);
            for x in [f.x0, f.x1] {
                let (i0, i1) = within(&xs, x, FRAME_NEAR);
                for i in i0..i1 {
                    for j in j0..j1.saturating_sub(1) {
                        framed[2 * (j * nx + i) + 1] = true;
                    }
                }
            }
        }
        let mut grid = Grid {
            framed,
            near_x: near_lines(&xs),
            near_y: near_lines(&ys),
            xs,
            ys,
            x_rate,
            y_rate,
            blocked,
            h_open,
            v_open,
            runs: Lists::new(2 * nx * ny),
            marks: Lists::new(nx * ny),
            ports: ends.iter().map(|&(p0, p1)| [p0, p1]).collect(),
            stub_pts: vec![Vec::new(); ends.len()],
            stubs_at: Lists::new(nx * ny),
            dist_to: vec![0.0; 4 * nx * ny],
            prev: vec![0; 4 * nx * ny],
            seen: vec![0; 4 * nx * ny],
            done: vec![0; 4 * nx * ny],
            stamp: 0,
            spent: 0,
        };
        grid.reserve_stubs(d, ends);
        grid
    }

    /// Mark every edge's stubs — the stretch straight out of each port
    /// — as taken by that edge before any route is searched, so a route
    /// found earlier pays to cross or touch a stub drawn later just as
    /// it would a route.
    fn reserve_stubs(&mut self, d: &Architecture, ends: &[(Pt, Pt)]) {
        for (k, (e, &(p0, p1))) in d.edges.iter().zip(ends).enumerate() {
            for (p, side) in [(p0, e.from_side), (p1, e.to_side)] {
                let out = dir_of(side);
                let n = DIRS[out];
                let Some(mut v) = self.node_at((p.0 + n.0 * STUB, p.1 + n.1 * STUB)) else {
                    continue;
                };
                let ew = horizontal(side);
                self.marks.push(v, (k, ew));
                self.stubs_at.push(v, k);
                // Inwards to the port, up to the box's clearance.
                while let Some((w, _, _, key)) = self.step(v, (out + 2) % 4) {
                    let q = self.point(w);
                    if (q.0 - p.0) * n.0 + (q.1 - p.1) * n.1 < -EPS || self.blocked[w] {
                        break;
                    }
                    self.runs.push(key, k);
                    self.marks.push(w, (k, ew));
                    self.stub_pts[k].push(w);
                    self.stubs_at.push(w, k);
                    v = w;
                }
            }
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
    /// by A* over (point, heading), for an edge whose ends are `mine`
    /// and whose stubs cover the points `own`. Turning on a line a
    /// foreign edge takes, running through a point where one turns or
    /// through a foreign stub costs [`TOUCH_COST`]; so does turning
    /// into the port at a goal a foreign edge passes. The estimate is the Manhattan distance plus a bend for every
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
        own: &[usize],
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
        let alien: Vec<bool> = self
            .ports
            .iter()
            .map(|ps| !ps.iter().any(|q| mine.iter().any(|m| same(*q, *m))))
            .collect();
        let foreign = |_: &Grid, r: usize| alien[r];
        // Whether a foreign edge passes through `v` (or a point within
        // NEAR of it), and whether one turns right at `v` (runs through
        // it both ways).
        let nx = self.xs.len();
        let touched = |grid: &Grid, v: usize| {
            let ((i0, i1), (j0, j1)) = (grid.near_x[v % nx], grid.near_y[v / nx]);
            (j0..=j1).any(|j| {
                (i0..=i1).any(|i| grid.marks.iter(j * nx + i).any(|(r, _)| foreign(grid, r)))
            })
        };
        // Foreign edges on the step with `key`, or on a parallel step
        // within NEAR of it.
        let sharing = |grid: &Grid, key: usize| {
            let (a, south) = (key / 2, key % 2 == 1);
            let (i, j) = (a % nx, a / nx);
            let ((lo, hi), stride) = if south {
                (grid.near_x[i], 2)
            } else {
                (grid.near_y[j], 2 * nx)
            };
            let first = key - (if south { i } else { j } - lo) * stride;
            (0..=hi - lo)
                .map(|k| {
                    grid.runs
                        .iter(first + k * stride)
                        .filter(|&r| foreign(grid, r))
                        .count()
                })
                .sum::<usize>()
        };
        let cornered = |grid: &Grid, v: usize| {
            grid.marks.iter(v).any(|(r, ew)| {
                !ew && grid.marks.iter(v).any(|(q, e2)| q == r && e2) && foreign(grid, r)
            })
        };
        // Exact while the diagram's searches have been cheap; past each
        // SEARCH_BUDGET of settled states, later searches trust their
        // estimate more (weighted A*), trading a slightly dearer route
        // for a search that no longer floods the lattice.
        let weight = match self.spent / SEARCH_BUDGET {
            0 => 1.0,
            1 => 1.5,
            2 | 3 => 2.0,
            _ => 3.0,
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
            self.spent += 1;
            let (v, d) = (s / 4, s % 4);
            let turn_here = if touched(self, v) { TOUCH_COST } else { 0.0 };
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
                if (self.blocked[w] || own.contains(&w)) && w != goal {
                    continue;
                }
                let shared = sharing(self, key);
                let rate = if self.framed[key] {
                    rate * FRAME_HUG
                } else {
                    rate
                };
                let mut c = self.dist_to[s] + len * rate + OVERLAP_COST * len * shared as f64;
                let east_west = nd % 2 == 0;
                let across = self
                    .marks
                    .iter(w)
                    .any(|(r, ew)| ew != east_west && foreign(self, r));
                if w != goal && self.stubs_at.iter(w).any(|r| foreign(self, r)) {
                    c += TOUCH_COST;
                } else if across && w != goal {
                    c += CROSS_COST;
                }
                if nd != d {
                    c += BEND_COST + turn_here;
                }
                if cornered(self, w) {
                    c += TOUCH_COST;
                }
                if w == goal && nd != d_in {
                    c += if nd == (d_in + 2) % 4 {
                        2.0 * BEND_COST
                    } else {
                        BEND_COST
                    };
                    if touched(self, w) {
                        c += TOUCH_COST;
                    }
                }
                let ws = w * 4 + nd;
                if self.seen[ws] != stamp || c < self.dist_to[ws] - EPS {
                    self.seen[ws] = stamp;
                    self.dist_to[ws] = c;
                    self.prev[ws] = s;
                    heap.push(Reverse(((c + weight * est(self, w, nd)).to_bits(), ws)));
                }
            }
        }
        None
    }

    /// Mark edge `r`'s routed path so later edges pay to cross, touch
    /// or share it.
    fn record(&mut self, path: &[usize], r: usize) {
        let nx = self.xs.len();
        for w in path.windows(2) {
            let (a, b) = (w[0].min(w[1]), w[0].max(w[1]));
            let east_west = a / nx == b / nx;
            let key = 2 * a + usize::from(!east_west);
            self.runs.push(key, r);
            for p in [a, b] {
                self.marks.push(p, (r, east_west));
            }
        }
    }

    /// The corner points of edge `k`'s route from port `p0` (side
    /// `side0`) to port `p1` (side `side1`): straight out of `p0` along
    /// its normal for [`STUB`], along the lattice, and straight into
    /// `p1`.
    fn route(&mut self, k: usize, p0: Pt, side0: ArchSide, p1: Pt, side1: ArchSide) -> Vec<Pt> {
        let (d0, d1) = (dir_of(side0), dir_of(side1));
        let (n0, n1) = (DIRS[d0], DIRS[d1]);
        let s0 = (p0.0 + n0.0 * STUB, p0.1 + n0.1 * STUB);
        let s1 = (p1.0 + n1.0 * STUB, p1.1 + n1.1 * STUB);
        let path = match (self.node_at(s0), self.node_at(s1)) {
            (Some(a), Some(b)) => {
                let own = self.stub_pts[k].clone();
                self.search(a, d0, b, (d1 + 2) % 4, [p0, p1], &own)
            }
            _ => None,
        };
        let mut corners = vec![p0];
        match path {
            Some(path) => {
                self.record(&path, k);
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

/// Whether two points coincide (within [`EPS`] on each axis).
fn same(a: Pt, b: Pt) -> bool {
    (a.0 - b.0).abs() < EPS && (a.1 - b.1).abs() < EPS
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

// ------------------------------------------------------------------
// Edge label chips
// ------------------------------------------------------------------

/// Room a label chip keeps from every box, junction dot and group
/// title.
const CHIP_GAP: f64 = 3.0;
/// Room between two label chips: a little of the line shows between
/// two chips on one line, so they read as two labels, not one.
const CHIP_APART: f64 = 2.0 * CHIP_GAP;
/// Room kept between a route's end (its port, and any arrowhead there)
/// and a chip on the route's first or last leg.
const CHIP_END: f64 = 14.0;
/// Room kept between a route's corner and a chip on one of its legs,
/// so the chip covers neither the rounded bend nor the next leg.
const CHIP_CORNER: f64 = CORNER_R + 2.0;
/// How far off its line a chip set beside a leg keeps, and how much
/// further out each fallback row beside it goes.
const BESIDE_GAP: f64 = 3.0;
const BESIDE_STEP: f64 = 14.0;
/// Rows of fallback spots beside each leg, on each side.
const BESIDE_ROWS: usize = 3;
/// Spots tried along one leg, spread end to end (plus its middle).
const CHIP_SLIDES: usize = 9;
/// What a chip spot costs. Covering a box, a junction, a title or
/// another chip is never done while any other spot is free; a line
/// through the chip (another edge's, or a bend of its own route) is
/// next worst: the line reads as stopping at the chip. A chip on a
/// trunk it shares with another edge (a fan-in) labels both, so it
/// costs (by how much of it lies along the trunk) more than one set
/// just beside its own stretch of line, and each row further out more
/// again. The rest only picks the tidiest of the free
/// spots: the middle of a long leg, flat legs over upright ones, inside
/// a frame rather than across its side, and on the canvas already
/// drawn rather than widening it.
const CHIP_BLOCKED: f64 = 1e6;
const CHIP_CROSS: f64 = 2_000.0;
const CHIP_SHARED: f64 = 900.0;
const CHIP_BESIDE: f64 = 150.0;
const CHIP_FAR: f64 = 120.0;
const CHIP_SPILL: f64 = 50.0;
const CHIP_FRAME: f64 = 25.0;
const CHIP_UPRIGHT: f64 = 20.0;
/// Per px a spot sits off the middle of its leg, and per px its leg is
/// shorter than the route's longest.
const CHIP_OFF_MID: f64 = 0.25;
const CHIP_SHORT_LEG: f64 = 0.05;

/// A place a label chip may go: its centre, the leg of its route (by
/// index of the leg's first corner) it labels, whether it sits on that
/// leg's line (else beside it), and its price before collisions.
struct Spot {
    c: Pt,
    leg: usize,
    on: bool,
    base: f64,
}

/// The box a chip of width `w` centred on `c` draws.
fn chip_rect(c: Pt, w: f64) -> Rect {
    Rect {
        x0: c.0 - w / 2.0,
        y0: c.1 - LABEL_BOX_H / 2.0,
        x1: c.0 + w / 2.0,
        y1: c.1 + LABEL_BOX_H / 2.0,
    }
}

/// Whether two rects share interior.
fn overlaps(a: &Rect, b: &Rect) -> bool {
    a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1
}

/// Whether `a` lies within `b`.
fn within(a: &Rect, b: &Rect) -> bool {
    a.x0 >= b.x0 - EPS && a.x1 <= b.x1 + EPS && a.y0 >= b.y0 - EPS && a.y1 <= b.y1 + EPS
}

/// Whether the segment `p`–`q` passes through the interior of `r`
/// (Liang–Barsky clipping; a segment along `r`'s outline does not).
fn seg_hits(p: Pt, q: Pt, r: &Rect) -> bool {
    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (pk, qk) in [
        (-dx, p.0 - r.x0),
        (dx, r.x1 - p.0),
        (-dy, p.1 - r.y0),
        (dy, r.y1 - p.1),
    ] {
        if pk.abs() < EPS {
            if qk <= EPS {
                return false;
            }
        } else if pk < 0.0 {
            t0 = t0.max(qk / pk);
        } else {
            t1 = t1.min(qk / pk);
        }
    }
    t0 < t1
}

/// Positions to try along a leg for a chip centre between `first` and
/// `last`: [`CHIP_SLIDES`] spread end to end, and the leg's middle `mid`
/// (where it falls between them).
fn slides(first: f64, last: f64, mid: f64) -> Vec<f64> {
    let mut v = vec![mid.clamp(first, last)];
    for j in 0..CHIP_SLIDES {
        let u = first + (last - first) * j as f64 / (CHIP_SLIDES - 1) as f64;
        if v.iter().all(|&o| (o - u).abs() >= 1.0) {
            v.push(u);
        }
    }
    v
}

/// Every spot a chip of width `w` may take on `route`: on each straight
/// leg that holds it clear of the leg's ends ([`CHIP_END`] at a port,
/// [`CHIP_CORNER`] at a bend), and in [`BESIDE_ROWS`] rows beside every
/// leg on both sides.
fn spots(route: &[Pt], w: f64) -> Vec<Spot> {
    let n = route.len();
    let longest = route
        .windows(2)
        .map(|s| dist(s[0], s[1]))
        .fold(0.0, f64::max);
    let inset = |corner: usize| {
        if corner == 0 || corner + 1 == n {
            CHIP_END
        } else {
            CHIP_CORNER
        }
    };
    let mut out = Vec::new();
    for (i, s) in route.windows(2).enumerate() {
        let (a, b) = (s[0], s[1]);
        let flat = (a.1 - b.1).abs() < EPS;
        if !flat && (a.0 - b.0).abs() >= EPS {
            continue;
        }
        // Coordinates along the leg (x on a flat leg) and of its line.
        let (u0, u1, line) = if flat {
            (a.0, b.0, a.1)
        } else {
            (a.1, b.1, a.0)
        };
        let (lo, hi) = (u0.min(u1), u0.max(u1));
        if hi - lo < EPS {
            continue;
        }
        let (in_lo, in_hi) = if u0 <= u1 {
            (inset(i), inset(i + 1))
        } else {
            (inset(i + 1), inset(i))
        };
        let (along, across) = if flat {
            (w, LABEL_BOX_H)
        } else {
            (LABEL_BOX_H, w)
        };
        let mid = (lo + hi) / 2.0;
        let pt = |u: f64, off: f64| {
            if flat {
                (u, line + off)
            } else {
                (line + off, u)
            }
        };
        let shape = (longest - (hi - lo)) * CHIP_SHORT_LEG + if flat { 0.0 } else { CHIP_UPRIGHT };
        let (first, last) = (lo + in_lo + along / 2.0, hi - in_hi - along / 2.0);
        if first <= last {
            for u in slides(first, last, mid) {
                out.push(Spot {
                    c: pt(u, 0.0),
                    leg: i,
                    on: true,
                    base: shape + (u - mid).abs() * CHIP_OFF_MID,
                });
            }
        }
        // Beside the line a chip covers neither the line nor its
        // arrowhead, so it may run from end to end of the leg (the
        // boxes and the next legs keep it clear of the port and the
        // bend); on a leg too short for it, it overhangs one end.
        let flush = along / 2.0 + CHIP_GAP + 1.0;
        let (first, last) = (lo + flush, hi - flush);
        let at = if first <= last {
            slides(first, last, mid)
        } else {
            vec![mid, first, last]
        };
        for row in 0..BESIDE_ROWS {
            let off = across / 2.0 + BESIDE_GAP + row as f64 * BESIDE_STEP;
            // Above (or right of) the line first: a tie goes there.
            for (tie, sign) in [(0.0, -1.0), (1.0, 1.0)] {
                let sign = if flat { sign } else { -sign };
                for &u in &at {
                    out.push(Spot {
                        c: pt(u, sign * off),
                        leg: i,
                        on: false,
                        base: CHIP_BESIDE
                            + row as f64 * CHIP_FAR
                            + shape
                            + (u - mid).abs() * CHIP_OFF_MID
                            + tie,
                    });
                }
            }
        }
    }
    out
}

/// Side of a [`Buckets`] cell.
const BUCKET: f64 = 64.0;

/// Rects filed under the [`BUCKET`]-px cells of a grid that they
/// touch, so the few near a chip spot are found without testing them
/// all. Positions off the grid count as its edge cells, which keeps
/// every overlapping pair in a shared cell.
struct Buckets {
    x0: f64,
    y0: f64,
    nx: usize,
    ny: usize,
    cells: Vec<Vec<u32>>,
}

impl Buckets {
    fn new(area: Rect, rects: &[Rect]) -> Buckets {
        let cells = |len: f64| ((len / BUCKET).ceil() as usize).max(1);
        let (nx, ny) = (cells(area.x1 - area.x0), cells(area.y1 - area.y0));
        let mut b = Buckets {
            x0: area.x0,
            y0: area.y0,
            nx,
            ny,
            cells: vec![Vec::new(); nx * ny],
        };
        for (k, r) in rects.iter().enumerate() {
            let (i0, i1, j0, j1) = b.span(r);
            for j in j0..=j1 {
                for i in i0..=i1 {
                    b.cells[j * nx + i].push(k as u32);
                }
            }
        }
        b
    }

    /// The cells (columns `i0..=i1`, rows `j0..=j1`) that `r` touches.
    fn span(&self, r: &Rect) -> (usize, usize, usize, usize) {
        let at =
            |v: f64, o: f64, n: usize| (((v - o) / BUCKET).floor().max(0.0) as usize).min(n - 1);
        (
            at(r.x0, self.x0, self.nx),
            at(r.x1, self.x0, self.nx),
            at(r.y0, self.y0, self.ny),
            at(r.y1, self.y0, self.ny),
        )
    }

    /// Into `out`, once each: the rects filed in a cell `r` touches.
    fn near(&self, r: &Rect, out: &mut Vec<u32>) {
        out.clear();
        let (i0, i1, j0, j1) = self.span(r);
        for j in j0..=j1 {
            for i in i0..=i1 {
                out.extend(&self.cells[j * self.nx + i]);
            }
        }
        out.sort_unstable();
        out.dedup();
    }
}

/// What label chips must keep off and should keep within.
struct ChipField<'a> {
    /// Every edge's route, as its corner points.
    routes: &'a [Vec<Pt>],
    /// Every straight leg of every route: its edge, its index on that
    /// route, and its ends; filed by its bounding box in `leg_at`.
    legs: Vec<(usize, usize, Pt, Pt)>,
    leg_at: Buckets,
    /// Boxes, junction dots and group titles, grown by [`CHIP_GAP`];
    /// filed in `block_at`.
    blocks: &'a [Rect],
    block_at: Buckets,
    frames: &'a [Rect],
    /// The drawing so far: boxes, frames and routes, with margins.
    bounds: Rect,
}

impl<'a> ChipField<'a> {
    fn new(routes: &'a [Vec<Pt>], blocks: &'a [Rect], frames: &'a [Rect], bounds: Rect) -> Self {
        let legs: Vec<(usize, usize, Pt, Pt)> = routes
            .iter()
            .enumerate()
            .flat_map(|(k, r)| {
                r.windows(2)
                    .enumerate()
                    .map(move |(i, s)| (k, i, s[0], s[1]))
            })
            .collect();
        let boxes: Vec<Rect> = legs
            .iter()
            .map(|&(_, _, p, q)| Rect {
                x0: p.0.min(q.0),
                y0: p.1.min(q.1),
                x1: p.0.max(q.0),
                y1: p.1.max(q.1),
            })
            .collect();
        ChipField {
            routes,
            leg_at: Buckets::new(bounds, &boxes),
            legs,
            blocks,
            block_at: Buckets::new(bounds, blocks),
            frames,
            bounds,
        }
    }

    /// Price of spot `s` for edge `k`'s chip of width `w`, before other
    /// chips are considered (see [`CHIP_BLOCKED`] and the costs after
    /// it). `near` is scratch space.
    fn price(&self, k: usize, s: &Spot, w: f64, near: &mut Vec<u32>) -> f64 {
        let r = chip_rect(s.c, w);
        let mut cost = s.base;
        self.block_at.near(&r, near);
        if near.iter().any(|&b| overlaps(&r, &self.blocks[b as usize])) {
            cost += CHIP_BLOCKED;
        }
        let route = &self.routes[k];
        let (a, b) = (route[s.leg], route[s.leg + 1]);
        let flat = (a.1 - b.1).abs() < EPS;
        let line = if flat { a.1 } else { a.0 };
        // The chip's extent along its leg, and how much of that another
        // edge runs along too.
        let (lo, hi) = if flat { (r.x0, r.x1) } else { (r.y0, r.y1) };
        let mut shared = 0.0;
        // A line that grazes the chip's outline reads as running into it.
        let hit = r.grow(1.5);
        // Whatever runs through the chip, or along its leg's line beside
        // it.
        let strip = if flat {
            Rect {
                x0: lo,
                y0: line - NEAR,
                x1: hi,
                y1: line + NEAR,
            }
        } else {
            Rect {
                x0: line - NEAR,
                y0: lo,
                x1: line + NEAR,
                y1: hi,
            }
        };
        let span = Rect {
            x0: hit.x0.min(strip.x0),
            y0: hit.y0.min(strip.y0),
            x1: hit.x1.max(strip.x1),
            y1: hit.y1.max(strip.y1),
        };
        self.leg_at.near(&span, near);
        for &l in near.iter() {
            let (j, i, p0, p1) = self.legs[l as usize];
            if j == k && i == s.leg {
                continue;
            }
            if j != k {
                let along = if flat {
                    ((p0.1 - p1.1).abs() < EPS && (p0.1 - line).abs() < NEAR)
                        .then(|| (p0.0.min(p1.0), p0.0.max(p1.0)))
                } else {
                    ((p0.0 - p1.0).abs() < EPS && (p0.0 - line).abs() < NEAR)
                        .then(|| (p0.1.min(p1.1), p0.1.max(p1.1)))
                };
                if let Some((q0, q1)) = along {
                    shared += (hi.min(q1) - lo.max(q0)).max(0.0);
                    if s.on {
                        continue;
                    }
                }
            }
            if seg_hits(p0, p1, &hit) {
                cost += CHIP_CROSS;
            }
        }
        cost += CHIP_SHARED * (shared / (hi - lo)).min(1.0);
        for f in self.frames {
            if overlaps(&r, f) && !within(&r, f) {
                cost += CHIP_FRAME;
            }
        }
        if !within(&r, &self.bounds) {
            cost += CHIP_SPILL;
        }
        cost
    }
}

/// Centre of each edge's label chip, index-parallel with `widths` (a
/// chip's width, `None` for an unlabelled edge): the cheapest of its
/// [`spots`] by [`ChipField::price`], never on a chip placed before it.
/// The edges with the fewest free spots choose first, so a chip with
/// room to spare does not take the only spot another one had; and a
/// chip left with nowhere but on another one moves together with that
/// one ([`Chips::repair`]): two labelled edges on one line between two boxes
/// each take an end of it, rather than the first one taking its middle.
fn place_chips(field: &ChipField, widths: &[Option<f64>]) -> Vec<Option<Pt>> {
    let mut near = Vec::new();
    let priced: Vec<Vec<(Spot, f64)>> = widths
        .iter()
        .enumerate()
        .map(|(k, w)| match w {
            Some(w) => spots(&field.routes[k], *w)
                .into_iter()
                .map(|s| {
                    let p = field.price(k, &s, *w, &mut near);
                    (s, p)
                })
                .collect(),
            None => Vec::new(),
        })
        .map(|mut v: Vec<(Spot, f64)>| {
            // Cheapest first (a tie keeps the order the spots came in).
            v.sort_by(|a, b| a.1.total_cmp(&b.1));
            v
        })
        .collect();
    let mut order: Vec<usize> = (0..widths.len()).filter(|&k| widths[k].is_some()).collect();
    order.sort_by_key(|&k| priced[k].iter().filter(|(_, p)| *p < CHIP_SHARED).count());
    let mut chips = Chips {
        priced: &priced,
        widths,
        at: vec![None; widths.len()],
    };
    for &k in &order {
        let (c, p) = chips
            .cheapest(k, &[k])
            .unwrap_or((field.routes[k][0], CHIP_BLOCKED));
        chips.at[k] = Some((c, p));
    }
    for &k in &order {
        chips.repair(k);
    }
    chips.at.iter().map(|a| a.map(|(c, _)| c)).collect()
}

/// Label chips being placed: each edge's priced spots (cheapest
/// first), its chip's width, and where its chip is (with that spot's
/// price, plus [`CHIP_BLOCKED`] if it lies on another chip).
struct Chips<'a> {
    priced: &'a [Vec<(Spot, f64)>],
    widths: &'a [Option<f64>],
    at: Vec<Option<(Pt, f64)>>,
}

/// Cheapest spots of each of two chips that [`Chips::repair`] tries
/// together.
const PAIR_TRIES: usize = 40;

impl Chips<'_> {
    /// The chip of edge `k` centred on `c`, grown by [`CHIP_APART`].
    fn rect(&self, k: usize, c: Pt) -> Rect {
        chip_rect(c, self.widths[k].unwrap_or_default()).grow(CHIP_APART)
    }

    /// Whether a chip of edge `k` on `c` lies on any placed chip but
    /// those of the edges in `skip`.
    fn on_chip(&self, k: usize, c: Pt, skip: &[usize]) -> bool {
        let r = self.rect(k, c);
        self.at.iter().enumerate().any(|(j, a)| {
            !skip.contains(&j)
                && a.is_some_and(|(o, _)| {
                    overlaps(&r, &chip_rect(o, self.widths[j].unwrap_or_default()))
                })
        })
    }

    /// Edge `k`'s cheapest spot with its price, a spot on a placed chip
    /// (but those of `skip`) costing [`CHIP_BLOCKED`] more: its first
    /// spot clear of everything, or else the least bad of them all.
    fn cheapest(&self, k: usize, skip: &[usize]) -> Option<(Pt, f64)> {
        let spots = &self.priced[k];
        let cost = |s: &Spot, p: f64| {
            p + if self.on_chip(k, s.c, skip) {
                CHIP_BLOCKED
            } else {
                0.0
            }
        };
        let clear = spots
            .iter()
            .find(|(s, p)| *p < CHIP_BLOCKED && !self.on_chip(k, s.c, skip))
            .map(|(s, p)| (s.c, *p));
        clear.or_else(|| {
            spots
                .iter()
                .map(|(s, p)| (s.c, cost(s, *p)))
                .min_by(|a, b| a.1.total_cmp(&b.1))
        })
    }

    /// If edge `k`'s chip was kept off its cheapest spot by exactly one
    /// other chip (or could only go on top of it), place the two anew
    /// together: the cheapest pair of their spots that clear each other
    /// and every other chip, when that beats where they are.
    fn repair(&mut self, k: usize) {
        let Some((_, pk)) = self.at[k] else {
            return;
        };
        let Some(&(Spot { c: ideal, .. }, best)) = self.priced[k].first() else {
            return;
        };
        if pk <= best + EPS {
            return;
        }
        let r = self.rect(k, ideal);
        let under: Vec<usize> = (0..self.at.len())
            .filter(|&j| {
                j != k
                    && self.at[j].is_some_and(|(o, _)| {
                        overlaps(&r, &chip_rect(o, self.widths[j].unwrap_or_default()))
                    })
            })
            .collect();
        let [j] = under[..] else {
            return;
        };
        let Some((_, pj)) = self.at[j] else {
            return;
        };
        let skip = [k, j];
        let free = |e: usize| {
            let mut v: Vec<(Pt, f64)> = self.priced[e]
                .iter()
                .filter(|(s, p)| *p < CHIP_BLOCKED && !self.on_chip(e, s.c, &skip))
                .map(|(s, p)| (s.c, *p))
                .collect();
            v.sort_by(|a, b| a.1.total_cmp(&b.1));
            v.truncate(PAIR_TRIES);
            v
        };
        let (fk, fj) = (free(k), free(j));
        let mut best: Option<(Pt, f64, Pt, f64)> = None;
        for &(a, qa) in &fk {
            let ra = self.rect(k, a);
            for &(b, qb) in &fj {
                let rb = chip_rect(b, self.widths[j].unwrap_or_default());
                if !overlaps(&ra, &rb) && best.map_or(true, |(_, x, _, y)| qa + qb < x + y) {
                    best = Some((a, qa, b, qb));
                }
            }
        }
        if let Some((a, qa, b, qb)) = best {
            if qa + qb < pk + pj {
                self.at[k] = Some((a, qa));
                self.at[j] = Some((b, qb));
            }
        }
    }
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
    for (g, c) in as_.scene.clusters.iter().enumerate() {
        s.push_str(&format!(
            "<rect x='{:.1}' y='{:.1}' width='{:.1}' height='{:.1}' \
             rx='8' fill='#f7f8fc' stroke='#d5d9ec' stroke-width='2'/>\n",
            c.x, c.y, c.w, c.h
        ));
        if !c.title.is_empty() {
            let (tx, ty) = as_
                .title_at
                .get(g)
                .copied()
                .unwrap_or((c.x + GROUP_PAD, c.y + TITLE_CY));
            s.push_str(&format!(
                "<text x='{:.1}' y='{:.1}' dy='0.33em' font-weight='bold' \
                 fill='{}'>{}</text>\n",
                tx,
                ty,
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

    /// Diagrams an independent review drew badly: an edge rising into
    /// the top port of a group's first service under a title wider than
    /// half the box (struck or grazed it, with a wiggle), a realistic
    /// 18-service tier diagram (a jog on another edge drew a false
    /// junction), a label chip over the top row (cut off by the canvas)
    /// and random tangles (two unrelated edges sharing a line).
    const REPROS: &[&str] = &[
        "architecture-beta
    service user(user)[Engineer]
    group obs(server)[Observability]
    service prom(monitoring)[Prometheus] in obs
    service graf(chart)[Grafana] in obs
    user:B --> T:prom
    prom:R -- L:graf",
        "architecture-beta
    service user(user)[User]
    group frontend(cloud)[Frontend]
    service web(web)[Web App] in frontend
    user:B --> T:web",
        "architecture-beta
    group vpc(cloud)[Production VPC]
    group pub(internet)[Public Subnet] in vpc
    group priv(server)[Private Subnet] in vpc
    service igw(internet)[Internet Gateway]
    service alb(network)[Load Balancer] in pub
    service app(server)[App Server] in priv
    service db(database)[Database] in priv
    igw:B --> T:alb
    alb:B --> T:app
    app:R -- L:db",
        "architecture-beta
    group edge(cloud)[Edge]
    service cdn(internet)[CDN] in edge
    service waf(firewall)[WAF] in edge
    service lb(network)[Load Balancer] in edge
    group app(server)[Application Tier]
    service web1(server)[Web 1] in app
    service web2(server)[Web 2] in app
    service web3(server)[Web 3] in app
    service worker(cpu)[Worker] in app
    group data(database)[Data Tier]
    service pg(database)[Postgres Primary] in data
    service pgr(database)[Postgres Replica] in data
    service redis(cache)[Redis] in data
    service s3(disk)[Object Storage] in data
    group obs(monitoring)[Observability]
    service prom(metrics)[Prometheus] in obs
    service graf(chart)[Grafana] in obs
    service loki(logs)[Loki] in obs
    cdn:R --> L:waf
    waf:R --> L:lb
    lb:B --> T:web1
    lb:B --> T:web2
    lb:B --> T:web3
    web1:B -- T:pg
    web2:B -- T:redis
    web3:B -- T:s3
    worker:L -- R:web3
    worker:B -- T:pg
    pg:R -- L:pgr
    prom:L -- R:worker
    prom:B -- T:graf
    loki:L -- R:graf",
        "architecture-beta
    group public_api(cloud)[Public API]
    group private_api(cloud)[Private API] in public_api
    service database(database)[Database] in private_api
    service server(server)[Server] in private_api
    service gateway(internet)[Gateway] in public_api
    service lb(network)[Load Balancer]
    group storage(disk)[Storage]
    service s3(disk)[Object Store] in storage
    service bk(disk)[Backup] in storage
    lb:R --> L:gateway
    gateway:R --> L:server
    server:R -- L:database
    database:B -- T:s3
    s3:R -- L:bk
    server:B -- T:bk",
        "architecture-beta
    service a(server)[A]
    service b(server)[B]
    a:R -[]- L:b
    a:B -[\"quoted, label\"]-> T:b",
        "architecture-beta
    group g0(internet)[Backend Services]
    group g1(server)[Backend Services] in g0
    group g2(server)[Observability]
    service s0(cloud)[Service 0] in g0
    service s1(monitoring)[Service 1] in g2
    service s2(cloud)[Service 2] in g0
    service s3(internet)[Service 3]
    service s4(database)[Service 4] in g2
    service s5(database)[Service 5] in g2
    service s6(server)[Service 6] in g2
    service s7(cloud)[Service 7]
    service s8(database)[Service 8] in g2
    service s9(disk)[Service 9] in g0
    service s10(database)[Service 10] in g2
    s5:T --> B:s6
    s9:T --> B:s4
    s9:R --> T:s7
    s9:B -- R:s5
    s4:B -- T:s10
    s7:L --> R:s2
    s0:R -- L:s9
    s0:T --> B:s1
    s7:B -- T:s6",
    ];

    /// A deterministic pseudo-random diagram (xorshift on `seed`): `ns`
    /// services, most in one of `ng` groups (some nested), and `ne`
    /// edges between random services, mostly on facing ports as real
    /// diagrams have them, each with a 1-in-3 chance of any side.
    fn tangle(seed: u64, ns: usize, ng: usize, ne: usize) -> String {
        let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        let mut below = |n: usize| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x % n as u64) as usize
        };
        let titles = [
            "Frontend",
            "Backend Services",
            "Data",
            "Observability",
            "Data Storage Layer",
            "Private API",
        ];
        let icons = ["cloud", "server", "database", "disk", "internet"];
        let mut s = String::from("architecture-beta\n");
        for g in 0..ng {
            let parent = if g > 0 && below(4) == 0 {
                format!(" in g{}", below(g))
            } else {
                String::new()
            };
            let (icon, title) = (icons[below(5)], titles[below(6)]);
            s += &format!("group g{g}({icon})[{title}]{parent}\n");
        }
        for i in 0..ns {
            let grp = if ng > 0 && below(5) != 0 {
                format!(" in g{}", below(ng))
            } else {
                String::new()
            };
            s += &format!("service s{i}({})[Service {i}]{grp}\n", icons[below(5)]);
        }
        let sides = ["L", "R", "T", "B"];
        let facing = |a: &str| match a {
            "L" => "R",
            "R" => "L",
            "T" => "B",
            _ => "T",
        };
        for _ in 0..ne {
            let a = below(ns);
            let b = (a + 1 + below(ns - 1)) % ns;
            let sa = sides[below(4)];
            let sb = if below(3) != 0 {
                facing(sa)
            } else {
                sides[below(4)]
            };
            let arrow = if below(2) == 0 { "-->" } else { "--" };
            s += &format!("s{a}:{sa} {arrow} {sb}:s{b}\n");
        }
        s
    }

    /// No edge turns on a line another edge runs along, and no two
    /// edges share a line for more than a few px — unless they share an
    /// end service, where merging into one trunk is meant — and no edge
    /// runs through a junction it does not end at. Each draws a
    /// junction that is not there.
    fn assert_no_false_junction(src: &str) {
        if let Some(why) = false_junction(src) {
            panic!("{src}\n{why}");
        }
    }

    /// The first false junction [`assert_no_false_junction`] looks for.
    fn false_junction(src: &str) -> Option<String> {
        let (as_, routes) = build(&arch(src));
        let sc = as_.scene;
        for j in sc.nodes.iter().filter(|n| n.shape == Shape::Circle) {
            for e in sc.edges.iter().filter(|e| e.from != j.id && e.to != j.id) {
                let near = e
                    .waypoints
                    .windows(2)
                    .map(|w| seg_dist((j.x, j.y), w[0], w[1]))
                    .fold(f64::INFINITY, f64::min);
                if near < CLEAR {
                    return Some(format!("{}->{} passes junction {}", e.from, e.to, j.id));
                }
            }
        }
        // Each route's straight legs, and the corners between them.
        type Legs = Vec<(Pt, Pt)>;
        let drawn: Vec<(Legs, Vec<Pt>)> = routes
            .iter()
            .map(|r| {
                let legs = r.windows(2).map(|w| (w[0], w[1])).collect();
                (legs, r[1..r.len() - 1].to_vec())
            })
            .collect();
        for (i, e) in sc.edges.iter().enumerate() {
            for (j, f) in sc.edges.iter().enumerate() {
                if i == j
                    || [&e.from, &e.to]
                        .iter()
                        .any(|s| **s == f.from || **s == f.to)
                {
                    continue;
                }
                for &c in &drawn[i].1 {
                    for &(a, b) in &drawn[j].0 {
                        if seg_dist(c, a, b) < 1.5 {
                            return Some(format!(
                                "{}->{} turns at {c:?} on {}->{}",
                                e.from, e.to, f.from, f.to
                            ));
                        }
                    }
                }
                if j < i {
                    continue;
                }
                let mut shared = 0.0;
                for &(a0, a1) in &drawn[i].0 {
                    for &(b0, b1) in &drawn[j].0 {
                        // How far the two legs overlap along x (k = 0) or y.
                        let along = |k: usize| {
                            let (u, v, w2, z) = if k == 0 {
                                (a0.0, a1.0, b0.0, b1.0)
                            } else {
                                (a0.1, a1.1, b0.1, b1.1)
                            };
                            (u.max(v).min(w2.max(z)) - u.min(v).max(w2.min(z))).max(0.0)
                        };
                        let (ha, hb) = ((a0.1 - a1.1).abs() < EPS, (b0.1 - b1.1).abs() < EPS);
                        if ha && hb && (a0.1 - b0.1).abs() < NEAR {
                            shared += along(0);
                        } else if !ha && !hb && (a0.0 - b0.0).abs() < NEAR {
                            shared += along(1);
                        }
                    }
                }
                if shared > 4.0 {
                    return Some(format!(
                        "{}->{} and {}->{} share {shared}px",
                        e.from, e.to, f.from, f.to
                    ));
                }
            }
        }
        None
    }

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
        for src in std::iter::once(&CORPUS).chain(MORE).chain(REPROS) {
            let as_ = scene(&arch(src));
            let sc = &as_.scene;
            for (g, c) in sc.clusters.iter().enumerate() {
                if c.title.is_empty() {
                    continue;
                }
                // The title as drawn, at its wide-erring width, and the
                // clearance less a px: no stroke comes within ~6px of a
                // title's ends or ~2px of its top and bottom.
                let (x, y) = as_.title_at[g];
                let r = Rect {
                    x0: x,
                    y0: y - TITLE_HALF_H,
                    x1: x + title_px(&c.title),
                    y1: y + TITLE_HALF_H,
                }
                .grow_xy(TITLE_CLEAR - 1.0, TITLE_CLEAR_Y - 1.0);
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
    fn title_widths_err_wide_of_what_a_browser_draws() {
        // Bold 14px Helvetica/Arial with a colour emoji, as Chrome
        // measures the ink (getExtentOfChar): the old estimate counted
        // the emoji as 1em and bold as 6% wider, ~5px short on
        // "🗄️ Observability" — an edge beside it went through the "y".
        for (title, chrome) in [
            ("\u{1F5C4}\u{FE0F} Observability", 110.06),
            ("\u{1F5C4}\u{FE0F} Application Tier", 126.38),
            ("\u{2601}\u{FE0F} Frontend", 82.02),
            ("\u{1F310} Public Subnet", 114.7),
            ("\u{1F4BE} Storage", 73.48),
            ("\u{1F4C8} Backend Services", 141.22),
            ("\u{1F6E2}\u{FE0F} Data Storage Layer", 148.97),
            ("\u{2601}\u{FE0F} Group 12", 82.83),
            ("Observability", 88.7),
            ("Utilities", 50.58),
            ("Billing", 42.78),
            ("Kubernetes Cluster", 129.16),
        ] {
            assert!(
                title_px(title) >= chrome,
                "{title}: {} < {chrome}",
                title_px(title)
            );
        }
    }

    #[test]
    fn an_edge_into_a_top_port_runs_straight_beside_the_title() {
        // The edge rises straight into Prometheus's top port; the title
        // moves right of its line instead of the edge wiggling round it
        // (and grazing it), and the frame widens to hold it.
        for src in &REPROS[..3] {
            let as_ = scene(&arch(src));
            let sc = &as_.scene;
            let e = &sc.edges[0];
            assert_eq!(e.waypoints.len(), 2, "{src}\n{:?}", e.waypoints);
            let x = e.waypoints[0].0;
            for (g, c) in sc.clusters.iter().enumerate() {
                let (tx, _) = as_.title_at[g];
                let right = tx + title_px(&c.title);
                assert!(
                    right + TITLE_CLEAR < x || tx > x + TITLE_CLEAR,
                    "{src}\n{} spans {tx}..{right}, the edge runs at {x}",
                    c.id
                );
                assert!(right + GROUP_PAD <= c.x + c.w + EPS, "{} sticks out", c.id);
            }
        }
        // With nothing rising through it, a title keeps its place.
        let as_ = scene(&arch(CORPUS));
        for (g, c) in as_.scene.clusters.iter().enumerate() {
            assert_eq!(as_.title_at[g], (c.x + GROUP_PAD, c.y + TITLE_CY));
        }
        let svg = to_svg(&scene(&arch(REPROS[0])));
        let as_ = scene(&arch(REPROS[0]));
        assert!(svg.contains(&format!("<text x='{:.1}'", as_.title_at[0].0)));
    }

    #[test]
    fn no_edge_draws_a_junction_that_is_not_there() {
        // The 18-service tier diagram ran worker -> pg six px up the
        // web2 -> redis line and off it again: a T on the Redis edge.
        // Random tangles (one in fourteen used to) turned on a foreign
        // line or shared one for hundreds of px.
        for src in std::iter::once(&CORPUS).chain(MORE).chain(REPROS) {
            assert_no_false_junction(src);
        }
        // c -- d runs along the junction's row, straight at its dot.
        assert_no_false_junction(
            "architecture-beta
    service a(server)[A]
    junction j
    service b(server)[B]
    service c(server)[C]
    service d(server)[D]
    a:B -- T:j
    j:B -- T:b
    a:B -- R:c
    a:B -- L:d
    c:R -- L:d",
        );
        // Random tangles of up to 16 services with up to four edges more
        // than services, between random ports: a dense enough tangle
        // can leave a route no clean way into its port, so this is a
        // rate. The reviewer's harness found a false junction in about
        // one such diagram in fourteen; it is now under one in fifty.
        let mut touching = Vec::new();
        for seed in 1..=200u64 {
            let ns = 3 + (seed as usize * 7) % 13;
            let ne = 1 + (seed as usize * 5) % (ns + 4);
            let src = tangle(seed, ns, (seed % 5) as usize, ne);
            if let Some(why) = false_junction(&src) {
                touching.push(format!("seed {seed}: {why}"));
            }
            assert_no_edge_through_a_box(&src);
            assert_layout_is_clean(&src);
        }
        assert!(touching.len() <= 4, "{touching:#?}");
    }

    #[test]
    fn routes_and_label_chips_stay_on_the_canvas() {
        // a:B -[label]-> T:b runs over the top row; its chip used to sit
        // at y = -2, cut off. The drawing moves down (or right) instead.
        let sc = scene(&arch(REPROS[5])).scene;
        let (_, c, _) = sc.edges[1].label.clone().unwrap();
        assert!(c.1 - LABEL_BOX_H / 2.0 >= OUTER - EPS, "chip at {c:?}");
        for src in std::iter::once(&CORPUS).chain(MORE).chain(REPROS) {
            let sc = scene(&arch(src)).scene;
            let on = |p: Pt, m: f64| {
                p.0 >= m - EPS
                    && p.1 >= m - EPS
                    && p.0 <= sc.width - m + EPS
                    && p.1 <= sc.height - m + EPS
            };
            for e in &sc.edges {
                assert!(
                    e.waypoints.iter().all(|&p| on(p, OUTER)),
                    "{src}\n{:?}",
                    e.waypoints
                );
                if let Some((_, c, w)) = &e.label {
                    let h = LABEL_BOX_H / 2.0;
                    assert!(on((c.0 - w / 2.0, c.1 - h), 0.0) && on((c.0 + w / 2.0, c.1 + h), 0.0));
                }
            }
            for n in &sc.nodes {
                assert!(
                    on((n.x - n.w / 2.0, n.y - n.h / 2.0), PAD)
                        && on((n.x + n.w / 2.0, n.y + n.h / 2.0), PAD)
                );
            }
        }
    }

    #[test]
    fn linked_groups_keep_their_own_grid() {
        // Six groups of 2x4 services wired like a grid, linked in a
        // chain with two extra cross links, so groups 4 and 5 are each
        // reached from two other groups at once. One placement pass for
        // the whole component let other groups' services push their
        // cells around; each group now keeps its own grid.
        let mut src = String::from("architecture-beta\n");
        for g in 0..6 {
            src += &format!("group g{g}(cloud)[Group {g}]\n");
            for i in 0..8 {
                src += &format!("service s{g}_{i}(server)[Service {g}.{i}] in g{g}\n");
            }
        }
        let mut inner = Vec::new();
        for g in 0..6 {
            for i in 0..8 {
                if i % 4 < 3 {
                    inner.push(format!("s{g}_{i}:R --> L:s{g}_{}", i + 1));
                }
                if i < 4 {
                    inner.push(format!("s{g}_{i}:B --> T:s{g}_{}", i + 4));
                }
            }
        }
        let links = [
            "s0_3:R --> L:s1_0",
            "s1_7:B --> T:s2_0",
            "s2_3:R --> L:s3_0",
            "s3_7:B --> T:s4_0",
            "s4_3:R --> L:s5_0",
            "s4_4:T -- B:s2_0",
            "s5_7:T -- B:s3_5",
        ];
        for l in inner.iter().map(String::as_str).chain(links) {
            src += l;
            src.push('\n');
        }
        assert_layout_is_clean(&src);
        let sc = scene(&arch(&src)).scene;
        for l in &inner {
            let (a, rest) = l.split_once(':').unwrap();
            let b = rest.rsplit(':').next().unwrap();
            let (na, nb) = (node(&sc, a), node(&sc, b));
            if l.contains(":R") {
                assert!((na.y - nb.y).abs() < 0.5 && nb.x > na.x, "{l}");
            } else {
                assert!((na.x - nb.x).abs() < 0.5 && nb.y > na.y, "{l}");
            }
        }
        // A service outside the group that the walk put in the cell b
        // wants no longer pushes b off a's row: cells are taken per
        // container, and the stranger moves aside instead.
        let src = "architecture-beta
    service x(server)[X]
    service y(server)[Y]
    group g[G]
    service a(server)[A] in g
    service b(server)[B] in g
    x:R -- T:y
    x:B -- T:a
    a:R -- L:b";
        assert_layout_is_clean(src);
        let sc = scene(&arch(src)).scene;
        let (a, b) = (node(&sc, "a"), node(&sc, "b"));
        assert!((a.y - b.y).abs() < 0.5 && b.x > a.x);
        let (x, y) = (node(&sc, "x"), node(&sc, "y"));
        assert!(y.x > x.x && y.y > x.y, "x:R -- T:y stays diagonal");
    }

    /// Labelled diagrams an independent review drew with label chips on
    /// top of each other: two labelled edges merging into one trunk into
    /// a port (both chips on the trunk, one hiding the other), a fan-in
    /// from two clients, and a chip wider than its leg that hid its own
    /// bends and half of a neighbour's chip.
    const CHIP_REPROS: &[&str] = &[
        "architecture-beta
    group client(internet)[Clients]
    group api(cloud)[API Layer]
    group data(database)[Data]
    service browser(web)[Browser] in client
    service app(mobile)[Mobile App] in client
    service gw(api)[Gateway] in api
    service orders(server)[Orders] in api
    service users(user)[Users] in api
    service pg(database)[Postgres] in data
    service cache(cache)[Redis] in data
    service bus(queue)[Event Bus] in data
    browser:R -[REST over https]-> L:gw
    app:R -[https]-> L:gw
    gw:R -[route]-> L:orders
    gw:B -[auth]-> T:users
    orders:R -[sql]-> L:pg
    orders:B -[publish]-> T:bus
    users:R -[lookup]-> L:cache
    users:R -[read replica]-> L:pg",
        "architecture-beta
    service a(server)[Alpha]
    service b(database)[Beta]
    service c(disk)[Gamma]
    service d(internet)[Delta with a fairly long label that must wrap]
    a:T -[upward label at top]-> B:b
    a:L -[left side chip label]-> R:c
    c:T -[x]- B:d
    b:L -[very long edge label text here]- T:c",
        "architecture-beta
    service api(server)[API]
    service worker(cpu)[Worker]
    service db(database)[Database]
    api:R -[reads]-> L:db
    worker:R -[writes]-> L:db",
    ];

    /// `src` with a label on every other edge, the labels (short and
    /// long) taken in turn.
    fn labelled(src: &str) -> String {
        let labels = [
            "calls",
            "reads config",
            "publishes order events",
            "x",
            "replicates to standby",
        ];
        let mut k = 0;
        src.lines()
            .map(|l| {
                let arrow = [" --> ", " -- "].into_iter().find(|a| l.contains(a));
                let Some(arrow) = arrow else {
                    return format!("{l}\n");
                };
                k += 1;
                if k % 2 == 1 {
                    return format!("{l}\n");
                }
                let text = labels[(k / 2) % labels.len()];
                let tip = if arrow == " --> " { "->" } else { "-" };
                format!("{}\n", l.replacen(arrow, &format!(" -[{text}]{tip} "), 1))
            })
            .collect()
    }

    /// What is wrong with where `src`'s label chips went: a chip on
    /// another chip, a box or junction, or a group title (`hard`, never
    /// allowed), or a line through it other than the leg it labels —
    /// another edge's, or a bend of its own route (`soft`). A chip on a
    /// trunk its edge shares with others (a fan-in) is not a fault.
    fn chip_faults(src: &str) -> (Vec<String>, Vec<String>) {
        let as_ = scene(&arch(src));
        let sc = &as_.scene;
        let (mut hard, mut soft) = (Vec::new(), Vec::new());
        let chips: Vec<(usize, &str, Pt, Rect)> = sc
            .edges
            .iter()
            .enumerate()
            .filter_map(|(k, e)| {
                let (text, c, w) = e.label.as_ref()?;
                Some((k, text.as_str(), *c, chip_rect(*c, *w)))
            })
            .collect();
        for (i, &(_, t, _, r)) in chips.iter().enumerate() {
            for &(_, u, _, q) in &chips[i + 1..] {
                if overlaps(&r.grow(CHIP_GAP), &q) {
                    hard.push(format!("chip '{t}' overlaps (or touches) chip '{u}'"));
                }
            }
            for n in &sc.nodes {
                if overlaps(&r, &Rect::around(n)) {
                    hard.push(format!("chip '{t}' covers {}", n.id));
                }
            }
            for (g, c) in sc.clusters.iter().enumerate() {
                let f = Rect {
                    x0: c.x,
                    y0: c.y,
                    x1: c.x + c.w,
                    y1: c.y + c.h,
                };
                let tr = title_rect(f, as_.title_at[g].0, title_px(&c.title));
                if !c.title.is_empty() && overlaps(&r, &tr) {
                    hard.push(format!("chip '{t}' covers title {}", c.title));
                }
            }
        }
        for &(k, t, c, r) in &chips {
            // The line a chip sits on: flat through its centre, or upright.
            let carrier = |a: Pt, b: Pt| {
                ((a.1 - b.1).abs() < EPS && (a.1 - c.1).abs() < EPS)
                    || ((a.0 - b.0).abs() < EPS && (a.0 - c.0).abs() < EPS)
            };
            for (j, e) in sc.edges.iter().enumerate() {
                let through = e
                    .waypoints
                    .windows(2)
                    .any(|w| !carrier(w[0], w[1]) && seg_hits(w[0], w[1], &r));
                if through {
                    let whose = if j == k {
                        "its own bend"
                    } else {
                        "another edge"
                    };
                    soft.push(format!(
                        "{whose} ({}->{}) runs through chip '{t}'",
                        e.from, e.to
                    ));
                }
            }
        }
        (hard, soft)
    }

    #[test]
    fn label_chips_never_cover_each_other_a_box_or_a_title() {
        // Review: chips went to the middle of the longest flat leg with no
        // look at anything else, so 'read replica' hid 'sql' on a shared
        // trunk, 'https' covered the end of 'REST over https', and a chip
        // wider than its leg hid its own bends and half of the 'x' chip.
        for src in std::iter::once(CORPUS)
            .chain(MORE.iter().copied())
            .chain(REPROS.iter().copied())
            .chain(CHIP_REPROS.iter().copied())
        {
            let (hard, soft) = chip_faults(src);
            assert!(
                hard.is_empty() && soft.is_empty(),
                "{src}\n{hard:?}\n{soft:?}"
            );
        }
        // Each repro edge labels its own line: 'sql' and 'https' (whose
        // edges merge into a trunk another labelled edge also runs on)
        // sit by their own stretch of line, not on the trunk.
        let sc = scene(&arch(CHIP_REPROS[0])).scene;
        let chip = |text: &str| {
            sc.edges
                .iter()
                .find_map(|e| e.label.as_ref().filter(|l| l.0 == text))
                .map(|l| chip_rect(l.1, l.2))
                .unwrap()
        };
        let (orders, users) = (node(&sc, "orders"), node(&sc, "users"));
        let sql = chip("sql");
        assert!(sql.x0 < orders.x + orders.w / 2.0 + 40.0, "{sql:?}");
        let replica = chip("read replica");
        assert!(replica.y0 > users.y - users.h / 2.0, "on users' own leg");
        let app = node(&sc, "app");
        let https = chip("https");
        assert!(
            https.y1 > app.y - app.h / 2.0,
            "on app's own leg: {https:?}"
        );
        // Random tangles with every other edge labelled: never a chip on
        // a chip, a box or a title, and lines through chips only where a
        // dense tangle leaves no free spot by the line.
        let mut through = 0;
        let mut chips = 0;
        for seed in 1..=80u64 {
            let src = labelled(&tangle(
                seed,
                4 + (seed % 9) as usize,
                (seed % 4) as usize,
                8,
            ));
            let (hard, soft) = chip_faults(&src);
            assert!(hard.is_empty(), "{src}\n{hard:?}");
            through += soft.len();
            chips += src.matches("-[").count();
        }
        eprintln!("{through} of {chips}");
        assert!(chips > 200, "{chips}");
        assert!(
            through * 20 <= chips,
            "{through} of {chips} chips have a line through them"
        );
    }

    #[test]
    fn a_chip_beside_its_line_when_the_line_has_no_room() {
        // 'x' on c:T -- B:d: all but 30px of its only leg is a trunk shared
        // with b -> c, and a chip there would label both. It goes beside
        // its own stretch instead, clear of the trunk.
        let (as_, routes) = build(&arch(CHIP_REPROS[1]));
        let e = &as_.scene.edges[2];
        let (_, c, w) = e.label.clone().unwrap();
        let r = chip_rect(c, w);
        let leg = (routes[2][0], routes[2][1]);
        assert!((leg.0 .0 - leg.1 .0).abs() < EPS, "an upright leg");
        assert!(r.x1 < leg.0 .0 || r.x0 > leg.0 .0, "beside the line: {r:?}");
        // Its own stretch: nearer Delta (d, top) than the trunk's join.
        let d = node(&as_.scene, "d");
        assert!(r.y0 < d.y + d.h / 2.0 + 30.0, "{r:?}");
        // The long chip keeps clear of every bend of its own route.
        let (_, soft) = chip_faults(CHIP_REPROS[1]);
        assert!(soft.is_empty(), "{soft:?}");
    }

    #[test]
    fn chip_geometry_helpers() {
        let r = Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 10.0,
        };
        assert!(seg_hits((-5.0, 5.0), (15.0, 5.0), &r));
        assert!(seg_hits((5.0, 5.0), (5.0, 6.0), &r), "inside");
        assert!(!seg_hits((-5.0, 0.0), (15.0, 0.0), &r), "along the outline");
        assert!(!seg_hits((-5.0, 5.0), (-1.0, 5.0), &r), "short of it");
        assert!(
            seg_hits((-1.0, 11.0), (11.0, -1.0), &r),
            "a diagonal through it"
        );
        assert!(
            !seg_hits((8.0, 15.0), (15.0, 8.0), &r),
            "a diagonal past its corner"
        );
        assert!(!seg_hits((10.0, -5.0), (10.0, 15.0), &r));
        let s = slides(10.0, 90.0, 50.0);
        assert_eq!(s[0], 50.0, "the middle first");
        assert!(s.contains(&10.0) && s.contains(&90.0) && s.len() == CHIP_SLIDES);
        assert_eq!(slides(10.0, 90.0, 200.0)[0], 90.0, "the middle clamped");
    }

    #[test]
    fn the_lattice_marks_exactly_what_the_clearances_cover() {
        // Grid::new finds each clearance's lines by bisection instead of
        // testing every point against every clearance; it must block and
        // close exactly what the direct test does.
        let rects = [
            (10.0, 10.0, 60.0, 40.0),
            (55.0, 30.0, 120.0, 90.0),
            (0.0, 100.0, 30.0, 101.0),
            (70.0, 0.0, 71.0, 200.0),
            (35.0, 45.0, 36.0, 46.0),
        ];
        let obstacles: Vec<(Rect, bool)> = rects
            .iter()
            .map(|&(x0, y0, x1, y1)| (Rect { x0, y0, x1, y1 }, true))
            .collect();
        let plain: Vec<Rect> = obstacles.iter().map(|o| o.0).collect();
        let g = Grid::new(&obstacles, &plain, &[], &Architecture::default(), &[]);
        let (nx, ny) = (g.xs.len(), g.ys.len());
        let inside = |lo: f64, v: f64, hi: f64| lo + EPS < v && v < hi - EPS;
        for j in 0..ny {
            for i in 0..nx {
                let (x, y, v) = (g.xs[i], g.ys[j], j * nx + i);
                let blocked = obstacles
                    .iter()
                    .any(|(o, _)| inside(o.x0, x, o.x1) && inside(o.y0, y, o.y1));
                assert_eq!(g.blocked[v], blocked, "point {x},{y}");
                let h = i + 1 < nx
                    && !obstacles.iter().any(|(o, _)| {
                        inside(o.y0, y, o.y1) && x < o.x1 - EPS && g.xs[i + 1] > o.x0 + EPS
                    });
                assert_eq!(g.h_open[v], h, "step east of {x},{y}");
                let vv = j + 1 < ny
                    && !obstacles.iter().any(|(o, _)| {
                        inside(o.x0, x, o.x1) && y < o.y1 - EPS && g.ys[j + 1] > o.y0 + EPS
                    });
                assert_eq!(g.v_open[v], vv, "step south of {x},{y}");
            }
        }
    }

    #[test]
    fn output_is_deterministic() {
        for src in std::iter::once(&CORPUS).chain(MORE).chain(REPROS) {
            let a = arch(src);
            assert_eq!(to_svg(&scene(&a)), to_svg(&scene(&a)));
        }
    }
}
