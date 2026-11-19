use flowmaid::model::Document;
fn cub(q:&[(f64,f64);4],t:f64)->(f64,f64){let u=1.0-t;(u*u*u*q[0].0+3.0*u*u*t*q[1].0+3.0*u*t*t*q[2].0+t*t*t*q[3].0,u*u*u*q[0].1+3.0*u*u*t*q[1].1+3.0*u*t*t*q[2].1+t*t*t*q[3].1)}
fn main() {
    let path = std::env::args().nth(1).unwrap();
    let verbose = std::env::args().nth(2).is_some();
    let src = std::fs::read_to_string(path).unwrap();
    let d = match flowmaid::parser::parse_document(&src).unwrap() { Document::Class(d) => d, _ => panic!() };
    let cs = flowmaid::class::scene(&d);
    let mut hits=0; let mut wp=0; let mut detached=0;
    for e in &cs.scene.edges {
        if !e.waypoints.is_empty(){wp+=1;}
        for n in &cs.scene.nodes {
            if n.id==e.from || n.id==e.to {continue;}
            let inside=(1..100).map(|i|cub(&e.bezier,i as f64/100.0)).any(|p|(p.0-n.x).abs()<n.w/2.0-1.0&&(p.1-n.y).abs()<n.h/2.0-1.0);
            if inside {hits+=1; println!("HIT edge {}->{} passes through box {}", e.from,e.to,n.id);}
        }
        if let Some((t,m,_w))=&e.label { let dmin=(0..=200).map(|i|cub(&e.bezier,i as f64/200.0)).map(|p|((p.0-m.0).powi(2)+(p.1-m.1).powi(2)).sqrt()).fold(f64::INFINITY,f64::min); if dmin>6.0 {detached+=1; println!("DETACHED label {:?} {:.1}px from its drawn curve ({}->{})",t,dmin,e.from,e.to);} }
        if verbose { println!("edge {}->{} bez={:?} wps={:?}", e.from,e.to,e.bezier,e.waypoints); }
    }
    println!("edges={} with_waypoints={} box_hits={} detached_labels={}", cs.scene.edges.len(), wp, hits, detached);
}
