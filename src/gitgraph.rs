//! Git graph rendering: branches as horizontal lanes, commits as
//! circles, merges as curved joins.
//!
//! Like `pie`/`journey`/`mindmap` there is nothing draggable and no
//! `route()`: [`scene`] computes every coordinate and [`to_svg`]
//! serialises it. The returned [`GitScene::scene`] is a generic
//! [`crate::scene::Scene`] so terminal/GUI renderers can draw the
//! topology with no extra code; the SVG writer adds commit ids, tags,
//! and branch labels on top of the geometry.

use crate::model::{CommitKind, GitGraph, Shape};
use crate::scene::{escape, svg_open, Scene, SceneEdge, SceneNode, EDGE_COLOR, TEXT_COLOR};
use crate::style::accent;

/// Canvas margin.
pub const PAD: f64 = 30.0;
/// Horizontal space reserved for branch name labels on the left.
pub const BRANCH_LABEL_W: f64 = 70.0;
/// Horizontal distance between commit columns.
pub const COMMIT_DX: f64 = 90.0;
/// Vertical distance between branch lanes.
pub const BRANCH_DY: f64 = 70.0;
/// Commit circle radius.
pub const R: f64 = 12.0;
/// Font size for labels.
pub const FONT: u32 = 13;

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
/// circles + connecting edges) in a renderer-agnostic form; the SVG
/// writer also uses `branch_labels`, `commit_ids`, and `tags`.
#[derive(Debug, Clone)]
pub struct GitScene {
    pub scene: Scene,
    pub branch_labels: Vec<GitLabel>,
    pub commit_ids: Vec<GitLabel>,
    pub tags: Vec<GitLabel>,
}

/// Compute all git graph geometry.
pub fn scene(d: &GitGraph) -> GitScene {
    let mut nodes = Vec::with_capacity(d.commits.len());
    let mut edges = Vec::new();
    let mut branch_labels = Vec::new();
    let mut commit_ids = Vec::new();
    let mut tags = Vec::new();

    // Pre-compute branch lane Y positions.
    let branch_y: Vec<f64> = d
        .branches
        .iter()
        .map(|b| PAD + b.order as f64 * BRANCH_DY)
        .collect();

    // Commit circles.
    for (i, c) in d.commits.iter().enumerate() {
        let x = PAD + BRANCH_LABEL_W + c.seq as f64 * COMMIT_DX;
        let y = branch_y[c.branch];
        let color = commit_color(d, i);
        nodes.push(SceneNode {
            id: c.id.clone(),
            x,
            y,
            w: 2.0 * R,
            h: 2.0 * R,
            shape: Shape::Circle,
            label: c.id.clone(),
            style: crate::model::NodeStyle {
                fill: Some(color.fill.to_string()),
                stroke: Some(color.stroke.to_string()),
                stroke_width: Some(2.0),
                color: None,
            },
        });

        commit_ids.push(GitLabel {
            x,
            y: y + R + 14.0,
            text: c.id.clone(),
            anchor: "middle",
            baseline: "middle",
            font_size: FONT,
            font_weight: None,
            color: TEXT_COLOR,
        });

        if let Some(tag) = &c.tag {
            tags.push(GitLabel {
                x,
                y: y - R - 10.0,
                text: tag.clone(),
                anchor: "middle",
                baseline: "middle",
                font_size: FONT,
                font_weight: Some("bold"),
                color: color.stroke,
            });
        }
    }

    // Branch lines: connect consecutive commits on the same branch,
    // and connect a branch's fork point to its first commit.
    for b in &d.branches {
        let mut seq: Vec<usize> = d
            .commits
            .iter()
            .enumerate()
            .filter(|(_, c)| c.branch == b.order)
            .map(|(i, _)| i)
            .collect();
        seq.sort_by_key(|i| d.commits[*i].seq);

        if let Some(parent_idx) = b.parent_commit {
            if let Some(&first) = seq.first() {
                let x0 = PAD + BRANCH_LABEL_W + d.commits[parent_idx].seq as f64 * COMMIT_DX;
                let y0 = branch_y[d.commits[parent_idx].branch];
                let x1 = PAD + BRANCH_LABEL_W + d.commits[first].seq as f64 * COMMIT_DX;
                let y1 = branch_y[d.commits[first].branch];
                edges.push(SceneEdge {
                    from: d.commits[parent_idx].id.clone(),
                    to: d.commits[first].id.clone(),
                    bezier: [(x0, y0), (x0, y0), (x1, y1), (x1, y1)],
                    waypoints: Vec::new(),
                    kind: crate::model::EdgeKind::Open,
                    label: None,
                });
            }
        }

        for w in seq.windows(2) {
            let a = &d.commits[w[0]];
            let x0 = PAD + BRANCH_LABEL_W + a.seq as f64 * COMMIT_DX;
            let y0 = branch_y[a.branch];
            let b = &d.commits[w[1]];
            let x1 = PAD + BRANCH_LABEL_W + b.seq as f64 * COMMIT_DX;
            let y1 = branch_y[b.branch];
            edges.push(SceneEdge {
                from: a.id.clone(),
                to: b.id.clone(),
                bezier: [(x0, y0), (x0, y0), (x1, y1), (x1, y1)],
                waypoints: Vec::new(),
                kind: crate::model::EdgeKind::Open,
                label: None,
            });
        }
    }

    // Merge curves: from source branch head to the merge commit.
    for c in d.commits.iter() {
        if let Some(sp) = c.second_parent {
            let x1 = PAD + BRANCH_LABEL_W + c.seq as f64 * COMMIT_DX;
            let y1 = branch_y[c.branch];
            let x0 = PAD + BRANCH_LABEL_W + d.commits[sp].seq as f64 * COMMIT_DX;
            let y0 = branch_y[d.commits[sp].branch];
            // Quadratic control point: halfway horizontally, at the
            // source branch's lane, so the curve hugs the lane and then
            // bends to the merge commit.
            let cx = (x0 + x1) / 2.0;
            let cy = y0;
            edges.push(SceneEdge {
                from: d.commits[sp].id.clone(),
                to: c.id.clone(),
                bezier: [(x0, y0), (cx, cy), (cx, cy), (x1, y1)],
                waypoints: Vec::new(),
                kind: crate::model::EdgeKind::Open,
                label: None,
            });
        }
    }

    // Branch name labels on the left of each lane.
    for b in &d.branches {
        branch_labels.push(GitLabel {
            x: PAD,
            y: branch_y[b.order],
            text: b.name.clone(),
            anchor: "start",
            baseline: "middle",
            font_size: FONT,
            font_weight: Some("bold"),
            color: accent(b.order),
        });
    }

    let max_seq = d.commits.iter().map(|c| c.seq).max().unwrap_or(0) as f64;
    let max_order = d.branches.iter().map(|b| b.order).max().unwrap_or(0) as f64;
    let width = PAD + BRANCH_LABEL_W + max_seq * COMMIT_DX + R + PAD;
    let height = PAD + max_order * BRANCH_DY + R + PAD;
    let width = width.max(PAD + BRANCH_LABEL_W + PAD);
    let height = height.max(PAD + PAD);

    GitScene {
        scene: Scene {
            nodes,
            edges,
            clusters: Vec::new(),
            width,
            height,
        },
        branch_labels,
        commit_ids,
        tags,
    }
}

#[derive(Debug, Clone, Copy)]
struct CommitStyle {
    fill: &'static str,
    stroke: &'static str,
}

fn commit_color(d: &GitGraph, idx: usize) -> CommitStyle {
    let c = &d.commits[idx];
    let branch_color = accent(c.branch);
    match c.kind {
        CommitKind::Normal => CommitStyle {
            fill: branch_color,
            stroke: branch_color,
        },
        CommitKind::Reverse => CommitStyle {
            fill: "#ffffff",
            stroke: branch_color,
        },
        CommitKind::Highlight => CommitStyle {
            fill: branch_color,
            stroke: TEXT_COLOR,
        },
    }
}

/// Serialise a git graph scene to SVG.
pub fn to_svg(gs: &GitScene) -> String {
    let mut s = String::new();
    svg_open(
        &mut s,
        gs.scene.width,
        gs.scene.height,
        FONT,
        "Git graph diagram",
    );

    // Branch name labels.
    for l in &gs.branch_labels {
        write_label(&mut s, l);
    }

    // Branch lines and merge curves.
    for e in &gs.scene.edges {
        let [(x0, y0), (c1x, c1y), (c2x, c2y), (x1, y1)] = e.bezier;
        s.push_str(&format!(
            "<path d=\"M {:.1} {:.1} C {:.1} {:.1} {:.1} {:.1} {:.1} {:.1}\" \
             fill=\"none\" stroke=\"{}\" stroke-width=\"2\"/>\n",
            x0, y0, c1x, c1y, c2x, c2y, x1, y1, EDGE_COLOR
        ));
    }

    // Commit circles.
    for n in &gs.scene.nodes {
        let fill = n.style.fill.as_deref().unwrap_or_else(|| accent(0));
        let stroke = n.style.stroke.as_deref().unwrap_or(EDGE_COLOR);
        let sw = n.style.stroke_width.unwrap_or(2.0);
        s.push_str(&format!(
            "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" \
             fill=\"{}\" stroke=\"{}\" stroke-width=\"{:.1}\"/>\n",
            n.x, n.y, R, fill, stroke, sw
        ));
    }

    // Commit ids and tags.
    for l in &gs.commit_ids {
        write_label(&mut s, l);
    }
    for l in &gs.tags {
        write_label(&mut s, l);
    }

    s.push_str("</svg>\n");
    s
}

fn write_label(s: &mut String, l: &GitLabel) {
    let weight = l.font_weight.unwrap_or("normal");
    s.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"{}\" \
         dominant-baseline=\"{}\" font-size=\"{}\" font-weight=\"{}\" \
         fill=\"{}\">{}</text>\n",
        l.x,
        l.y,
        l.anchor,
        l.baseline,
        l.font_size,
        weight,
        l.color,
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
}
