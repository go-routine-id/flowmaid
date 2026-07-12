//! Engine layout bergaya Sugiyama (versi ringkas):
//!
//! 1. Deteksi back-edge lewat DFS agar siklus tidak merusak layering.
//! 2. Penentuan layer dengan longest-path (topological, ala Kahn).
//! 3. Pengurutan dalam layer memakai heuristik barycenter
//!    (mengurangi persilangan garis).
//! 4. Penentuan koordinat: packing per layer + penyelarasan ke
//!    tetangga (orang tua/anak) tanpa tumpang tindih.
//!
//! Semua dihitung dalam koordinat abstrak (b = breadth / lebar,
//! l = layer / kedalaman); renderer yang memetakan ke x,y final
//! sesuai arah diagram (TD/LR/BT/RL).

use crate::model::{Direction, Graph, Node, Shape};
use std::collections::VecDeque;

/// Posisi & ukuran satu node dalam koordinat abstrak.
pub struct Placed {
    /// Titik tengah pada sumbu breadth.
    pub b: f64,
    /// Titik tengah pada sumbu layer.
    pub l: f64,
    /// Ukuran node sepanjang sumbu breadth.
    pub bsize: f64,
    /// Ukuran node sepanjang sumbu layer.
    pub lsize: f64,
    /// Indeks layer.
    pub layer: usize,
}

pub struct LayoutResult {
    pub nodes: Vec<Placed>,
    pub total_b: f64,
    pub total_l: f64,
}

const PAD_X: f64 = 16.0;
const BASE_H: f64 = 38.0;
const MIN_W: f64 = 54.0;
const GAP_B: f64 = 48.0; // jarak antar node dalam satu layer
const GAP_L: f64 = 64.0; // jarak antar layer
const MARGIN: f64 = 28.0;

/// Estimasi lebar render teks (Helvetica ±14px) per kelas karakter.
/// Tanpa metrik font sungguhan ini tetap perkiraan, tapi jauh lebih
/// akurat daripada lebar rata: huruf kapital ±9.7px, i/l ±3.4px,
/// m/W ±12-13px, CJK/emoji ±14px.
pub fn text_width(s: &str) -> f64 {
    s.chars()
        .map(|c| match c {
            'i' | 'l' | 'j' => 3.4,
            ' ' | '.' | ',' | ':' | ';' | '!' | '\'' | 't' | 'f' | 'I' | '|' => 3.9,
            'r' | '(' | ')' | '[' | ']' | '-' | '/' => 4.7,
            's' | 'c' | 'k' | 'v' | 'x' | 'y' | 'z' | 'J' => 7.0,
            'm' | 'M' => 11.7,
            'w' => 10.1,
            'W' => 13.2,
            'A'..='Z' => 9.7,
            c if (c as u32) >= 0x2E80 => 14.0, // CJK, emoji, simbol lebar
            _ => 7.8,
        })
        .sum()
}

/// Ukuran intrinsik node (lebar, tinggi) dalam piksel, berdasarkan
/// bentuk dan estimasi lebar label.
pub fn intrinsic_size(node: &Node) -> (f64, f64) {
    let tw = text_width(&node.label);
    match node.shape {
        Shape::Rect | Shape::Rounded => ((tw + 2.0 * PAD_X).max(MIN_W), BASE_H),
        Shape::Stadium => ((tw + 2.0 * PAD_X + 12.0).max(MIN_W + 12.0), BASE_H),
        // Belah ketupat butuh ruang ekstra agar teks muat di tengah.
        Shape::Diamond => (((tw + 24.0) * 1.6).max(80.0), BASE_H * 1.7),
        Shape::Circle => {
            let d = (tw + 24.0).max(52.0);
            (d, d)
        }
    }
}

pub fn layout(g: &Graph) -> LayoutResult {
    let n = g.nodes.len();
    let mut adj: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    for (ei, e) in g.edges.iter().enumerate() {
        adj[e.from].push((e.to, ei));
    }

    // --- 1. Tandai back-edge (pemutus siklus) dengan DFS iteratif ---
    let mut state = vec![0u8; n]; // 0 belum, 1 di stack, 2 selesai
    let mut back = vec![false; g.edges.len()];
    for s in 0..n {
        if state[s] != 0 {
            continue;
        }
        state[s] = 1;
        let mut stack: Vec<(usize, usize)> = vec![(s, 0)];
        while !stack.is_empty() {
            let (u, ci) = *stack.last().unwrap();
            if ci < adj[u].len() {
                stack.last_mut().unwrap().1 += 1;
                let (v, ei) = adj[u][ci];
                if v == u {
                    back[ei] = true; // self-loop
                    continue;
                }
                match state[v] {
                    0 => {
                        state[v] = 1;
                        stack.push((v, 0));
                    }
                    1 => back[ei] = true, // edge kembali ke leluhur = siklus
                    _ => {}
                }
            } else {
                state[u] = 2;
                stack.pop();
            }
        }
    }

    // --- 2. Layering longest-path pada DAG (tanpa back-edge) ---
    let mut indeg = vec![0usize; n];
    for (ei, e) in g.edges.iter().enumerate() {
        if !back[ei] {
            indeg[e.to] += 1;
        }
    }
    let mut layer = vec![0usize; n];
    let mut q: VecDeque<usize> = (0..n).filter(|&v| indeg[v] == 0).collect();
    while let Some(u) = q.pop_front() {
        for &(v, ei) in &adj[u] {
            if back[ei] {
                continue;
            }
            if layer[u] + 1 > layer[v] {
                layer[v] = layer[u] + 1;
            }
            indeg[v] -= 1;
            if indeg[v] == 0 {
                q.push_back(v);
            }
        }
    }

    let nlayers = layer.iter().copied().max().unwrap_or(0) + 1;
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); nlayers];
    for v in 0..n {
        layers[layer[v]].push(v);
    }

    // Tetangga untuk barycenter & penyelarasan.
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut succs: Vec<Vec<usize>> = vec![Vec::new(); n];
    for e in &g.edges {
        if e.from == e.to {
            continue;
        }
        succs[e.from].push(e.to);
        preds[e.to].push(e.from);
    }

    // --- 3. Kurangi persilangan: sapuan barycenter bolak-balik ---
    let mut pos = vec![0.0f64; n];
    for lv in &layers {
        for (i, &v) in lv.iter().enumerate() {
            pos[v] = i as f64;
        }
    }
    for _ in 0..4 {
        for li in 1..nlayers {
            reorder(&mut layers[li], &preds, &mut pos);
        }
        for li in (0..nlayers.saturating_sub(1)).rev() {
            reorder(&mut layers[li], &succs, &mut pos);
        }
    }

    // --- 4. Koordinat ---
    // Ukuran node pada sumbu abstrak: untuk LR/RL, layer berjalan
    // horizontal sehingga lebar node menjadi "ukuran layer"-nya.
    let horizontal = matches!(g.direction, Direction::LR | Direction::RL);
    let mut bsize = vec![0.0f64; n];
    let mut lsize = vec![0.0f64; n];
    for v in 0..n {
        let (w, h) = intrinsic_size(&g.nodes[v]);
        if horizontal {
            bsize[v] = h;
            lsize[v] = w;
        } else {
            bsize[v] = w;
            lsize[v] = h;
        }
    }

    // Posisi layer (sumbu l): setiap layer setinggi node tertingginya.
    let mut lcoord = vec![0.0f64; nlayers];
    let mut cursor = MARGIN;
    for li in 0..nlayers {
        let lh = layers[li].iter().map(|&v| lsize[v]).fold(0.0f64, f64::max);
        lcoord[li] = cursor + lh / 2.0;
        cursor += lh + GAP_L;
    }
    let total_l = cursor - GAP_L + MARGIN;

    // Packing awal per layer, lalu ratakan ke tengah.
    let mut bpos = vec![0.0f64; n];
    let mut widths = vec![0.0f64; nlayers];
    for li in 0..nlayers {
        let mut c = 0.0;
        for &v in &layers[li] {
            bpos[v] = c + bsize[v] / 2.0;
            c += bsize[v] + GAP_B;
        }
        widths[li] = if layers[li].is_empty() { 0.0 } else { c - GAP_B };
    }
    let maxw = widths.iter().fold(0.0f64, |a, &b| a.max(b));
    for li in 0..nlayers {
        let off = MARGIN + (maxw - widths[li]) / 2.0;
        for &v in &layers[li] {
            bpos[v] += off;
        }
    }

    // Penyelarasan: geser node mendekati rata-rata posisi tetangganya,
    // sambil menjaga urutan dan jarak minimum (tidak tumpang tindih).
    for li in 1..nlayers {
        align_pass(&layers[li], &preds, &mut bpos, &bsize);
    }
    for li in (0..nlayers.saturating_sub(1)).rev() {
        align_pass(&layers[li], &succs, &mut bpos, &bsize);
    }
    for li in 1..nlayers {
        align_pass(&layers[li], &preds, &mut bpos, &bsize);
    }

    // Normalisasi supaya diagram mulai dari MARGIN.
    let mut minb = f64::INFINITY;
    let mut maxb = f64::NEG_INFINITY;
    for v in 0..n {
        minb = minb.min(bpos[v] - bsize[v] / 2.0);
        maxb = maxb.max(bpos[v] + bsize[v] / 2.0);
    }
    if n == 0 {
        minb = 0.0;
        maxb = 0.0;
    }
    let shift = MARGIN - minb;
    for v in 0..n {
        bpos[v] += shift;
    }
    let total_b = (maxb - minb) + 2.0 * MARGIN;

    let nodes = (0..n)
        .map(|v| Placed {
            b: bpos[v],
            l: lcoord[layer[v]],
            bsize: bsize[v],
            lsize: lsize[v],
            layer: layer[v],
        })
        .collect();

    LayoutResult {
        nodes,
        total_b,
        total_l,
    }
}

/// Urutkan ulang satu layer berdasarkan rata-rata posisi tetangga
/// (barycenter). Node tanpa tetangga mempertahankan posisinya.
fn reorder(layer: &mut Vec<usize>, nbrs: &[Vec<usize>], pos: &mut [f64]) {
    let mut keyed: Vec<(f64, usize)> = layer
        .iter()
        .map(|&v| {
            let ns = &nbrs[v];
            let key = if ns.is_empty() {
                pos[v]
            } else {
                ns.iter().map(|&u| pos[u]).sum::<f64>() / ns.len() as f64
            };
            (key, v)
        })
        .collect();
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
    layer.clear();
    for (i, (_, v)) in keyed.into_iter().enumerate() {
        layer.push(v);
        pos[v] = i as f64;
    }
}

/// Geser tiap node (urutan tetap) ke posisi rata-rata tetangganya
/// selama tidak menabrak node di kirinya.
fn align_pass(order: &[usize], nbrs: &[Vec<usize>], bpos: &mut [f64], bsize: &[f64]) {
    let mut min_edge = f64::NEG_INFINITY;
    for &v in order {
        let ns = &nbrs[v];
        let desired = if ns.is_empty() {
            bpos[v]
        } else {
            ns.iter().map(|&u| bpos[u]).sum::<f64>() / ns.len() as f64
        };
        let c = desired.max(min_edge + bsize[v] / 2.0);
        bpos[v] = c;
        min_edge = c + bsize[v] / 2.0 + GAP_B;
    }
}
