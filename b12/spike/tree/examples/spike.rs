//! Corridor spike harness (B12.2, throwaway): route a diagram at given
//! block positions and time it.
//!   spike <file.mmd> <file.pos> [--svg out.svg]
//! Prints: name route_ms drag_ms(worst) drag_mean frames edge_frame_x edge_frame_pairs x_drawn edge_node_x defects lateral_x
use flowmaid::quality::{self, fastest_ms, DRAG_RUNS, ROUTE_RUNS};
use flowmaid::Document;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let src = std::fs::read_to_string(&args[0]).expect("read mmd");
    let pos = std::fs::read_to_string(&args[1]).expect("read pos");
    let g = match flowmaid::parser::parse_document(&src).expect("parses") {
        Document::Flowchart(g) | Document::State(g) => g,
        _ => panic!("not a flowchart"),
    };
    let mut centres = vec![(0.0, 0.0); g.nodes.len()];
    let mut sizes = vec![(0.0, 0.0); g.nodes.len()];
    for line in pos.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        let i = g.nodes.iter().position(|n| n.id == f[0]).expect("node id");
        centres[i] = (f[1].parse().unwrap(), f[2].parse().unwrap());
        sizes[i] = (f[3].parse().unwrap(), f[4].parse().unwrap());
    }
    let routed = flowmaid::scene::route_sized(&g, &centres, &sizes);
    if std::env::var_os("FM_SPIKE_DBG2").is_some() { return; }
    let route_ms = fastest_ms(ROUTE_RUNS, || flowmaid::scene::route_sized(&g, &centres, &sizes));
    let prof = quality::prof_take();
    eprintln!("stages(ms): {}", prof.iter().map(|(k, v)| format!("{k} {v:.2}")).collect::<Vec<_>>().join(", "));
    let t0 = fastest_ms(ROUTE_RUNS, || flowmaid::scene::route_sized(&g, &centres, &sizes));
    let _ = t0;
    let frames = quality::drag_frames(&routed);
    let (mut worst, mut sum) = (0.0f64, 0.0);
    for (_, c) in &frames {
        let ms = fastest_ms(DRAG_RUNS, || flowmaid::scene::route_sized(&g, c, &sizes));
        worst = worst.max(ms);
        sum += ms;
    }
    let m = quality::measure(&g, &routed);
    let name = std::path::Path::new(&args[0]).file_stem().unwrap().to_string_lossy().to_string();
    println!(
        "{name}\troute_ms {route_ms:.2}\tdrag_ms {worst:.2}\tdrag_mean {:.2}\tframes {}\tedge_frame_x {}\tedge_frame_pairs {}\tx_drawn {}\tedge_node_x {}\tdefects {}\tlateral_x {}\tlbl_edge {}\tchip_crowd {}",
        sum / frames.len().max(1) as f64,
        frames.len(),
        m.edge_frame_x,
        m.edge_frame_pairs,
        m.x_drawn,
        m.edge_node_x,
        m.defects(),
        m.lateral_x,
        m.lbl_edge,
        m.chip_crowd,
    );
    if let Some(i) = args.iter().position(|a| a == "--svg") {
        std::fs::write(&args[i + 1], flowmaid::scene::to_svg(&routed)).expect("write svg");
    }
}
