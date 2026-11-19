
    #[test]
    fn zz_dbg_scratch() {
        let path = std::env::var("DBG_FILE").unwrap();
        let src = std::fs::read_to_string(&path).unwrap();
        let g = match crate::parser::parse_document(&src).unwrap() {
            crate::model::Document::Flowchart(g) | crate::model::Document::State(g) => g,
            _ => panic!(),
        };
        let lo = crate::layout::layout(&g);
        for (i, n) in g.nodes.iter().enumerate() {
            let p = &lo.nodes[i];
            eprintln!("node {i} {} layer={} b={:.1} l={:.1} bs={:.1} ls={:.1}", n.id, p.layer, p.b, p.l, p.bsize, p.lsize);
        }
        for (i, e) in g.edges.iter().enumerate() {
            eprintln!("edge {i} {}->{} path={:?}", g.nodes[e.from].id, g.nodes[e.to].id, lo.edge_paths[i]);
        }
        let s = scene(&g);
        for e in &s.edges {
            eprintln!("scene {}->{} wps={} bez={:?}", e.from, e.to, e.waypoints.len(), e.bezier);
        }
    }
}
