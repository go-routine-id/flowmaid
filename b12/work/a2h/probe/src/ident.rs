//! Identity probe: one line of hashes per input file, built against the
//! pre-package and the post-package crate; the two outputs must be equal.
//! Usage: ident <files...>   (ON=1 is only meaningful for the `now` build)
use flowmaid::model::Document;
use flowmaid::scene::{self, Hit, Scene};

fn fnv(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn scene_sig(sc: &Scene) -> String {
    // Geometry a host paints from, not only the SVG.
    let mut s = String::new();
    for n in &sc.nodes {
        s += &format!("{}:{:.3},{:.3},{:.3},{:.3};", n.id, n.x, n.y, n.w, n.h);
    }
    for e in &sc.edges {
        s += &format!("{}>{}:{:?}|{:?};", e.from, e.to, e.bezier, e.waypoints);
    }
    for c in &sc.clusters {
        s += &format!("{}:{:.3},{:.3},{:.3},{:.3};", c.id, c.x, c.y, c.w, c.h);
    }
    s += &format!("{:.3}x{:.3}", sc.width, sc.height);
    s
}

fn picks(sc: &Scene) -> String {
    let mut s = String::new();
    let (w, h) = (sc.width.max(1.0), sc.height.max(1.0));
    let step = (w.max(h) / 60.0).max(4.0);
    let mut y = 0.0;
    while y <= h {
        let mut x = 0.0;
        while x <= w {
            match sc.hit_test(x, y, 4.0) {
                Some(Hit::Node(i)) => s += &format!("n{i},"),
                Some(Hit::Edge(i)) => s += &format!("e{i},"),
                Some(Hit::Cluster(i)) => s += &format!("c{i},"),
                None => s.push('.'),
            }
            x += step;
        }
        y += step;
    }
    let r = sc.hits_in_rect(w * 0.2, h * 0.2, w * 0.7, h * 0.6);
    s += &format!("{r:?}");
    s
}

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    for f in files {
        let name = std::path::Path::new(&f)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let Ok(src) = std::fs::read_to_string(&f) else {
            println!("{name}\tUNREADABLE");
            continue;
        };
        let svg = match std::panic::catch_unwind(|| flowmaid::render_svg(&src)) {
            Ok(Ok(s)) => format!("{:016x}", fnv(&s)),
            Ok(Err(e)) => format!("ERR({})", fnv(&e.to_string())),
            Err(_) => "PANIC".to_string(),
        };
        let doc = flowmaid::parser::parse_document(&src);
        let g = match doc {
            Ok(Document::Flowchart(g)) | Ok(Document::State(g)) => g,
            _ => {
                println!("{name}\tsvg={svg}\tother");
                continue;
            }
        };
        let r = std::panic::catch_unwind(|| {
            let sc = scene::scene(&g);
            let base: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.x, n.y)).collect();
            let mut out = String::new();
            out += &format!("\tscene={:016x}", fnv(&scene_sig(&sc)));
            out += &format!("\tpick={:016x}", fnv(&picks(&sc)));
            let rt = scene::route(&g, &base);
            out += &format!(
                "\troute={:016x}/{:016x}",
                fnv(&scene::to_svg(&rt)),
                fnv(&scene_sig(&rt))
            );
            out += &format!("\trpick={:016x}", fnv(&picks(&rt)));
            // Drags: every node (capped) moved by a few offsets, full
            // route and route_partial.
            let n = base.len();
            let mut hr = 0u64;
            let mut hp = 0u64;
            let stride = (n / 12).max(1);
            for v in (0..n).step_by(stride) {
                for (k, &(dx, dy)) in [(7.0, 5.0), (-90.0, 40.0), (160.0, -120.0), (1.0, 0.0)]
                    .iter()
                    .enumerate()
                {
                    let mut pos = base.clone();
                    pos[v].0 += dx;
                    pos[v].1 += dy;
                    if k < 2 {
                        let r = scene::route(&g, &pos);
                        hr ^= fnv(&scene_sig(&r)).rotate_left((v as u32 * 4 + k as u32) % 63);
                    }
                    let p = scene::route_partial(&g, &pos, &sc, &base);
                    hp ^= fnv(&(scene_sig(&p) + &scene::to_svg(&p)))
                        .rotate_left((v as u32 * 4 + k as u32) % 63);
                    let p2 = scene::route_partial(&g, &pos, &rt, &base);
                    hp ^= fnv(&scene_sig(&p2)).rotate_left((v as u32 * 4 + k as u32 + 7) % 63);
                }
            }
            out += &format!("\tdrag={hr:016x}\tpartial={hp:016x}");
            out += &format!("\tmmd={:016x}", fnv(&flowmaid::to_mermaid(&g)));
            let c = flowmaid::fold::scene_compact(
                &g,
                &flowmaid::fold::CompactOptions::for_extent(600.0),
            );
            out += &format!("\tfold={:016x}", fnv(&scene_sig(&c.scene)));
            // Sizes given by a host.
            let sizes: Vec<(f64, f64)> = sc.nodes.iter().map(|n| (n.w + 10.0, n.h + 6.0)).collect();
            let ss = scene::scene_sized(&g, &sizes);
            out += &format!("\tsized={:016x}", fnv(&scene_sig(&ss)));
            let rs = scene::route_sized(&g, &base, &sizes);
            out += &format!("\trsized={:016x}", fnv(&scene_sig(&rs)));
            out
        });
        match r {
            Ok(s) => println!("{name}\tsvg={svg}{s}"),
            Err(_) => println!("{name}\tsvg={svg}\tPANIC"),
        }
    }
}
