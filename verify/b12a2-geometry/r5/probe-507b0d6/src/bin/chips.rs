use flowmaid::quality::{measure_source, with_blocks};
fn main() {
    for f in std::env::args().skip(1) {
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(&f).unwrap();
        if let Ok(r) = with_blocks(true, || measure_source(&src, false)) {
            println!("{name}\tbm={}\tS edge_chip={} title_cut={} chip_crowd={} lateral_x={}\tR edge_chip={} title_cut={} lateral_x={}", u8::from(r.block_mode), r.statik.edge_chip, r.statik.title_cut, r.statik.chip_crowd, r.statik.lateral_x, r.routed.edge_chip, r.routed.title_cut, r.routed.lateral_x);
        }
    }
}
