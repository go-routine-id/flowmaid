//! Benchmark sederhana: `cargo run --release --example bench`
//! Mode emit:          `cargo run --release --example bench -- emit > big.mmd`

use std::time::Instant;

/// LCG deterministik agar tetap pure-std dan hasilnya reproducible.
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

/// Bangun teks diagram berlapis: `layers` lapis x `width` node per lapis,
/// tiap node punya 2 edge ke lapis berikutnya, plus beberapa back-edge
/// dari lapis terakhir ke lapis pertama (menguji routing terberat).
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

/// Jalankan `f` tiga kali, ambil waktu terbaik (ms) untuk meredam noise.
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

fn main() {
    if std::env::args().nth(1).as_deref() == Some("emit") {
        print!("{}", synth(200, 25));
        return;
    }
    println!(
        "{:>6} {:>6} {:>6} | {:>9} {:>9} {:>9} | {:>8}",
        "node", "edge", "lapis", "parse", "layout", "render*", "SVG"
    );
    let cases: &[(usize, usize)] = &[(10, 5), (20, 10), (50, 20), (100, 25), (200, 25), (2, 2500)];
    for &(l, w) in cases {
        let src = synth(l, w);
        let (g, tp) = best(|| flowrs::parser::parse(&src).unwrap());
        let (_lay, tl) = best(|| flowrs::layout::layout(&g));
        let (svg, tr) = best(|| flowrs::render::render(&g));
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
    println!("* render sudah termasuk memanggil layout di dalamnya");
}
