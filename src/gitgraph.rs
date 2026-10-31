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
    edge_path_d, escape, style_attr, svg_open, Scene, SceneEdge, SceneNode, SvgOptions, EDGE_COLOR,
    TEXT_COLOR,
};
use crate::style::{accent, accent_ink, text_on, ACCENTS, ACCENT_INKS};

/// Canvas margin around the drawn content.
pub const PAD: f64 = 24.0;
/// Minimum width of the branch-name column left of the first commit in
/// LR; a longer name's pill widens it. (0.32 and earlier: a fixed
/// width, and TB/BT reserved the same room for the names, where the
/// canvas is now fitted round them instead.)
pub const BRANCH_LABEL_W: f64 = 70.0;
/// Minimum distance between neighbouring commit columns (LR) / rows
/// (TB, BT). In LR a column moves further right whenever the ids, tags
/// and elbow legs around it need the room to stay clear of each other.
/// (0.32 and earlier: a fixed 90px pitch.)
pub const COMMIT_DX: f64 = 50.0;
/// Distance between branch lanes in LR; the minimum in TB/BT, where a
/// lane moves over as far as the ids and tags beside it need. (0.32 and
/// earlier: a fixed pitch in every orientation.)
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
/// Clear space kept between a label or tag and a line or dot.
const GAP: f64 = 8.0;
/// Clear space between two neighbouring ids / tags, wide enough that
/// two ids never read as one phrase.
const TEXT_GAP: f64 = 14.0;
/// Centre-to-centre spacing of legs that leave one commit on the same
/// side (they run side by side instead of on top of each other).
const LEG_PITCH: f64 = EDGE_W + 1.5;
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
/// Each name's and tag's `color` is its lane's accent ink (the accent
/// darkened as needed to read on white), so a host drawing plain text
/// gets readable lane-coloured names and tags. The SVG writer draws a
/// branch name on a pill of the lane's accent (in white or dark text,
/// whichever reads on it), with a dashed guide along the lane — a
/// start-anchored name begins a horizontal lane (LR), a centred one
/// heads a vertical lane (TB above the commits, BT below) — and a tag
/// on a white luggage-label pill outlined in its colour.
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
    /// How far the leg sits from its commit's centre along the commit
    /// flow (toward later commits is positive; TB frame for BT), when
    /// it shares the commit's side with other legs.
    off: f64,
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
    /// Per commit, the (least, greatest) `Link::off` of its legs toward
    /// the higher lanes, and toward the lower lanes.
    offs_high: Vec<(f64, f64)>,
    offs_low: Vec<(f64, f64)>,
    /// Leg spans `(low lane, high lane, off)` crossing each column.
    legs_at: Vec<Vec<(usize, usize, f64)>>,
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
                off: 0.0,
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
            off: 0.0,
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

    // Group the legs by (commit, side). Legs that share one run side
    // by side, ordered so none crosses another where it turns: those
    // whose run comes from earlier commits (turning back) first, the
    // longer one outside; then those running on to later commits, the
    // longer one outside again.
    let owner_of = |l: &Link| match l.leg {
        Leg::None => None,
        Leg::AtFrom => Some((l.from, l.to)),
        Leg::AtTo => Some((l.to, l.from)),
    };
    let mut groups: Vec<(usize, bool, usize)> = links
        .iter()
        .enumerate()
        .filter_map(|(k, l)| owner_of(l).map(|(o, t)| (o, lane[t] > lane[o], k)))
        .collect();
    groups.sort_by_key(|&(o, high, k)| {
        let l = &links[k];
        let reach = lane[l.from].abs_diff(lane[l.to]) as isize;
        let order = match l.leg {
            Leg::AtTo => (0, reach),
            _ => (1, -reach),
        };
        (o, high, order, k)
    });
    let mut start = 0;
    while start < groups.len() {
        let (o, high, _) = groups[start];
        let k = groups[start..]
            .iter()
            .take_while(|&&(o2, h2, _)| (o2, h2) == (o, high))
            .count();
        if k > 1 {
            // Stay inside the dot, however many share it.
            let pitch = LEG_PITCH.min((2.0 * R - EDGE_W) / (k - 1) as f64);
            for (j, &(_, _, li)) in groups[start..start + k].iter().enumerate() {
                links[li].off = (j as f64 - (k - 1) as f64 / 2.0) * pitch;
            }
        }
        start += k;
    }

    let mut leg_high = vec![false; n];
    let mut leg_low = vec![false; n];
    let mut offs_high = vec![(0.0f64, 0.0f64); n];
    let mut offs_low = vec![(0.0f64, 0.0f64); n];
    let mut legs_at = vec![Vec::new(); n];
    for l in &links {
        let Some((owner, other)) = owner_of(l) else {
            continue;
        };
        let (a, b) = (lane[owner], lane[other]);
        let (flag, offs) = if b > a {
            (&mut leg_high[owner], &mut offs_high[owner])
        } else {
            (&mut leg_low[owner], &mut offs_low[owner])
        };
        *flag = true;
        *offs = (offs.0.min(l.off), offs.1.max(l.off));
        legs_at[col[owner]].push((a.min(b), a.max(b), l.off));
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
        offs_high,
        offs_low,
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
/// tags above it — and each band remembers how far right it is taken
/// (and whether by text); a column's dot, id, tag and legs (a leg
/// crosses the id bands of the lanes it spans but the lowest, and the
/// tag bands of all but the highest) each need to start past their
/// band's mark, by `TEXT_GAP` between two texts and `GAP` otherwise.
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
    // (rightmost end, whether that end is text) per band.
    let mut reach = vec![(column_start, false); 3 * p.lanes];
    let mut pos = vec![(0.0, 0.0); n];
    let mut ids: Vec<Option<GitLabel>> = vec![None; n];
    let mut tags = vec![None; n];
    let mut prev_x = f64::NEG_INFINITY;
    for &i in &p.at_col {
        let c = &d.commits[i];
        let l = p.lane[i];
        // (band, start, end relative to the column, is text).
        let mut items: Vec<(usize, f64, f64, bool)> = vec![(dot(l), -R, R, false)];
        let id_w = text_w(&c.id, FONT, false);
        // An id its own leg would cross starts right of the leg(s);
        // likewise a tag.
        let id_at = if p.leg_high[i] {
            p.offs_high[i].1 + LEG_CLEAR
        } else {
            -id_w / 2.0
        };
        items.push((id(l), id_at, id_at + id_w, true));
        let tag_at = c.tag.as_deref().map(|t| {
            let w = tag_pill_w(t);
            let at = if p.leg_low[i] {
                p.offs_low[i].1 + LEG_CLEAR
            } else {
                -w / 2.0
            };
            items.push((tag(l), at, at + w, true));
            at
        });
        for &(lo, hi, off) in &p.legs_at[p.col[i]] {
            let (a, b) = (off - EDGE_W / 2.0, off + EDGE_W / 2.0);
            items.extend((lo..hi).map(|k| (id(k), a, b, false)));
            items.extend((lo + 1..=hi).map(|k| (tag(k), a, b, false)));
        }
        let x = items
            .iter()
            .fold(prev_x + COMMIT_DX, |x, &(band, at, _, text)| {
                let (end, end_text) = reach[band];
                let gap = if text && end_text { TEXT_GAP } else { GAP };
                x.max(end + gap - at)
            });
        for &(band, _, end, text) in &items {
            if x + end > reach[band].0 {
                reach[band] = (x + end, text);
            }
        }
        prev_x = x;
        let y = lane_y(l);
        pos[i] = (x, y);
        let anchor = if p.leg_high[i] { "start" } else { "middle" };
        let id_x = if p.leg_high[i] { x + id_at } else { x };
        ids[i] = Some(id_label(&c.id, id_x, y + ID_DY, anchor));
        if let (Some(t), Some(at)) = (c.tag.as_deref(), tag_at) {
            tags[i] = Some(tag_label(t, x + at, y - R - SIDE_GAP - TAG_H / 2.0, l));
        }
    }
    Placement {
        pos,
        ids: ids
            .into_iter()
            .map(|l| l.expect("every column placed"))
            .collect(),
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

    // An id or tag its own commit's leg(s) would cross moves up, clear
    // of the topmost of them.
    let clear = |offs: (f64, f64)| (-offs.0).max(0.0) + EDGE_W / 2.0 + 2.0;
    let mut pos = vec![(0.0, 0.0); n];
    let mut ids = Vec::with_capacity(n);
    let mut tags = vec![None; n];
    let mut top = -R;
    for (i, c) in d.commits.iter().enumerate() {
        let (x, y) = (lane_x[p.lane[i]], row_y(i));
        pos[i] = (x, y);
        let id_y = if p.leg_low[i] {
            y - TEXT_H / 2.0 - clear(p.offs_low[i])
        } else {
            y
        };
        ids.push(id_label(&c.id, x - R - SIDE_GAP, id_y, "end"));
        top = top.min(id_y - TEXT_H / 2.0);
        if let Some(t) = &c.tag {
            let tag_y = if p.leg_high[i] {
                y - TAG_H / 2.0 - clear(p.offs_high[i])
            } else {
                y
            };
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
            (
                c.0 + (from.0 - c.0) / len * r,
                c.1 + (from.1 - c.1) / len * r,
            )
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
            let (mut p0, mut p1) = (pl.pos[l.from], pl.pos[l.to]);
            // A leg sharing its commit's side starts off-centre (still
            // under the dot), along the commit flow.
            let shift = |q: &mut Pt| match d.orientation {
                GitOrientation::LR => q.0 += l.off,
                GitOrientation::TB => q.1 += l.off,
                GitOrientation::BT => q.1 -= l.off,
            };
            match l.leg {
                Leg::AtFrom => shift(&mut p0),
                Leg::AtTo => shift(&mut p1),
                Leg::None => {}
            }
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
        let (_, y0, x1, y1) = name_pill(l);
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
        let fill = pill_fill(l.color);
        s.push_str(&format!(
            "<rect x=\"{x0:.1}\" y=\"{y0:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"5\" \
             fill=\"{}\"/>\n",
            x1 - x0,
            y1 - y0,
            escape(fill)
        ));
        write_label(&mut s, l, text_on(fill), None);
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

/// The pill colour for a branch name inked in `ink`: the palette accent
/// that ink darkens (the lane's line colour), or the ink itself when it
/// is not a palette ink.
fn pill_fill(ink: &str) -> &str {
    ACCENT_INKS
        .iter()
        .position(|&c| c == ink)
        .map_or(ink, |i| ACCENTS[i])
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
        format!(
            " stroke=\"{c}\" stroke-width=\"3\" stroke-linejoin=\"round\" paint-order=\"stroke\""
        )
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
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Document;
    use crate::parser::parse_document;

    fn git(src: &str) -> GitGraph {
        match parse_document(src).unwrap() {
            Document::GitGraph(g) => g,
            other => panic!("expected gitGraph, got {:?}", other),
        }
    }

    #[test]
    fn linear_commits_on_main() {
        let g = git("gitGraph\ncommit\ncommit\ncommit");
        assert_eq!(g.branches.len(), 1);
        assert_eq!(g.branches[0].name, "main");
        assert_eq!(g.commits.len(), 3);
        assert_eq!(g.commits[0].parent, None);
        assert_eq!(g.commits[1].parent, Some(0));
        assert_eq!(g.commits[2].parent, Some(1));
        let sc = scene(&g);
        assert_eq!(sc.scene.nodes.len(), 3);
        assert_eq!(sc.scene.edges.len(), 2);
    }

    #[test]
    fn branch_checkout_merge() {
        let g = git("gitGraph\n\
             commit id: \"init\"\n\
             branch feat\n\
             checkout feat\n\
             commit id: \"a\"\n\
             checkout main\n\
             commit id: \"b\"\n\
             merge feat id: \"m\" tag: \"v1\"");
        assert_eq!(g.branches.len(), 2);
        assert_eq!(g.commits.len(), 4);
        let merge = g.commits.last().unwrap();
        assert_eq!(merge.id, "m");
        assert!(merge.second_parent.is_some());
        assert_eq!(merge.tag.as_deref(), Some("v1"));
        let sc = scene(&g);
        // 3 branch-line edges + 1 merge curve.
        assert_eq!(sc.scene.edges.len(), 4);
        let svg = to_svg(&sc);
        assert!(svg.contains(">m</text>"));
        assert!(svg.contains(">v1</text>"));
        assert!(svg.contains(">feat</text>"));
    }

    #[test]
    fn switch_alias_for_checkout() {
        let g = git("gitGraph\n\
             commit id: \"init\"\n\
             branch feat\n\
             switch feat\n\
             commit id: \"a\"\n\
             switch main\n\
             commit id: \"b\"");
        assert_eq!(g.branches.len(), 2);
        assert_eq!(g.current_branch, 0);
        assert_eq!(g.commits[1].id, "a");
        assert_eq!(g.commits[1].branch, 1);
    }
    #[test]
    fn commit_kinds() {
        let g = git("gitGraph\n\
             commit type: NORMAL\n\
             commit type: REVERSE\n\
             commit type: HIGHLIGHT");
        assert_eq!(g.commits[0].kind, CommitKind::Normal);
        assert_eq!(g.commits[1].kind, CommitKind::Reverse);
        assert_eq!(g.commits[2].kind, CommitKind::Highlight);
    }

    #[test]
    fn default_orientation_is_lr() {
        let g = git("gitGraph\ncommit");
        assert_eq!(g.orientation, GitOrientation::LR);
        let sc = scene(&g);
        assert!(sc.scene.width > sc.scene.height);
    }

    #[test]
    fn tb_orientation_swaps_axes() {
        let g = git("gitGraph TB\ncommit\ncommit");
        assert_eq!(g.orientation, GitOrientation::TB);
        let sc = scene(&g);
        assert!(sc.scene.height > sc.scene.width);
        // Branch label sits above the lane.
        let main_label = sc.branch_labels.iter().find(|l| l.text == "main").unwrap();
        assert_eq!(main_label.anchor, "middle");
        assert!(main_label.y < sc.scene.nodes[0].y);
    }

    #[test]
    fn bt_orientation_flows_upward() {
        let g = git("gitGraph BT\ncommit id: \"first\"\ncommit id: \"second\"");
        assert_eq!(g.orientation, GitOrientation::BT);
        let sc = scene(&g);
        let first = sc.scene.nodes.iter().find(|n| n.id == "first").unwrap();
        let second = sc.scene.nodes.iter().find(|n| n.id == "second").unwrap();
        // In BT, sequence grows upward: second is above first.
        assert!(second.y < first.y);
        let main_label = sc.branch_labels.iter().find(|l| l.text == "main").unwrap();
        assert!(main_label.y > first.y);
    }

    #[test]
    fn unknown_orientation_is_rejected() {
        let e = parse_document("gitGraph XX\ncommit").unwrap_err();
        assert!(
            e.message.contains("unknown gitGraph orientation"),
            "got: {}",
            e.message
        );
    }

    #[test]
    fn a_commit_colour_is_escaped_on_its_way_into_the_attribute() {
        // gitGraph fills NodeStyle from the palette, so only an embedder
        // driving the public scene API can put text here — which is the
        // path the render-site escaping exists to cover, and the reason
        // this call site needs a test of its own.
        let d = match crate::parser::parse_document("gitGraph\ncommit\n").unwrap() {
            crate::model::Document::GitGraph(g) => g,
            _ => panic!("not a gitGraph"),
        };
        let mut gs = scene(&d);
        gs.scene.nodes[0].style.fill = Some("x\" onload=1".to_string());
        gs.scene.nodes[0].style.stroke = Some("y' onload=1".to_string());
        let svg = to_svg(&gs);
        assert!(
            !svg.contains("\" onload") && !svg.contains("' onload"),
            "a commit colour escaped its attribute:\n{svg}"
        );
        assert!(svg.contains("&quot;"), "{svg}");
    }

    /// Report Q5: lane names (and tags) were written in the raw accent
    /// on the white canvas — amber 2.6:1, green 3.2:1. Names now sit
    /// on a pill of the lane's accent (Q12) in whichever text reads on
    /// it; tags keep the lane ink on their white pill.
    #[test]
    fn lane_names_and_tags_read_on_white() {
        let n = crate::style::ACCENTS.len();
        let mut src = String::from("gitGraph\ncommit id: \"c0\" tag: \"t0\"\n");
        for i in 1..n {
            src.push_str(&format!(
                "branch lane{i}\ncheckout lane{i}\ncommit id: \"c{i}\" tag: \"t{i}\"\n"
            ));
        }
        let gs = scene(&git(&src));
        let svg = to_svg(&gs);
        for i in 0..n {
            let lane = if i == 0 {
                "main".to_string()
            } else {
                format!("lane{i}")
            };
            let pill = crate::style::accent(i);
            assert!(svg.contains(&format!("fill=\"{pill}\"/>")), "{lane} pill");
            let ink = crate::style::assert_text_reads(&svg, &lane, pill);
            assert_eq!(ink, crate::style::text_on(pill), "{lane}");
            // A host drawing the name as plain text gets the lane ink.
            assert_eq!(gs.branch_labels[i].color, crate::style::accent_ink(i));
            let ink = crate::style::assert_text_reads(&svg, &format!("t{i}"), TAG_FILL);
            assert_eq!(ink, crate::style::accent_ink(i), "tag t{i}");
        }
        // The commit dots keep the full-strength accent.
        assert!(svg.contains(&format!("fill=\"{}\"", crate::style::accent(2))));
    }

    // ── Report Q12 / P0 #15 / P1: the gitGraph look ─────────────────

    /// The comparison corpus diagram (`gitgraph.mmd`).
    const CORPUS: &str = "gitGraph
    commit id: \"init\"
    branch feat
    checkout feat
    commit id: \"a\"
    commit id: \"b\"
    checkout main
    commit id: \"c\"
    merge feat id: \"merge-feat\" tag: \"v1.0\"
    branch hotfix
    checkout hotfix
    commit id: \"fix\" type: HIGHLIGHT
    checkout main
    merge hotfix id: \"merge-hotfix\"
    commit id: \"release\" type: REVERSE tag: \"v1.1\"";

    /// Long ids and a long branch name (report P1 / missed).
    const LONG: &str = "gitGraph
    commit id: \"initial-scaffold\"
    commit id: \"add-authentication\"
    branch feature/login-page
    checkout feature/login-page
    commit id: \"login-form\"
    commit id: \"validate-input\"
    checkout main
    merge feature/login-page id: \"merge-login\" tag: \"v0.1-alpha\"
    commit id: \"release-candidate-final\"";

    /// Main merged down into a branch that moved on since it forked.
    const MERGE_DOWN: &str = "gitGraph
    commit id: \"c1\"
    branch develop
    checkout develop
    commit id: \"d1\"
    checkout main
    commit id: \"c2\"
    checkout develop
    commit id: \"d2\"
    merge main id: \"sync\"
    commit id: \"d3\"
    checkout main
    merge develop id: \"rel\" tag: \"v1.0\"";

    /// Lanes reordered with `order:`, typed merges, two forks from one
    /// commit.
    const ORDERED: &str = "gitGraph
    commit id: \"init\" tag: \"v0\"
    branch dev order: 2
    branch feat order: 1
    checkout dev
    commit id: \"d1\"
    checkout feat
    commit id: \"f1\"
    checkout main
    commit id: \"m1\"
    merge dev id: \"md\" type: HIGHLIGHT
    merge feat id: \"mf\" type: REVERSE tag: \"v2\"";

    /// Four lanes with crossings and default (numeric) ids.
    const MANY: &str = "gitGraph
    commit
    branch develop
    commit
    branch feature
    commit
    checkout develop
    commit
    branch hotfix
    commit
    checkout feature
    commit
    checkout main
    merge hotfix
    checkout develop
    merge feature
    commit
    checkout main
    merge develop tag: \"v2.0\"
    commit";

    /// Every case in every orientation.
    fn all_cases() -> Vec<(String, GitScene)> {
        let mut out = Vec::new();
        for src in [CORPUS, LONG, MERGE_DOWN, ORDERED, MANY] {
            for header in ["gitGraph", "gitGraph TB:", "gitGraph BT:"] {
                let src = src.replacen("gitGraph", header, 1);
                let gs = scene(&git(&src));
                out.push((src, gs));
            }
        }
        out
    }

    /// The drawn path of an edge, sampled every couple of px: the two
    /// legs and the true arc of an elbow, or the straight line.
    fn drawn(e: &SceneEdge) -> Vec<Pt> {
        let [p0, c, _, p1] = e.bezier;
        let corner = c != p0 && c != p1;
        let pts = if corner {
            elbow_points(p0, c, p1)
        } else {
            vec![p0, p1]
        };
        let mut out = vec![pts[0]];
        for w in pts.windows(2) {
            let k = (dist(w[0], w[1]) / 2.0).ceil().max(1.0) as usize;
            for j in 1..=k {
                let t = j as f64 / k as f64;
                out.push((
                    w[0].0 + (w[1].0 - w[0].0) * t,
                    w[0].1 + (w[1].1 - w[0].1) * t,
                ));
            }
        }
        out
    }

    fn inside(r: Rect, q: Pt, pad: f64) -> bool {
        q.0 > r.0 - pad && q.0 < r.2 + pad && q.1 > r.1 - pad && q.1 < r.3 + pad
    }

    fn overlap(a: Rect, b: Rect) -> bool {
        a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
    }

    /// Every text box drawn: ids, tag pills, name pills.
    fn label_boxes(gs: &GitScene) -> Vec<(String, Rect)> {
        let ids = gs.commit_ids.iter().map(|l| (l.text.clone(), text_box(l)));
        let tags = gs
            .tags
            .iter()
            .map(|l| (format!("tag {}", l.text), tag_pill(l)));
        let names = gs
            .branch_labels
            .iter()
            .map(|l| (format!("branch {}", l.text), name_pill(l)));
        ids.chain(tags).chain(names).collect()
    }

    /// Report label-strikethrough: no fork / merge / branch line runs
    /// through a commit id, a tag or a branch name — in any
    /// orientation, with long ids and reordered lanes too.
    #[test]
    fn no_line_strikes_a_label() {
        for (src, gs) in all_cases() {
            let boxes = label_boxes(&gs);
            for e in &gs.scene.edges {
                for q in drawn(e) {
                    for (text, r) in &boxes {
                        assert!(
                            !inside(*r, q, EDGE_W / 2.0),
                            "{}->{} strikes {text:?} at {q:?}\n{src}",
                            e.from,
                            e.to
                        );
                    }
                }
            }
        }
    }

    /// Report P1 (long ids collide and clip, TB tags off canvas) and
    /// tag-top-margin: no two labels overlap, and everything drawn sits
    /// inside the canvas with margin to spare.
    #[test]
    fn labels_never_overlap_and_stay_on_canvas() {
        for (src, gs) in all_cases() {
            let boxes = label_boxes(&gs);
            for (i, (ta, a)) in boxes.iter().enumerate() {
                for (tb, b) in &boxes[i + 1..] {
                    assert!(!overlap(*a, *b), "{ta:?} overlaps {tb:?}\n{src}");
                }
                let canvas = (PAD / 2.0, PAD / 2.0, gs.scene.width, gs.scene.height);
                let canvas = (
                    canvas.0,
                    canvas.1,
                    canvas.2 - PAD / 2.0,
                    canvas.3 - PAD / 2.0,
                );
                assert!(
                    a.0 >= canvas.0 && a.1 >= canvas.1 && a.2 <= canvas.2 && a.3 <= canvas.3,
                    "{ta:?} {a:?} leaves the {:.0}x{:.0} canvas\n{src}",
                    gs.scene.width,
                    gs.scene.height
                );
            }
            // Ids and tags clear every commit glyph.
            for n in &gs.scene.nodes {
                let dot = (n.x - R, n.y - R, n.x + R, n.y + R);
                for (t, b) in &boxes {
                    assert!(!overlap(dot, *b), "{t:?} covers commit {}\n{src}", n.id);
                }
            }
        }
    }

    /// A line only ever touches the commits it joins: a merge into a
    /// branch that committed since takes the free lane instead of
    /// running through those commits (report: mermaid reroutes too).
    #[test]
    fn lines_pass_no_other_commit() {
        for (src, gs) in all_cases() {
            for e in &gs.scene.edges {
                for n in &gs.scene.nodes {
                    if n.id == e.from || n.id == e.to {
                        continue;
                    }
                    for q in drawn(e) {
                        assert!(
                            dist(q, (n.x, n.y)) > R + EDGE_W / 2.0,
                            "{}->{} runs through {}\n{src}",
                            e.from,
                            e.to,
                            n.id
                        );
                    }
                }
            }
        }
        let gs = scene(&git(MERGE_DOWN));
        let sync = gs
            .scene
            .edges
            .iter()
            .find(|e| e.to == "sync" && e.from == "c2");
        let sync = sync.expect("the c2 -> sync merge");
        // It runs along main (main's colour) and drops into sync.
        assert_eq!(sync.meta.style.stroke.as_deref(), Some(accent(0)));
        let node = |id: &str| gs.scene.nodes.iter().find(|n| n.id == id).unwrap();
        assert_eq!(sync.bezier[1], (node("sync").x, node("c2").y));
    }

    /// Report hockey-stick-curves / fork-overlaps-trunk: a fork drops
    /// straight from its parent (no stretch of it rides on the trunk)
    /// and a merge climbs straight into the merge commit, each through
    /// one rounded corner.
    #[test]
    fn forks_and_merges_are_rounded_elbows() {
        let gs = scene(&git(CORPUS));
        let edge = |a: &str, b: &str| {
            gs.scene
                .edges
                .iter()
                .find(|e| e.from == a && e.to == b)
                .unwrap_or_else(|| panic!("no {a}->{b}"))
        };
        let node = |id: &str| gs.scene.nodes.iter().find(|n| n.id == id).unwrap();

        let fork = edge("init", "a");
        let (p0, p1) = (fork.bezier[0], fork.bezier[3]);
        assert_eq!(fork.bezier[1], fork.bezier[2], "one corner");
        assert_eq!(
            fork.bezier[1],
            (p0.0, p1.1),
            "the corner is below the parent"
        );
        for q in drawn(fork) {
            // Nothing of the fork lies on the trunk outside the dot.
            if (q.1 - p0.1).abs() < EDGE_W {
                assert!(dist(q, p0) < R, "fork rides the trunk at {q:?}");
            }
        }
        let merge = edge("b", "merge-feat");
        let m = node("merge-feat");
        let c = merge.bezier[1];
        assert_eq!(c.1, merge.bezier[0].1, "runs along feat first");
        assert!((c.0 - m.x).abs() < R, "then climbs at the merge commit");
        // The SVG writes the corner as a true arc.
        let svg = to_svg(&gs);
        assert!(
            svg.contains(&format!(" A {ELBOW_R:.1} {ELBOW_R:.1} 0 0 ")),
            "{svg}"
        );
        // Hosts spline through the waypoints (and picking measures that
        // spline): it must stay on the drawn elbow.
        for e in &gs.scene.edges {
            let path = drawn(e);
            for q in crate::scene::edge_polyline(e) {
                let near = path
                    .iter()
                    .map(|&p| dist(p, q))
                    .fold(f64::INFINITY, f64::min);
                assert!(
                    near < 1.5,
                    "{}->{}: spline leaves the elbow by {near}",
                    e.from,
                    e.to
                );
            }
        }
    }

    /// Two legs leaving one commit on the same side run side by side
    /// (merge-feat takes feat's merge from the left and forks hotfix to
    /// the right), never on top of each other.
    #[test]
    fn legs_sharing_a_commit_run_side_by_side() {
        let gs = scene(&git(CORPUS));
        let m = gs
            .scene
            .nodes
            .iter()
            .find(|n| n.id == "merge-feat")
            .unwrap();
        let leg_x = |a: &str, b: &str| {
            let e = gs
                .scene
                .edges
                .iter()
                .find(|e| e.from == a && e.to == b)
                .unwrap();
            e.bezier[1].0
        };
        let (merge_in, fork_out) = (leg_x("b", "merge-feat"), leg_x("merge-feat", "fix"));
        assert!(
            merge_in < m.x && m.x < fork_out,
            "{merge_in} {} {fork_out}",
            m.x
        );
        assert!((fork_out - merge_in - LEG_PITCH).abs() < 1e-9);
        assert!(
            fork_out - m.x < R && m.x - merge_in < R,
            "both start under the dot"
        );
        // Two forks from one commit: the longer leg runs outside, so
        // the shorter one's turn never crosses it.
        let gs = scene(&git(ORDERED));
        let leg = |b: &str| {
            let e = gs
                .scene
                .edges
                .iter()
                .find(|e| e.from == "init" && e.to == b)
                .unwrap();
            e.bezier[1].0
        };
        assert!(leg("d1") < leg("f1"), "dev (farther lane) outside");
    }

    /// Report monochrome-edges / thin-strokes: every line takes the
    /// colour of the lane it runs along (a fork its new branch's, a
    /// merge its branch's) and is drawn 5px wide with round caps.
    #[test]
    fn lines_take_their_lane_colour() {
        let gs = scene(&git(CORPUS));
        let colour = |a: &str, b: &str| {
            let e = gs
                .scene
                .edges
                .iter()
                .find(|e| e.from == a && e.to == b)
                .unwrap();
            assert_eq!(e.meta.style.stroke_width, Some(EDGE_W));
            e.meta.style.stroke.clone().unwrap()
        };
        assert_eq!(colour("init", "c"), accent(0));
        assert_eq!(colour("init", "a"), accent(1));
        assert_eq!(colour("a", "b"), accent(1));
        assert_eq!(colour("b", "merge-feat"), accent(1));
        assert_eq!(colour("merge-feat", "fix"), accent(2));
        assert_eq!(colour("fix", "merge-hotfix"), accent(2));
        let svg = to_svg(&gs);
        for k in 0..3 {
            assert!(
                svg.contains(&format!(
                    "stroke=\"{}\" stroke-width=\"5.0\" stroke-linecap=\"round\"",
                    accent(k)
                )),
                "lane {k}: {svg}"
            );
        }
        assert!(!svg.contains(&format!("stroke=\"{EDGE_COLOR}\"")), "{svg}");
    }

    /// Report merge-arrow-not-emitted: the scene and the SVG agree —
    /// no arrowheads, as in mermaid.
    #[test]
    fn merges_carry_no_arrowhead() {
        let gs = scene(&git(CORPUS));
        assert!(gs.scene.edges.iter().all(|e| e.kind == EdgeKind::Open));
        assert!(!to_svg(&gs).contains("marker"));
    }

    /// Report P0 #15, no-merge-glyph, reverse-as-hollow,
    /// highlight-too-subtle: each commit kind has a glyph of its own.
    #[test]
    fn commit_kinds_have_distinct_glyphs() {
        let gs = scene(&git(CORPUS));
        let node = |id: &str| gs.scene.nodes.iter().find(|n| n.id == id).unwrap();
        assert_eq!(node("init").shape, Shape::Circle);
        assert_eq!(node("init").label, "");
        // A merge is a ring around a light core.
        assert_eq!(node("merge-feat").shape, Shape::DoubleCircle);
        // REVERSE: a filled disc crossed out — no longer a hollow ring
        // (which read as a merge).
        let rev = node("release");
        assert_eq!(rev.shape, Shape::Circle);
        assert_eq!(rev.label, REVERSE_MARK);
        assert_eq!(rev.style.fill.as_deref(), Some(accent(0)));
        // HIGHLIGHT: a square.
        assert_eq!(node("fix").shape, Shape::Rect);
        let svg = to_svg(&gs);
        let m = node("merge-feat");
        assert!(svg.contains(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" fill=\"{HOLE_FILL}\"/>",
            m.x,
            m.y,
            R * 0.45
        )));
        let k = R * 0.45;
        assert!(svg.contains(&format!("<path d=\"M {:.1} {:.1} L", rev.x - k, rev.y - k)));
        let f = node("fix");
        assert!(svg.contains(&format!(
            "<rect x=\"{:.1}\" y=\"{:.1}\"",
            f.x - f.w / 2.0,
            f.y - f.h / 2.0
        )));
        // The type decides a merge's glyph, as in mermaid.
        let gs = scene(&git(ORDERED));
        let node = |id: &str| gs.scene.nodes.iter().find(|n| n.id == id).unwrap();
        assert_eq!(node("md").shape, Shape::Rect);
        assert_eq!(node("mf").label, REVERSE_MARK);
        // Picking matches the glyph: the square's corner is the commit.
        let sq = node("md");
        let corner = (sq.x + sq.w / 2.0 - 1.0, sq.y - sq.h / 2.0 + 1.0);
        let i = gs.scene.nodes.iter().position(|n| n.id == "md").unwrap();
        assert_eq!(gs.scene.node_at(corner.0, corner.1), Some(i));
    }

    /// Report tags-look-like-branch-names / no-lane-guides: tags are
    /// luggage labels, branch names pills, and every lane has a dashed
    /// guide.
    #[test]
    fn tags_pills_and_lane_guides() {
        for src in [CORPUS, &CORPUS.replacen("gitGraph", "gitGraph TB:", 1)] {
            let gs = scene(&git(src));
            let svg = to_svg(&gs);
            assert_eq!(svg.matches("stroke-dasharray=\"3 4\"").count(), 3, "{svg}");
            assert_eq!(svg.matches(" rx=\"5\"").count(), 3, "name pills: {svg}");
            assert_eq!(
                svg.matches(&format!("rx=\"{:.1}\" fill=\"{TAG_FILL}\"", TAG_H / 2.0))
                    .count(),
                2,
                "tag pills: {svg}"
            );
            let tag = gs.tags.iter().find(|t| t.text == "v1.0").unwrap();
            assert_eq!(tag.font_size, TAG_FONT);
            assert_eq!(tag.font_weight, None, "tags are not branch-name bold");
        }
    }

    /// Report P1 / missed (long ids): ids are measured — two long ids
    /// in a row keep `TEXT_GAP` apart, the last one is not cut off, and
    /// a long branch name's pill pushes the first commit right instead
    /// of running into it.
    #[test]
    fn long_ids_and_names_get_room() {
        let gs = scene(&git(LONG));
        let id = |t: &str| text_box(gs.commit_ids.iter().find(|l| l.text == t).unwrap());
        let (a, b) = (id("initial-scaffold"), id("add-authentication"));
        assert!(b.0 - a.2 >= TEXT_GAP - 1e-9, "{a:?} {b:?}");
        let last = id("release-candidate-final");
        assert!(
            last.2 <= gs.scene.width - PAD + 1e-9,
            "{last:?} {}",
            gs.scene.width
        );
        let pill = name_pill(&gs.branch_labels[1]);
        let first = &gs.scene.nodes[0];
        assert!(pill.2 + GAP <= first.x - R, "{pill:?} vs {}", first.x);
        // Short ids pack tighter than the old fixed 90px pitch.
        let gs = scene(&git("gitGraph\ncommit id: \"a\"\ncommit id: \"b\""));
        let (n0, n1) = (&gs.scene.nodes[0], &gs.scene.nodes[1]);
        assert_eq!(n1.x - n0.x, COMMIT_DX);
    }

    /// Report P1: in TB/BT a tag on the first lane used to hang off the
    /// left edge (x = 14, text-anchor end). Tags now sit right of their
    /// commit and the canvas fits around every label.
    #[test]
    fn tb_tags_stay_on_canvas() {
        let src = "gitGraph TB:\ncommit id: \"init\" tag: \"v0.1-alpha\"\ncommit id: \"two\"";
        for src in [src.to_string(), src.replacen("TB:", "BT:", 1)] {
            let gs = scene(&git(&src));
            let t = tag_pill(&gs.tags[0]);
            let n = &gs.scene.nodes[0];
            assert!(t.0 > n.x + R, "tag right of its commit");
            assert!(
                t.0 >= PAD - 1e-9 && t.2 <= gs.scene.width - PAD + 1e-9,
                "{t:?}"
            );
        }
    }

    /// `branch x order: n` moves lanes (and colours follow the lane).
    #[test]
    fn branch_order_moves_lanes() {
        let gs = scene(&git(ORDERED));
        let y = |id: &str| gs.scene.nodes.iter().find(|n| n.id == id).unwrap().y;
        assert!(
            y("init") < y("f1") && y("f1") < y("d1"),
            "feat (order 1) above dev (order 2)"
        );
        let fill = |id: &str| {
            let n = gs.scene.nodes.iter().find(|n| n.id == id).unwrap();
            n.style.fill.clone().unwrap()
        };
        assert_eq!(fill("f1"), accent(1));
        assert_eq!(fill("d1"), accent(2));
    }

    /// The generic scene stays a working interactive scene: commits
    /// and elbows pick where they are drawn.
    #[test]
    fn scene_picks_commits_and_elbows() {
        let gs = scene(&git(CORPUS));
        let s = &gs.scene;
        for (i, n) in s.nodes.iter().enumerate() {
            assert_eq!(s.hit_test(n.x, n.y, 3.0), Some(crate::scene::Hit::Node(i)));
        }
        let k = s
            .edges
            .iter()
            .position(|e| e.from == "init" && e.to == "a")
            .unwrap();
        let e = &s.edges[k];
        // Halfway down the fork's leg.
        let q = (e.bezier[0].0, (e.bezier[0].1 + e.bezier[3].1) / 2.0);
        assert_eq!(s.hit_test(q.0, q.1, 3.0), Some(crate::scene::Hit::Edge(k)));
    }

    #[test]
    fn rendering_is_deterministic() {
        for (src, gs) in all_cases() {
            assert_eq!(to_svg(&gs), to_svg(&scene(&git(&src))));
        }
    }

    /// A hand-built graph whose `order`s skip and repeat still packs
    /// its lanes, and an empty one renders.
    #[test]
    fn hand_built_orders_pack() {
        let mut g = git("gitGraph\ncommit\nbranch a\ncommit\nbranch b\ncommit");
        g.branches[1].order = 40;
        g.branches[2].order = 7;
        let gs = scene(&g);
        let ys: Vec<f64> = gs.scene.nodes.iter().map(|n| n.y).collect();
        assert!(ys[0] < ys[2] && ys[2] < ys[1], "{ys:?}");
        assert!(ys[1] - ys[0] <= 2.0 * BRANCH_DY + 1e-9);
        let svg = to_svg(&scene(&GitGraph::default()));
        assert!(svg.contains(">main</text>"));
    }
}
