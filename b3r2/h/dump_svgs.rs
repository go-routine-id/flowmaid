use crate::model::Document;
use crate::scene::{route, scene, to_svg};
#[test]
fn dump_svgs() {
    let list = std::env::var("DS_LIST").unwrap();
    let out = std::env::var("DS_OUT").unwrap();
    std::fs::create_dir_all(&out).unwrap();
    for path in std::fs::read_to_string(&list).unwrap().lines().map(str::trim).filter(|l| !l.is_empty()) {
        let name = std::path::Path::new(path).file_stem().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(path).unwrap();
        let g = match crate::parser::parse_document(&src) {
            Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g,
            _ => continue,
        };
        let st = scene(&g);
        let c: Vec<(f64, f64)> = st.nodes.iter().map(|n| (n.x, n.y)).collect();
        let rt = route(&g, &c);
        for (m, sc) in [("static", &st), ("routed", &rt)] {
            for d in crate::quality::measure(&g, sc).details.iter().filter(|d| d.contains("lbl_") || d.contains("gapnode")) {
                println!("{name} {m} {d}");
            }
        }
        std::fs::write(format!("{out}/{name}.static.svg"), to_svg(&st)).unwrap();
        std::fs::write(format!("{out}/{name}.routed.svg"), to_svg(&rt)).unwrap();
        if let Ok(pairs) = std::env::var("DS_EDGES") {
            for pair in pairs.split(',') {
                let Some((f, t)) = pair.split_once("->") else { continue };
                for (m, sc) in [("static", &st), ("routed", &rt)] {
                    for e in sc.edges.iter().filter(|e| e.from == f && e.to == t) {
                        println!("{name} {m} {f}->{t} bezier={:?}", e.bezier.iter().map(|p| (p.0.round(), p.1.round())).collect::<Vec<_>>());
                        println!("{name} {m} {f}->{t} wps={:?}", e.waypoints.iter().map(|p| ((p.0*10.0).round()/10.0, (p.1*10.0).round()/10.0)).collect::<Vec<_>>());
                    }
                }
            }
        }
    }
}
