use flowmaid::{parser::parse_document, Document};
fn main() {
    for f in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&f).unwrap();
        let g = match parse_document(&src).unwrap() { Document::Flowchart(g) | Document::State(g) => g, _ => continue };
        let mut c = [0usize; 4]; let mut n = 0;
        let mut ex = Vec::new();
        for nd in &g.nodes { if nd.label.contains('\n') { continue; } n += 1; let w = flowmaid::layout::text_width(&nd.label);
            for (i, t) in [120.0, 150.0, 175.0, 200.0].iter().enumerate() { if w > *t { c[i] += 1; } }
            if w > 120.0 && ex.len() < 4 { ex.push(format!("{:.0}:{}", w, nd.label)); } }
        let el = g.edges.iter().filter_map(|e| e.label.as_ref()).filter(|l| !l.contains('\n') && flowmaid::layout::text_width(l) > 200.0).count();
        println!("{f}: nodes(no-break)={n} >120:{} >150:{} >175:{} >200:{} | edge labels >200: {el} | {:?}", c[0], c[1], c[2], c[3], ex);
    }
}
