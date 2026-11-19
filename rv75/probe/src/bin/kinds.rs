fn main() {
    let path = std::env::args().nth(1).unwrap();
    let src = std::fs::read_to_string(&path).unwrap();
    let g = flowmaid::parser::parse(&src).unwrap();
    let sc = flowmaid::scene::scene(&g);
    let (mut cubic, mut spline, mut ortho) = (vec![], vec![], vec![]);
    for e in &sc.edges {
        let name = format!("{}->{}", e.from, e.to);
        if e.waypoints.is_empty() { cubic.push(name); continue; }
        let w = &e.waypoints;
        let mut axis = 0.0; let mut tot = 0.0;
        for p in w.windows(2) { let dx = (p[1].0-p[0].0).abs(); let dy=(p[1].1-p[0].1).abs(); let l=(dx*dx+dy*dy).sqrt(); tot+=l; if dx<0.5 || dy<0.5 { axis+=l; } }
        if axis/tot > 0.9 { ortho.push(name) } else { spline.push(format!("{} ({} wps, {:.0}% axis)", name, w.len(), 100.0*axis/tot)) }
    }
    println!("single cubic {}: {:?}\nfree spline {}: {:?}\northo {}", cubic.len(), cubic, spline.len(), spline, ortho.len());
}
