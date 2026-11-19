fn main() {
    for path in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&path).unwrap();
        let r = flowmaid::quality::measure_source(&src, false).unwrap();
        let m = &r.statik;
        println!("{path}: edge_edge_x={} xgap_max={} dash_edges={} xnotches={} ortho_edges={} overlap_pairs={} tight_pairs={} wiggles={} edge_frame_pairs={} edge_node_pairs={} aspect={:.2} w={} h={} min_edge_gap={:.1} hug_runs={} node_near={}",
          m.edge_edge_x, m.xgap_max, m.dash_edges, m.xnotches, m.ortho_edges, m.overlap_pairs, m.tight_pairs, m.wiggles, m.edge_frame_pairs, m.edge_node_pairs, m.aspect, m.width, m.height, m.min_edge_gap, m.hug_runs, m.node_near);
    }
}
