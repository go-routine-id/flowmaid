//! Simple benchmark: `cargo run --release --example bench`
//! Emit mode:         `cargo run --release --example bench -- emit > big.mmd`
//! Files mode:        `cargo run --release --example bench -- files [--runs N] <paths...>`
//!
//! Files mode times the whole public pipeline, [`flowmaid::render_svg`]
//! (parse + layout + SVG), on real `.mmd` files: per file, 3 warm-up
//! renders, then `N` timed ones (default 30). It prints TSV, one row per
//! file sorted by path so the output order never depends on the shell:
//! `name  type  median_ms  min_ms  svg_bytes`. A file that fails to parse
//! gets type `error` and the message on stderr.

use std::time::Instant;

/// Deterministic LCG to stay pure-std with reproducible results.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n
    }
}

/// Build a layered diagram: `layers` layers x `width` nodes per
/// layer, each node gets 2 edges to the next layer, plus a few
/// back-edges from the last layer to the first (stresses the
/// heaviest routing path).
fn synth(layers: usize, width: usize) -> String {
    let mut s = String::from("flowchart TD\n");
    let mut rng = Lcg(42);
    for l in 0..layers.saturating_sub(1) {
        for w in 0..width {
            let from = l * width + w;
            for _ in 0..2 {
                let to = (l + 1) * width + rng.next(width);
                s.push_str(&format!(
                    "N{}[Node {}] --> N{}[Node {}]\n",
                    from, from, to, to
                ));
            }
        }
    }
    for _ in 0..layers {
        let a = (layers - 1) * width + rng.next(width);
        let b = rng.next(width);
        s.push_str(&format!("N{} --> N{}\n", a, b));
    }
    s
}

/// Run `f` three times, keep the best time (ms) to dampen noise.
fn best<T>(mut f: impl FnMut() -> T) -> (T, f64) {
    let mut t_best = f64::INFINITY;
    let mut out = None;
    for _ in 0..3 {
        let t = Instant::now();
        let v = f();
        t_best = t_best.min(t.elapsed().as_secs_f64() * 1e3);
        out = Some(v);
    }
    (out.unwrap(), t_best)
}

/// Warm-up renders per file before timing, so first-call costs (page
/// faults, allocator growth) stay out of the median.
const FILE_WARMUPS: usize = 3;
/// Timed renders per file unless `--runs N` says otherwise.
const FILE_RUNS: usize = 30;

/// The diagram type of a parsed document, as its header names it.
fn doc_type(doc: &flowmaid::Document) -> &'static str {
    use flowmaid::Document as D;
    match doc {
        D::Flowchart(_) => "flowchart",
        D::State(_) => "state",
        D::Er(_) => "er",
        D::Class(_) => "class",
        D::Sequence(_) => "sequence",
        D::Pie(_) => "pie",
        D::Mindmap(_) => "mindmap",
        D::Journey(_) => "journey",
        D::GitGraph(_) => "gitgraph",
        D::Architecture(_) => "architecture",
        D::Timeline(_) => "timeline",
        D::Sankey(_) => "sankey",
    }
}

/// `files [--runs N] <paths...>`: median and min wall time of
/// `render_svg` per file, as TSV.
fn files(args: &[String]) {
    let mut runs = FILE_RUNS;
    let mut paths = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--runs" {
            runs = it
                .next()
                .and_then(|n| n.parse().ok())
                .filter(|&n| n > 0)
                .expect("--runs takes a positive number");
        } else {
            paths.push(a.as_str());
        }
    }
    paths.sort_unstable();
    paths.dedup();
    println!("name\ttype\tmedian_ms\tmin_ms\tsvg_bytes");
    for path in paths {
        let name = std::path::Path::new(path)
            .file_stem()
            .map_or(path.into(), |s| s.to_string_lossy());
        let src =
            std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {}", path, e));
        let kind = match flowmaid::parser::parse_document(&src) {
            Ok(doc) => doc_type(&doc),
            Err(e) => {
                eprintln!("{}: {}", path, e);
                println!("{}\terror\t-\t-\t0", name);
                continue;
            }
        };
        let mut bytes = 0;
        for _ in 0..FILE_WARMUPS {
            bytes = flowmaid::render_svg(&src).map_or(0, |s| s.len());
        }
        let mut ts: Vec<f64> = (0..runs)
            .map(|_| {
                let t = Instant::now();
                let svg = flowmaid::render_svg(&src);
                let dt = t.elapsed().as_secs_f64() * 1e3;
                drop(std::hint::black_box(svg));
                dt
            })
            .collect();
        ts.sort_by(f64::total_cmp);
        println!(
            "{}\t{}\t{:.3}\t{:.3}\t{}",
            name,
            kind,
            ts[ts.len() / 2],
            ts[0],
            bytes
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("files") {
        files(&args[1..]);
        return;
    }
    if args.first().map(String::as_str) == Some("emit") {
        print!("{}", synth(200, 25));
        return;
    }
    println!(
        "{:>6} {:>6} {:>6} | {:>9} {:>9} {:>9} | {:>8}",
        "nodes", "edges", "layers", "parse", "layout", "render*", "SVG"
    );
    let cases: &[(usize, usize)] = &[(10, 5), (20, 10), (50, 20), (100, 25), (200, 25), (2, 2500)];
    for &(l, w) in cases {
        let src = synth(l, w);
        let (g, tp) = best(|| flowmaid::parser::parse(&src).unwrap());
        let (_lay, tl) = best(|| flowmaid::layout::layout(&g));
        let (svg, tr) = best(|| flowmaid::render::render(&g));
        println!(
            "{:>6} {:>6} {:>6} | {:>7.2}ms {:>7.2}ms {:>7.2}ms | {:>6}KB",
            g.nodes.len(),
            g.edges.len(),
            l,
            tp,
            tl,
            tr,
            svg.len() / 1024
        );
    }
    println!("* render includes running layout internally");
}
