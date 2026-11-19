//! Verifier's probe (scratch tree only): per input, the static harness
//! row with the block layout off and on, in one process.
//! Usage: bulkcmp <files...>
use flowmaid::quality::{measure_source, with_blocks, Metrics};

fn hard(m: &Metrics) -> (usize, String) {
    let parts = [
        ("edge_node_x", m.edge_node_x),
        ("shared_ports", m.shared_ports),
        ("arrow_on_start", m.arrow_on_start),
        ("wiggles", m.wiggles),
        ("frame_side", m.frame_side),
        ("node_overlap", m.node_overlap),
        ("node_frame", m.node_frame),
        ("frame_overlap", m.frame_overlap),
        ("own_frame_x", m.own_frame_x),
    ];
    let n = parts.iter().map(|p| p.1).sum();
    let s = parts
        .iter()
        .filter(|p| p.1 > 0)
        .map(|p| format!("{}={}", p.0, p.1))
        .collect::<Vec<_>>()
        .join(",");
    (n, s)
}

fn main() {
    for f in std::env::args().skip(1) {
        let name = std::path::Path::new(&f)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let Ok(src) = std::fs::read_to_string(&f) else {
            continue;
        };
        let r = std::panic::catch_unwind(|| {
            let off = with_blocks(false, || measure_source(&src, false));
            let on = with_blocks(true, || measure_source(&src, false));
            (off, on)
        });
        match r {
            Ok((Ok(a), Ok(b))) => {
                for (mode, x, y) in [("static", &a.statik, &b.statik), ("routed", &a.routed, &b.routed)] {
                    let (ha, sa) = hard(x);
                    let (hb, sb) = hard(y);
                    println!(
                        "{name}\t{mode}\tbm={}\trows_eq={}\thard\t{ha}\t{hb}\t[{sa}]->[{sb}]\tdefects\t{}\t{}\tx_drawn\t{}\t{}\tefp\t{}\t{}\tovl\t{}\t{}\tfan_x\t{}\t{}",
                        u8::from(b.block_mode),
                        u8::from(b.rows_equal),
                        x.defects(),
                        y.defects(),
                        x.x_drawn,
                        y.x_drawn,
                        x.edge_frame_pairs,
                        y.edge_frame_pairs,
                        x.overlap_pairs,
                        y.overlap_pairs,
                        x.fan_x,
                        y.fan_x
                    );
                }
            }
            Ok(_) => {}
            Err(_) => println!("{name}\tPANIC"),
        }
    }
}
