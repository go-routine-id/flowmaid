//! Block-mode harness rows per file: static and routed.
use flowmaid::quality::{measure_source, with_blocks};
fn main() {
    for f in std::env::args().skip(1) {
        let name = std::path::Path::new(&f).file_stem().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(&f).unwrap();
        match with_blocks(true, || measure_source(&src, false)) {
            Ok(r) => {
                let s = &r.statik; let t = &r.routed;
                println!("{name}\tbm={}\tS: lat={} x={} tcut={} fan={} lblchip={} lbledge={} def={}\tR: lat={} x={} tcut={} fan={} def={}",
                    u8::from(r.block_mode), s.lateral_x, s.x_drawn, s.title_cut, s.fan_x, s.lbl_chip, s.lbl_edge, s.defects(), t.lateral_x, t.x_drawn, t.title_cut, t.fan_x, t.defects());
            }
            Err(e) => println!("{name}\tERR {e}"),
        }
    }
}
