import re,sys
p='src/class.rs'
s=open(p).read()
a=s.index("/// Draw each class's self-loops apart and clear of the rest. The")
b=s.index("/// The same edge drawn the other way round: ends, curve and route")
new = r'''/// Draw each class's self-loops apart and clear of the rest. The
/// layout stacks a node's loops on one side, each starting where the
/// one before ends, so two loops wind round each other and an end glyph
/// can land on the other loop's end (both glyphs on one point when the
/// source order turns one loop round); and it sets a loop's label on
/// the loop's apex, where on a small box the chip hides most of the
/// loop. Here the loops alternate between that side and the opposite
/// one, and loops sharing a side nest, the outer one's ends past the
/// inner one's, their glyphs at opposite ends. Loops whose edge has a
/// cardinality (`carded[j]`) nest outside the rest, so each card sits
/// beside its own end instead of out past the outer loops, next to
/// theirs. The labels of a side sit past its outermost loop, side by
/// side along the face. When the loops so shared would run under
/// another box (a sibling close beside the class), across another line
/// or label, or close by a box, they take the faces where they do so
/// least ([`loop_faces`]); the lines and labels they are measured
/// against are the relations as the layout drew them and the loops of
/// the classes drawn before (classes are drawn in source order: two
/// siblings' loops used to meet in the gap between them, each label on
/// the other's loops). A class's only loop keeps the layout's curve
/// while it stays on the layout's side.
fn part_self_loops(sc: &mut Scene, carded: &[bool]) {
    let idx = node_index(sc);
    let mut loops: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (j, e) in sc.edges.iter().enumerate() {
        if e.from == e.to && e.meta.curve.is_none() {
            if let Some(&i) = idx.get(e.from.as_str()) {
                loops.entry(i).or_default().push(j);
            }
        }
    }
    if loops.is_empty() {
        return;
    }
    let rects: Vec<Rect> = sc.nodes.iter().map(node_rect).collect();
    // Every edge as it is drawn so far, with the box round it, and
    // whether it is drawn for good: a loop still to be parted is not —
    // it counts for the classes after its own, once it has its face.
    let mut drawn: Vec<(Rect, Vec<(f64, f64)>)> = sc
        .edges
        .iter()
        .map(|e| {
            let line = edge_polyline(e);
            (bounds(&line), line)
        })
        .collect();
    let mut settled: Vec<bool> = vec![true; sc.edges.len()];
    for &j in loops.values().flatten() {
        settled[j] = false;
    }
    for (i, js) in loops {
        let n = sc.nodes[i].clone();
        let side = face_normal(&n, sc.edges[js[0]].bezier[0]);
        // The faces in order of preference: the layout's side, the
        // opposite one, then the two across it.
        let outs = [
            side,
            (-side.0, -side.1),
            (-side.1, side.0),
            (side.1, -side.0),
        ];
        let widths: Vec<Option<f64>> = js
            .iter()
            .map(|&j| sc.edges[j].label.as_ref().map(|l| l.2))
            .collect();
        let outer: Vec<bool> = js
            .iter()
            .map(|&j| carded.get(j).copied().unwrap_or(false))
            .collect();
        // As far from the box as any loop or loop label of it can lie:
        // the deepest nest's apex, a label past it, and a row of every
        // label along a face.
        let reach = 0.75 * (LOOP_REACH + (js.len() - 1) as f64 * LOOP_NEST.1)
            + widths
                .iter()
                .flatten()
                .map(|w| w + LABEL_BOX_H + LOOP_LABEL_GAP)
                .sum::<f64>()
            + LOOP_LIFT
            + LOOP_CLEAR;
        let hood = grown(rects[i], reach);
        let field = LoopField {
            boxes: rects
                .iter()
                .enumerate()
                .filter(|&(k, &r)| k != i && rects_meet(hood, r))
                .map(|(_, &r)| r)
                .collect(),
            lines: drawn
                .iter()
                .zip(&settled)
                .filter(|&((r, _), &settled)| settled && rects_meet(hood, *r))
                .map(|((_, line), _)| {
                    let pieces: Vec<[(f64, f64); 2]> = line
                        .windows(2)
                        .filter(|s| rects_meet(hood, bounds(s)))
                        .map(|s| [s[0], s[1]])
                        .collect();
                    (bounds(pieces.as_flattened()), pieces)
                })
                .collect(),
            chips: sc
                .edges
                .iter()
                .zip(&settled)
                .filter(|&(e, &settled)| settled && e.from == e.to)
                .filter_map(|(e, _)| label_chip(e))
                .filter(|&c| rects_meet(hood, c))
                .collect(),
        };
        let faces = loop_faces(&n, &outs, &widths, &outer, &field);
        for (f, &out) in outs.iter().enumerate() {
            let on = on_face(&faces, f, &outer);
            let labels: Vec<Option<f64>> = on.iter().map(|&k| widths[k]).collect();
            for (&k, (curve, spot)) in on.iter().zip(face_loops(&n, out, &labels)) {
                let e = &mut sc.edges[js[k]];
                if js.len() > 1 || f != 0 {
                    e.bezier = curve;
                    e.waypoints.clear();
                }
                if let (Some((_, m, _)), Some(spot)) = (&mut e.label, spot) {
                    *m = spot;
                }
            }
        }
        for &j in &js {
            let line = edge_polyline(&sc.edges[j]);
            drawn[j] = (bounds(&line), line);
            settled[j] = true;
        }
    }
}

/// The box round `pts` (inside out when there are none).
fn bounds(pts: &[(f64, f64)]) -> Rect {
    pts.iter().fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |r, p| (r.0.min(p.0), r.1.min(p.1), r.2.max(p.0), r.3.max(p.1)),
    )
}

/// `r` with `by` more on every side.
fn grown(r: Rect, by: f64) -> Rect {
    (r.0 - by, r.1 - by, r.2 + by, r.3 + by)
}

/// Whether segments `a`-`b` and `c`-`d` cross, clear of their ends.
fn segments_cross(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
    let side = |o: (f64, f64), p: (f64, f64), q: (f64, f64)| {
        (p.0 - o.0) * (q.1 - o.1) - (p.1 - o.1) * (q.0 - o.0)
    };
    side(c, d, a) * side(c, d, b) < 0.0 && side(a, b, c) * side(a, b, d) < 0.0
}

/// How far a self-loop's region keeps from another box.
const LOOP_CLEAR: f64 = 4.0;
/// Air between two loop labels in a row past their loops: more than
/// the label stage keeps between labels ([`LABEL_GAP`] plus its overlap
/// tolerance), so it leaves the row as drawn. Closer, it would take
/// the pair for crowded and could move one onto the loops, where an
/// inner loop's apex leaves a spot clear of boxes and labels (the
/// stage does not mind strokes there).
const LOOP_LABEL_GAP: f64 = LABEL_GAP + 2.0;

/// A self-loop as [`face_loops`] draws it: its curve, and its label's
/// centre when it has a label.
type SelfLoop = ([(f64, f64); 4], Option<(f64, f64)>);

/// Self-loops nested on the face of `n` with outward normal `out`, one
/// per entry of `labels` (its label's width, if it has one), innermost
/// first, as [`part_self_loops`] draws them: each loop's curve, and its
/// label's centre past the outermost loop.
fn face_loops(n: &SceneNode, out: (f64, f64), labels: &[Option<f64>]) -> Vec<SelfLoop> {
    let t = (-out.1, out.0);
    let face = (n.x + out.0 * n.w / 2.0, n.y + out.1 * n.h / 2.0);
    let at = |along: f64, off: f64| {
        (
            face.0 + t.0 * along + out.0 * off,
            face.1 + t.1 * along + out.1 * off,
        )
    };
    let on_side = labels.len();
    // Each label's room along the face, and where the row of them
    // (centred on the face) puts each one's centre.
    let wide = |w: f64| LABEL_BOX_H * out.0.abs() + w * out.1.abs() + LOOP_LABEL_GAP;
    let row: f64 = labels.iter().flatten().map(|&w| wide(w)).sum();
    let mut next = -row / 2.0;
    labels
        .iter()
        .enumerate()
        .map(|(q, w)| {
            let qf = q as f64;
            let half = LOOP_HALF + qf * LOOP_NEST.0;
            let reach = LOOP_REACH + qf * LOOP_NEST.1;
            let lift = LOOP_LIFT + qf * LOOP_NEST.2;
            // The glyph (at the curve's end) alternates between the
            // ends of loops nested on one side.
            let s = if q % 2 == 0 { 1.0 } else { -1.0 };
            let curve = [
                at(-half * s, 0.0),
                at(-lift * s, reach),
                at(lift * s, reach),
                at(half * s, 0.0),
            ];
            // Past the outermost loop on the side (a cubic's apex
            // reaches 3/4 of its controls' offset), the labels of
            // nested loops side by side along the face.
            let spot = w.map(|w| {
                let outer = LOOP_REACH + (on_side - 1) as f64 * LOOP_NEST.1;
                let chip = (w / 2.0) * out.0.abs() + 11.0 * out.1.abs();
                let along = next + wide(w) / 2.0;
                next += wide(w);
                at(along, 0.75 * outer + 6.0 + chip)
            });
            (curve, spot)
        })
        .collect()
}

/// The loops of a class (indexes into its loops) that `faces` gives
/// face `f`, innermost first: those without cardinalities (`outer`),
/// then those with, each in source order.
fn on_face(faces: &[usize], f: usize, outer: &[bool]) -> Vec<usize> {
    let mut on: Vec<usize> = (0..faces.len()).filter(|&k| faces[k] == f).collect();
    on.sort_by_key(|&k| outer[k]);
    on
}

/// What stands round a class while its self-loops choose their faces
/// ([`loop_faces`]), as far as a loop or loop label of it can reach.
struct LoopField {
    /// The other boxes.
    boxes: Vec<Rect>,
    /// The other lines, each the box round its pieces here and those
    /// pieces: the relations as the layout drew them (their orthogonal
    /// routes come later, between the same faces) and the self-loops
    /// already parted.
    lines: Vec<(Rect, Vec<[(f64, f64); 2]>)>,
    /// Those loops' labels.
    chips: Vec<Rect>,
}

/// One loop of a nest on a face as [`loop_faces`] costs it: its line,
/// the box round it, how many boxes of the field it runs under and how
/// many times it crosses the field's lines and labels.
struct Nested {
    line: Vec<(f64, f64)>,
    bbox: Rect,
    under: usize,
    cross: usize,
}

/// The face (an index into `outs`, the four outward normals of `n` in
/// order of preference) for each of a class's self-loops, `widths`
/// their labels' widths, `outer` whether each nests outermost (as
/// [`on_face`] orders them): alternately the first two faces, unless
/// that is not clean. Then each way of sharing the loops among the four
/// faces is tried, round robin as before, and the one kept that costs
/// least, counting in turn
///
/// 1. the loops and loop labels under another box of `field` — each
///    loop and each label by itself, as drawn, so a deeper nest under a
///    box costs more than a shallow one and no loop is sent under a box
///    to spare anything below;
/// 2. the crossings: of a loop with a line of the field (a relation
///    ending inside the nest crosses every loop round its end, one
///    passing by crosses each loop it meets twice) or with a loop on
///    another face of the class, and each label struck by a line or
///    lying on another label;
/// 3. the boxes within [`LOOP_CLEAR`] of a face's loops and labels;
/// 4. the faces used and the nesting: the preferred faces first, the
///    shallowest nests.
fn loop_faces(
    n: &SceneNode,
    outs: &[(f64, f64); 4],
    widths: &[Option<f64>],
    outer: &[bool],
    field: &LoopField,
) -> Vec<usize> {
    let m = widths.len();
    // Round robin over the faces with loops still to take.
    let share = |counts: [usize; 4]| {
        let mut left = counts;
        let mut faces = Vec::with_capacity(m);
        let mut f = 0;
        while faces.len() < m {
            if left[f] > 0 {
                left[f] -= 1;
                faces.push(f);
            }
            f = (f + 1) % 4;
        }
        faces
    };
    let default = share([m.div_ceil(2), m / 2, 0, 0]);
    // Up to eight loops (165 ways to share them); more keep the default.
    if m > 8 {
        return default;
    }
    let hits = |line: &[(f64, f64)], r: Rect| line.windows(2).any(|s| rect_hits_segment(r, s[0], s[1]));
    // Each face's nest, as deep as it can get: a loop's curve depends
    // on its face and its place in the nest alone.
    let nests: Vec<Vec<Nested>> = outs
        .iter()
        .map(|&out| {
            face_loops(n, out, &vec![None; m])
                .iter()
                .map(|(curve, _)| {
                    let line = sample_cubic(curve, 16);
                    let bbox = bounds(&line);
                    let under = field
                        .boxes
                        .iter()
                        .filter(|&&b| rects_meet(bbox, b) && hits(&line, b))
                        .count();
                    let crossings: usize = field
                        .lines
                        .iter()
                        .filter(|(r, _)| rects_meet(bbox, *r))
                        .flat_map(|(_, pieces)| pieces)
                        .map(|p| {
                            line.windows(2)
                                .filter(|s| segments_cross(s[0], s[1], p[0], p[1]))
                                .count()
                        })
                        .sum();
                    let struck = field.chips.iter().filter(|&&c| hits(&line, c)).count();
                    Nested {
                        line,
                        bbox,
                        under,
                        cross: crossings + struck,
                    }
                })
                .collect()
        })
        .collect();
    // What a sharing costs: (under, crossings, close, preference).
    let cost = |faces: &[usize]| {
        let (mut under, mut cross, mut close, mut pref) = (0, 0, 0, 0);
        // The loops and labels of the faces before.
        let mut loops: Vec<&Nested> = Vec::new();
        let mut chips: Vec<Rect> = Vec::new();
        for (f, &out) in outs.iter().enumerate() {
            let on = on_face(faces, f, outer);
            let c = on.len();
            if c == 0 {
                continue;
            }
            pref += 2 * f + 3 * c * (c - 1) / 2;
            let nest = &nests[f][..c];
            let labels: Vec<Option<f64>> = on.iter().map(|&k| widths[k]).collect();
            let own: Vec<Rect> = face_loops(n, out, &labels)
                .iter()
                .zip(&labels)
                .filter_map(|(&(_, spot), &w)| {
                    let (m, w) = (spot?, w?);
                    Some((m.0 - w / 2.0 - 1.0, m.1 - 11.0, m.0 + w / 2.0 + 1.0, m.1 + 11.0))
                })
                .collect();
            let mut region = bounds(&[]);
            let mut grow = |r: Rect| region = (region.0.min(r.0), region.1.min(r.1), region.2.max(r.2), region.3.max(r.3));
            for l in nest {
                under += l.under;
                cross += l.cross;
                cross += loops
                    .iter()
                    .filter(|o| rects_meet(l.bbox, o.bbox))
                    .map(|o| {
                        l.line
                            .windows(2)
                            .map(|s| o.line.windows(2).filter(|t| segments_cross(s[0], s[1], t[0], t[1])).count())
                            .sum::<usize>()
                    })
                    .sum::<usize>();
                cross += chips.iter().filter(|&&c| hits(&l.line, c)).count();
                grow(l.bbox);
            }
            for &chip in &own {
                under += field.boxes.iter().filter(|&&b| rects_meet(chip, b)).count();
                cross += field
                    .lines
                    .iter()
                    .filter(|(r, pieces)| rects_meet(chip, *r) && pieces.iter().any(|p| rect_hits_segment(chip, p[0], p[1])))
                    .count();
                cross += field.chips.iter().filter(|&&o| rects_meet(chip, o)).count();
                cross += loops.iter().filter(|o| hits(&o.line, chip)).count();
                cross += chips.iter().filter(|&&o| rects_meet(chip, o)).count();
                grow(chip);
            }
            let region = grown(region, LOOP_CLEAR);
            close += field.boxes.iter().filter(|&&b| rects_meet(region, b)).count();
            loops.extend(nest);
            chips.extend(own);
        }
        (under, cross, close, pref)
    };
    let mut best = (cost(&default), default);
    if (best.0 .0, best.0 .1, best.0 .2) == (0, 0, 0) {
        return best.1;
    }
    for a in 0..=m {
        for b in 0..=m - a {
            for c in 0..=m - a - b {
                let faces = share([a, b, c, m - a - b - c]);
                let k = cost(&faces);
                if k < best.0 {
                    best = (k, faces);
                }
            }
        }
    }
    best.1
}

'''
s=s[:a]+new+s[b:]
# drop the test module's own segments_cross
t0=s.index("    fn segments_cross(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {", s.index("mod tests"))
t1=s.index("    /// The box round a glyph.", t0)
print(repr(s[t0-200:t0]))
print(repr(s[t0:t1]))
open(p,'w').write(s)
