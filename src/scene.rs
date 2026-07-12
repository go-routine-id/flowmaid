//! Geometri siap-pakai untuk aplikasi interaktif (editor drag & drop,
//! hit-testing, ekspor). Semua koordinat di modul ini FINAL (piksel
//! layar, sudah mengikuti arah diagram) — berbeda dengan koordinat
//! abstrak internal `layout`.
//!
//! Alur khas aplikasi drag & drop:
//! 1. `scene(&graph)` — tata letak otomatis + geometri edge.
//! 2. Simpan posisi node di state aplikasi; gambar dengan painter.
//! 3. Saat node digeser, panggil `route(&graph, &posisi)` untuk
//!    menghitung ulang kurva edge mengikuti posisi baru (klasifikasi
//!    arah edge mengikuti posisi relatif aktual, bukan layer).
//! 4. `to_svg(&scene)` untuk mengekspor susunan apa pun ke SVG.

use crate::layout::{intrinsic_size, layout, text_width, Placed};
use crate::model::{Direction, EdgeKind, Graph, Shape};
use std::collections::HashMap;

const MARGIN: f64 = 28.0;
/// Jarak antar edge paralel (menghubungkan pasangan node yang sama).
const PARALLEL_GAP: f64 = 16.0;

const EDGE_COLOR: &str = "#44507a";
const NODE_FILL: &str = "#eef1fb";
const NODE_STROKE: &str = "#5b6dc0";
const TEXT_COLOR: &str = "#232840";

/// Node dengan posisi & ukuran final. Pusat di (x, y).
#[derive(Debug, Clone)]
pub struct SceneNode {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub shape: Shape,
    pub label: String,
}

/// Edge dengan kurva bezier kubik final.
#[derive(Debug, Clone)]
pub struct SceneEdge {
    /// Titik bezier: awal, kontrol1, kontrol2, akhir.
    pub bezier: [(f64, f64); 4],
    pub kind: EdgeKind,
    /// (teks, pusat kotak, lebar kotak) bila edge berlabel.
    pub label: Option<(String, (f64, f64), f64)>,
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub nodes: Vec<SceneNode>,
    pub edges: Vec<SceneEdge>,
    pub width: f64,
    pub height: f64,
}

/// Tata letak otomatis: jalankan layout lalu petakan seluruh geometri
/// ke koordinat final. Ini sumber kebenaran yang juga dipakai
/// `render::render`.
pub fn scene(g: &Graph) -> Scene {
    let lo = layout(g);

    // Ekstent breadth tiap layer, dipakai back-edge untuk merutekan
    // dirinya mengitari seluruh node pada layer-layer yang dilewati.
    let nlayers = lo.nodes.iter().map(|p| p.layer).max().map_or(0, |m| m + 1);
    let mut lay_ext = vec![(f64::INFINITY, f64::NEG_INFINITY); nlayers];
    for p in &lo.nodes {
        let e = &mut lay_ext[p.layer];
        e.0 = e.0.min(p.b - p.bsize / 2.0);
        e.1 = e.1.max(p.b + p.bsize / 2.0);
    }

    // Geometri edge (koordinat abstrak) dengan pemisahan edge paralel.
    let offs = parallel_offsets(g);
    let mut abs_edges: Vec<([(f64, f64); 4], Option<((f64, f64), f64)>)> =
        Vec::with_capacity(g.edges.len());
    for (e, off) in g.edges.iter().zip(offs) {
        let a = &lo.nodes[e.from];
        let b = &lo.nodes[e.to];
        let pts = edge_points(
            a,
            b,
            g.nodes[e.from].shape,
            g.nodes[e.to].shape,
            e.from == e.to,
            lo.total_b,
            off,
            &lay_ext,
        );
        let label = e.label.as_ref().map(|l| {
            let mid = cubic_mid(pts[0], pts[1], pts[2], pts[3]);
            (mid, text_width(l) + 14.0)
        });
        abs_edges.push((pts, label));
    }

    // Kanvas dari bounding box node + kurva + label. Kurva bezier
    // selalu berada dalam convex hull titik kontrolnya, sehingga bbox
    // titik kontrol menjamin tidak ada yang terpotong.
    let mut bb = Bbox::new();
    for p in &lo.nodes {
        bb.add(p.b - p.bsize / 2.0, p.l - p.lsize / 2.0);
        bb.add(p.b + p.bsize / 2.0, p.l + p.lsize / 2.0);
    }
    for (pts, label) in &abs_edges {
        for &(bp, lp) in pts {
            bb.add(bp, lp);
        }
        if let Some((m, w)) = label {
            bb.add(m.0 - w / 2.0, m.1 - 10.0);
            bb.add(m.0 + w / 2.0, m.1 + 10.0);
        }
    }
    let (minb, _maxb, minl, _maxl) = bb.finish();
    let offb = MARGIN - minb;
    let offl = MARGIN - minl;
    let total_b = (bb.1 - bb.0) + 2.0 * MARGIN;
    let total_l = (bb.3 - bb.2) + 2.0 * MARGIN;

    // Pemetaan koordinat abstrak (b, l) -> final (x, y).
    let dir = g.direction;
    let tf = move |p: (f64, f64)| -> (f64, f64) {
        let b = p.0 + offb;
        let l = p.1 + offl;
        match dir {
            Direction::TD => (b, l),
            Direction::BT => (b, total_l - l),
            Direction::LR => (l, b),
            Direction::RL => (total_l - l, b),
        }
    };
    let horizontal = matches!(dir, Direction::LR | Direction::RL);
    let (width, height) = if horizontal {
        (total_l, total_b)
    } else {
        (total_b, total_l)
    };

    let nodes = g
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let (x, y) = tf((lo.nodes[i].b, lo.nodes[i].l));
            let (w, h) = intrinsic_size(n); // ukuran final tak berubah oleh arah
            SceneNode {
                x,
                y,
                w,
                h,
                shape: n.shape,
                label: n.label.clone(),
            }
        })
        .collect();

    let edges = g
        .edges
        .iter()
        .zip(abs_edges)
        .map(|(e, (pts, label))| SceneEdge {
            bezier: [tf(pts[0]), tf(pts[1]), tf(pts[2]), tf(pts[3])],
            kind: e.kind,
            label: e
                .label
                .clone()
                .zip(label)
                .map(|(t, (m, w))| (t, tf(m), w)),
        })
        .collect();

    Scene {
        nodes,
        edges,
        width,
        height,
    }
}

/// Rutekan ulang edge untuk posisi node kustom (mis. hasil drag).
/// `centers[i]` = pusat node ke-i dalam koordinat final. Posisi node
/// TIDAK dinormalisasi (tetap persis seperti yang diberikan) agar
/// diagram tidak "berenang" saat digeser; `to_svg` yang menangani
/// translasi saat ekspor.
pub fn route(g: &Graph, centers: &[(f64, f64)]) -> Scene {
    assert_eq!(
        centers.len(),
        g.nodes.len(),
        "jumlah posisi harus sama dengan jumlah node"
    );
    let sizes: Vec<(f64, f64)> = g.nodes.iter().map(intrinsic_size).collect();
    let placed: Vec<Placed> = (0..g.nodes.len())
        .map(|i| Placed {
            b: centers[i].0,
            l: centers[i].1,
            bsize: sizes[i].0,
            lsize: sizes[i].1,
            layer: 0,
        })
        .collect();

    let offs = parallel_offsets(g);
    let mut edges = Vec::with_capacity(g.edges.len());
    for (e, off) in g.edges.iter().zip(offs) {
        let a = &placed[e.from];
        let b = &placed[e.to];
        let pts = free_edge(
            a,
            b,
            g.nodes[e.from].shape,
            g.nodes[e.to].shape,
            e.from == e.to,
            off,
        );
        let label = e.label.as_ref().map(|l| {
            (
                l.clone(),
                cubic_mid(pts[0], pts[1], pts[2], pts[3]),
                text_width(l) + 14.0,
            )
        });
        edges.push(SceneEdge {
            bezier: pts,
            kind: e.kind,
            label,
        });
    }

    let nodes: Vec<SceneNode> = g
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| SceneNode {
            x: centers[i].0,
            y: centers[i].1,
            w: sizes[i].0,
            h: sizes[i].1,
            shape: n.shape,
            label: n.label.clone(),
        })
        .collect();

    let mut bb = Bbox::new();
    grow_scene(&mut bb, &nodes, &edges);
    let (_, maxx, _, maxy) = bb.finish();
    Scene {
        nodes,
        edges,
        width: maxx + MARGIN,
        height: maxy + MARGIN,
    }
}

/// Serialisasi Scene apa pun (otomatis maupun hasil drag) ke SVG.
/// Konten ditranslasikan agar mulai dari MARGIN, jadi koordinat
/// negatif pun aman.
pub fn to_svg(sc: &Scene) -> String {
    let mut bb = Bbox::new();
    grow_scene(&mut bb, &sc.nodes, &sc.edges);
    let (minx, maxx, miny, maxy) = bb.finish();
    let tx = MARGIN - minx;
    let ty = MARGIN - miny;
    let width = (maxx - minx) + 2.0 * MARGIN;
    let height = (maxy - miny) + 2.0 * MARGIN;
    let t = |p: (f64, f64)| (p.0 + tx, p.1 + ty);

    let mut s = String::new();
    s.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.0}\" height=\"{h:.0}\" \
         viewBox=\"0 0 {w:.0} {h:.0}\" font-family=\"Helvetica, Arial, sans-serif\" \
         font-size=\"14\">\n",
        w = width,
        h = height
    ));
    s.push_str(&format!(
        "<defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"8.5\" refY=\"5\" \
         markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\">\
         <path d=\"M 0 1 L 9 5 L 0 9 z\" fill=\"{}\"/></marker></defs>\n",
        EDGE_COLOR
    ));
    s.push_str(&format!(
        "<rect width=\"{:.0}\" height=\"{:.0}\" fill=\"#ffffff\"/>\n",
        width, height
    ));

    let mut edge_labels = String::new();
    for e in &sc.edges {
        let q: Vec<(f64, f64)> = e.bezier.iter().map(|&p| t(p)).collect();
        let (dash, sw) = match e.kind {
            EdgeKind::Dotted => (" stroke-dasharray=\"5 4\"", 1.7),
            EdgeKind::Thick => ("", 3.4),
            _ => ("", 1.7),
        };
        let marker = if matches!(e.kind, EdgeKind::Open) {
            ""
        } else {
            " marker-end=\"url(#arrow)\""
        };
        s.push_str(&format!(
            "<path d=\"M {:.1} {:.1} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}\" \
             fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}/>\n",
            q[0].0, q[0].1, q[1].0, q[1].1, q[2].0, q[2].1, q[3].0, q[3].1,
            EDGE_COLOR, sw, dash, marker
        ));
        if let Some((text, m, w)) = &e.label {
            let qm = t(*m);
            edge_labels.push_str(&format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"20\" rx=\"4\" \
                 fill=\"#ffffff\" stroke=\"#d5d9ec\"/>\n",
                qm.0 - w / 2.0,
                qm.1 - 10.0,
                w
            ));
            edge_labels.push_str(&format!(
                "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
                 fill=\"{}\">{}</text>\n",
                qm.0,
                qm.1,
                TEXT_COLOR,
                escape(text)
            ));
        }
    }

    for n in &sc.nodes {
        let (cx, cy) = t((n.x, n.y));
        let (w, h) = (n.w, n.h);
        let style = format!(
            "fill=\"{}\" stroke=\"{}\" stroke-width=\"1.6\"",
            NODE_FILL, NODE_STROKE
        );
        match n.shape {
            Shape::Rect | Shape::Rounded | Shape::Stadium => {
                let rx = match n.shape {
                    Shape::Rounded => 9.0,
                    Shape::Stadium => h / 2.0,
                    _ => 3.0,
                };
                s.push_str(&format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
                     rx=\"{:.1}\" {}/>\n",
                    cx - w / 2.0,
                    cy - h / 2.0,
                    w,
                    h,
                    rx,
                    style
                ));
            }
            Shape::Circle => {
                s.push_str(&format!(
                    "<circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"{:.1}\" {}/>\n",
                    cx,
                    cy,
                    w / 2.0,
                    style
                ));
            }
            Shape::Diamond => {
                s.push_str(&format!(
                    "<polygon points=\"{:.1},{:.1} {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}\" {}/>\n",
                    cx,
                    cy - h / 2.0,
                    cx + w / 2.0,
                    cy,
                    cx,
                    cy + h / 2.0,
                    cx - w / 2.0,
                    cy,
                    style
                ));
            }
        }
        s.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" dy=\"0.33em\" text-anchor=\"middle\" \
             fill=\"{}\">{}</text>\n",
            cx,
            cy,
            TEXT_COLOR,
            escape(&n.label)
        ));
    }

    s.push_str(&edge_labels);
    s.push_str("</svg>\n");
    s
}

// ---------------------------------------------------------------
// Geometri internal
// ---------------------------------------------------------------

/// Bounding box sederhana (minx, maxx, miny, maxy).
struct Bbox(f64, f64, f64, f64);
impl Bbox {
    fn new() -> Self {
        Bbox(
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        )
    }
    fn add(&mut self, x: f64, y: f64) {
        self.0 = self.0.min(x);
        self.1 = self.1.max(x);
        self.2 = self.2.min(y);
        self.3 = self.3.max(y);
    }
    fn finish(&self) -> (f64, f64, f64, f64) {
        if self.0.is_finite() {
            (self.0, self.1, self.2, self.3)
        } else {
            (0.0, 0.0, 0.0, 0.0)
        }
    }
}

fn grow_scene(bb: &mut Bbox, nodes: &[SceneNode], edges: &[SceneEdge]) {
    for n in nodes {
        bb.add(n.x - n.w / 2.0, n.y - n.h / 2.0);
        bb.add(n.x + n.w / 2.0, n.y + n.h / 2.0);
    }
    for e in edges {
        for &(x, y) in &e.bezier {
            bb.add(x, y);
        }
        if let Some((_, m, w)) = &e.label {
            bb.add(m.0 - w / 2.0, m.1 - 10.0);
            bb.add(m.0 + w / 2.0, m.1 + 10.0);
        }
    }
}

fn pair(a: usize, b: usize) -> (usize, usize) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

/// Offset per edge agar edge paralel (pasangan node sama) terpisah.
fn parallel_offsets(g: &Graph) -> Vec<f64> {
    let mut count: HashMap<(usize, usize), usize> = HashMap::new();
    for e in &g.edges {
        *count.entry(pair(e.from, e.to)).or_insert(0) += 1;
    }
    let mut seen: HashMap<(usize, usize), usize> = HashMap::new();
    g.edges
        .iter()
        .map(|e| {
            let k = pair(e.from, e.to);
            let i = *seen.entry(k).and_modify(|x| *x += 1).or_insert(0);
            (i as f64 - (count[&k] as f64 - 1.0) / 2.0) * PARALLEL_GAP
        })
        .collect()
}

/// Titik keluar/masuk edge pada tepi node.
///
/// Untuk bentuk kotak-kotakan, titik disebar sepanjang sisi atas/bawah
/// ke arah node lawan (bukan menumpuk di tengah); stadium dibatasi ke
/// bagian datarnya saja agar tidak melayang di ujung membulat. Untuk
/// lingkaran/belah ketupat dipakai perpotongan tepi eksak.
fn anchor(p: &Placed, shape: Shape, other: (f64, f64), off: f64, bottom: bool) -> (f64, f64) {
    match shape {
        Shape::Diamond | Shape::Circle => border(p, shape, (other.0 + off * 4.0, other.1)),
        _ => {
            let flat = match shape {
                Shape::Stadium => p.bsize / 2.0 - p.lsize / 2.0 - 4.0,
                _ => p.bsize / 2.0 - 12.0,
            }
            .max(0.0);
            let bias = ((other.0 - p.b) * 0.35 + off).clamp(-flat, flat);
            let l = if bottom {
                p.l + p.lsize / 2.0
            } else {
                p.l - p.lsize / 2.0
            };
            (p.b + bias, l)
        }
    }
}

/// Geometri edge untuk tata letak berlapis (dipakai `scene`).
#[allow(clippy::too_many_arguments)]
fn edge_points(
    a: &Placed,
    b: &Placed,
    sa: Shape,
    sb: Shape,
    self_loop: bool,
    total_b: f64,
    off: f64,
    lay_ext: &[(f64, f64)],
) -> [(f64, f64); 4] {
    if self_loop {
        return loop_points(a, off);
    }

    if b.layer != a.layer {
        let down = b.layer > a.layer;
        let p0 = anchor(a, sa, (b.b, b.l), off, down);
        let p3 = anchor(b, sb, (a.b, a.l), off, !down);

        if down {
            let dl = ((p3.1 - p0.1) * 0.5).max(20.0);
            if b.layer - a.layer > 1 && (a.b - b.b).abs() < 30.0 {
                // Mitigasi: edge panjang yang segaris dengan kolom node
                // dilengkungkan ke samping agar tidak lewat di belakang
                // node pada layer antara. (Solusi sejati: virtual node.)
                let bow = if a.b >= total_b / 2.0 {
                    60.0 + off
                } else {
                    -60.0 + off
                };
                return [
                    p0,
                    (p0.0 + bow, p0.1 + dl * 0.6),
                    (p3.0 + bow, p3.1 - dl * 0.6),
                    p3,
                ];
            }
            return [p0, (p0.0, p0.1 + dl), (p3.0, p3.1 - dl), p3];
        }

        // Back-edge: rutekan mengitari sisi terdekat dari SEMUA layer
        // yang dilalui. Kontrol diselesaikan analitik agar puncak kurva
        // (t=0.5) berada ~24px di luar node terluar:
        // x(0.5) = 0.125*(x0+x3) + 0.75*BT.
        let (l0, l1) = (b.layer, a.layer);
        let mut ext_l = f64::INFINITY;
        let mut ext_r = f64::NEG_INFINITY;
        for li in l0..=l1 {
            ext_l = ext_l.min(lay_ext[li].0);
            ext_r = ext_r.max(lay_ext[li].1);
        }
        let ends = p0.0 + p3.0;
        let mid = ends / 2.0;
        let bt_r = ((4.0 / 3.0) * (ext_r + 24.0) - ends / 6.0).max(ext_r + 40.0);
        let bt_l = ((4.0 / 3.0) * (ext_l - 24.0) - ends / 6.0).min(ext_l - 40.0);
        let bt = (if (bt_r - mid) <= (mid - bt_l) { bt_r } else { bt_l }) + off;
        return [p0, (bt, p0.1 - 40.0), (bt, p3.1 + 40.0), p3];
    }

    // Satu layer: dari sisi ke sisi, melengkung ke bawah.
    let p0 = border(a, sa, (b.b, b.l));
    let p3 = border(b, sb, (a.b, a.l));
    let drop = a.lsize.max(b.lsize) / 2.0 + 22.0 + off;
    [
        p0,
        (p0.0 * 0.65 + p3.0 * 0.35, a.l + drop),
        (p0.0 * 0.35 + p3.0 * 0.65, b.l + drop),
        p3,
    ]
}

/// Geometri edge untuk posisi bebas (dipakai `route`): klasifikasi
/// vertikal/horizontal mengikuti posisi relatif aktual kedua node.
fn free_edge(
    a: &Placed,
    b: &Placed,
    sa: Shape,
    sb: Shape,
    self_loop: bool,
    off: f64,
) -> [(f64, f64); 4] {
    if self_loop {
        return loop_points(a, off);
    }
    let dx = b.b - a.b;
    let dy = b.l - a.l;
    if dy.abs() >= dx.abs() {
        // Dominan vertikal: keluar bawah/atas, masuk atas/bawah.
        let down = dy >= 0.0;
        let p0 = anchor(a, sa, (b.b, b.l), off, down);
        let p3 = anchor(b, sb, (a.b, a.l), off, !down);
        let s = if down { 1.0 } else { -1.0 };
        let dl = (dy.abs() * 0.45).max(24.0);
        [
            p0,
            (p0.0, p0.1 + s * dl),
            (p3.0, p3.1 - s * dl),
            p3,
        ]
    } else {
        // Dominan horizontal: tepi ke tepi.
        let p0 = border(a, sa, (b.b, b.l + off * 4.0));
        let p3 = border(b, sb, (a.b, a.l + off * 4.0));
        let s = if dx >= 0.0 { 1.0 } else { -1.0 };
        let dl = (dx.abs() * 0.45).max(24.0);
        [
            p0,
            (p0.0 + s * dl, p0.1),
            (p3.0 - s * dl, p3.1),
            p3,
        ]
    }
}

/// Self-loop kecil di sisi kanan node; loop paralel ditumpuk vertikal.
fn loop_points(a: &Placed, off: f64) -> [(f64, f64); 4] {
    let r = a.b + a.bsize / 2.0;
    let ext = 48.0 + off.abs() * 0.8;
    [
        (r, a.l - 8.0 + off),
        (r + ext, a.l - 28.0 + off),
        (r + ext, a.l + 28.0 + off),
        (r, a.l + 8.0 + off),
    ]
}

/// Titik potong garis (dari pusat node menuju `toward`) dengan
/// tepi bentuk node — supaya panah menempel di tepi, bukan di pusat.
fn border(p: &Placed, shape: Shape, toward: (f64, f64)) -> (f64, f64) {
    let dx = toward.0 - p.b;
    let dy = toward.1 - p.l;
    if dx.abs() < 1e-6 && dy.abs() < 1e-6 {
        return (p.b, p.l);
    }
    let hw = p.bsize / 2.0;
    let hh = p.lsize / 2.0;
    let t = match shape {
        Shape::Circle => hw / (dx * dx + dy * dy).sqrt(),
        // Belah ketupat: |x/hw| + |y/hh| = 1
        Shape::Diamond => 1.0 / (dx.abs() / hw + dy.abs() / hh),
        // Bentuk lain didekati sebagai persegi panjang.
        _ => {
            let tx = if dx.abs() > 1e-6 { hw / dx.abs() } else { f64::INFINITY };
            let ty = if dy.abs() > 1e-6 { hh / dy.abs() } else { f64::INFINITY };
            tx.min(ty)
        }
    };
    (p.b + dx * t, p.l + dy * t)
}

/// Titik tengah kurva bezier kubik (t = 0.5) untuk menaruh label.
fn cubic_mid(p0: (f64, f64), c1: (f64, f64), c2: (f64, f64), p3: (f64, f64)) -> (f64, f64) {
    (
        (p0.0 + 3.0 * c1.0 + 3.0 * c2.0 + p3.0) / 8.0,
        (p0.1 + 3.0 * c1.1 + 3.0 * c2.1 + p3.1) / 8.0,
    )
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn scene_dan_render_konsisten() {
        let g = parse("flowchart LR\nA --> B{X}\nB -->|y| C").unwrap();
        let s = scene(&g);
        assert_eq!(s.nodes.len(), 3);
        assert_eq!(s.edges.len(), 2);
        assert!(s.width > 0.0 && s.height > 0.0);
        assert_eq!(to_svg(&s), crate::render::render(&g));
    }

    #[test]
    fn route_menempel_di_tepi_node() {
        let g = parse("A[Kiri] --> B[Kanan]").unwrap();
        let s0 = scene(&g);
        let mut pos: Vec<(f64, f64)> = s0.nodes.iter().map(|n| (n.x, n.y)).collect();
        pos[1] = (pos[0].0 + 400.0, pos[0].1); // "drag" B jauh ke kanan
        let s1 = route(&g, &pos);
        let e = s1.edges[0].bezier;
        let a = &s1.nodes[0];
        let b = &s1.nodes[1];
        assert!(
            (e[0].0 - (a.x + a.w / 2.0)).abs() < 1.0,
            "edge harus keluar dari sisi kanan A"
        );
        assert!(
            (e[3].0 - (b.x - b.w / 2.0)).abs() < 1.0,
            "edge harus masuk dari sisi kiri B"
        );
    }
}
