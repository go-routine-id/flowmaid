//! Architecture-beta diagram rendering: nested groups of services,
//! connected by orthogonal edges between named port sides.
//!
//! Like `pie`/`journey`/`mindmap`/`gitgraph`, the layout is fully
//! automatic and the result is exposed as a generic [`Scene`] so
//! terminal/GUI hosts can draw it without knowing the source syntax.

use crate::model::{ArchSide, Architecture, NodeStyle, Shape};
use crate::scene::{
    escape, svg_open, Scene, SceneCluster, SceneEdge, SceneNode, EDGE_COLOR, TEXT_COLOR,
};
use crate::style::accent;

/// Canvas margin around the whole diagram.
pub const PAD: f64 = 24.0;
/// Default service box size.
pub const SERVICE_W: f64 = 150.0;
pub const SERVICE_H: f64 = 70.0;
/// Padding inside a group box.
pub const GROUP_PAD: f64 = 24.0;
/// Height reserved for a group title.
pub const GROUP_TITLE_H: f64 = 26.0;
/// Space between top-level groups.
pub const GROUP_GAP: f64 = 50.0;
/// Space between children inside a group.
pub const CHILD_GAP_X: f64 = 28.0;
pub const CHILD_GAP_Y: f64 = 28.0;
/// Font size for service and group labels.
pub const FONT: u32 = 13;
/// Font size for icon glyphs inside service boxes.
pub const ICON_FONT: u32 = 16;

/// Map a Mermaid-style icon name to a Unicode glyph.
///
/// flowmaid is zero-dependency and has no icon font, so we render a
/// single glyph as a lightweight visual hint. Unknown icon names are
/// silently ignored (only the title is drawn).
pub fn icon_glyph(name: &str) -> Option<&'static str> {
    Some(match name.trim().to_ascii_lowercase().as_str() {
        "cloud" => "☁",
        "database" | "db" => "🗄",
        "server" => "🖥",
        "disk" | "storage" => "💾",
        "internet" | "web" | "globe" => "🌐",
        "user" => "👤",
        "users" | "group" => "👥",
        "lock" | "secure" => "🔒",
        "key" => "🔑",
        "mail" | "email" | "envelope" => "✉",
        "phone" | "mobile" => "📱",
        "desktop" | "laptop" => "💻",
        "file" => "📄",
        "folder" => "📁",
        "code" => "🖥",
        "bug" => "🐛",
        "search" => "🔍",
        "chart" | "graph" => "📊",
        "cpu" | "processor" => "🧠",
        "memory" | "ram" => "🧮",
        "network" | "firewall" | "router" | "switch" => "🌐",
        "queue" | "message" => "📬",
        "cache" => "⚡",
        _ => return None,
    })
}

/// Positioned geometry for an architecture diagram.
#[derive(Debug, Clone)]
pub struct ArchScene {
    pub scene: Scene,
    /// Optional icon glyph for each service node, index-parallel with `scene.nodes`.
    pub icons: Vec<Option<String>>,
    /// Whether each edge (index-parallel with `scene.edges`) has an
    /// arrowhead pointing at its `from` / `to` endpoint.
    pub arrow_starts: Vec<bool>,
    pub arrow_ends: Vec<bool>,
}

/// A child of a group while it is being laid out.
#[derive(Debug, Clone, Copy)]
enum Child {
    Service(usize),
    Group(usize),
}

/// Compute the scene for an architecture diagram.
pub fn scene(d: &Architecture) -> ArchScene {
    let mut children: Vec<Vec<Child>> = vec![Vec::new(); d.groups.len()];
    let mut top_groups: Vec<usize> = Vec::new();

    for (i, g) in d.groups.iter().enumerate() {
        match g.parent {
            Some(p) => children[p].push(Child::Group(i)),
            None => top_groups.push(i),
        }
    }
    for (i, s) in d.services.iter().enumerate() {
        match s.group {
            Some(g) => children[g].push(Child::Service(i)),
            None => {
                // Services without a group become children of a synthetic
                // top-level group? For the MVP we simply skip them in the
                // layout — they have no box to sit in.
            }
        }
    }

    let mut group_size = vec![(0.0, 0.0); d.groups.len()];
    let mut group_rel = vec![(0.0, 0.0); d.groups.len()];
    let mut service_rel = vec![(0.0, 0.0); d.services.len()];

    // Size every group recursively. Top-level groups also need a size,
    // so we iterate them even if they are empty.
    for &g in &top_groups {
        layout_group(
            g,
            &children,
            &mut group_size,
            &mut group_rel,
            &mut service_rel,
        );
    }

    // Place top-level groups horizontally, then propagate positions down.
    let mut abs_group = vec![(0.0, 0.0); d.groups.len()];
    let mut abs_service = vec![(0.0, 0.0); d.services.len()];
    let mut x = PAD;
    let mut height: f64 = 0.0;
    for &g in &top_groups {
        let (w, h) = group_size[g];
        place_group(
            g,
            (x, PAD),
            &children,
            &group_rel,
            &service_rel,
            &mut abs_group,
            &mut abs_service,
        );
        x += w + GROUP_GAP;
        height = height.max(h);
    }

    let width = if x > PAD {
        x - GROUP_GAP + PAD
    } else {
        PAD * 2.0
    };
    let height = height + PAD * 2.0;

    let mut clusters = Vec::with_capacity(d.groups.len());

    for (i, g) in d.groups.iter().enumerate() {
        let (gx, gy) = abs_group[i];
        let (gw, gh) = group_size[i];
        let title = match g.icon.as_deref().and_then(icon_glyph) {
            Some(glyph) => format!("{} {}", glyph, g.title),
            None => g.title.clone(),
        };
        clusters.push(SceneCluster {
            id: g.id.clone(),
            x: gx,
            y: gy,
            w: gw,
            h: gh,
            title,
            depth: depth_of(i, d),
        });
    }

    let mut nodes = Vec::with_capacity(d.services.len());
    let mut icons: Vec<Option<String>> = Vec::with_capacity(d.services.len());

    for (i, s) in d.services.iter().enumerate() {
        let (sx, sy) = abs_service[i];
        icons.push(
            s.icon
                .as_deref()
                .and_then(icon_glyph)
                .map(|g| g.to_string()),
        );
        nodes.push(SceneNode {
            id: s.id.clone(),
            x: sx + SERVICE_W / 2.0,
            y: sy + SERVICE_H / 2.0,
            w: SERVICE_W,
            h: SERVICE_H,
            shape: Shape::Rounded,
            label: s.title.clone(),
            style: NodeStyle {
                fill: Some("#ffffff".to_string()),
                stroke: Some(accent(i).to_string()),
                stroke_width: Some(2.0),
                color: None,
            },
        });
    }

    let mut edges: Vec<SceneEdge> = Vec::with_capacity(d.edges.len());
    let mut arrow_starts: Vec<bool> = Vec::with_capacity(d.edges.len());
    let mut arrow_ends: Vec<bool> = Vec::with_capacity(d.edges.len());
    for e in &d.edges {
        let p0 = port(&abs_service, e.from, e.from_side);
        let p1 = port(&abs_service, e.to, e.to_side);
        let waypoints = orthogonal_route(p0, p1);
        let bezier = [p0, p0, p1, p1];
        edges.push(SceneEdge {
            from: d.services[e.from].id.clone(),
            to: d.services[e.to].id.clone(),
            bezier,
            waypoints,
            kind: if e.arrow_start || e.arrow_end {
                crate::model::EdgeKind::Arrow
            } else {
                crate::model::EdgeKind::Open
            },
            label: None,
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

fn depth_of(idx: usize, d: &Architecture) -> usize {
    let mut depth = 0;
    let mut cur = idx;
    while let Some(p) = d.groups[cur].parent {
        depth += 1;
        cur = p;
    }
    depth
}

fn layout_group(
    idx: usize,
    children: &[Vec<Child>],
    sizes: &mut [(f64, f64)],
    group_rel: &mut [(f64, f64)],
    service_rel: &mut [(f64, f64)],
) -> (f64, f64) {
    let kids = &children[idx];
    let n = kids.len();
    let cols = if n == 0 {
        1
    } else {
        (n as f64).sqrt().ceil().max(1.0) as usize
    };

    let mut x = GROUP_PAD;
    let mut y = GROUP_PAD + GROUP_TITLE_H;
    let mut row_h = 0.0;
    let mut max_w = GROUP_PAD;
    let mut max_h = y;

    #[derive(Debug)]
    struct Row {
        start: usize,
        end: usize,
        width: f64,
    }
    let mut rows: Vec<Row> = Vec::new();
    let mut row_start = 0usize;

    for (i, child) in kids.iter().enumerate() {
        if i > 0 && i % cols == 0 {
            let row_right = x - CHILD_GAP_X;
            rows.push(Row {
                start: row_start,
                end: i,
                width: row_right + GROUP_PAD,
            });
            x = GROUP_PAD;
            y += row_h + CHILD_GAP_Y;
            row_h = 0.0;
            row_start = i;
        }

        let (cw, ch) = match *child {
            Child::Service(s) => {
                service_rel[s] = (x, y);
                (SERVICE_W, SERVICE_H)
            }
            Child::Group(g) => {
                let size = layout_group(g, children, sizes, group_rel, service_rel);
                group_rel[g] = (x, y);
                size
            }
        };

        row_h = row_h.max(ch);
        max_w = max_w.max(x + cw + GROUP_PAD);
        max_h = max_h.max(y + ch + GROUP_PAD);
        x += cw + CHILD_GAP_X;
    }

    // Close the final row.
    if row_start < n {
        let row_right = x - CHILD_GAP_X;
        rows.push(Row {
            start: row_start,
            end: n,
            width: row_right + GROUP_PAD,
        });
    }

    let w = max_w.max(GROUP_PAD * 2.0 + 80.0);
    let h = max_h.max(GROUP_PAD + GROUP_TITLE_H + GROUP_PAD);
    sizes[idx] = (w, h);

    // Center each row horizontally inside the computed group width.
    for row in &rows {
        let offset = (w - row.width) / 2.0;
        if offset.abs() < f64::EPSILON {
            continue;
        }
        for child in &kids[row.start..row.end] {
            match *child {
                Child::Service(s) => service_rel[s].0 += offset,
                Child::Group(g) => group_rel[g].0 += offset,
            }
        }
    }

    (w, h)
}

fn place_group(
    idx: usize,
    top_left: (f64, f64),
    children: &[Vec<Child>],
    group_rel: &[(f64, f64)],
    service_rel: &[(f64, f64)],
    abs_group: &mut [(f64, f64)],
    abs_service: &mut [(f64, f64)],
) {
    abs_group[idx] = top_left;
    for child in &children[idx] {
        match *child {
            Child::Service(s) => {
                let (rx, ry) = service_rel[s];
                abs_service[s] = (top_left.0 + rx, top_left.1 + ry);
            }
            Child::Group(g) => {
                let (rx, ry) = group_rel[g];
                place_group(
                    g,
                    (top_left.0 + rx, top_left.1 + ry),
                    children,
                    group_rel,
                    service_rel,
                    abs_group,
                    abs_service,
                );
            }
        }
    }
}

fn port(abs_service: &[(f64, f64)], idx: usize, side: ArchSide) -> (f64, f64) {
    let (x, y) = abs_service[idx];
    let cx = x + SERVICE_W / 2.0;
    let cy = y + SERVICE_H / 2.0;
    match side {
        ArchSide::T => (cx, y),
        ArchSide::B => (cx, y + SERVICE_H),
        ArchSide::L => (x, cy),
        ArchSide::R => (x + SERVICE_W, cy),
    }
}

fn orthogonal_route(a: (f64, f64), b: (f64, f64)) -> Vec<(f64, f64)> {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    if dx.abs() < f64::EPSILON || dy.abs() < f64::EPSILON {
        return vec![a, b];
    }
    if dx.abs() > dy.abs() {
        let mid_x = (a.0 + b.0) / 2.0;
        vec![a, (mid_x, a.1), (mid_x, b.1), b]
    } else {
        let mid_y = (a.1 + b.1) / 2.0;
        vec![a, (a.0, mid_y), (b.0, mid_y), b]
    }
}

/// Serialise an architecture scene to SVG.
pub fn to_svg(as_: &ArchScene) -> String {
    let mut s = String::new();
    svg_open(
        &mut s,
        as_.scene.width,
        as_.scene.height,
        FONT,
        "Architecture diagram",
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

    // Group boxes.
    for c in &as_.scene.clusters {
        s.push_str(&format!(
            "<rect x='{:.1}' y='{:.1}' width='{:.1}' height='{:.1}' \
             rx='8' fill='#f7f8fc' stroke='#d5d9ec' stroke-width='2'/>\n",
            c.x, c.y, c.w, c.h
        ));
        s.push_str(&format!(
            "<text x='{:.1}' y='{:.1}' font-size='{}' font-weight='bold' \
             fill='{}'>{}</text>\n",
            c.x + GROUP_PAD,
            c.y + GROUP_PAD - 4.0,
            FONT,
            TEXT_COLOR,
            escape(&c.title)
        ));
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
        if as_.arrow_ends[i] {
            markers.push_str(" marker-end='url(#flowmaid-arch-arrow-end)'");
        }
        if as_.arrow_starts[i] {
            markers.push_str(" marker-start='url(#flowmaid-arch-arrow-start)'");
        }
        s.push_str(&format!(
            "<polyline points='{}' fill='none' stroke='{}' stroke-width='2'{} />\n",
            points, EDGE_COLOR, markers
        ));
    }

    // Service boxes.
    for (i, n) in as_.scene.nodes.iter().enumerate() {
        let fill = n.style.fill.as_deref().unwrap_or("#ffffff");
        let stroke = n.style.stroke.as_deref().unwrap_or(EDGE_COLOR);
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
        let label_y = if as_.icons[i].is_some() {
            n.y + 4.0
        } else {
            n.y
        };
        s.push_str(&format!(
            "<text x='{:.1}' y='{:.1}' dy='0.33em' text-anchor='middle' \
             font-size='{}' fill='{}'>{}\n",
            n.x,
            label_y,
            FONT,
            TEXT_COLOR,
            escape(&n.label)
        ));
        if let Some(icon) = as_.icons[i].as_deref() {
            s.push_str(&format!(
                "<tspan x='{:.1}' dy='-20' font-size='{}'>{}</tspan>\n",
                n.x, ICON_FONT, icon
            ));
        }
        s.push_str("</text>\n");
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
        assert!(svg.contains("🗄"));
        assert!(svg.contains("☁"));
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
}
