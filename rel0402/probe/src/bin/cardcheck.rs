// For each class diagram: drag each box (up to 12 per file) by a few
// offsets from a scene() base and a route() base, then once more from
// that frame (chained), and check every relation not attached to the
// moved box keeps its card_spots, path and glyphs. usage: cardcheck <f>...
use flowmaid::model::Document;
use flowmaid::{class, parser};

fn main() {
    let (mut files, mut frames, mut checks, mut bad, mut moved_far) = (0, 0, 0, 0, 0);
    for f in std::env::args().skip(1) {
        let Ok(src) = std::fs::read_to_string(&f) else { continue };
        let Ok(Document::Class(d)) = parser::parse_document(&src) else { continue };
        files += 1;
        let laid = class::scene(&d);
        let at: Vec<(f64, f64)> = laid.scene.nodes.iter().map(|n| (n.x, n.y)).collect();
        let n = d.classes.len();
        if n == 0 {
            continue;
        }
        let bases = [laid.clone(), class::route(&d, &at)];
        let step = (n / 12).max(1);
        for base in &bases {
            let base_cards = class::card_spots(base);
            for i in (0..n).step_by(step) {
                for (dx, dy) in [(40.0, 25.0), (-35.0, 60.0), (6.0, 3.0), (-150.0, -90.0)] {
                    let mut to = at.clone();
                    to[i] = (at[i].0 + dx, at[i].1 + dy);
                    let frame = class::route_partial(&d, &to, base, &at);
                    let mut next = to.clone();
                    next[i].0 += 7.0;
                    let chained = class::route_partial(&d, &next, &frame, &to);
                    for fr in [&frame, &chained] {
                        frames += 1;
                        let cards = class::card_spots(fr);
                        for (j, r) in d.relations.iter().enumerate() {
                            if r.from == i || r.to == i {
                                continue;
                            }
                            checks += 1;
                            let e0 = &base.scene.edges[j];
                            let e1 = &fr.scene.edges[j];
                            let same_path = format!("{:?}{:?}{:?}", e0.bezier, e0.waypoints, e0.label)
                                == format!("{:?}{:?}{:?}", e1.bezier, e1.waypoints, e1.label);
                            let same_head = format!("{:?}{:?}", class::rel_head(base, j), class::rel_tail(base, j))
                                == format!("{:?}{:?}", class::rel_head(fr, j), class::rel_tail(fr, j));
                            if cards[j] != base_cards[j] || !same_path || !same_head {
                                bad += 1;
                                if bad <= 10 {
                                    eprintln!("{f}: box {i} ({dx},{dy}) relation {j}: cards {:?} vs {:?} path {same_path} head {same_head}", cards[j], base_cards[j]);
                                }
                            }
                        }
                        // The redrawn relations' cards: count those that moved
                        // more than 20 px from where route() puts them (info).
                        let full = class::route(&d, &to);
                        let fc = class::card_spots(&full);
                        for (j, r) in d.relations.iter().enumerate() {
                            if r.from == i || r.to == i {
                                for s in 0..2 {
                                    if let (Some(a), Some(b)) = (cards[j][s], fc[j][s]) {
                                        if (a.0 - b.0).hypot(a.1 - b.1) > 20.0 { moved_far += 1; }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    println!("files {files} frames {frames} kept-relation checks {checks} bad {bad} (info: redrawn cards >20px from route's: {moved_far})");
}
