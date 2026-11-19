// Random drags (many nodes, overlaps, collapses) through route_partial chains: panics only.
use flowmaid::model::Document;
use flowmaid::parser::parse_document;
use flowmaid::{class, er, mindmap};
use std::panic::{catch_unwind, AssertUnwindSafe};
struct R(u64);
impl R { fn f(&mut self) -> f64 { self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); ((self.0 >> 11) as f64) / ((1u64 << 53) as f64) } }
fn main() {
    std::panic::set_hook(Box::new(|_| {}));
    let list = std::env::args().nth(1).unwrap();
    let (mut runs, mut panics) = (0, 0);
    let mut rng = R(42);
    for path in std::fs::read_to_string(&list).unwrap().lines() {
        let Ok(src) = std::fs::read_to_string(path) else { continue };
        let Ok(doc) = parse_document(&src) else { continue };
        let r = catch_unwind(AssertUnwindSafe(|| {
            match &doc {
                Document::Er(d) => {
                    let mut base = er::scene(d);
                    let mut at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                    for step in 0..12 {
                        let mut mv = at.clone();
                        let n = mv.len();
                        if n == 0 { break; }
                        match step % 4 {
                            0 => { let c = mv[0]; for p in mv.iter_mut() { *p = c; } }
                            1 => { for p in mv.iter_mut() { if rng.f() < 0.5 { p.0 += (rng.f() - 0.5) * 800.0; p.1 += (rng.f() - 0.5) * 800.0; } } }
                            2 => { let i = (rng.f() * n as f64) as usize % n; let j = (i + 1) % n; mv[i] = mv[j]; }
                            _ => { let i = (rng.f() * n as f64) as usize % n; mv[i].0 -= 5000.0 * rng.f(); }
                        }
                        let f = er::route_partial(d, &mv, &base, &at);
                        base = f; at = mv;
                    }
                }
                Document::Class(d) => {
                    let mut base = class::scene(d);
                    let mut at: Vec<(f64, f64)> = base.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
                    for step in 0..12 {
                        let mut mv = at.clone();
                        let n = mv.len();
                        if n == 0 { break; }
                        match step % 4 {
                            0 => { let c = mv[0]; for p in mv.iter_mut() { *p = c; } }
                            1 => { for p in mv.iter_mut() { if rng.f() < 0.5 { p.0 += (rng.f() - 0.5) * 800.0; p.1 += (rng.f() - 0.5) * 800.0; } } }
                            2 => { let i = (rng.f() * n as f64) as usize % n; let j = (i + 1) % n; mv[i] = mv[j]; }
                            _ => { let i = (rng.f() * n as f64) as usize % n; mv[i].1 -= 5000.0 * rng.f(); }
                        }
                        let f = class::route_partial(d, &mv, &base, &at);
                        let _ = class::to_svg(&f);
                        let _ = class::card_spots(&f);
                        base = f; at = mv;
                    }
                }
                Document::Mindmap(d) => {
                    let mut base = mindmap::scene(d);
                    let mut at: Vec<(f64, f64)> = base.nodes.iter().map(|n| (n.cx(), n.cy())).collect();
                    for _ in 0..6 {
                        let mut mv = at.clone();
                        for p in mv.iter_mut() { if rng.f() < 0.3 { p.0 += (rng.f() - 0.5) * 900.0; } }
                        let f = mindmap::route_partial(d, &mv, &base, &at);
                        let _ = f.hit_test(10.0, 10.0, 3.0);
                        base = f; at = mv;
                    }
                }
                _ => {}
            }
        }));
        runs += 1;
        if r.is_err() { panics += 1; println!("PANIC {path}"); }
    }
    println!("runs {runs} panics {panics}");
}
