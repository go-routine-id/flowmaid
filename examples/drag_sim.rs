//! Simulasi hasil drag & drop tanpa GUI: ambil layout otomatis,
//! "geser" dua node, rutekan ulang edge dengan `scene::route`,
//! lalu ekspor SVG. Ini pipeline persis yang dipakai aplikasi
//! desktop interaktif.

fn main() {
    let src = std::fs::read_to_string("examples/demo.mmd").unwrap();
    let g = flowrs::parser::parse(&src).unwrap();
    let s = flowrs::scene::scene(&g);
    let mut pos: Vec<(f64, f64)> = s.nodes.iter().map(|n| (n.x, n.y)).collect();
    // "drag": E (Tampilkan error) ke kiri-atas, F (Selesai) ke kanan.
    pos[4].0 -= 260.0;
    pos[4].1 -= 180.0;
    pos[5].0 += 240.0;
    pos[5].1 -= 120.0;
    let s2 = flowrs::scene::route(&g, &pos);
    std::fs::write("drag-sim.svg", flowrs::scene::to_svg(&s2)).unwrap();
    println!("tersimpan: drag-sim.svg");
}
