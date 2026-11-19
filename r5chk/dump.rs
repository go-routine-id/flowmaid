//! scratch: dump scene edges of a file (DUMP_FILE), DUMP_EDGES=comma list of indices
#[test]
fn dump_edges() {
    let f = std::env::var("DUMP_FILE").unwrap();
    let src = std::fs::read_to_string(&f).unwrap();
    let want: Vec<usize> = std::env::var("DUMP_EDGES").unwrap_or_default().split(',').filter_map(|s| s.parse().ok()).collect();
    let doc = crate::parser::parse_document(&src).unwrap();
    let mut out = String::new();
    use std::fmt::Write as _;
    match doc {
        crate::Document::Class(d) => {
            let b = crate::class::scene(&d);
            for (i, e) in b.scene.edges.iter().enumerate() {
                if want.is_empty() || want.contains(&i) {
                    let _ = writeln!(out, "class edge {i} {}->{} bez {:?} wps {}", e.from, e.to, e.bezier, e.waypoints.len());
                    if !want.is_empty() { let _ = writeln!(out, "   wps {:?}", e.waypoints); }
                }
            }
        }
        crate::Document::Flowchart(g) | crate::Document::State(g) => {
            let s0 = crate::scene::scene(&g);
            let s = if std::env::var("DUMP_ROUTE").is_ok() {
                let c: Vec<(f64, f64)> = s0.nodes.iter().map(|n| (n.x, n.y)).collect();
                crate::scene::route(&g, &c)
            } else { s0 };
            for c in &s.clusters { let _ = writeln!(out, "cluster {} {:.1},{:.1} {:.1}x{:.1}", c.id, c.x, c.y, c.w, c.h); }
            for n in &s.nodes { let _ = writeln!(out, "node {} {:.1},{:.1} {:.1}x{:.1}", n.id, n.x, n.y, n.w, n.h); }
            for (i, e) in s.edges.iter().enumerate() {
                if want.is_empty() || want.contains(&i) {
                    let _ = writeln!(out, "edge {i} {}->{} bez {:?} wps {}", e.from, e.to, e.bezier, e.waypoints.len());
                    if !want.is_empty() { let _ = writeln!(out, "   wps {:?}", e.waypoints.iter().map(|p| ((p.0*10.0).round()/10.0, (p.1*10.0).round()/10.0)).collect::<Vec<_>>()); }
                }
            }
        }
        _ => {}
    }
    std::fs::write(std::env::var("DUMP_OUT").unwrap(), out).unwrap();
}

/// scratch: per class file, how many relation edges keep a curve (not orthogonal)
#[test]
fn class_curves() {
    let list = std::fs::read_to_string(std::env::var("CC_LIST").unwrap()).unwrap();
    let mut out = String::new();
    use std::fmt::Write as _;
    for f in list.lines() {
        let f = f.split_whitespace().last().unwrap_or("");
        let Ok(src) = std::fs::read_to_string(f) else { continue };
        let Ok(doc) = crate::parser::parse_document(&src) else { continue };
        let sc = match doc {
            crate::Document::Class(d) => {
                let c = crate::class::scene(&d);
                let centres: Vec<(f64, f64)> = c.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                let r = crate::class::route(&d, &centres);
                vec![c.scene, r.scene]
            }
            _ => continue,
        };
        let mut counts = Vec::new();
        for s in &sc {
            let mut curved = Vec::new();
            for (i, e) in s.edges.iter().enumerate() {
                if e.from == e.to { continue; }
                let ortho = e.waypoints.len() >= 2 && e.waypoints.windows(2).all(|w| (w[0].0 - w[1].0).abs() < 0.5 || (w[0].1 - w[1].1).abs() < 0.5);
                if !ortho { curved.push(i); }
            }
            counts.push(format!("{} {:?}", curved.len(), curved));
        }
        let _ = writeln!(out, "{f}\t{}\t{}\t{:.0}x{:.0}", counts[0], counts[1], sc[0].width, sc[0].height);
    }
    std::fs::write(std::env::var("CC_OUT").unwrap(), out).unwrap();
}
