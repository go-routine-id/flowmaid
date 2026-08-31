//! Git graph rendering: branches as lanes, commits as circles, merges as
//! curved joins.
//!
//! Like `pie`/`journey`/`mindmap` there is nothing draggable and no
//! `route()`: [`scene`] computes every coordinate and [`to_svg`]
//! serialises it. The returned [`GitScene::scene`] is a generic
//! [`crate::scene::Scene`] so terminal/GUI renderers can draw the
//! topology with no extra code; the SVG writer adds commit ids, tags,
//! and branch labels on top of the geometry.

use crate::model::{CommitKind, GitGraph, GitOrientation, Shape};
use crate::scene::{escape, svg_open, Scene, SceneEdge, SceneNode, SvgOptions, EDGE_COLOR, TEXT_COLOR};
use crate::style::accent;

/// Canvas margin.
pub const PAD: f64 = 30.0;
/// Space reserved for branch-name labels on the perpendicular axis
/// (left in LR, top/bottom in TB/BT).
pub const BRANCH_LABEL_W: f64 = 70.0;
/// Distance between commit columns / rows along the commit-flow axis.
pub const COMMIT_DX: f64 = 90.0;
/// Distance between branch lanes on the branch axis.
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

    let max_seq = d.commits.iter().map(|c| c.seq).max().unwrap_or(0) as f64;
    let max_order = d.branches.iter().map(|b| b.order).max().unwrap_or(0) as f64;

    let commit_axis_len = PAD + BRANCH_LABEL_W + max_seq * COMMIT_DX + R + PAD;
    let branch_axis_len = PAD + max_order * BRANCH_DY + R + PAD;

    let (width, height) = match d.orientation {
        GitOrientation::LR => (commit_axis_len, branch_axis_len),
        GitOrientation::TB | GitOrientation::BT => (branch_axis_len, commit_axis_len),
    };
    let width = width.max(PAD + BRANCH_LABEL_W + PAD);
    let height = height.max(PAD + PAD);

    let branch_pos = |order: usize| PAD + order as f64 * BRANCH_DY;
    let commit_pos = |seq: usize| PAD + BRANCH_LABEL_W + seq as f64 * COMMIT_DX;

    let pos = |c: &crate::model::GitCommit| -> (f64, f64) {
        match d.orientation {
            GitOrientation::LR => (commit_pos(c.seq), branch_pos(c.branch)),
            GitOrientation::TB => (branch_pos(c.branch), commit_pos(c.seq)),
            GitOrientation::BT => (branch_pos(c.branch), height - commit_pos(c.seq)),
        }
    };

    // Commit circles, ids, and tags.
    for c in &d.commits {
        let (x, y) = pos(c);
        let color = commit_color(d, c);
        // The generic Scene label is left empty: commit ids/tags are drawn
        // from GitScene labels so their placement can differ per orientation.
        nodes.push(SceneNode {
            id: c.id.clone(),
            x,
            y,
            w: 2.0 * R,
            h: 2.0 * R,
            shape: Shape::Circle,
            label: String::new(),
            style: crate::model::NodeStyle {
                fill: Some(color.fill.to_string()),
                stroke: Some(color.stroke.to_string()),
                stroke_width: Some(2.0),
                color: None,
            },
        });

        // In vertical modes, place the id to the right of the circle and the
        // tag to the left so they don't overlap the branch lane.
        let (id_x, id_anchor) = match d.orientation {
            GitOrientation::LR => (x, "middle"),
            GitOrientation::TB | GitOrientation::BT => (x + R + 4.0, "start"),
        };
        let (tag_x, tag_anchor) = match d.orientation {
            GitOrientation::LR => (x, "middle"),
            GitOrientation::TB | GitOrientation::BT => (x - R - 4.0, "end"),
        };

        commit_ids.push(GitLabel {
            x: id_x,
            y: y + R + 14.0,
            text: c.id.clone(),
            anchor: id_anchor,
            baseline: "middle",
            font_size: FONT,
            font_weight: None,
            color: TEXT_COLOR,
        });

        if let Some(tag) = &c.tag {
            tags.push(GitLabel {
                x: tag_x,
                y: y - R - 10.0,
                text: tag.clone(),
                anchor: tag_anchor,
                baseline: "middle",
                font_size: FONT,
                font_weight: Some("bold"),
                color: color.stroke,
            });
        }
    }

    // Branch lines and fork joins.
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
                let p0 = pos(&d.commits[parent_idx]);
                let p1 = pos(&d.commits[first]);
                edges.push(SceneEdge {
                    from: d.commits[parent_idx].id.clone(),
                    to: d.commits[first].id.clone(),
                    bezier: edge_bezier(p0, p1, d.orientation),
                    waypoints: Vec::new(),
                    kind: crate::model::EdgeKind::Open,
                    label: None,
                });
            }
        }

        for w in seq.windows(2) {
            let a = &d.commits[w[0]];
            let b = &d.commits[w[1]];
            let p0 = pos(a);
            let p1 = pos(b);
            edges.push(SceneEdge {
                from: a.id.clone(),
                to: b.id.clone(),
                bezier: edge_bezier(p0, p1, d.orientation),
                waypoints: Vec::new(),
                kind: crate::model::EdgeKind::Open,
                label: None,
            });
        }
    }

    // Merge curves: from source branch head to the merge commit.
    // Use an arrowhead so the merge direction reads clearly.
    for c in d.commits.iter() {
        if let Some(sp) = c.second_parent {
            let p0 = pos(&d.commits[sp]);
            let p1 = pos(c);
            edges.push(SceneEdge {
                from: d.commits[sp].id.clone(),
                to: c.id.clone(),
                bezier: edge_bezier(p0, p1, d.orientation),
                waypoints: Vec::new(),
                kind: crate::model::EdgeKind::Arrow,
                label: None,
            });
        }
    }

    // Branch name labels.
    for b in &d.branches {
        branch_labels.push(match d.orientation {
            GitOrientation::LR => GitLabel {
                x: PAD,
                y: branch_pos(b.order),
                text: b.name.clone(),
                anchor: "start",
                baseline: "middle",
                font_size: FONT,
                font_weight: Some("bold"),
                color: accent(b.order),
            },
            GitOrientation::TB => GitLabel {
                x: branch_pos(b.order),
                y: PAD,
                text: b.name.clone(),
                anchor: "middle",
                baseline: "middle",
                font_size: FONT,
                font_weight: Some("bold"),
                color: accent(b.order),
            },
            GitOrientation::BT => GitLabel {
                x: branch_pos(b.order),
                y: height - PAD,
                text: b.name.clone(),
                anchor: "middle",
                baseline: "middle",
                font_size: FONT,
                font_weight: Some("bold"),
                color: accent(b.order),
            },
        });
    }

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

fn commit_color(_d: &GitGraph, c: &crate::model::GitCommit) -> CommitStyle {
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

/// Build a bezier for an edge. Same-branch edges are straight; edges that
/// cross from one lane to another bend from the source lane toward the
/// target, with the control point staying on the source lane so the curve
/// hugs the branch axis before turning.
fn edge_bezier(p0: (f64, f64), p1: (f64, f64), orientation: GitOrientation) -> [(f64, f64); 4] {
    let same_lane = match orientation {
        GitOrientation::LR => (p0.1 - p1.1).abs() < f64::EPSILON,
        GitOrientation::TB | GitOrientation::BT => (p0.0 - p1.0).abs() < f64::EPSILON,
    };
    if same_lane {
        return [p0, p0, p1, p1];
    }
    match orientation {
        GitOrientation::LR => {
            let cx = (p0.0 + p1.0) / 2.0;
            [(p0.0, p0.1), (cx, p0.1), (cx, p0.1), (p1.0, p1.1)]
        }
        GitOrientation::TB | GitOrientation::BT => {
            let cy = (p0.1 + p1.1) / 2.0;
            [(p0.0, p0.1), (p0.0, cy), (p0.0, cy), (p1.0, p1.1)]
        }
    }
}

/// Serialise a git graph scene to SVG.
pub fn to_svg(gs: &GitScene) -> String {
    to_svg_with(gs, &SvgOptions::default())
}

/// [`to_svg`] with explicit viewport options (see [`SvgOptions`]).
pub fn to_svg_with(gs: &GitScene, opts: &SvgOptions) -> String {
    let mut s = String::new();
    svg_open(
        &mut s,
        gs.scene.width,
        gs.scene.height,
        FONT,
        "Git graph diagram",
        opts,
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
        let fill = crate::scene::style_attr(n.style.fill.as_deref(), accent(0));
        let stroke = crate::scene::style_attr(n.style.stroke.as_deref(), EDGE_COLOR);
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
}
