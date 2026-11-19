//! shuf <files...>: width, static crossings, hard zeros, foreign-frame pairs under the six
//! edge-line shuffles of tests/quality.rs::edge_line_order_barely_moves_the_dense_layouts,
//! with the block layout on and off.
use flowmaid::quality::{measure_source, with_blocks};
fn hard(m: &flowmaid::quality::Metrics) -> usize {
    m.edge_node_x + m.shared_ports + m.arrow_on_start + m.wiggles + m.frame_side + m.node_overlap + m.node_frame + m.frame_overlap + m.own_frame_x
}
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let lines: Vec<&str> = src.lines().collect();
        let is_edge = |l: &str| ["-->", "-.->", "==>"].iter().any(|a| l.contains(a));
        let at: Vec<usize> = (0..lines.len()).filter(|&i| is_edge(lines[i])).collect();
        let mut variants: Vec<(String, String)> = vec![("source".into(), src.clone())];
        for seed in 1..=6u64 {
            let mut x = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15);
            let mut order: Vec<&str> = at.iter().map(|&i| lines[i]).collect();
            for i in (1..order.len()).rev() {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                order.swap(i, (x % (i as u64 + 1)) as usize);
            }
            let mut out: Vec<&str> = lines.clone();
            for (&i, l) in at.iter().zip(order) {
                out[i] = l;
            }
            variants.push((format!("shuffle{seed}"), out.join("\n")));
        }
        for on in [false, true] {
            for (v, s) in &variants {
                let rep = with_blocks(on, || measure_source(s, false)).expect("flowchart");
                let m = &rep.statik;
                println!(
                    "{name}\tblocks={}\t{v}\twidth={:.0}\theight={:.0}\tx_drawn={}\tdefects={}\tedge_frame_pairs={}\thard={}\tfan_x={}",
                    u8::from(on), rep.static_scene.width, rep.static_scene.height, m.x_drawn, m.defects(), m.edge_frame_pairs, hard(m), m.fan_x
                );
            }
        }
    }
}
