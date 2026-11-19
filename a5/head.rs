//! Git graph rendering: branches as lanes, commits as dots, forks and
//! merges as rounded elbows in the colour of the lane they run along.
//!
//! Like `pie`/`journey`/`mindmap` there is nothing draggable and no
//! `route()`: [`scene`] computes every coordinate and [`to_svg`]
//! serialises it. The returned [`GitScene::scene`] is a generic
//! [`crate::scene::Scene`] so terminal/GUI renderers can draw the
//! topology with no extra code: every edge carries its lane colour and
//! line width in `meta.style`, an elbow its rounded corner as dense
//! `waypoints` (and as both control points of its `bezier`), and every
//! commit its glyph as the node shape — a merge is a
//! [`Shape::DoubleCircle`], a HIGHLIGHT a square [`Shape::Rect`], a
//! REVERSE a circle labelled [`REVERSE_MARK`]. The SVG writer adds
//! commit ids, tags, branch-name pills and lane guides on top.
//!
//! Layout. A branch's lane is the rank of its `GitBranch::order`, and
//! every commit gets a column (LR) / row (TB, BT) of its own in
//! sequence order. A cross-lane edge is mermaid's elbow: a leg straight
//! across the lanes at one end and a run along a lane to the other end,
//! the run on the lane farther from main (a fork drops from its parent
//! and runs along the new branch; a merge runs along its branch and
//! climbs into the merge commit) — unless a commit sits on that
//! stretch of the lane, when the run takes the other lane, whose
//! stretch is always free. Labels stay off the lines: a commit id or
//! tag that its own commit's leg would cross steps aside of it, and in
//! LR each column sits far enough right that no id, tag or leg reaches
//! into the one before it (so long ids never run together); in TB/BT a
//! lane sits far enough over to fit the ids beside it.

use crate::layout::{text_width, TEXT_CALIBRATION};
use crate::model::{CommitKind, EdgeKind, EdgeMeta, GitGraph, GitOrientation, NodeStyle, Shape};
use crate::scene::{
    edge_path_d, escape, style_attr, svg_open, Scene, SceneEdge, SceneNode, SvgOptions,
    EDGE_COLOR, TEXT_COLOR,
};
use crate::style::{accent, accent_ink, text_on};

/// Canvas margin around the drawn content.
pub const PAD: f64 = 24.0;
/// Minimum width of the branch-name column left of the first commit in
/// LR; a longer name's pill widens it.
pub const BRANCH_LABEL_W: f64 = 70.0;
/// Minimum distance between neighbouring commit columns (LR) / rows
/// (TB, BT). In LR a column moves further right whenever the ids, tags
/// and elbow legs around it need the room to stay clear of each other.
pub const COMMIT_DX: f64 = 50.0;
/// Distance between branch lanes in LR; the minimum in TB/BT, where a
/// lane moves over as far as the ids and tags beside it need.
pub const BRANCH_DY: f64 = 70.0;
/// Commit dot radius.
pub const R: f64 = 11.0;
/// Font size for commit ids and branch names.
pub const FONT: u32 = 13;
/// The label a REVERSE commit's scene node carries (its other commits'
/// nodes have none): a host drawing node labels shows the commit
/// crossed out, the way the SVG writer draws it.
pub const REVERSE_MARK: &str = "\u{2715}";

/// Stroke width of branch lines, forks and merges (round-capped).
const EDGE_W: f64 = 5.0;
/// Corner radius of a fork / merge elbow.
const ELBOW_R: f64 = 16.0;
/// Commit id centre below its dot (LR).
const ID_DY: f64 = R + 14.0;
/// Gap between a dot and the id / tag beside it (TB, BT) or the tag
/// above it (LR).
const SIDE_GAP: f64 = 5.0;
/// Clear space kept between any two labels, tags, legs and lanes.
const GAP: f64 = 8.0;
/// An id or tag stepped aside of its commit's leg starts this far
/// from the leg's centre line.
const LEG_CLEAR: f64 = EDGE_W / 2.0 + 4.0;
/// Height of the box a 13px id is kept clear in.
const TEXT_H: f64 = 16.0;
/// Tag (luggage-label) font size, pill height, text padding, and the
/// extra room at its left end for the hole.
const TAG_FONT: u32 = 12;
const TAG_H: f64 = 18.0;
const TAG_PX: f64 = 6.0;
const TAG_HOLE: f64 = 6.0;
/// Tag pill fill (the canvas white: the tag text is lane ink, made to
/// read on white), distinct from the solid branch-name pills.
const TAG_FILL: &str = "#ffffff";
/// Branch-name text on its pill: every accent ink carries white text
/// at the same contrast it has on the white canvas.
const PILL_TEXT: &str = "#ffffff";
/// Branch-name pill height and text padding.
const PILL_H: f64 = 22.0;
const PILL_PX: f64 = 9.0;
/// Dashed guide along every lane.
const GUIDE_COLOR: &str = "#c9cdd9";
/// Knock-out behind commit ids (the canvas colour): a line that passes
/// close by never shows through the letters.
const HALO: &str = "#ffffff";
/// Fill of a merge commit's centre and of a highlight commit's inset.
const HOLE_FILL: &str = "#ffffff";
/// Outline of a HIGHLIGHT commit's square.
const HIGHLIGHT_STROKE: &str = TEXT_COLOR;

/// A label positioned in scene coordinates.
#[derive(Debug, Clone)]
pub struct GitLabel {
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub anchor: &'static str,
    pub baseline: &'static str,
    pub font_size: u32,
    pub font_weight: Option<&'static str>,
    pub color: &'static str,
}

/// Geometry for a whole git graph. `scene` holds the topology (commit
/// glyphs + connecting edges) in a renderer-agnostic form; the SVG
/// writer also uses `branch_labels`, `commit_ids`, and `tags`.
///
/// Each label's `color` is its lane's accent ink (the accent darkened
/// as needed to read on white), so a host drawing plain text gets
/// readable lane-coloured names and tags. The SVG writer draws a branch
/// name in white on a pill of that colour, with a dashed guide along
/// the lane — a start-anchored name begins a horizontal lane (LR), a
/// centred one heads a vertical lane (TB above the commits, BT below)
/// — and a tag on a white luggage-label pill outlined in it.
#[derive(Debug, Clone)]
pub struct GitScene {
    pub scene: Scene,
    pub branch_labels: Vec<GitLabel>,
    pub commit_ids: Vec<GitLabel>,
    pub tags: Vec<GitLabel>,
}

/// `(x0, y0, x1, y1)`.
type Rect = (f64, f64, f64, f64);
type Pt = (f64, f64);

/// Estimated width of `text` at `size` px.
fn text_w(text: &str, size: u32, bold: bool) -> f64 {
    let w = text_width(text) * size as f64 / TEXT_CALIBRATION;
    if bold {
        w * 1.06
    } else {
        w
    }
}

/// Box of a label's text, from its anchor and estimated width.
fn text_box(l: &GitLabel) -> Rect {
    let w = text_w(&l.text, l.font_size, l.font_weight == Some("bold"));
    let x0 = match l.anchor {
        "start" => l.x,
        "end" => l.x - w,
        _ => l.x - w / 2.0,
    };
    let h = l.font_size as f64 + 3.0;
    (x0, l.y - h / 2.0, x0 + w, l.y + h / 2.0)
}

/// The pill a branch name sits on.
fn name_pill(l: &GitLabel) -> Rect {
    let (x0, _, x1, _) = text_box(l);
    (
        x0 - PILL_PX,
        l.y - PILL_H / 2.0,
        x1 + PILL_PX,
        l.y + PILL_H / 2.0,
    )
}

/// The luggage-label pill a tag sits on (the hole at its left end).
fn tag_pill(l: &GitLabel) -> Rect {
    let (x0, _, x1, _) = text_box(l);
    (
        x0 - TAG_PX - TAG_HOLE,
        l.y - TAG_H / 2.0,
        x1 + TAG_PX,
        l.y + TAG_H / 2.0,
    )
}

/// Width of the pill [`tag_pill`] draws for `text`.
fn tag_pill_w(text: &str) -> f64 {
    text_w(text, TAG_FONT, false) + 2.0 * TAG_PX + TAG_HOLE
}

/// A tag label whose pill's left end is at `x0`, centred on `y`.
fn tag_label(text: &str, x0: f64, y: f64, lane: usize) -> GitLabel {
    let tw = text_w(text, TAG_FONT, false);
    GitLabel {
        x: x0 + TAG_HOLE + TAG_PX + tw / 2.0,
        y,
        text: text.to_string(),
        anchor: "middle",
        baseline: "middle",
        font_size: TAG_FONT,
        font_weight: None,
        color: accent_ink(lane),
    }
}

/// A commit id label.
fn id_label(text: &str, x: f64, y: f64, anchor: &'static str) -> GitLabel {
    GitLabel {
        x,
        y,
        text: text.to_string(),
        anchor,
        baseline: "middle",
        font_size: FONT,
        font_weight: None,
        color: TEXT_COLOR,
    }
}

/// Where an elbow's leg — the stretch straight across the lanes — is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leg {
    /// Same lane: a straight line.
    None,
    /// At the `from` commit; the run follows the `to` commit's lane.
    AtFrom,
    /// At the `to` commit; the run follows the `from` commit's lane.
    AtTo,
}

/// One drawn connection between two commits (by commit index).
#[derive(Debug, Clone, Copy)]
struct Link {
    from: usize,
    to: usize,
    leg: Leg,
    /// The lane the line runs along — its colour.
    lane: usize,
}

/// The lane-and-column skeleton every orientation lays out from.
struct Plan {
    /// Lane of every commit (by commit index).
    lane: Vec<usize>,
    /// Column (LR) / row (TB, BT) of every commit (by commit index).
    col: Vec<usize>,
    /// Commit index in each column.
    at_col: Vec<usize>,
    /// Lane of every branch (by branch index).
    branch_lane: Vec<usize>,
    lanes: usize,
    links: Vec<Link>,
    /// Commits whose own leg heads toward the higher lanes (below in
    /// LR, right in TB/BT) and toward the lower lanes.
    leg_high: Vec<bool>,
    leg_low: Vec<bool>,
    /// Leg spans `(low lane, high lane)` crossing each column.
    legs_at: Vec<Vec<(usize, usize)>>,
}

fn plan(d: &GitGraph) -> Plan {
    let n = d.commits.len();
    // Lanes rank the branches by `order` (ties by index), so a
    // hand-built graph with gaps or repeats still packs its lanes.
    let mut by_lane: Vec<usize> = (0..d.branches.len()).collect();
    by_lane.sort_by_key(|&i| (d.branches[i].order, i));
    let mut branch_lane = vec![0; d.branches.len()];
    for (lane, &i) in by_lane.iter().enumerate() {
        branch_lane[i] = lane;
    }
    // A commit naming a branch that does not exist (only a hand-built
    // graph can) gets a lane of its own past the real ones.
    let lane: Vec<usize> = d
        .commits
        .iter()
        .map(|c| branch_lane.get(c.branch).copied().unwrap_or(c.branch))
        .collect();
    let lanes = lane
        .iter()
        .map(|&l| l + 1)
        .max()
        .unwrap_or(0)
        .max(d.branches.len());

    let mut at_col: Vec<usize> = (0..n).collect();
    at_col.sort_by_key(|&i| (d.commits[i].seq, i));
    let mut col = vec![0; n];
    for (c, &i) in at_col.iter().enumerate() {
        col[i] = c;
    }

    // Whether lane `l` has a commit strictly between columns a and b.
    let busy = |l: usize, a: usize, b: usize| {
        let (lo, hi) = (a.min(b), a.max(b));
        (lo + 1..hi).any(|c| lane[at_col[c]] == l)
    };
    let link = |from: usize, to: usize| -> Link {
        let (lu, lv) = (lane[from], lane[to]);
        if lu == lv {
            return Link {
                from,
                to,
                leg: Leg::None,
                lane: lu,
            };
        }
        let (cu, cv) = (col[from], col[to]);
        // Mermaid's elbow runs along the lane farther from main; take
        // the other lane only when a commit sits on that stretch.
        let at_from = if lv > lu {
            !busy(lv, cu, cv) || busy(lu, cu, cv)
        } else {
            busy(lu, cu, cv) && !busy(lv, cu, cv)
        };
        Link {
            from,
            to,
            leg: if at_from { Leg::AtFrom } else { Leg::AtTo },
            lane: if at_from { lv } else { lu },
        }
    };

    // Same order as ever: per branch its fork then its line, then the
    // merges.
    let mut links = Vec::new();
    for (bi, b) in d.branches.iter().enumerate() {
        let mut on: Vec<usize> = (0..n).filter(|&i| d.commits[i].branch == bi).collect();
        on.sort_by_key(|&i| col[i]);
        if let (Some(p), Some(&first)) = (b.parent_commit.filter(|&p| p < n), on.first()) {
            links.push(link(p, first));
        }
        for w in on.windows(2) {
            links.push(link(w[0], w[1]));
        }
    }
    for (i, c) in d.commits.iter().enumerate() {
        if let Some(sp) = c.second_parent.filter(|&sp| sp < n) {
            links.push(link(sp, i));
        }
    }

    let mut leg_high = vec![false; n];
    let mut leg_low = vec![false; n];
    let mut legs_at = vec![Vec::new(); n];
    for l in &links {
        let (owner, other) = match l.leg {
            Leg::None => continue,
            Leg::AtFrom => (l.from, l.to),
            Leg::AtTo => (l.to, l.from),
        };
        let (a, b) = (lane[owner], lane[other]);
        if b > a {
            leg_high[owner] = true;
        } else {
            leg_low[owner] = true;
        }
        legs_at[col[owner]].push((a.min(b), a.max(b)));
    }

    Plan {
        lane,
        col,
        at_col,
        branch_lane,
        lanes,
        links,
        leg_high,
        leg_low,
        legs_at,
    }
}

/// Positions before the canvas is fitted around them.
struct Placement {
    /// Commit centres (by commit index).
    pos: Vec<Pt>,
    ids: Vec<GitLabel>,
    /// Tag of every commit that has one (by commit index).
    tags: Vec<Option<GitLabel>>,
    /// Branch names (by branch index).
    names: Vec<GitLabel>,
}

/// Bold width of each branch name's pill.
fn name_pill_ws(d: &GitGraph) -> Vec<f64> {
    d.branches
        .iter()
        .map(|b| text_w(&b.name, FONT, true) + 2.0 * PILL_PX)
        .collect()
}

fn name_label(text: &str, x: f64, y: f64, anchor: &'static str, lane: usize) -> GitLabel {
    GitLabel {
        x,
        y,
        text: text.to_string(),
        anchor,
        baseline: "middle",
        font_size: FONT,
        font_weight: Some("bold"),
        color: accent_ink(lane),
    }
}

/// LR: lanes are rows `BRANCH_DY` apart; columns sweep left to right,
/// each placed as far left as `COMMIT_DX` and its contents allow.
/// Every lane has three bands — its line (dots), the ids below it, the
/// tags above it — and each band remembers how far right it is taken;
/// a column's dot, id, tag and legs (a leg crosses the id bands of the
/// lanes it spans but the lowest, and the tag bands of all but the
/// highest) each need to start `GAP` past their band's mark.
fn layout_lr(d: &GitGraph, p: &Plan) -> Placement {
    let n = d.commits.len();
    let lane_y = |l: usize| l as f64 * BRANCH_DY;
    let pills = name_pill_ws(d);
    let column_start = PAD + pills.iter().fold(BRANCH_LABEL_W, |a, &w| a.max(w));
    let names = d
        .branches
        .iter()
        .enumerate()
        .map(|(bi, b)| {
            let lane = p.branch_lane[bi];
            name_label(&b.name, PAD + PILL_PX, lane_y(lane), "start", lane)
        })
        .collect();

    let (dot, id, tag) = (|l: usize| 3 * l, |l: usize| 3 * l + 1, |l: usize| 3 * l + 2);
    let mut reach = vec![column_start; 3 * p.lanes];
    let mut pos = vec![(0.0, 0.0); n];
    let mut ids: Vec<Option<GitLabel>> = vec![None; n];
    let mut tags = vec![None; n];
    let mut prev_x = f64::NEG_INFINITY;
    for &i in &p.at_col {
        let c = &d.commits[i];
        let l = p.lane[i];
        let mut items: Vec<(usize, f64, f64)> = vec![(dot(l), -R, R)];
        let id_w = text_w(&c.id, FONT, false);
        let id_at = if p.leg_high[i] {
            LEG_CLEAR
        } else {
            -id_w / 2.0
        };
        items.push((id(l), id_at, id_at + id_w));
        let tag_at = c.tag.as_deref().map(|t| {
            let w = tag_pill_w(t);
            let at = if p.leg_low[i] { LEG_CLEAR } else { -w / 2.0 };
            items.push((tag(l), at, at + w));
            at
        });
        for &(lo, hi) in &p.legs_at[p.col[i]] {
            let half = EDGE_W / 2.0;
            items.extend((lo..hi).map(|k| (id(k), -half, half)));
            items.extend((lo + 1..=hi).map(|k| (tag(k), -half, half)));
        }
        let x = items
            .iter()
            .fold(prev_x + COMMIT_DX, |x, &(band, at, _)| {
                x.max(reach[band] + GAP - at)
            });
        for &(band, _, end) in &items {
            reach[band] = reach[band].max(x + end);
        }
        prev_x = x;
        let y = lane_y(l);
        pos[i] = (x, y);
        let anchor = if p.leg_high[i] { "start" } else { "middle" };
        let id_x = if p.leg_high[i] { x + LEG_CLEAR } else { x };
        ids[i] = Some(id_label(&c.id, id_x, y + ID_DY, anchor));
        if let (Some(t), Some(at)) = (c.tag.as_deref(), tag_at) {
            tags[i] = Some(tag_label(t, x + at, y - R - SIDE_GAP - TAG_H / 2.0, l));
        }
    }
    Placement {
        pos,
        ids: ids.into_iter().map(|l| l.expect("every column placed")).collect(),
        tags,
        names,
    }
}

/// TB / BT: rows `COMMIT_DX` apart (an id or tag beside a dot is one
/// text line tall, so rows never need more); lanes are columns, each
/// at least `BRANCH_DY` past the previous and far enough for the ids
/// left of its dots and the tags right of the previous lane's dots to
/// stop short of the neighbouring lane line. BT is TB flipped upside
/// down.
fn layout_tb(d: &GitGraph, p: &Plan, flip: bool) -> Placement {
    let n = d.commits.len();
    let row_y = |i: usize| p.col[i] as f64 * COMMIT_DX;
    let lane_clear = EDGE_W / 2.0 + GAP;
    let pills = name_pill_ws(d);
    let mut id_room = vec![0.0f64; p.lanes];
    let mut tag_room = vec![0.0f64; p.lanes];
    let mut pill_room = vec![0.0f64; p.lanes];
    for (i, c) in d.commits.iter().enumerate() {
        let l = p.lane[i];
        id_room[l] = id_room[l].max(R + SIDE_GAP + text_w(&c.id, FONT, false));
        if let Some(t) = &c.tag {
            tag_room[l] = tag_room[l].max(R + SIDE_GAP + tag_pill_w(t));
        }
    }
    for (bi, &w) in pills.iter().enumerate() {
        let l = p.branch_lane[bi];
        pill_room[l] = pill_room[l].max(w / 2.0);
    }
    let mut lane_x = vec![0.0f64; p.lanes];
    for l in 1..p.lanes {
        let pitch = BRANCH_DY
            .max(id_room[l] + lane_clear)
            .max(tag_room[l - 1] + lane_clear)
            .max(pill_room[l - 1] + pill_room[l] + GAP);
        lane_x[l] = lane_x[l - 1] + pitch;
    }

    // An id or tag its own commit's leg would cross moves up off it.
    let id_shift = TEXT_H / 2.0 + EDGE_W / 2.0 + 2.0;
    let tag_shift = TAG_H / 2.0 + EDGE_W / 2.0 + 2.0;
    let mut pos = vec![(0.0, 0.0); n];
    let mut ids = Vec::with_capacity(n);
    let mut tags = vec![None; n];
    let mut top = -R;
    for (i, c) in d.commits.iter().enumerate() {
        let (x, y) = (lane_x[p.lane[i]], row_y(i));
        pos[i] = (x, y);
        let id_y = if p.leg_low[i] { y - id_shift } else { y };
        ids.push(id_label(&c.id, x - R - SIDE_GAP, id_y, "end"));
        top = top.min(id_y - TEXT_H / 2.0);
        if let Some(t) = &c.tag {
            let tag_y = if p.leg_high[i] { y - tag_shift } else { y };
            tags[i] = Some(tag_label(t, x + R + SIDE_GAP, tag_y, p.lane[i]));
            top = top.min(tag_y - TAG_H / 2.0);
        }
    }
    let name_y = top.min(-R) - GAP - PILL_H / 2.0;
    let mut names: Vec<GitLabel> = d
        .branches
        .iter()
        .enumerate()
        .map(|(bi, b)| {
            let lane = p.branch_lane[bi];
            name_label(&b.name, lane_x[lane], name_y, "middle", lane)
        })
        .collect();
    if flip {
        for q in &mut pos {
            q.1 = -q.1;
        }
        for l in ids
            .iter_mut()
            .chain(tags.iter_mut().flatten())
            .chain(names.iter_mut())
        {
            l.y = -l.y;
        }
    }
    Placement {
        pos,
        ids,
        tags,
        names,
    }
}

/// The corner of the elbow from `p0` to `p1` with its leg at `leg`, or
/// `None` for a straight line.
fn elbow_corner(p0: Pt, p1: Pt, leg: Leg, o: GitOrientation) -> Option<Pt> {
    let lr = o == GitOrientation::LR;
    match leg {
        Leg::None => None,
        // The leg is at the start: cross the lanes first.
        Leg::AtFrom if lr => Some((p0.0, p1.1)),
        Leg::AtFrom => Some((p1.0, p0.1)),
        // The leg is at the end: run along the start's lane first.
        Leg::AtTo if lr => Some((p1.0, p0.1)),
        Leg::AtTo => Some((p0.0, p1.1)),
    }
}

/// The rounded corner of the elbow `p0 → c → p1`: where the arc leaves
/// the first leg, where it joins the second, its radius, and the SVG
/// sweep flag.
fn elbow_arc(p0: Pt, c: Pt, p1: Pt) -> (Pt, Pt, f64, u8) {
    let (d1, d2) = (dist(p0, c), dist(c, p1));
    let r = ELBOW_R.min(d1).min(d2);
    let along = |from: Pt, len: f64| {
        if len == 0.0 {
            c
        } else {
            (c.0 + (from.0 - c.0) / len * r, c.1 + (from.1 - c.1) / len * r)
        }
    };
    let (a, b) = (along(p0, d1), along(p1, d2));
    // Screen coordinates (y down): a positive cross product is a
    // clockwise turn, SVG's sweep-flag 1.
    let cross = (c.0 - p0.0) * (p1.1 - c.1) - (c.1 - p0.1) * (p1.0 - c.0);
    (a, b, r, u8::from(cross > 0.0))
}

fn dist(a: Pt, b: Pt) -> f64 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

/// The elbow `p0 → c → p1` as a dense polyline: the straight legs and
/// the rounded corner sampled finely enough that a host splining
/// through the points (see [`SceneEdge::waypoints`]) draws the same
/// shape the SVG writer's arc does.
fn elbow_points(p0: Pt, c: Pt, p1: Pt) -> Vec<Pt> {
    const STEPS: usize = 8;
    let (a, b, r, _) = elbow_arc(p0, c, p1);
    let mut out = vec![p0];
    if r > 0.0 {
        let (u1, u2) = (
            ((a.0 - c.0) / r, (a.1 - c.1) / r),
            ((b.0 - c.0) / r, (b.1 - c.1) / r),
        );
        // Centre of the corner circle; the arc runs from `a` (angle 0)
        // to `b` (a quarter turn).
        let o = (a.0 + b.0 - c.0, a.1 + b.1 - c.1);
        for k in 0..=STEPS {
            let t = k as f64 / STEPS as f64 * std::f64::consts::FRAC_PI_2;
            out.push((
                o.0 - r * (u2.0 * t.cos() + u1.0 * t.sin()),
                o.1 - r * (u2.1 * t.cos() + u1.1 * t.sin()),
            ));
        }
    } else {
        out.push(c);
    }
    out.push(p1);
    out.dedup();
    out
}

/// Compute all git graph geometry.
pub fn scene(d: &GitGraph) -> GitScene {
    let p = plan(d);
    let mut pl = match d.orientation {
        GitOrientation::LR => layout_lr(d, &p),
        GitOrientation::TB => layout_tb(d, &p, false),
        GitOrientation::BT => layout_tb(d, &p, true),
    };

    // Fit the canvas around everything drawn: dots, ids, tag and name
    // pills (lines run between dots, so the dots bound them too).
    let mut bb = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    let mut grow = |r: Rect| {
        bb = (bb.0.min(r.0), bb.1.min(r.1), bb.2.max(r.2), bb.3.max(r.3));
    };
    for &(x, y) in &pl.pos {
        grow((x - R, y - R, x + R, y + R));
    }
    pl.ids.iter().for_each(|l| grow(text_box(l)));
    pl.tags.iter().flatten().for_each(|l| grow(tag_pill(l)));
    pl.names.iter().for_each(|l| grow(name_pill(l)));
    if !bb.0.is_finite() {
        bb = (0.0, 0.0, 0.0, 0.0);
    }
    let (dx, dy) = (PAD - bb.0, PAD - bb.1);
    let width = bb.2 - bb.0 + 2.0 * PAD;
    let height = bb.3 - bb.1 + 2.0 * PAD;
    for q in &mut pl.pos {
        *q = (q.0 + dx, q.1 + dy);
    }
    for l in pl
        .ids
        .iter_mut()
        .chain(pl.tags.iter_mut().flatten())
        .chain(pl.names.iter_mut())
    {
        l.x += dx;
        l.y += dy;
    }

    let nodes = d
        .commits
        .iter()
        .enumerate()
        .map(|(i, c)| commit_node(c, pl.pos[i], p.lane[i]))
        .collect();
    let edges = p
        .links
        .iter()
        .map(|l| {
            let (p0, p1) = (pl.pos[l.from], pl.pos[l.to]);
            let (bezier, waypoints) = match elbow_corner(p0, p1, l.leg, d.orientation) {
                Some(c) => ([p0, c, c, p1], elbow_points(p0, c, p1)),
                None => ([p0, p0, p1, p1], Vec::new()),
            };
            let mut meta = EdgeMeta::default();
            meta.style.stroke = Some(accent(l.lane).to_string());
            meta.style.stroke_width = Some(EDGE_W);
            SceneEdge {
                from: d.commits[l.from].id.clone(),
                to: d.commits[l.to].id.clone(),
                bezier,
                waypoints,
                // No arrowheads, as in mermaid: the lane colour and the
                // elbow's shape say which way a merge goes.
                kind: EdgeKind::Open,
                ends: Default::default(),
                meta,
                label: None,
            }
        })
        .collect();

    GitScene {
        scene: Scene {
            nodes,
            edges,
            clusters: Vec::new(),
            width,
            height,
        },
        branch_labels: pl.names,
        commit_ids: pl.ids,
        tags: pl.tags.into_iter().flatten().collect(),
    }
}

/// A commit's scene node: its glyph as the shape, in its lane's colour.
/// The commit type decides the glyph before merge-ness does, as in
/// mermaid: a HIGHLIGHT is a square and a REVERSE a crossed-out disc
/// even on a merge commit; a plain merge is a ring around a light core.
fn commit_node(c: &crate::model::GitCommit, (x, y): Pt, lane: usize) -> SceneNode {
    let color = accent(lane);
    let merge = c.second_parent.is_some();
    let (shape, side, stroke, label, ink) = match c.kind {
        CommitKind::Highlight => (Shape::Rect, 2.0 * R - 2.0, HIGHLIGHT_STROKE, "", None),
        CommitKind::Reverse => (
            Shape::Circle,
            2.0 * R,
            color,
            REVERSE_MARK,
            Some(text_on(color)),
        ),
        CommitKind::Normal if merge => (Shape::DoubleCircle, 2.0 * R, color, "", None),
        CommitKind::Normal => (Shape::Circle, 2.0 * R, color, "", None),
    };
    // Commit ids / tags are drawn from the GitScene labels so their
    // placement can differ per orientation; only the REVERSE mark
    // rides on the node.
    SceneNode {
        id: c.id.clone(),
        x,
        y,
        w: side,
        h: side,
        shape,
        label: label.to_string(),
        style: NodeStyle {
            fill: Some(color.to_string()),
            stroke: Some(stroke.to_string()),
            stroke_width: Some(2.0),
            color: ink.map(str::to_string),
        },
        link: None,
    }
}

/// Serialise a git graph scene to SVG.
pub fn to_svg(gs: &GitScene) -> String {
    to_svg_with(gs, &SvgOptions::default())
}

/// [`to_svg`] with explicit viewport options (see [`SvgOptions`]).
pub fn to_svg_with(gs: &GitScene, opts: &SvgOptions) -> String {
    let (w, h) = (gs.scene.width, gs.scene.height);
    let mut s = String::new();
    svg_open(&mut s, w, h, FONT, "Git graph diagram", opts);

    // Lane guides, then the branch-name pills that head them.
    for l in &gs.branch_labels {
        let (x0, y0, x1, y1) = name_pill(l);
        let (gx0, gy0, gx1, gy1) = if l.anchor == "start" {
            (x1 + 4.0, l.y, w - PAD, l.y)
        } else if l.y < h / 2.0 {
            (l.x, y1 + 4.0, l.x, h - PAD)
        } else {
            (l.x, PAD, l.x, y0 - 4.0)
        };
        if gx1 > gx0 || gy1 > gy0 {
            s.push_str(&format!(
                "<line x1=\"{gx0:.1}\" y1=\"{gy0:.1}\" x2=\"{gx1:.1}\" y2=\"{gy1:.1}\" \
                 stroke=\"{GUIDE_COLOR}\" stroke-width=\"1\" stroke-dasharray=\"3 4\"/>\n"
            ));
        }
    }
    for l in &gs.branch_labels {
        let (x0, y0, x1, y1) = name_pill(l);
        s.push_str(&format!(
            "<rect x=\"{x0:.1}\" y=\"{y0:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"5\" \
             fill=\"{}\"/>\n",
            x1 - x0,
            y1 - y0,
            escape(l.color)
        ));
        write_label(&mut s, l, PILL_TEXT, None);
    }

    // Branch lines, forks and merges.
    for e in &gs.scene.edges {
        let stroke = style_attr(e.meta.style.stroke.as_deref(), EDGE_COLOR);
        let sw = e.meta.style.stroke_width.unwrap_or(EDGE_W);
        s.push_str(&format!(
            "<path d=\"{}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"{sw:.1}\" \
             stroke-linecap=\"round\" stroke-linejoin=\"round\"/>\n",
            edge_d(e)
        ));
    }

    // Commit glyphs.
    for n in &gs.scene.nodes {
        let fill = style_attr(n.style.fill.as_deref(), accent(0));
        let stroke = style_attr(n.style.stroke.as_deref(), EDGE_COLOR);
        let sw = n.style.stroke_width.unwrap_or(2.0);
        let r = n.w.min(n.h) / 2.0;
        match n.shape {
            Shape::Rect => {
                s.push_str(&format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"3\" \
                     fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw:.1}\"/>\n",
                    n.x - n.w / 2.0,
                    n.y - n.h / 2.0,
                    n.w,
                    n.h
                ));
            }
            _ => {
                s.push_str(&format!(
                    "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{r:.1}\" \
                     fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{sw:.1}\"/>\n",
                    n.x, n.y
                ));
            }
        }
        if n.shape == Shape::DoubleCircle {
            s.push_str(&format!(
                "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{HOLE_FILL}\"/>\n",
                n.x,
                n.y,
                r * 0.45
            ));
        }
        if n.label == REVERSE_MARK {
            let k = r * 0.45;
            let ink = style_attr(n.style.color.as_deref(), HOLE_FILL);
            s.push_str(&format!(
                "<path d=\"M {:.1} {:.1} L {:.1} {:.1} M {:.1} {:.1} L {:.1} {:.1}\" \
                 stroke=\"{ink}\" stroke-width=\"2.5\" stroke-linecap=\"round\"/>\n",
                n.x - k,
                n.y - k,
                n.x + k,
                n.y + k,
                n.x - k,
                n.y + k,
                n.x + k,
                n.y - k
            ));
        }
    }

    // Commit ids, haloed, and tags on their luggage-label pills.
    for l in &gs.commit_ids {
        write_label(&mut s, l, l.color, Some(HALO));
    }
    for l in &gs.tags {
        let (x0, y0, x1, y1) = tag_pill(l);
        let ink = escape(l.color);
        s.push_str(&format!(
            "<rect x=\"{x0:.1}\" y=\"{y0:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"{:.1}\" \
             fill=\"{TAG_FILL}\" stroke=\"{}\" stroke-width=\"1.2\"/>\n\
             <circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"1.6\" fill=\"{}\"/>\n",
            x1 - x0,
            y1 - y0,
            TAG_H / 2.0,
            ink,
            x0 + TAG_HOLE / 2.0 + 3.0,
            l.y,
            ink
        ));
        write_label(&mut s, l, l.color, None);
    }

    s.push_str("</svg>\n");
    s
}

/// SVG path data for one edge: an elbow (both bezier controls on its
/// corner) as two straight legs and a true circular arc; a straight
/// line as a line; anything an embedder put in its place through the
/// shared edge painter.
fn edge_d(e: &SceneEdge) -> String {
    let [p0, c1, c2, p1] = e.bezier;
    let axis = |a: Pt, b: Pt| a.0 == b.0 || a.1 == b.1;
    if c1 == c2 && c1 != p0 && c1 != p1 && axis(p0, c1) && axis(c1, p1) {
        let (a, b, r, sweep) = elbow_arc(p0, c1, p1);
        format!(
            "M {:.1} {:.1} L {:.1} {:.1} A {r:.1} {r:.1} 0 0 {sweep} {:.1} {:.1} L {:.1} {:.1}",
            p0.0, p0.1, a.0, a.1, b.0, b.1, p1.0, p1.1
        )
    } else if c1 == p0 && c2 == p1 && e.waypoints.is_empty() {
        format!("M {:.1} {:.1} L {:.1} {:.1}", p0.0, p0.1, p1.0, p1.1)
    } else {
        edge_path_d(e, |q| q)
    }
}

/// One label's `<text>`, in `fill` (its own colour, or white on a
/// pill), with an optional knock-out `halo` behind the glyphs.
fn write_label(s: &mut String, l: &GitLabel, fill: &str, halo: Option<&str>) {
    let weight = l.font_weight.unwrap_or("normal");
    let halo = halo.map_or(String::new(), |c| {
        format!(" stroke=\"{c}\" stroke-width=\"3\" stroke-linejoin=\"round\" paint-order=\"stroke\"")
    });
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"{}\" \
         dominant-baseline=\"{}\" font-size=\"{}\" font-weight=\"{}\" \
         fill=\"{}\"{halo}>{}</text>\n",
        l.x,
        l.y,
        l.anchor,
        l.baseline,
        l.font_size,
        weight,
        escape(fill),
        escape(&l.text)
    ));
}
