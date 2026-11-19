fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (mode, path, cf) = (&a[0], &a[1], &a[2]);
    let src = std::fs::read_to_string(path).unwrap();
    let g = flowmaid::parser::parse(&src).unwrap();
    let s = flowmaid::scene::scene(&g);
    let centres: Vec<(f64, f64)> = if mode == "dump" {
        let c: Vec<(f64,f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
        std::fs::write(cf, c.iter().map(|p| format!("{:?} {:?}", p.0, p.1)).collect::<Vec<_>>().join("\n")).unwrap();
        c
    } else if mode == "own" {
        s.nodes.iter().map(|n| (n.x, n.y)).collect()
    } else {
        std::fs::read_to_string(cf).unwrap().lines().map(|l| { let v: Vec<f64> = l.split(' ').map(|x| x.parse().unwrap()).collect(); (v[0], v[1]) }).collect()
    };
    let r = flowmaid::scene::route(&g, &centres);
    let m = flowmaid::quality::measure(&g, &r);
    println!("{mode}: defects={} overlap_pairs={} corner_touch={} edge_chip={} xnotches={} band_pairs={}", m.defects(), m.overlap_pairs, m.corner_touch, m.edge_chip, m.xnotches, m.band_pairs);
}
