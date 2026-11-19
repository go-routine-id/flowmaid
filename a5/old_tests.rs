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
    /// on the white canvas — amber 2.6:1, green 3.2:1.
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
            let ink = crate::style::assert_text_reads(&svg, &lane, "#ffffff");
            assert_eq!(ink, crate::style::accent_ink(i), "{lane}");
            let ink = crate::style::assert_text_reads(&svg, &format!("t{i}"), "#ffffff");
            assert_eq!(ink, crate::style::accent_ink(i), "tag t{i}");
        }
        // The commit dots keep the full-strength accent.
        assert!(svg.contains(&format!("fill=\"{}\"", crate::style::accent(2))));
    }
}
