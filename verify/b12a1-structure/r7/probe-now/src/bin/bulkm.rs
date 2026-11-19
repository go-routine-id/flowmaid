//! Bulk metrics probe: per input, static and routed rows with the block
//! layout on (MODE=on, default) or off. One line per (file, mode).
use flowmaid::quality::{measure_source, with_blocks, Metrics};

fn row(m: &Metrics) -> String {
    let hard = m.edge_node_x + m.shared_ports + m.arrow_on_start + m.wiggles + m.frame_side
        + m.node_overlap + m.node_frame + m.frame_overlap + m.own_frame_x;
    format!(
        "hard={hard}\tenx={}\tsp={}\taos={}\twig={}\tfside={}\tnov={}\tnfr={}\tfov={}\town={}\tefp={}\tenp={}\tovl={}\tx={}\tfan={}\tgaps={}\tgapmax={:.1}\tdef={}\tchip={}\tlbl_node={}\tlbl_chip={}\tW={:.0}\tH={:.0}",
        m.edge_node_x, m.shared_ports, m.arrow_on_start, m.wiggles, m.frame_side, m.node_overlap,
        m.node_frame, m.frame_overlap, m.own_frame_x, m.edge_frame_pairs, m.edge_node_pairs,
        m.overlap_pairs, m.x_drawn, m.fan_x, m.end_gaps, m.end_gap_max, m.defects(), m.edge_chip,
        m.lbl_node, m.lbl_chip, m.width, m.height
    )
}

fn main() {
    let on = std::env::var("MODE").map(|m| m != "off").unwrap_or(true);
    for f in std::env::args().skip(1) {
        let name = std::path::Path::new(&f).file_name().unwrap().to_string_lossy().to_string();
        let Ok(src) = std::fs::read_to_string(&f) else { continue };
        let r = std::panic::catch_unwind(|| with_blocks(on, || measure_source(&src, false)));
        match r {
            Ok(Ok(rep)) => {
                println!("{name}\tstatic\tbm={}\tdet={}\t{}", u8::from(rep.block_mode), u8::from(rep.deterministic), row(&rep.statik));
                println!("{name}\trouted\tbm={}\tdet={}\t{}", u8::from(rep.block_mode), u8::from(rep.deterministic), row(&rep.routed));
            }
            Ok(Err(_)) => {}
            Err(_) => println!("{name}\tPANIC"),
        }
    }
}
