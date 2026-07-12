# flowrs

Mesin diagram flowchart mini ala MermaidJS, ditulis dalam Rust murni tanpa dependency eksternal. Menerima teks bersintaks Mermaid dan menghasilkan SVG.

## Cara pakai

```bash
cargo build --release

# dari file
./target/release/flowrs examples/demo.mmd -o demo.svg

# atau lewat pipe
cat examples/lr.mmd | ./target/release/flowrs > lr.svg

# saat pengembangan
cargo run -- examples/demo.mmd -o demo.svg
cargo test
```

Bisa juga dipakai sebagai library (crate ini adalah lib + bin):

```rust
let svg = flowrs::render_svg("flowchart TD\nA[Mulai] --> B[Selesai]")?;
```

## Sintaks yang didukung

Header menentukan arah aliran: `flowchart TD` (atas-bawah, alias `TB`), `LR` (kiri-kanan), `RL`, atau `BT`. Kata `graph` juga diterima. Baris yang diawali `%%` adalah komentar, dan `;` memisahkan beberapa statement dalam satu baris.

Bentuk node: `A[teks]` persegi, `A(teks)` sudut bulat, `A([teks])` stadium, `A{teks}` belah ketupat, `A((teks))` lingkaran. Label boleh dibungkus kutip untuk melindungi karakter khusus: `A["teks [aneh]"]`.

Garis penghubung: `-->` panah, `---` tanpa panah, `-.->` putus-putus, `==>` tebal. Label garis ditulis `-->|teks|`. Rantai `A --> B --> C` didukung, begitu juga siklus (`E --> B` yang kembali ke atas) dan self-loop (`A --> A`).

Contoh lengkap ada di `examples/demo.mmd` dan `examples/lr.mmd`.

## Arsitektur

Pipeline tiga tahap, satu modul per tahap:

1. `parser.rs` — parser tulis-tangan berbasis kursor karakter. Setiap baris di-parse menjadi rantai node dan edge, dengan pesan error bernomor baris.
2. `layout.rs` — algoritma Sugiyama versi ringkas: (a) DFS menandai *back-edge* agar siklus tidak merusak perhitungan, (b) *longest-path layering* menempatkan node ke lapisan, (c) sapuan *barycenter* bolak-balik mengurangi persilangan garis, (d) penentuan koordinat dengan packing per lapisan lalu penyelarasan ke rata-rata posisi tetangga tanpa tumpang tindih. Semua dihitung dalam koordinat abstrak (breadth × layer) sehingga keempat arah diagram cukup ditangani satu transformasi di akhir.
3. `render.rs` — memetakan koordinat abstrak ke x,y sesuai arah, lalu menggambar kurva bezier dengan panah (marker SVG), memotong garis tepat di tepi bentuk (persegi, lingkaran, belah ketupat punya rumus perpotongan sendiri), dan menaruh label di titik tengah kurva.

`model.rs` berisi struktur data bersama (`Graph`, `Node`, `Edge`, enum bentuk dan arah).

Untuk aplikasi interaktif ada modul `scene`: `scene()` menghasilkan geometri final siap gambar (posisi node, kurva bezier edge), `route()` merutekan ulang edge untuk posisi node kustom seperti hasil drag pengguna, dan `to_svg()` mengekspor susunan apa pun. `render()` kini hanya wrapper dari pipeline yang sama. Lihat `examples/drag_sim.rs` dan aplikasi demo egui di folder `flowrs-demo`.

## Performa

Benchmark bawaan ada di `examples/bench.rs` (pure std, graf sintetis deterministik) — jalankan dengan `cargo run --release --example bench`. Hasil pengukuran pada Linux x86_64, rustc 1.75, build release, waktu terbaik dari 3 run:

| node  | edge   | parse   | layout  | render* | SVG      |
|------:|-------:|--------:|--------:|--------:|---------:|
| 49    | 100    | 0,04 ms | 0,03 ms | 0,29 ms | 23 KB    |
| 200   | 400    | 0,16 ms | 0,08 ms | 1,13 ms | 97 KB    |
| 1.000 | 2.010  | 0,84 ms | 0,50 ms | 6,16 ms | 505 KB   |
| 2.500 | 5.050  | 1,97 ms | 1,30 ms | 16,35 ms| 1.278 KB |
| 5.000 | 10.150 | 4,16 ms | 2,75 ms | 34,92 ms| 2.618 KB |

\* kolom render sudah termasuk memanggil layout di dalamnya.

End-to-end lewat CLI untuk kasus 5.000 node — termasuk baca 10.151 baris input dan tulis SVG 2,7 MB — sekitar 60 ms dengan RAM puncak ±9 MB. Kasus jebakan kuadratik (2 lapis × 2.500 node selebar-lebarnya) selesai 21 ms, jadi skala praktisnya linear. Artinya untuk pemakaian realtime: re-render dari nol tiap ketikan aman untuk diagram wajar (±0,3 ms), dan budget 60 fps (16 ms) baru tersentuh sekitar 2.500 node. Bottleneck bukan algoritma melainkan pembentukan string SVG. Angka tentu bergantung hardware — ukur ulang di mesinmu dengan perintah di atas, dan selalu pakai `--release` (debug build ±10× lebih lambat).

## Interaktivitas & aplikasi desktop

Selain SVG statis, engine mengekspos API interaktif untuk aplikasi GUI: `flowrs::scene(&graph)` mengembalikan `Scene` — posisi, ukuran, dan bentuk setiap node plus kurva setiap edge dalam koordinat final — siap digambar painter framework mana pun. `Scene::hit_test(x, y)` mendeteksi node di bawah kursor (sadar-bentuk: lingkaran dan belah ketupat dites eksak), dan `Scene::move_node(i, x, y)` memindahkan node saat di-drag sambil merutekan ulang edge yang menempel, *tanpa* menjalankan ulang layout — sehingga node tidak melompat balik. `scene_to_svg(&scene)` mengekspor kondisi apa pun, termasuk setelah di-drag.

Demo lengkapnya ada di `demo-egui/` (crate terpisah; engine tetap tanpa dependency): editor teks live di kiri dengan pola *last good render*, diagram drag & drop di kanan, drop file `.mmd` ke jendela untuk memuatnya, dan tombol ekspor SVG. Jalankan dengan `cd demo-egui && cargo run --release`. `Cargo.lock`-nya sengaja dipin agar terkompilasi sampai rustc 1.75; di toolchain lebih baru bebas `cargo update`. Untuk framework lain: Tauri/Dioxus tinggal suntik string SVG ke webview; iced punya widget svg; Slint dan GTK4 merender SVG native; atau gambar `Scene` langsung dengan painter masing-masing seperti yang dilakukan demo egui ini.

## Publikasi ke crates.io

Metadata publish sudah terisi di `Cargo.toml` dan paket sudah lolos `cargo package` (termasuk build verifikasi). Tiga hal tersisa yang butuh keputusanmu: (1) **nama** — `flowrs` dan `mermaid-rs` sudah dipakai orang; `flowmaid` masih tersedia per Juli 2026 — ganti field `name`, lalu sesuaikan `use flowrs::` di `examples/` dan `flowrs-demo`; (2) **lisensi** — manifest memakai konvensi ekosistem `MIT OR Apache-2.0`, ganti bila perlu dan tambahkan file LICENSE-nya; (3) isi field `repository` setelah repo publiknya dibuat. Setelah itu tinggal `cargo login <token>` (token dari akun crates.io) lalu `cargo publish`. Nama di crates.io berlaku siapa-cepat-dia-dapat.

## Keterbatasan & ide pengembangan

Yang sudah ditangani: kanvas dihitung dari bounding box seluruh titik kontrol kurva sehingga self-loop dan back-edge tidak pernah terpotong; edge paralel (pasangan node sama) dipisah otomatis; edge panjang yang segaris dengan kolom node dilengkungkan ke samping sebagai mitigasi.

Yang masih terbuka: lebar teks diestimasi (±8 px per karakter) karena tidak ada metrik font, jadi label sangat panjang atau CJK bisa meleset; mitigasi edge panjang hanyalah heuristik — solusi sejatinya *virtual node* per lapisan yang dilewati; label edge bisa bertabrakan dengan node lain pada diagram padat; kutip ber-escape (`\"`) dalam label belum didukung. Fitur Mermaid yang enak dijadikan latihan berikutnya: `subgraph`, fan-out `A --> B & C`, garis `-.-` dan `--teks-->`, bentuk silinder `[( )]`, serta styling per node (`style`/`classDef`).
