use flowmaid::scene::{scene, route, SceneEdge};
type P=(f64,f64);
fn cross(p:P,q:P,r:P,s:P)->bool{ let o=|a:P,b:P,c:P|(b.0-a.0)*(c.1-a.1)-(b.1-a.1)*(c.0-a.0); let (d1,d2,d3,d4)=(o(r,s,p),o(r,s,q),o(p,q,r),o(p,q,s)); d1*d2< -1e-6 && d3*d4< -1e-6 }
fn poly(e:&SceneEdge)->Vec<P>{ if e.waypoints.len()>=2 { e.waypoints.clone() } else { let b=e.bezier; (0..=24).map(|k|{let t=k as f64/24.0; let u=1.0-t; let w=[u*u*u,3.0*u*u*t,3.0*u*t*t,t*t*t]; (b.iter().zip(w).map(|(p,k)|p.0*k).sum(), b.iter().zip(w).map(|(p,k)|p.1*k).sum())}).collect() } }
fn main(){
    let mut tot=[0usize;2]; let mut files=[0usize;2]; let mut ex=vec![];
    for f in std::env::args().skip(1){
        let src=std::fs::read_to_string(&f).unwrap();
        let g = match flowmaid::parser::parse_document(&src) { Ok(flowmaid::model::Document::Flowchart(g)) | Ok(flowmaid::model::Document::State(g)) => g, _ => continue };
        let s=scene(&g); let c:Vec<P>=s.nodes.iter().map(|n|(n.x,n.y)).collect(); let r=route(&g,&c);
        for (m,sc) in [(0,&s),(1,&r)] {
            let ps:Vec<Vec<P>>=sc.edges.iter().map(poly).collect(); let mut here=0;
            for i in 0..sc.edges.len(){ for j in i+1..sc.edges.len(){
                let (a,b)=(&sc.edges[i],&sc.edges[j]);
                if a.from==a.to || b.from==b.to { continue; }
                let share = a.from==b.from || a.to==b.to;
                if !share { continue; }
                let mut n=0; for x in ps[i].windows(2){ for y in ps[j].windows(2){ if cross(x[0],x[1],y[0],y[1]) {n+=1;} } }
                if n>=2 { here+=1; if m==0 && ex.len()<8 { ex.push(format!("{} {}->{} x {}->{} ({n})", std::path::Path::new(&f).file_stem().unwrap().to_string_lossy(), a.from,a.to,b.from,b.to)); } }
            }}
            tot[m]+=here; if here>0 {files[m]+=1;}
        }
    }
    println!("sibling pairs crossing >=2 times: static {} in {} files, routed {} in {} files", tot[0], files[0], tot[1], files[1]);
    for e in ex { println!("  {e}"); }
}
