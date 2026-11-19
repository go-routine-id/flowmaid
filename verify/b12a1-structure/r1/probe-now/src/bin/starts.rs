//! starts <files...>: static x_drawn (and routed) with the block ordering forced to k = 1..12 starts,
//! then with the granted starts; blocks on.
use flowmaid::quality::{measure_source, with_block_order_starts, with_blocks};
fn main() {
    let max: usize = std::env::var("KMAX").ok().and_then(|v| v.parse().ok()).unwrap_or(12);
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let mut row = format!("{name:38}");
        for k in 1..=max {
            let rep = with_blocks(true, || with_block_order_starts(k, || measure_source(&src, false))).expect("flowchart");
            row += &format!(" {:>4}", rep.statik.x_drawn);
        }
        let rep = with_blocks(true, || measure_source(&src, false)).expect("flowchart");
        let off = with_blocks(false, || measure_source(&src, false)).expect("flowchart");
        row += &format!(" | granted {:>4} | off {:>4}", rep.statik.x_drawn, off.statik.x_drawn);
        println!("{row}");
    }
}
