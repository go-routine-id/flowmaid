//! Layout-quality report for a directory of flowchart `.mmd` files.
//!
//! ```text
//! cargo run --release --example quality -- <dir> [--svg <out-dir>]
//! FLOWMAID_QUALITY_DIR=<dir> cargo run --release --example quality
//! cargo run --release --example quality -- <dir> --drag
//! ```
//!
//! Prints one TSV row per file and mode (`static` = the layout,
//! `routed` = `scene::route` fed the static centres, what the
//! playground draws), then a totals row per mode. Columns are
//! documented on `flowmaid::quality::Metrics`; `defects` is
//! `Metrics::defects`, `route_ms` the fastest `route()` call.
//! `--svg` also writes `<name>.static.svg` and `<name>.routed.svg`;
//! `--details` lists every crossing / wiggle / shared port on stderr.
//! The last columns are timings: `scene_ms` (fastest `scene()`), and
//! with `--drag` the drag benchmark — every node moved along the
//! drawing's diagonal and nudged each way (`quality::drag_frames`),
//! `route()` timed per frame: `drag_ms` the worst frame, `drag_mean`
//! the mean, `drag_over` how many frames exceed
//! `quality::DRAG_FRAME_MS` (all 0 without `--drag`).

use flowmaid::model::Graph;
use flowmaid::quality::{self, Agg, Metrics};
use flowmaid::scene::Scene;
use flowmaid::Document;
use std::path::{Path, PathBuf};

fn usage() -> ! {
    eprintln!("usage: quality <dir> [--svg <out-dir>] [--details]  (or set FLOWMAID_QUALITY_DIR)");
    std::process::exit(2)
}

/// Fold one column of `rows` the way the metric says.
fn fold(vals: &[f64], agg: Agg) -> f64 {
    match agg {
        Agg::Sum => vals.iter().sum(),
        Agg::Min => vals.iter().copied().fold(f64::INFINITY, f64::min),
        Agg::Max => vals.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        Agg::Mean => vals.iter().sum::<f64>() / vals.len().max(1) as f64,
    }
}

/// Integers print bare, everything else with two decimals.
fn num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v:.2}")
    }
}

/// One measured row: metrics, `route_ms`, `deterministic`, timings.
type Row = (Metrics, f64, f64, [f64; 4]);

/// Timing columns after the late fields, with how totals fold them.
const TIMINGS: [(&str, Agg); 4] = [
    ("scene_ms", Agg::Max),
    ("drag_ms", Agg::Max),
    ("drag_mean", Agg::Mean),
    ("drag_over", Agg::Sum),
];

fn row(name: &str, mode: &str, m: &Metrics, extra: &[f64], timing: &[f64]) -> String {
    let mut cols = vec![name.to_string(), mode.to_string()];
    cols.extend(m.fields().iter().map(|&(_, v, _)| num(v)));
    cols.push(num(m.defects() as f64));
    cols.extend(extra.iter().map(|&v| num(v)));
    cols.extend(m.late_fields().iter().map(|&(_, v, _)| num(v)));
    cols.extend(timing.iter().map(|&v| num(v)));
    cols.join("\t")
}

/// Worst, mean and over-budget count of `route()` over every drag
/// frame of the diagram (fastest of `quality::DRAG_RUNS` per frame).
fn drag(g: &Graph, statik: &Scene) -> [f64; 3] {
    let frames = quality::drag_frames(statik);
    let (mut worst, mut sum, mut over) = (0.0f64, 0.0, 0.0);
    for (_, c) in &frames {
        let ms = quality::fastest_ms(quality::DRAG_RUNS, || flowmaid::scene::route(g, c));
        worst = worst.max(ms);
        sum += ms;
        if ms > quality::DRAG_FRAME_MS {
            over += 1.0;
        }
    }
    [worst, sum / frames.len().max(1) as f64, over]
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut dir: Option<PathBuf> = None;
    let mut svg_out: Option<PathBuf> = None;
    let mut details = false;
    let mut dragging = false;
    while let Some(a) = args.next() {
        if a == "--details" {
            details = true;
        } else if a == "--drag" {
            dragging = true;
        } else if a == "--svg" {
            svg_out = Some(args.next().map(PathBuf::from).unwrap_or_else(|| usage()));
        } else if dir.is_none() {
            dir = Some(PathBuf::from(a));
        } else {
            usage();
        }
    }
    let dir = dir
        .or_else(|| std::env::var_os("FLOWMAID_QUALITY_DIR").map(PathBuf::from))
        .unwrap_or_else(|| usage());
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
        .collect();
    files.sort();
    if let Some(out) = &svg_out {
        std::fs::create_dir_all(out).expect("create svg dir");
    }

    let header: Vec<&str> = Metrics::default().fields().iter().map(|f| f.0).collect();
    let late: Vec<&str> = Metrics::default()
        .late_fields()
        .iter()
        .map(|f| f.0)
        .collect();
    let timing: Vec<&str> = TIMINGS.iter().map(|t| t.0).collect();
    println!(
        "file\tmode\t{}\tdefects\troute_ms\tdeterministic\t{}\t{}",
        header.join("\t"),
        late.join("\t"),
        timing.join("\t")
    );
    let mut per_mode: [Vec<Row>; 2] = [Vec::new(), Vec::new()];
    for path in &files {
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(path).expect("read mmd");
        let rep = match quality::measure_source(&src, true) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("skip {name}: {e}");
                continue;
            }
        };
        let det = f64::from(u8::from(rep.deterministic));
        let g = match flowmaid::parser::parse_document(&src) {
            Ok(Document::Flowchart(g) | Document::State(g)) => g,
            _ => unreachable!("measured as a flowchart above"),
        };
        let dr = if dragging {
            drag(&g, &rep.static_scene)
        } else {
            [0.0; 3]
        };
        for (k, (mode, m)) in [("static", &rep.statik), ("routed", &rep.routed)]
            .into_iter()
            .enumerate()
        {
            let ms = if k == 1 { rep.route_ms } else { 0.0 };
            // Static rows time `scene()`, routed rows the drags.
            let t = if k == 0 {
                [rep.scene_ms, 0.0, 0.0, 0.0]
            } else {
                [0.0, dr[0], dr[1], dr[2]]
            };
            println!("{}", row(&name, mode, m, &[ms, det], &t));
            if details {
                for d in &m.details {
                    eprintln!("{name}\t{mode}\t{d}");
                }
            }
            per_mode[k].push((m.clone(), ms, det, t));
        }
        if let Some(out) = &svg_out {
            write(
                out,
                &name,
                "static",
                &flowmaid::scene::to_svg(&rep.static_scene),
            );
            write(
                out,
                &name,
                "routed",
                &flowmaid::scene::to_svg(&rep.routed_scene),
            );
        }
    }
    for (k, mode) in ["static", "routed"].into_iter().enumerate() {
        let rows = &per_mode[k];
        if rows.is_empty() {
            continue;
        }
        let fields: Vec<Vec<(&str, f64, Agg)>> = rows.iter().map(|r| r.0.fields()).collect();
        let mut cols = vec!["TOTAL".to_string(), mode.to_string()];
        for c in 0..fields[0].len() {
            let vals: Vec<f64> = fields.iter().map(|f| f[c].1).collect();
            cols.push(num(fold(&vals, fields[0][c].2)));
        }
        let defects: Vec<f64> = rows.iter().map(|r| r.0.defects() as f64).collect();
        cols.push(num(fold(&defects, Agg::Sum)));
        let ms: Vec<f64> = rows.iter().map(|r| r.1).collect();
        cols.push(num(fold(&ms, Agg::Max)));
        let det: Vec<f64> = rows.iter().map(|r| r.2).collect();
        cols.push(num(fold(&det, Agg::Min)));
        let late: Vec<Vec<(&str, f64, Agg)>> = rows.iter().map(|r| r.0.late_fields()).collect();
        for c in 0..late[0].len() {
            let vals: Vec<f64> = late.iter().map(|f| f[c].1).collect();
            cols.push(num(fold(&vals, late[0][c].2)));
        }
        for (c, &(_, agg)) in TIMINGS.iter().enumerate() {
            let vals: Vec<f64> = rows.iter().map(|r| r.3[c]).collect();
            cols.push(num(fold(&vals, agg)));
        }
        println!("{}", cols.join("\t"));
    }
}

fn write(out: &Path, name: &str, mode: &str, svg: &str) {
    let p = out.join(format!("{name}.{mode}.svg"));
    std::fs::write(&p, svg).unwrap_or_else(|e| panic!("write {}: {e}", p.display()));
}
