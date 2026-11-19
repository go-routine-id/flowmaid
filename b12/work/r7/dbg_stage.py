import os
R=os.environ['DBGTREE']+'/'
def sub1(p,old,new):
    s=open(R+p).read()
    assert s.count(old)==1,(p,old[:60],s.count(old))
    open(R+p,'w').write(s.replace(old,new))
sub1('src/edgepass.rs',"""/// The edge stages over `sc` with its context built.
fn stages(""","""pub(crate) fn xxstage(name: &str, g: &Graph, sc: &Scene) { // XXDBG
    if std::env::var("FM_XXSTAGE").is_err() { return; } // XXDBG
    let m = crate::quality::measure(g, sc); // XXDBG
    eprintln!("XXSTAGE {name}: fan_x {} x_drawn {} edge_node_x {} overlap {}", m.fan_x, m.x_drawn, m.edge_node_x, m.overlap_pairs); // XXDBG
    if std::env::var("FM_XXFAN").is_ok() { for d in &m.details { if d.starts_with("fan_x") || d.contains(" overlap ") || d.contains(" touch ") { eprintln!("XXSTAGE   {d}"); } } } // XXDBG
    let want = std::env::var("FM_XXE").unwrap_or_default(); // XXDBG
    for e in &sc.edges { // XXDBG
        let id = format!("{}->{}", e.from, e.to); // XXDBG
        if !want.split(',').any(|w| w == id) { continue; } // XXDBG
        let pts: Vec<(f64, f64)> = if e.waypoints.is_empty() { e.bezier.to_vec() } else { e.waypoints.clone() }; // XXDBG
        let mut out: Vec<(f64, f64)> = Vec::new(); // XXDBG
        for (i, &p) in pts.iter().enumerate() { if i == 0 || i + 1 == pts.len() { out.push(p); continue; } let (a, b) = (pts[i - 1], pts[i + 1]); let cr = (p.0 - a.0) * (b.1 - p.1) - (p.1 - a.1) * (b.0 - p.0); if cr.abs() > 1e-6 { out.push(p); } } // XXDBG
        eprintln!("XXSTAGE   {id} {}", out.iter().map(|p| format!("({:.0},{:.0})", p.0, p.1)).collect::<Vec<_>>().join(" ")); // XXDBG
    } // XXDBG
} // XXDBG

/// The edge stages over `sc` with its context built.
fn stages(""")
sub1('src/edgepass.rs', "    let xxdbg = |tag: &str, sc: &Scene| {", "    let xxdbg = |tag: &str, sc: &Scene| { xxstage(tag, g, sc);")
print("stage dbg on")
