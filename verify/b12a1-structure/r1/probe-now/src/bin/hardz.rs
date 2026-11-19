//! hardz <files...>: hard zeros (the sum tests/quality.rs::block_fixtures_are_not_drawn_worse takes), static and
//! routed, with the block layout off and on, with the parts; plus defects, x_drawn, edge_frame_pairs, overlap_pairs, fan_x.
use flowmaid::quality::{measure_source, with_blocks, Metrics};
fn parts(m: &Metrics) -> (usize, String) {
    let v = [
        ("edge_node_x", m.edge_node_x), ("shared_ports", m.shared_ports), ("arrow_on_start", m.arrow_on_start),
        ("wiggles", m.wiggles), ("frame_side", m.frame_side), ("node_overlap", m.node_overlap),
        ("node_frame", m.node_frame), ("frame_overlap", m.frame_overlap), ("own_frame_x", m.own_frame_x),
    ];
    let s: usize = v.iter().map(|x| x.1).sum();
    (s, v.iter().filter(|x| x.1 > 0).map(|x| format!("{}={}", x.0, x.1)).collect::<Vec<_>>().join(","))
}
fn main() {
    let all = std::env::var("ALL").is_ok();
    for f in std::env::args().skip(1) {
        let Ok(src) = std::fs::read_to_string(&f) else { continue };
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let (Ok(off), Ok(on)) = (
            std::panic::catch_unwind(|| with_blocks(false, || measure_source(&src, false))),
            std::panic::catch_unwind(|| with_blocks(true, || measure_source(&src, false))),
        ) else { println!("{name}\tPANIC"); continue };
        let (Ok(off), Ok(on)) = (off, on) else { continue };
        for (mode, a, b) in [("static", &off.statik, &on.statik), ("routed", &off.routed, &on.routed)] {
            let (ha, pa) = parts(a);
            let (hb, pb) = parts(b);
            let worse = hb > ha || b.defects() > a.defects() || b.x_drawn > a.x_drawn || b.edge_frame_pairs > a.edge_frame_pairs || b.overlap_pairs > a.overlap_pairs || b.fan_x > a.fan_x;
            if all || worse {
                println!(
                    "{name}\t{mode}\tbm={}\trows_eq={}\thard {ha}->{hb} [{pa}]->[{pb}]\tdefects {}->{}\tx_drawn {}->{}\tefp {}->{}\tovl {}->{}\tfan_x {}->{}",
                    u8::from(on.block_mode), u8::from(on.rows_equal), a.defects(), b.defects(), a.x_drawn, b.x_drawn, a.edge_frame_pairs, b.edge_frame_pairs, a.overlap_pairs, b.overlap_pairs, a.fan_x, b.fan_x
                );
            }
        }
    }
}
