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
}
