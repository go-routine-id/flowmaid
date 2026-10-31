# Laporan perbandingan flowmaid 0.32 dengan mermaid.js 12.2.0

Laporan ini menggabungkan temuan per-diagram yang sudah diverifikasi dari 25 diagram korpus dengan hasil audit gaya default. Temuan yang dibantah sudah dibuang. Temuan yang saling bertentangan ditandai di §2.14. Repo tidak diubah sama sekali. Semua prototipe dikerjakan di salinan scratchpad.

---

## 1. Ringkasan eksekutif

**Kesimpulan:** kesan "jelek" itu benar. Penyebabnya bukan warna atau font. Kalau dilihat per elemen, flowmaid sering *lebih baik* daripada mermaid. Chip labelnya putih dan rapi, diamond kompak, node seukuran teks, dan SVG-nya portabel (`<text>` tanpa `foreignObject`, 5–10× lebih kecil). Judul cluster juga tidak pernah tertabrak edge. Untuk pie, journey, timeline, sankey, sequence, dan `<<choice>>`/`<<fork>>` di state, flowmaid sudah setara atau lebih baik. Flowmaid kalah di **komposisi**: bentuk edge, back-edge, layout cluster pada diagram padat, dan beberapa bug yang membuat informasi hilang.

**Baseline yang benar:** flowchart mermaid 12.2.0 memakai layout ELK dengan `look: "neo"`. Semua edge-nya **orthogonal dengan sudut membulat ~5px** (`generateRoundedPath(…, 5)`), bukan spline dagre. Jadi target "mirip mermaid" berarti edge orthogonal.

**Lima alasan terbesar flowmaid terlihat lebih buruk:**

1. **Ada tiga gaya edge dalam satu gambar** (sekitar 14 diagram):
   - cubic S-curve bebas antar-rank bersebelahan (`edge_points`, scene.rs:3337/3344);
   - B-spline lewat dummy (`spline_d`, scene.rs:3738);
   - orthogonal hanya untuk edge yang "rusak" dan di-reroute router (router.rs:7-8, 4210-4262).

   Mermaid memakai satu gaya untuk semua edge. Menurut audit global, ini faktor visual nomor satu.
2. **Back-edge digambar sebagai satu cubic "bow" yang lebar** (scene.rs:3347-3370, sekitar 11 diagram). Akibatnya:
   - bentuk lasso (demo, back-edge) dan V-loop menggantung (lr, wide-labels 230px);
   - panah masuk pada sudut ~14° dan setengahnya tertutup node;
   - edge keluar dari frame-nya sendiri (advanced);
   - router terpaksa me-reroute lewat tengah kipas (fanout);
   - kanvas membengkak.
3. **Masalah kepala panah:**
   - kepala pada `==>` berukuran 2× (scene.rs:2494);
   - glyph sempit (≈10.7×9.5px, mermaid 10.5×12.8);
   - stroke menyembul melewati ujung kepala;
   - segmen terakhir tidak tegak lurus sisi node.
4. **Layout diagram ber-cluster dan diagram padat salah struktur:**
   - subgraph bersarang hilang;
   - edge ke subgraph tidak memengaruhi ranking;
   - `direction` di dalam subgraph diabaikan;
   - node tanpa edge diparkir di rank 0;
   - DFS FAS membalik edge mayoritas;
   - cluster bertumpuk ke samping.

   Hasilnya frame besar yang kosong, diagram LR jadi portrait, dan jumlah crossing 1.6–4× mermaid.
5. **Informasi hilang atau artinya salah:**
   - note di state dan class, `«interface»`, dan 33 komentar ER dibuang;
   - chip label ER menutupi lingkaran "zero" sehingga `o{` terbaca `|{`;
   - hierarki class terbalik;
   - label panjang tidak pernah di-wrap (diamond 616×65).

Masalah sekunder: palet per-shape dan stroke tipis (1.6/1.7px) tidak tergolong *penyebab utama*. Namun teks putih di atas warna aksen gagal WCAG AA di 8 tipe diagram.

**Kabar baiknya**, beberapa perbaikan berdampak terbesar kecil ukurannya dan sebagian sudah diprototipe:

| Perbaikan | Ukuran | Hasil prototipe |
|---|---|---|
| Kepala panah thick | 1 baris | — |
| bbox kanvas | 1 helper | back-edge 351→314, stress 251→220 |
| Parser subgraph bersarang + fallback proxy | ~6 baris | 620 test lulus |
| Quick fix back-edge | kecil | lebar demo 383→321 |
| Bug sumbu label LR | kecil | tinggi wide-labels 413→201 |

---

## 2. Tema lintas-diagram

Severity: **T** = tinggi, **S** = sedang, **R** = rendah. Effort: S / M / L / XL.

### 2.1 Idiom edge campur aduk, bukan orthogonal

- **Diagram:** demo, subgraph, lr, stress, fanout, long-labels, back-edge, fanout-frames, dense-td, dense-lr, bench-75, dan class (class.rs punya painter cubic sendiri). Gitgraph dan architecture punya renderer sendiri dengan gejala yang sama.
- **Severity:** T. Ini faktor visual terbesar.
- **Akar masalah:**
  - Edge antar-rank bersebelahan selalu berupa satu cubic `[p0,(p0.x,p0.y+dl),(p3.x,p3.y-dl),p3]` (scene.rs:3337/3344), sehingga jadi S setiap kali `p0.x≠p3.x`.
  - Chain dummy dicat dengan curveBasis (`spline_d`, scene.rs:3738). Hasilnya "kabel kendur" dan diagonal 900px+ (bench-75, wide-labels).
  - `router::repair` sengaja membiarkan edge yang bersih apa adanya (router.rs:7-8; trigger 4210-4266). Hanya edge yang rusak yang jadi orthogonal lewat `smooth()` (router.rs:2570).
  - `Score` bersifat leksikografis (router.rs:443-472) dan `better` (router.rs:4515) menolak tukar frame dengan overlap. Di dense-td, budget 50× tidak mengubah output satu byte pun, jadi masalahnya **bukan budget**.
  - `H_WEIGHT=1.5` (router.rs:227) membuat A* berhenti di jalur suboptimal. Di fanout-frames rute yang dipilih berbiaya 972, padahal ada L-path 924. Dengan nilai 1.0, g3 lewat celah 40px dan n21 jadi satu L.
- **Perbaikan bertahap:**
  - (a) Untuk edge antar-rank bersebelahan dengan |dx|>~20, pakai Z `[p0,(x0,mid),(x3,mid),p3]` yang dicat `EdgeCurve::Rounded` (scene.rs:376-392, `ROUNDED_CURVE_RADIUS` dari 5 jadi 6-7). Prototipe B di audit sudah cocok dengan mermaid untuk edge yes/no di demo.
  - (b) Ubah chain dummy jadi polyline orthogonal: run lurus melalui koordinat dummy, dengan jog di garis tengah gap rank. Lalu serahkan ke `nudge()`/`spread()` agar channel bersama dipisah.
  - (c) Tambah `GAP_L` per gap sesuai jumlah leg horizontal yang lewat (`count × EDGE_SEP`).
  - (d) Pakai `H_WEIGHT=1.0` untuk scene statis. Nilai 1.5 tetap untuk drag/`route()`.
  - (e) Bila semua kandidat punya jumlah frame yang sama, bandingkan overlap/tight lebih dulu.
- **Peringatan:** mengubah kontrol spline atau back-edge jadi Z *di painter saja* memperburuk fanout (muncul detour besar, Prototipe B kolom kanan). Long edge dan back-edge harus diurus router (lattice + `smooth`).
- **Effort:** (a) S–M. Keseluruhan L.
- **Dampak:** terbesar. S-hook, kink kepala panah, diagonal panjang, dan edge yang "dikepang" hilang. Kepala panah otomatis tegak lurus sisi node, dan crossing yang tergambar mengikuti urutan dummy.

### 2.2 Back-edge sebagai satu cubic "bow"

- **Diagram:** demo, back-edge, lr, wide-labels, stress, fanout, state (retry), class (charges), dense-td (n21→n16), bench-75 (hairpin), advanced ('fail' dan Cache→Gate keluar frame).
- **Severity:** T.
- **Akar masalah:**
  - scene.rs:3370 `[p0,(bt,p0.1-40),(bt,p3.1+40),p3]`. Lead sepanjang flow tetap 40px, sementara offset menyamping 130–280px. Akibatnya tangen ujung hampir sejajar sisi node, dan panah masuk pada sudut 13–17°. Edge digambar sebelum node (scene.rs:2858 vs 2935), jadi kepalanya tertimpa.
  - scene.rs:3356 `for li in l0..=l1` ikut membersihkan layer *node ujung* sendiri. Apex terdorong 24px ke luar Show error, padahal lane mermaid ada *di dalam* rentang E.
  - scene.rs:3360 `clear = 24 + label_w/2` dipakai di sumbu breadth bahkan pada LR, padahal yang benar tinggi label (20px). Di wide-labels ini menghasilkan loop 230px, kanvas 2.5× lebih tinggi, dan Database yang melorot. Bug yang sama ada di scene.rs:798, scene.rs:730 (`bow_blocked`), dan layout.rs:2138-2142 (`drawn_breadth`).
  - Kedua ujung di-anchor ke *pusat node lawan* (scene.rs:3323-3324, bias 0.35 di scene.rs:3293), bukan ke sisi tempat bow keluar. Akibatnya start back-edge hanya 14px dari panah yang masuk (demo, back-edge, state), dan apex diamond jadi penuh sesak (stress).
  - layout.rs:1361-1376 sengaja tidak memberi channel dummy untuk back-edge. Aturan bow diduplikasi di `drawn_breadth` (layout.rs:2099-2150, `BOW_CLEAR` di layout.rs:126), jadi **dua tempat harus diubah bersamaan**.
  - Hanya apex di t=0.5 yang dijamin bebas. `bow_blocked` (scene.rs:719) hanya berjalan di jalur clustered. Di fanout bow menembus Worker 1, lalu router "menyelamatkannya" dengan jalur interior yang punya 2 crossing.
  - 2-cycle antar-layer bersebelahan (lr): `anchor()` men-clamp `flat = bsize/2-12 = 7px` sehingga `PARALLEL_GAP ±8` hilang. Tie-break `port_order` (ports.rs:331-336) lalu menghasilkan urutan port yang bersilang.
  - Router tidak pernah menyentuh bow yang "bersih" (router.rs:4249). Segmen di luar frame sendiri tidak diberi biaya (router.rs:787-801). Pelebaran frame melewatkan edge tanpa waypoint (scene.rs:1091).
- **Perbaikan:**
  1. *Quick fix (sudah diprototipe):* bersihkan hanya node selain kedua ujung (`layer_extents(&lo, |v| v != e.from && v != e.to)`), anchor kedua ujung ke arah `bt`, dan skalakan lead dengan `max(40, 0.5*|bt-p3.b|)` atau tambahkan stub tegak lurus. Terapkan aturan yang sama di `drawn_breadth`. Hasil demo: lebar 383→321, sudut masuk ~37°, 612 lib test lulus. Gate wide-labels `lbl_crowd`=4 masih trip, jadi back-edge berlabel perlu disetel.
  2. *Bug sumbu label LR:* ukuran breadth label = `LABEL_BOX_H*lines` untuk LR/RL, dan `text_width+pad` untuk TD/BT. Hasil: tinggi wide-labels 413→201 (187 bila digabung fix bbox). Gate `MAX_ASPECT` di tests/quality.rs:226-245 harus direvisi di commit yang sama (lihat §2.13).
  3. *Paritas:* lane orthogonal `[p0,(p0.x,p0.y-16),(lane,·),(lane,·),(p3.x,p3.y+16),p3]`, dengan lane = extent layer *di antara* kedua ujung + 24, dan sudut dibulatkan oleh `nudge`. Prototipe rv_be menghasilkan topologi persis mermaid, lebar 351→290, dan 11 diagram korpus berubah (state 403→318, class 774→668).
  4. *2-cycle bersebelahan:* reversed S di dalam gap, lane eksplisit untuk kembarannya, dan `port_order` yang sadar pasangan. Tinggi lr 299→226.
  5. *Edge yang kedua ujungnya di satu cluster:* frame tumbuh menutupi hull bow dan labelnya, plus soft penalty router untuk segmen di luar frame terdalam yang memuat kedua ujung.
- **Effort:** 1–2 S (M karena gate). 3–5 M. Integrasi penuh L.
- **Dampak:** T. Elemen paling "jelek" di 6+ diagram hilang dan kanvas jadi lebih ramping.

### 2.3 Kepala panah dan ujung edge

- **Diagram:** semua flowchart. Kepala 2× terlihat di demo, advanced, lr, stress, back-edge, dan shapes.
- **Severity:** T untuk `==>`, S untuk sisanya.
- **Akar masalah:**
  - scene.rs:2494 `user_space: m == Marker::Arrow && st.stroke_width.is_some()`. Edge `==>` mendapat lebar dari jenisnya (`EDGE_WIDTH_THICK` 3.4, scene.rs:2442/2865), sehingga memakai `#arrow` dasar yang berunit strokeWidth (`ARROW_MARKER_UNITS` 7, scene.rs:2447). Hasilnya kepala ~21–24px.
  - Glyph `M 0 1 L 9 5 L 0 9` (scene.rs:2806-2808, 2540; arrow-start 2598-2602; salinan di advance.rs:5772).
  - Path berakhir tepat di border. Dengan `refX` 8.5 sementara ujung glyph di 9, stroke berujung tumpul menyembul di sisi ujung kepala, dan ujungnya overshoot 0.5 unit.
  - `orient=auto` mengikuti tangen akhir cubic yang masih membelok. Hasilnya kink 15–21° (lr) dan sekitar 15 kepala miring 44–86° (bench-75).
  - Stub router 14.5 (`ROUTE_CLEAR` 12.5 + `STUB_EXTRA` 2, router.rs:2701) melawan `ARROW_MARKER_PX` 11.9. Hanya tersisa ~2.6px lurus sebelum kepala. Nilai 14.5 juga hampir sama dengan `FRAME_PAD` 14, sehingga leg mendarat di garis frame.
  - Bug laten: labels.rs:92/903/913 dan casing.rs:40/262 memakai `ARROW_MARKER_PX` sebagai konstanta. Ruang di sekitar kepala thick terlalu kecil, dan metrik tidak menyadarinya.
- **Perbaikan:**
  - Satu marker `userSpaceOnUse` untuk *semua* edge (audit Tier A #2): `viewBox 0 0 10 10`, `refX 9`, `markerWidth/Height 11`, path `M 0 0 L 10 5 L 0 10 z`. Ubah `ARROW_MARKER_UNITS` 7.0→5.5 agar `ARROW_MARKER_PX` ≈ 11 dan pemakainya tetap konsisten. Ini otomatis memperbaiki edge thick. Alternatif minimal: `|| matches!(e.kind, EdgeKind::Thick | EdgeKind::ThickOpen)`.
  - **Wajib dirilis bersamaan:** trim path saat emit (cubic tunggal, `spline_d`, `curve_geometry`) sepanjang ~0.45–0.6 panjang kepala, dengan `refX` diatur agar ujung tetap di border. Tiga prototipe membuktikan kepala kecil di atas stroke 3.4 tanpa trim justru membuat stub *lebih* terlihat.
  - Stub akhir router dibuat ≥ 2×`ARROW_MARKER_PX` + radius sudut (~30). Untuk kurva, segmen sepanjang `ARROW_MARKER_PX` terakhir dibuat lurus.
  - Dua test byte-exact perlu diperbarui: `diagrams_without_link_style_render_exactly_as_before` dan `edge_markers_draw_at_the_right_ends`.
- **Effort:** marker S, trim M.
- **Dampak:** T per biaya. Kolom A prototipe langsung terlihat lebih tajam.

### 2.4 Penempatan port dan anchor

- **Diagram:** demo, subgraph, fanout, long-labels, class, stress, advanced, dense-td, dense-lr, bench-75, lr, wide-labels, fanout-frames, state.
- **Severity:** S. Bagian besar dari kesan "kusut".
- **Akar masalah:**
  - `anchor()` di scene.rs:3286-3300: `bias = ((other.0 - p.b)*0.35 + off).clamp(-flat, flat)` dengan `flat = bsize/2-12`, diterapkan di **kedua** ujung. Panah mendarat 27–44px dari tengah (long-labels 43.7px, subgraph 28px). Pada fan-out bias jenuh di ±flat, sehingga beberapa ujung punya posisi target yang identik.
  - `place()` di ports.rs:256-290 adalah PAV isotonik tanpa bobot. Ujung edge yang sudah lurus ikut tergeser 7px dan jadi S-wobble (advanced Build→Scan, fanout C→D, stress). Ujung yang sendirian di sisinya tidak pernah dipindah (ports.rs:460).
  - Tidak ada distribusi merata. Port dipadatkan per `PORT_SPACING` 14 (ports.rs:23) mulai dari pojok: fanout punya jarak 14/28.9/28.9/14, dan hub dense-td menaruh 7 ujung di 113 dari 161 unit.
  - `ports::assign` berjalan *sebelum* `router::repair`, lalu hanya `respace` (edgepass.rs:309-321). Offset yang sudah basi bertahan sampai output.
  - Pindah sisi node murah: `SIDE_SWITCH_COST` 48 (router.rs:153, `terms()` 2690-2722, `alt:true` di semua try router.rs:4361). Edge forward keluar atau masuk lewat sisi samping: dense-td 11/47 edge, dense-lr 6 edge, dan 25 kepala di bench-75 masuk dari atas/bawah pada diagram LR.
  - Node LR hanya setinggi 38px (`BASE_H`). `capacity()` (ports.rs:348) memaksa spill ke tutup dan pojok (hub dense-lr; svc bench-75 berjarak 7.5px).
- **Perbaikan** (dirilis bersama, karena masing-masing memperburuk yang lain bila sendirian):
  - bias 0 untuk ujung yang sendirian di sisinya;
  - slot merata untuk k≥3 ujung, `s = min(room, max(PORT_SPACING, usable/(k+1)))`, diurutkan menurut heading;
  - weighted PAV: ujung edge lurus berbobot tinggi, ujung back-edge rendah;
  - straight-pair snap setelah tahap ports;
  - `assign` penuh dijalankan ulang untuk grup yang berubah setelah repair;
  - tinggi node LR ≥ `(k-1)*PORT_SPACING + 2*PORT_INSET`;
  - di mode static, sisi non-flow untuk edge forward dihargai 6–8×`BEND_COST`.
- **Coupling yang sudah terbukti:**
  - bias=0 saja membuat S makin curam (long-labels: 92px menyamping dalam 63px) dan memadatkan sisi yang punya banyak ujung (fanout);
  - `SIDE_SWITCH_COST`×20 saja memperburuk dense-td (crossing 85→96, defect 42→46).

  Paket ini perlu §2.6 (ruang rank berlabel) dan distribusi port hub.
- **Effort:** M.
- **Dampak:** S–T. Spine jadi lurus, panah mendarat di tengah, hub rapi.

### 2.5 Geometri edge tidak mengenal outline shape

- **Diagram:** shapes (T), advanced, dense-lr, dense-td, fanout-frames, wide-labels.
- **Akar masalah:**
  - `anchor()` hanya menghitung titik border yang presisi untuk Diamond, Circle, DoubleCircle, dan State* (scene.rs:3274-3302). `border()` (3693-3724) menganggap shape lain sebagai rect.
  - `span()` di ports.rs (189-242) hanya punya Profile Flat, Round, dan Diamond (`_ => rect()`). Ujung di parallelogram dan hexagon melayang 7–10px; di cylinder 1–8px.
  - Router `flat_shape` (2961-2971), `outline_gap` (417-431), dan quality.rs:470-485 juga memperlakukan shape ini sebagai rect, jadi regresinya tidak pernah terdeteksi.
  - Cylinder:
    - label dipusatkan di bbox (scene.rs:3129-3135), ~8px terlalu tinggi;
    - tutupnya tidak diisi (3029-3048);
    - `ry` tetap 8 (3031);
    - stroke 1.6 hard-coded (3037, 3052-3053, 3111).

    Semua ini terduplikasi di advance.rs (~5520-5640).
- **Perbaikan:**
  - helper `outline_n(node, side, t)`: parallelogram linear dengan `k=min(14,w/4)`, hexagon profil V, cylinder elips;
  - stage `snap_ends` di `edgepass::finish_fixed` setelah `router::nudge` (edgepass.rs:308), yang menggeser ujung beserta kontrol tetangganya;
  - Profile baru di ports.rs;
  - quality dan router memakai outline yang benar.

  Untuk cylinder:
  - label di `cy+0.75*ry` atau `cy+ry`;
  - body path melewati busur belakang supaya tutup terisi;
  - `ry` diskalakan dengan lebar (`(w/2)/(2.5+w/50)` seperti mermaid);
  - pakai stroke yang sudah di-resolve.

  Ubah kedua renderer.
- **Effort:** snap M, cylinder S.
- **Dampak:** S. Menghapus kesan "panah tidak menyentuh node" yang terlihat seperti bug.

### 2.6 Ruang label di edge pendek dan penempatan label

- **Diagram:** demo, back-edge, long-labels, fanout, dense-td, lr, wide-labels, state, stress, dense-lr, class.
- **Severity:** S.
- **Akar masalah:**
  - layout.rs:1895-1912: gap rank hanya tumbuh bila `at - ends < ext` (layout.rs:1908), artinya hanya bila label sama sekali tidak muat. Di TD, `GAP_L` 60 (layout.rs:57) selalu lebih besar dari 32, sehingga chip 20px menelan sepertiga gap dan edge tampak seperti dua kait.
  - Di LR, trigger membandingkan dengan lebar label *tanpa* `LABEL_CLEAR` dan kepala panah, padahal rumus pertumbuhannya memakai `2*LABEL_CLEAR`. Akibatnya 'denied' di lr terdorong ke node 401. Di wide-labels label tergeser ke samping karena kepala panah adalah obstacle keras (labels.rs:1478-1487).
  - `rank()` (layout.rs:989) tidak punya `minlen`. Dummy label hanya ada untuk chain (layout.rs:1396-1408). Dagre memakai `minlen 2` ditambah dummy label.
  - `gap_need` (layout.rs:1855-1877) hanya meminta 6px antar-chip sebaris: 'valid'/'invalid' di state berjarak 5.7px, dan secara laten 7.2px di long-labels setelah wrap.
  - `channel_label_point` (labels.rs:53-64) baru snap ke spline bila drift > 17px. Akibatnya 'health probe' meleset 9.6px dan 'contains' 18.5px.
  - Tie di `reads_own` (labels.rs:1159-1161) membuat label 'parallel' menutupi edge saudaranya (stress).
  - Offset `beside_line` sampai ~62px (labels.rs:1855-1957): 4 label di dense-lr lepas dari edge-nya. `LABEL_HUB_PREFER` 0.75 (labels.rs:88) membuat label mendarat di tikungan.
- **Perbaikan:**
  - TD/BT: gap ≥ `ext + 2*(ARROW_MARKER_PX + 12..18)` untuk pasangan layer yang membawa edge pendek berlabel. Ini sudah tervalidasi, lihat `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rev_ll/acw.png`.
  - LR: `need = ext + 2*LABEL_CLEAR + ARROW_MARKER_PX` di trigger *dan* rumus pertumbuhan. Bedakan menurut arah: versi naif yang berlaku untuk semua arah memecah 2 test gap LR dan beberapa ceiling.
  - Label ditaruh di run lurus: S di bagian atas, lalu vertikal lurus melewati label. Atau Z dengan label di leg vertikal.
  - Ambang "terbaca terpisah" 20–24px di `gap_need`.
  - `channel_label_point` selalu snap ke spline (toleransi ≤2px).
  - `reads_own` strict, ditambah offset paralel ≥ `label_w/2 + STROKE_PAD + EDGE_SEP` untuk grup yang berisi edge berlabel.
  - Versi setia dagre (L): `minlen=2` dengan dummy label. Ini juga memperbaiki off-centre di §2.4.
- **Effort:** gap S–M, label S–M, `minlen` L.
- **Dampak:** S–T untuk diagram keputusan yes/no, yang sangat umum.

### 2.7 Bounding box kanvas dihitung dari titik kontrol Bézier

- **Diagram:** demo (margin kanan 65px), lr, stress, back-edge, wide-labels, state, class.
- **Akar masalah:**
  - Titik kontrol ikut ditambahkan ke bbox di scene.rs:886-912 dan `grow_scene` (3199-3201), termasuk fallback bezier dari edge yang punya waypoint.
  - `edgepass::refit` hanya bisa membesar (edgepass.rs:339).
  - Untuk Scene API LR dengan label lebar, ekstent label masuk ke sumbu yang salah (scene.rs:916-918): `scene()` melaporkan tinggi 624 padahal SVG 413, dan node bergeser 119px.
- **Perbaikan:**
  - `cubic_bbox` (akar turunan per sumbu) atau `edge_polyline(e)` plus setengah stroke dan kepala, di kedua tempat;
  - `refit` di mode Static menghitung ulang;
  - tukar ekstent untuk arah horizontal.

  Cek `channels_keep_edges_apart_and_off_chips`, yang naik dari 2 ke 3 di eksperimen.
- **Effort:** S. **Dampak:** S di 7 diagram, murah.

### 2.8 Subgraph dan cluster

- **Diagram:** advanced, subgraph, state, fanout-frames, dense-td, dense-lr, bench-75.

| Sub-isu | Akar (file:line) | Perbaikan | Effort |
|---|---|---|---|
| Semua cluster sama warna; nesting tidak terbaca; stroke 1.55:1 terhadap putih | scene.rs:2820-2833 hard-code `#f7f8fd/#c9cfe8/1.4`; `SceneCluster.depth` (scene.rs:126-127) tidak dipakai | Palet per depth atau per top-level (tint `style::ACCENTS`), stroke ≥3:1, chip judul ikut fill. Audit Tier A #8: judul 13px weight 600 | S |
| Ruang mati 26px di bawah baris pertama setiap cluster (86 vs 60), muncul tanpa label | layout.rs:1486-1496 `lextra = extra*2.0` + `lcoord = cursor + lh/2` (1919) | Slot asimetris h_lo/h_hi. Prototipe: 786→734 dan 612 lib test lulus, **tetapi 4 gate dense-td gagal** (min_edge_gap 1.78<12.1, edge_frame_pairs 40>28, defects 52>42, edge_node_x 4>0), artinya routing hilir bergantung pada 26px itu | M |
| Padding 14px; node di bawah frame hanya 32px; edge menunggangi border | `SUB_PAD` scene.rs:999, `FRAME_PAD` layout.rs:74, `FRAME_GAP` 8; stub 14.5 ≈ FRAME_PAD | Padding 20–24, `FRAME_GAP` ≈ GAP_L/2, stub tidak boleh berakhir di garis frame | S |
| Edge keluar dari frame sendiri | router.rs:788 skip `info.allowed`; scene.rs:1091 widening mengabaikan edge tanpa waypoint | Soft penalty dan frame tumbuh (§2.2 #5) | M |
| `direction` di subgraph/composite diabaikan | scene.rs:1018 satu pass Sugiyama global; `Subgraph.direction` hanya dibaca di layout.rs:1199-1209 dan scene.rs:1621 | Compound layout (child Graph → proxy) | L |
| Node tanpa edge di rank 0 → frame lebar dan kosong | layout.rs:1046-1078; `tighten_sources` (1149-1182) melewatkan node tanpa out-edge; grid butuh ≥4 (scene.rs:1011) | Pindahkan ke median layer cluster-nya (heuristik M); network simplex (XL) | M/XL |
| Gate `narrow()` salah sasaran | `RIBBON_ENTRY` 4.0 (layout.rs:85, 505): dense-td 2.33 dan dense-lr 1.53 tidak memicu; fanout-frames 4.1 memicu tapi area **naik 9%** (`better()` layout.rs:593 tidak mengecek area atau panjang edge); `stack_candidates` membuang pasangan dua arah (layout.rs:960) | Cek area dan panjang edge di `better`; kandidat "wrap" pada rank yang sama | M |
| Frame bench-75 setinggi 818px untuk 2 node | dummy mengadopsi path cluster (layout.rs:1549-1592) | Adopsi hanya di layer sebelah endpoint | M |
| Panah ke frame di ketinggian acak; tepi kiri frame tidak rata; frame bersebelahan selisih 8.5px; member berjarak 50px | scene.rs:1497-1503; layout.rs:1944; cluster_raw_boxes scene.rs:2296-2333; `GAP_B` layout.rs:56 / pair_sep 2987-3027 | Pusat frame; start-align; snap top/bottom; ~24px untuk member tanpa edge | S–M |

### 2.9 Layering dan ordering global (graf besar)

- **Diagram:** bench-75 (T), dense-td, dense-lr, lr, fanout-frames.
- **Akar masalah:**
  - DFS FAS mengikuti urutan deklarasi (layout.rs:1016-1047). Bench-75 mendapat 9 pembalikan padahal 5 cukup, termasuk edge mayoritas seperti svc2→db. Hasilnya hanya 33/75 edge mengarah ke kanan (mermaid 65), dan zona tersusun portrait.
  - Longest-path tanpa kompaksi cluster (layout.rs:1049-1074).
  - Neighbour BK duplikat (layout.rs:1361-1365 dan 2512-2521) membuat kipas keputusan di lr asimetris.
  - Sangat bergantung pada urutan deklarasi. Di dense-td, memindah satu baris `n45` mengubah lebar 4249→6661 (+57%), akibat rank-only pull tanpa chain (layout.rs:1414-1436) dan keep-best strict (layout.rs:1648).
  - `reorder`/`transpose` (layout.rs:2304-2398) tidak punya tie-break urutan deklarasi, dan transpose melewatkan pasangan dengan cluster path berbeda.
  - Crossing: dense-lr ~61 vs ~15 di mermaid; bench-75 364 vs 222, dengan 14 pasangan edge bersilang 2–3 kali.
- **Perbaikan:**
  - dedup BK (S, terverifikasi);
  - tie-break deklarasi setelah crossing minimization (S);
  - node yang hanya terhubung ke cluster memakai key cluster itu (M);
  - greedy FAS Eades-Lin-Smyth + sifting (L);
  - acyclicity tingkat cluster (supernode; edge minoritas jadi back-edge) (L);
  - network simplex dengan nesting edge ala dagre (XL).

### 2.10 Gap mask di persilangan

- **Diagram:** fanout, dense-td, dense-lr (59 gap), bench-75 (324 gap).
- **Akar masalah:**
  - Disk ~9px per crossing (casing.rs:29-32).
  - Cost greedy `(k-1)²+n²` (casing.rs:843-851) membagi gap ke garis yang berbeda, sehingga dua gap berjarak 14px memotong garis yang berlawanan (fanout).
  - Aturan anti-dash (quality.rs:70-79) masih membolehkan 2 potongan per 100px, jadi garis solid terbaca dashed.
  - Potongan 12px dari port membuat stub tampak terlepas.
- **Perbaikan:**
  - crossing yang berdekatan di satu garis diberi satu under-line (S);
  - edge yang lebih pendek dari ~3×`DASH_PERIOD` maksimal 1 potongan;
  - aturan konsisten: di LR, leg vertikal selalu di bawah leg horizontal;
  - tidak ada potongan dalam ~24px dari ujung;
  - setelah edge orthogonal, pakai hop semicircle seperti mermaid (M).
- **Dampak:** S. Volumenya terutama akibat §2.9.

### 2.11 Token tema, warna, dan kontras

- **Diagram:** semua.
- **Fakta:**
  - Palet per-shape (style.rs:21-80) dikunci oleh test `every_shape_has_a_distinct_stroke` (style.rs:100-113).
  - Tidak ada opsi tema (`SvgOptions` scene.rs:3791-3806, issue #57). `%%{init: {"theme": …}}%%` dan frontmatter diabaikan.
  - **`classDef default` diabaikan** (parser.rs:1985-1989). Akibatnya user sama sekali tidak bisa membuat tampilan netral, padahal ini fitur mermaid standar.
  - Stroke node 1.6 (scene.rs:2979, 3037, 3052-3053, 3111) dan edge 1.7 (2441). Beberapa stroke pastel kontrasnya <3:1 (shapes).
  - Dotted `"5 4"` (scene.rs:2444) terbaca sebagai dashed.
  - **Teks putih di atas aksen gagal AA di 8 renderer:** class.rs:240-251, er.rs:225-236, seq.rs:653-658, journey.rs:259, timeline.rs:301, pie.rs:258-260, mindmap.rs:41, dan nama lane gitgraph. Rasio kontras dengan putih: amber 2.62, green 3.21, teal 3.47, coral 3.55, steel 3.50, rose 4.14. Akarnya satu: `ACCENTS` (style.rs:84-93) ditambah `#ffffff` yang hard-coded, dan belum ada helper luminance.
  - Palet 8 warna berputar ulang: sankey punya 3 bentrok, slice pie ke-9 = slice 0, aktor journey ke-9 = aktor 1. Warna note sequence = aksen peserta ke-3 (seq.rs:37-38 vs style.rs:87).
- **Perbaikan:**
  - helper `contrast()` di style.rs untuk memilih teks putih atau `#111827` per fill (S, satu perubahan untuk 8 tipe). Jangan pakai `TEXT_COLOR #232840`, yang hanya 3.0–3.5:1 di indigo dan rose;
  - honour `classDef default` (S);
  - opsi `theme` dan baca init/frontmatter (M);
  - token Tier A: `EDGE_WIDTH` 2.0, thick 3.5, node stroke 1.8, `MARGIN` 16;
  - shadow halus yang bisa dimatikan;
  - dotted `"2 3"`.
- **Severity:** S. Bukan penyebab utama, tapi kontras adalah masalah aksesibilitas.

### 2.12 Ukuran node dan teks

- **Diagram:** long-labels (T), wide-labels, subgraph, shapes, lr, stress, state, dense-td.
- **Akar masalah:**
  - Label plain tidak di-wrap (layout.rs:389). Wrap hanya untuk markdown (markdown.rs:464) dengan `MD_WRAP_WIDTH` 175 yang mengasumsikan font 16px. Mermaid 12.2 memakai 14px dengan wrap 120px.
  - Diamond tanpa batas aspek (layout.rs:406) menghasilkan bentuk sliver 616×65.
  - `BASE_H` 38, `MIN_W` 54, `LINE_H` 17 (layout.rs:50, 52, 166).
  - `<<choice>>` kosong berukuran 80×64.6.
- **Perbaikan:**
  - wrap greedy di 120px untuk label **tanpa break eksplisit**, tanpa memotong kata, dan membaca `flowchart.wrappingWidth`;
  - tinggi diamond `H = max(1.7·BASE_H, W/1.6)`;
  - `BASE_H` 42, `LINE_H` 20, `MIN_W` ~96;
  - choice kosong 28×28, atau `Shape::Choice` tersendiri.
- **Effort:** wrap M, sisanya S ditambah update snapshot.
- **Dampak:** T untuk long-labels (lebar 672 vs 358 di mermaid).

### 2.13 Meta: test dan metrik kualitas mengunci atau tidak melihat defect

- **Gate yang mengunci defect:**
  - `layout_shape_gate` di tests/quality.rs:226-245 (`MAX_ASPECT` 5, `WIDE_LABELS_CENTER_OFF`) justru melindungi loop 230px di wide-labels. Setelah fix sumbu, aspect jadi 9.17 dan gagal gate, padahal mermaid sendiri 13.6.
  - labels.rs:2103 `wide_lr_labels_step_beside_their_line_off_the_arrowhead` mengunci label di samping garis.
  - style.rs:100-113 mengunci palet per-shape.
- **Metrik yang tidak melihat defect:**
  - wiggle hanya menghitung backtrack searah flow (router.rs:855-867, quality.rs:1207-1225), jadi hairpin menyamping tidak terhitung;
  - outline shape tidak dicek (quality.rs:470-485);
  - frame sendiri tidak dicek (router.rs:788);
  - run paralel yang berjarak 12–26px sepanjang 1100px tidak terdeteksi;
  - crossing skeleton layout (0) tidak sama dengan crossing yang tergambar (2) di fanout-frames;
  - render.rs:191 hanya menghitung jumlah subgraph.
- **Perbaikan:**
  - tambah metrik: lateral wiggle, endpoint-on-outline, edge-outside-own-frame, run hampir paralel yang panjang, sudut ujung terhadap normal sisi, rasio ink terhadap kanvas;
  - revisi gate yang mengunci defect;
  - tambah tes regresi korpus, misalnya advanced = 3 rect cluster dan `CI.bottom < Platform.top`.
- **Effort:** M. **Dikerjakan paling awal**, karena banyak prototipe "gagal" hanya karena ceiling lama.

### 2.14 Temuan yang bertentangan dan keputusan yang perlu diambil

1. **Palet semantik vs default netral.** Audit global menyarankan mempertahankannya sebagai identitas, dan banyak daftar "better" memujinya. Reviewer demo, lr, shapes, long-labels, dan subgraph menyarankan default netral. *Rekomendasi:* pertahankan palet semantik sebagai default, tambah `theme` (neutral/mermaid) dan honour `classDef default`, lalu gelapkan stroke sampai ≥3:1. Bukti menunjukkan warna bukan penyebab utama.
2. **Auto-wrap.** long-labels meminta wrap 120px. dense-td, dense-lr, fanout-frames, dan wide-labels memuji flowmaid karena *tidak* me-wrap (mermaid menghasilkan menara 4–8 baris dan orphan). *Resolusi:* wrap hanya label tanpa `<br>` eksplisit, tanpa memotong kata, dan lebarnya bisa diatur.
3. **Lebar wrap.** Audit menyarankan 175px (`MD_WRAP_WIDTH`). Reviewer long-labels menunjukkan mermaid 12.2 me-wrap di 120px dengan font 14px, dan asumsi 16px di markdown.rs:24-31 salah.
4. **Ukuran node.** Sizing seukuran teks dipuji karena kompak, tapi dikeluhkan karena spine tidak rata. Usulan `MIN_W` berkisar 80–152. *Kompromi:* 96 (Tier A).
5. **`GAP_L` 60→50.** Audit mengusulkannya, tetapi reviewer subgraph membuktikan gap lebih pendek memperparah S-curve sebelum edge orthogonal, dan isu label-rank justru butuh ruang lebih. *Rekomendasi:* turunkan hanya setelah §2.1(a) dan §2.6.
6. **Prototipe lane back-edge.** Reviewer back-edge melaporkan hanya 1 lib test gagal. Reviewer demo melaporkan regresi ceiling di bench-75/dense-td, dan quick fix men-trip `lbl_crowd` di wide-labels. Keduanya benar di cakupan masing-masing, jadi gate harus di-rebaseline (§2.13).
7. **Z di painter (Prototipe B).** Bagus untuk edge antar-rank bersebelahan, tetapi memperburuk fanout. Long edge dan back-edge harus diurus router.
8. **Dev→CI di advanced.** Audit menulis edge "lari keluar kanvas" (`M 656.5 68 … 656.5 28`). Reviewer menyebutnya panah ke atas menuju border frame di samping judul. Ini gejala yang sama (stub ke border atas frame).
9. **Komentar `%%` di akhir baris** dibuang di class dan sequence. Hasilnya lebih bersih daripada mermaid, yang mencetak teksnya, tetapi ini divergensi yang perlu didokumentasikan.
10. Klaim class "Arrowhead types differ in visual weight" **dibantah** dan dibuang.

---

## 3. Bug kebenaran (prioritas tertinggi)

### P0: terlihat di korpus (informasi hilang, makna salah, atau overlap)

| # | Bug | Diagram | Akar (file:line) | Perbaikan | Effort |
|---|---|---|---|---|---|
| 1 | Subgraph bersarang "Async Workers" tidak digambar (2 rect untuk 3 subgraph) | advanced | parser.rs:2613-2624 re-claim node ke blok yang masih terbuka sehingga Workers kosong; scene.rs:2324 melewatkan bbox tak hingga | Jangan re-claim node milik subgraph yang sudah ditutup. Tambah `assert_eq!(workers.nodes.len(), 3)` di render.rs:191. **Diprototipe, 620 test lulus** | S |
| 2 | `Reg --> Platform` / `Rollback --> Platform` tidak memengaruhi ranking, sehingga cluster CI berdampingan dengan Platform, bukan di atasnya | advanced | `sub_edge_proxies` scene.rs:1547-1583: entry set kosong, scene.rs:1034 tidak membuat stand-in | Fallback ke member pertama, atau DFS yang mengabaikan back-edge internal. **Diprototipe** | S |
| 3 | Dev→CI digambar sebagai panah ke atas ke border frame sendiri | advanced | scene.rs:1236-1262, `own_box_stub` 1403-1430; stand-in dan rank group 1031-1046 (juga penyebab gap kosong dan 'fail' jadi back-edge) | Gambar stand-in sebagai edge Dev→Build sungguhan, atau buang rank group-nya | S |
| 4 | Chip label ER menutupi lingkaran "zero" sehingga kardinalitas terbaca salah. Pada gap yang dilebarkan pas untuk label, **kedua** glyph tertutup | er | er.rs:121-126 (Open/Dotted tanpa marker, jadi placer tidak tahu glyph); layout.rs:1895-1907; label digambar terakhir er.rs:273-278 | `glyph_reach(card)`, re-centre label di bentangan bebas, end reserve di layout, test "glyph di luar rect label" | M |
| 5 | Hierarki class terbalik (parent/interface/composite di bawah anak), membentuk siklus palsu, arc 'charges' raksasa, dan 1 crossing | class | parser.rs:3302-3307 menormalisasi (from,to); class.rs:134 meranking dengan arah semantik | `head_at_from` di `ClassRel`; edge layout dalam urutan sumber; glyph di ujung yang benar | M |
| 6 | class.rs mengabaikan `SceneEdge.waypoints`: edge yang di-reroute digambar lurus menembus kotak, dan segitiga inheritance jadi polygon ukuran nol (repro: 4 edge tembus, 3 segitiga hilang). Label 'contains' meleset 18.5px | class (er.rs:200 kemungkinan sama) | class.rs:208-219 hanya menggambar `e.bezier`; router mengisi `bezier=[p0,p0,p3,p3]` (router.rs:3874, 4597) | `edge_path_d(e)` bersama (dari scene.rs:2885-2896) dan `edge_polyline` untuk glyph/kartu | S–M |
| 7 | Panah dependency 'charges' setengahnya di dalam PaymentMethod | class | `head(q[3],q[2])` class.rs:220 mengikuti tangen; box digambar setelah glyph (229-262) | Orientasi menurut normal sisi; glyph digambar setelah box | S |
| 8 | Informasi dibuang diam-diam: note state, note class, `«interface»`, 33 komentar atribut ER, `" : "` sebelum return type | state, class, er | parser.rs:4885-4893; parser.rs:3149-3153; parser.rs:3128-3131; er.rs:34-41, 167-173; class.rs:168-172 | Model dan render. Note sebagai node sintetis dengan edge dotted (pola mermaid) | S–M masing-masing |
| 9 | Ujung edge melayang dari outline parallelogram/hexagon (7–10px) dan cylinder (1–8px) | shapes, advanced, dense-lr | §2.5 | `outline_n` + `snap_ends` | M |
| 10 | `direction LR` di composite/subgraph diabaikan: state jadi kolom 126×368 dan kanvas 403×1133 | state, flowchart | scene.rs:1018 | Compound layout | L |
| 11 | Sequence: endpoint pesan di pusat lifeline sehingga kepala menembus bar aktivasi; bar `activate` eksplisit mulai/berakhir 14px telat; note di dalam `alt` menjorok keluar frame (kanan **dan** kiri); divider digambar di atas bar (laten) | sequence | seq.rs:322; seq.rs:364-365; seq.rs:281-282 dan 247-251; seq.rs:529-585 | `edge(p, toward_x)` sadar bar; `last_msg_y`; post-pass melebarkan frame; urutan cat diubah | S–M |
| 12 | Architecture: edge web→api tersembunyi di belakang Mobile App (terbaca Web-Mobile-API); svc→cache lewat dalam kotak PostgreSQL sehingga Redis tampak lepas | architecture | `orthogonal_route` tanpa sisi port dan obstacle (architecture.rs:204-206, 388-401); layout grid urutan sumber (124-142, 251-342); edge dicat sebelum node (452-511) | Router sadar port dan obstacle; test "tidak ada segmen menembus node" | M |
| 13 | Label 'parallel' menutupi edge yang tidak berlabel; thick A==>D menembus apex self-loop | stress | labels.rs:1159-1161; `loop_points` scene.rs:3679-3688 selalu di kanan; layout.rs:1350 dan `pair_sep` tanpa ruang loop; self-loop bukan obstacle (edgepass.rs ~196-199, router.rs:4013) | `reads_own` strict + offset paralel lebar; ruang loop di sisi bebas. **Harus bersamaan** dengan pemilihan sisi back-edge yang sadar crossing, karena tanpa itu channel jatuh 2px dari bow | M |
| 14 | Pasangan edge saling menimpa 30–200 unit (solid dan dashed menyatu); 11 edge memotong 28 pasangan frame asing | dense-td | router.rs:4200-4262, 4515 (pasangan clean yang diterima lebih dulu tidak pernah dicari ulang) | Pass kedua untuk pasangan overlap/tight; overlap dinilai sebelum frame bila frame setara | M |
| 15 | Gitgraph: REVERSE digambar sebagai ring kosong (terbaca merge/empty); kurva merge mencoret label "merge-hotfix"; merge tidak bisa dibedakan dari commit biasa | gitgraph | gitgraph.rs:274-277; 121-130 dan 297-301; 342-351 | Disc dengan tanda X; halo atau label dipindah; glyph merge | S |
| 16 | Pie: `accTitle`/`accDescr` dibuang dan aria-label selalu "Pie chart"; label "4%" keluar dari wedge-nya | pie | parser.rs:3785-3788, pie.rs:190; pie.rs:252-265 | a11y; fit test posisi label | S–M |

### P1: laten (input mermaid yang valid gagal atau rusak)

- **Flowchart:**
  - `classDef default` diabaikan (parser.rs:1985-1989);
  - tema dari init/frontmatter diabaikan;
  - output bergantung pada urutan deklarasi (+57% lebar di dense-td);
  - Scene API untuk LR dengan label lebar salah geometri (scene.rs:916-918).
- **Class:**
  - baris `<<interface>> Name` yang berdiri sendiri menghasilkan parse error;
  - `class Box~T~` menghasilkan dua kelas;
  - `direction LR` diabaikan (class.rs:122, parser.rs:3151).
- **ER:**
  - `direction` menghasilkan parse error (parser.rs:2949);
  - alias entitas `p[Person]` ditolak;
  - arah LR di-hard-code (er.rs:114), padahal default mermaid TB.
- **Gitgraph:**
  - `gitGraph TB:` / `LR:` ditolak (parser.rs:43-55);
  - `branch dev order: 2` jadi nama branch (parser.rs:4329-4331);
  - `merge x type: HIGHLIGHT` kehilangan type (parser.rs:4419);
  - tag di TB keluar kanvas (gitgraph.rs:116-119);
  - id panjang bertabrakan dan terpotong (gitgraph.rs:23, 66, 121-130; `BRANCH_LABEL_W` 70 di :21).
- **Architecture:**
  - `junction` ditolak (parser.rs:4608);
  - modifier `{group}` ditolak (parser.rs:4777-4801);
  - service tanpa group ditumpuk di (0,0) (architecture.rs:97-105);
  - label panjang meluap dari kotak 150px.
- **Journey:**
  - skor 3.5 atau -1 ditolak (parser.rs:4081-4084; mermaid memakai `Number()`);
  - nama section panjang terpotong atau hilang (journey.rs:117-122, 256-263);
  - aktor yang banyak meluap (journey.rs:188-198).
- **Timeline:**
  - nama section yang lebar meluap dan jadi tidak terlihat (timeline.rs:166-167, 226-227, 299-305);
  - judul panjang terpotong (timeline.rs:193/253; journey.rs:134 sama, karena `text_width` dikalibrasi untuk 14px regular, bukan 17px bold).
- **Sankey:**
  - label bertimpa saat `width: 450` atau nama panjang, karena tidak ada pengukuran teks (sankey.rs:433-451);
  - `nodeColors` diabaikan.
- **Pie:**
  - lebar legend salah ukur (pie.rs:126, 145);
  - pembulatan half-even (pie.rs:260);
  - label slice 1–3.99% disembunyikan (`MIN_LABEL_FRAC` pie.rs:33), padahal mermaid menampilkannya.
- **Mindmap:**
  - `((circle))` jadi elips (mindmap.rs:93-96);
  - node default identik dengan rounded (parser.rs:3998);
  - `::icon`/`:::class` dibuang (parser.rs:3900);
  - dokumentasi basi (render.rs:86-87, model.rs:1132-1135).

---

## 4. Tabel per-diagram

| Diagram | Verdict | Isu teratas | Yang lebih baik di flowmaid |
|---|---|---|---|
| demo | Lebih bersih dari mermaid, kecuali back-edge dan kepala thick | Loop E→B berbentuk lasso, panah 14° setengah tertutup; kepala `==>` 2×; rank berlabel sempit sehingga yes/no jadi kait | Spine lurus; diamond 102×65 (mermaid 171×171); chip label; node seukuran teks |
| advanced | Bentuknya bagus, strukturnya salah | Async Workers hilang; edge ke Platform tidak memengaruhi ranking; Dev→CI stub ke atas; 'fail' dan Cache→Gate keluar frame; kepala thick 2× | Self-loop W1 benar (mermaid rusak); Rollback→Platform pendek; urutan Worker terjaga; tidak ada edge menembus judul |
| subgraph | Benar secara semantik; kalah ritme dan ukuran | Ruang mati 26px di bawah baris pertama cluster; spine tidak rata; nested tidak terbedakan; fan-out off-centre | Judul cluster tidak pernah dicoret (mermaid mencoret "Platform" dan "Event bus"); 4.3 KB vs 38.6 KB |
| lr | Satu V-loop membuatnya tampak rusak | V-loop CACHE→SVC (+73px); kepala thick 2×; 'denied' menempel di 401; kipas keputusan asimetris (dup BK) | Diamond kompak; chip label (garis dotted mermaid menembus label); lebar 790 vs 1074 |
| shapes | Paling dekat ke mermaid | Ujung edge melayang di sisi miring; label cylinder terlalu tinggi dan tutupnya kosong; stroke tipis berkontras rendah | Semua shape kotak setinggi 38px; diamond dan hexagon kompak |
| stress | Elemen bagus, edge berebut satu strip 65px | Thick menembus self-loop; 'parallel' di edge yang salah; back-edge menyilang thick; apex diamond penuh; S-wobble | Area 38% lebih kecil; self-loop yang jelas; 'no' dekat keputusannya |
| fanout | Komposisi seimbang, loop-back menembus kipas | Loop-back 'no' lewat interior (2 crossing, gap berlawanan); gaya edge campur; port Worker off-centre dan tidak merata | Simetris pada satu spine; area 44% lebih kecil |
| long-labels | Hampir semua kekalahan karena tidak ada wrap | Node 442×38, diamond 616×65 (9.5:1), lebar 672 vs 358; label menjepit gap 60px; panah meleset 43.7px | Tidak ada orphan wrap; ujung panah tepat di border (mermaid menyisakan gap 4px) |
| back-edge | Spine sempurna, back-edge-nya buruk | Arc "telinga", 41% kepala tertutup; start 14px dari 'no'; kepala thick 2× dan stub; kanvas +37px | Spine lurus (mermaid punya jog Z); diamond kompak |
| wide-labels | Bug sumbu label menghasilkan loop 230px | Bow LR memakai lebar label 337px; viewBox dari titik kontrol; Database melorot; label mengambang dengan 3 cara | Chip berpadding; label satu baris tanpa orphan; port terpisah per edge |
| fanout-frames | Node rapi, narrowing salah | g3 dipindah ke kolom kedua (+9% area); detour g3 terjepit; 7 edge kipas campur S dan elbow; 2 crossing yang bisa dihindari; tepi frame tidak rata | Menghormati `<br/>` (cylinder 3 baris vs 8); 27% lebih pendek |
| dense-td | Kompak (43% area), routing belum selesai | 3 gaya edge; simpul di hub n16; cluster berdampingan; 'amet' di sisi jauh (edge 3316 unit); node tanpa edge di rank 0 | Dotted terjaga (mermaid membuat 2 dari 3 jadi solid); judul tidak tertabrak; tidak ada edge menembus node |
| dense-lr | Bagian-bagiannya bagus, jaringan kusut, LR jadi portrait | Hub 12 edge keluar dari 3 sisi; gaya campur; frame kosong; ~61 crossing (mermaid ~15); 4 label lepas | Area 2.4× lebih kecil; tanpa detour perimeter 3000px |
| bench-75 | Kompak tapi kusut, arah LR hilang | DFS FAS (33/75 edge ke kanan); zona tidak berupa kolom; hairpin; 324 gap; 364 crossing (mermaid 222) | Aspek 0.88 vs 4.1; kepala tidak menumpuk jadi blob hitam |
| class | Lebih kecil tapi salah dibaca | Hierarki terbalik; panah 'charges' tertutup dan arc raksasa; note dan «interface» hilang; waypoint diabaikan; kontras header; kompartemen samar | Komentar `%%` dibuang; chip label; segitiga 16×14 lebih terbaca |
| er | Tabel lebih bersih, kardinalitas dan komentar hilang | Chip menutupi lingkaran zero; 33 komentar dibuang; LR hard-coded sehingga terbentuk kolom kiri; kontras header 3/4 gagal | Tanpa grid berat; hierarki tipe/nama/kunci; 602×950 vs 1519×1150 |
| state | Elemen lebih bersih, note hilang, LR diabaikan | Note hilang; composite LR jadi kolom; retry arc menyerempet sisi node; choice 80×64.6; cabang fork berbentuk S | `<<choice>>`/`<<fork>>` benar (mermaid 12.2 menggambar kotak); tanpa border ganda |
| sequence | Tidak jelek; masalah di presisi dan header | Panah tidak memperhitungkan bar; `activate` telat 14px; note keluar dari alt; warna note = peserta; actor bukan stick figure; tanpa footer | Separuh area; autonumber bold; frame full-width sejajar |
| pie | Lebih baik dari mermaid | "5%4%" menyatu/keluar wedge; teks putih gagal AA di 4/5 slice; radius 110 kecil; slice 0.35% hanya garis | Palet jelas; separator putih; judul di tengah; slice <1% tetap tampil |
| mindmap | Palet lebih tenang, layout ring terlalu jarang | Ink hanya 9% (ring 172); spoke tidak rata; root pucat; circle jadi elips; cloud mirip bang | Tanpa wrap janggal; urutan radial searah jarum jam |
| journey | Lebih bersih, kurva kepuasan hilang | Semua wajah di satu baseline; warna section = aktor = skor; band saling menempel; kontras label section | 573×250 vs 1300×540; wajah r=22 berwarna sesuai skor |
| gitgraph | Topologi benar, tampilan seperti draf | Semua edge navy 2px; kurva mencoret label merge; merge/REVERSE/HIGHLIGHT/tag tidak terbedakan; tanpa lane guide | Label horizontal 13px (mermaid 10px diputar); palet tenang |
| timeline | Lebih benar dari mermaid | Garis dashed mencoret teks event; periode dan event sama secara tipografi; sumbu samar; band hijau 3.21:1 | 6 periode tampil semua (mermaid membuang 3); 624×201 vs 995×643 |
| sankey | Setara atau lebih baik | Palet 8 warna bentrok (Losses = Electricity); label Electricity dan Industry sebaris tanpa pengukuran; 1 crossing tersisa | 1 crossing (mermaid 4); label di atas ribbon dan terpusat; tooltip |
| architecture | Mermaid sendiri rusak; flowmaid rapi tapi topologinya salah | Edge web tersembunyi; svc→cache lewat PostgreSQL; layout satu baris yang mengabaikan port; emoji 16px | Tidak ada node bertumpuk (mermaid: "MWobile App"); setiap service punya ikon |

---

## 5. Roadmap

Urutan di bawah dihitung dari **(dampak visual × jumlah diagram terdampak) / effort**. Item dalam satu baris yang sama memang harus dirilis bersamaan.

### Quick wins (S)

| # | Apa | Di mana | Kenapa | Ukuran sukses |
|---|---|---|---|---|
| Q1 | Marker panah satu ukuran (`userSpaceOnUse`) dengan glyph lebih lebar. **Satu PR dengan M1 (trim)** | scene.rs:2494, 2447, 2537-2543, 2598-2602, 2806-2808; advance.rs:5772 | Kepala 2× di 6 diagram; glyph sempit di semua diagram | Kepala thick = kepala normal (~11px); tidak ada piksel stroke di luar segitiga pada zoom 8×; 2 test marker diperbarui |
| Q2 | bbox kanvas dari ekstrem kurva | scene.rs:886-912, 3199-3201, 916-918; edgepass.rs:339 | Margin timpang di 7 diagram | Selisih margin kiri dan kanan ≤4px; back-edge 351→≤314; stress 251→≤220 |
| Q3 | Parser subgraph bersarang + fallback proxy subgraph | parser.rs:2613-2624; scene.rs:1547-1583 | Bug kebenaran P0 #1–#2 | advanced punya 3 rect cluster; `CI.bottom < Platform.top`; Reg→Platform jatuh lurus |
| Q4 | Back-edge quick fix (hanya layer di antara, anchor ke arah bow, lead diskalakan) + bug sumbu label LR + aturan yang sama di `drawn_breadth`, sekaligus revisi gate | scene.rs:3323-3324, 3356, 3360, 3370, 798, 730; layout.rs:2099-2150, 2138-2142; tests/quality.rs:226-245 | Sekitar 11 diagram; elemen paling jelek | demo lebar ≤321; wide-labels tinggi ≤201; sudut masuk panah ≥37° (target ≥60°); tidak ada kepala yang overlap node |
| Q5 | Helper kontras di style.rs, dipakai di 8 renderer | style.rs; class/er/seq/journey/timeline/pie/mindmap/gitgraph | Gagal AA di 8 tipe | Unit test: setiap `ACCENTS` × teks terpilih ≥4.5:1 |
| Q6 | Palet cluster per depth/top-level + stroke ≥3:1 | scene.rs:2820-2833 | Nesting tidak terbaca di 6 diagram | Frame bersarang punya fill berbeda; kontras stroke ≥3:1 |
| Q7 | Cylinder: label di `cy+0.75ry`, tutup terisi, stroke yang di-resolve, `ry` diskalakan | scene.rs:3029-3048, 3129-3135, 3037, 3052-3053, 3111; advance.rs | 5 diagram | Pusat teks di pusat body; lid ber-fill; `style stroke-width` berlaku untuk lid dan bar subroutine |
| Q8 | Dedup neighbour BK | layout.rs:2512-2521 (atau 1361-1365) | Kipas lr asimetris | Offset ERR di lr 6.6→0px; SVC/ERR simetris terhadap Auth? |
| Q9 | Honour `classDef default`; dotted `"2 3"` | parser.rs:1985-1989; scene.rs:2444 | Kompatibilitas mermaid; dotted terbaca dashed | Contoh `classDef default fill:#fff` berlaku untuk semua node tanpa kelas |
| Q10 | Token stroke Tier A: `EDGE_WIDTH` 2.0, thick 3.5, node 1.8 (termasuk yang hard-coded), `MARGIN` 16 | scene.rs:33, 2441-2442, 2979…; layout.rs:58 | Kesan tipis/samar (prototipe kolom A) | Snapshot diperbarui; tidak ada perubahan layout |
| Q11 | Sequence: y `activate` = pesan sebelumnya; frame menampung note; warna note di luar `ACCENTS`; bar dicat setelah divider; halo pada label frame | seq.rs:364-365, 281-282, 37-38, 529-585 | 4 bug presisi | Bar y0 = y pesan; tidak ada note yang memotong border frame |
| Q12 | Gitgraph: warna edge per branch, stroke 4–5 round cap, kontrol elbow di sudut, glyph merge/reverse/highlight, halo label, parse `TB:` | gitgraph.rs:297-305, 332-351, 274-281; parser.rs:43-55 | Tampilan draf yang paling mencolok | Tidak ada kurva yang memotong label; REVERSE punya tanda X |
| Q13 | ER: default TD + parse `direction` | er.rs:114; parser.rs:2949 | Kolom kiri dan portrait | Korpus er jadi landscape; `direction LR` diterima |
| Q14 | State: choice kosong 28×28; ambang label saudara 20–24 | layout.rs:406, 1863 | Choice terlalu besar; 'valid'/'invalid' menempel | Jarak antar-chip ≥20px |
| Q15 | Casing: satu under-line per kumpulan crossing di garis yang sama | casing.rs:517-533, 843-851 | Garis tampak putus (fanout dan lainnya) | Tidak ada pola gap berlawanan dalam 25px |
| Q16 | Polish tipe lain: pie (`LABEL_R` 0.72, `RADIUS` 130, lebar legend); journey (cy menurut skor, palet section, gutter); timeline (halo event, urutan dot dan garis, periode bold); sankey (palet anti-bentrok, backward sweep); mindmap (root gelap, circle `(d,d)`, taper lebih curam) | pie.rs:18/30/126/145; journey.rs:155/176/116-122; timeline.rs:321-337; sankey.rs:366-399/446; mindmap.rs:38-39/93-96/271 | Pertahankan keunggulan, buang cacat kecil | Label pie muat di wedge; sankey 0 crossing dan 0 node bersebelahan sewarna; tidak ada teks event yang tercoret |
| Q17 | Class: render waypoint lewat `edge_path_d` bersama; glyph menurut normal sisi dan digambar setelah box | class.rs:208-262; cek er.rs:200 | Bug P0 #6–#7 | Repro t1.mmd: 0 edge menembus kotak, 0 polygon berukuran nol |
| Q18 | Architecture: VS16 di `icon_glyph`; wrap group top-level | architecture.rs:38-65, 127-142 | Emoji campur warna/mono; aspek 6.6:1 | Semua ikon berwarna; aspek ≤2:1 |

### Medium (M)

| # | Apa | Di mana | Kenapa | Ukuran sukses |
|---|---|---|---|---|
| M0 | **Prasyarat:** metrik kualitas baru + revisi gate yang mengunci defect (§2.13) | tests/quality.rs; quality.rs:470-485, 1207-1225; router.rs:855-867; labels.rs:2103; style.rs:100-113 | Banyak prototipe gagal hanya karena ceiling lama | Metrik baru (endpoint-on-outline, lateral wiggle, edge-outside-own-frame, sudut ujung) melaporkan defect korpus saat ini |
| M1 | Trim path di bawah kepala + stub akhir lurus ≥ 2×`ARROW_MARKER_PX` | emit cubic/`spline_d`/`curve_geometry` scene.rs:2884-2897; router.rs:2701 | Pasangan Q1; kink kepala di semua diagram | Sudut kepala terhadap normal sisi ≤10° untuk ≥95% edge |
| M2 | Edge antar-rank bersebelahan jadi rounded Z (Prototipe B), dengan port terpusat dan label di run lurus | `edge_points` scene.rs:3337/3344; `EdgeCurve::Rounded` 376-392 | Separuh dari §2.1 dengan biaya kecil | Jumlah edge S-cubic di korpus berkurang ≥80% tanpa regresi crossing |
| M3 | Paket ruang label + port: gap berlabel TD/BT dan LR (dengan ruang kepala), bias 0 untuk ujung sendirian, port merata untuk k≥3, weighted PAV, straight-pair snap | layout.rs:1895-1912, 1855-1877; scene.rs:3293; ports.rs:256-290, 460 | 10+ diagram; harus satu paket (coupling) | long-labels: offset kedatangan 43.7→<5px; fanout: jarak port ±2px; gap berlabel ≥~72px |
| M4 | Back-edge sebagai lane orthogonal (terintegrasi router) + 2-cycle di dalam gap + frame containment | scene.rs:3347-3370; layout.rs:1361-1376; ports.rs:331-336; router.rs:788; scene.rs:1091 | Paritas penuh untuk §2.2 | Semua back-edge masuk ±10° dari normal; 0 segmen keluar frame sendiri; tinggi lr ≤226 |
| M5 | `H_WEIGHT` 1.0 untuk static + overlap dinilai sebelum frame bila frame setara + biaya pindah sisi untuk edge forward + re-plan port setelah repair | router.rs:227, 443-472, 4515, 153, 2690-2722; edgepass.rs:309-321 | Jalur suboptimal, port di sisi samping | fanout-frames 2→0 crossing; dense-td: edge yang memakai port samping 11→≤3; waktu render tetap dalam budget |
| M6 | Snap ujung ke outline shape + Profile di ports + metrik | §2.5 | Bug P0 #9 | Jarak ujung ke outline ≤1px untuk semua shape (test repro hex.mmd) |
| M7 | Auto-wrap label plain (120px, hanya tanpa break eksplisit) + batas aspek diamond | layout.rs:389, 406; markdown.rs:464 (factor `wrap_line`) | long-labels, wide-labels | long-labels lebar ≤~400px; aspek diamond ≤1.6 |
| M8 | Slot header cluster asimetris (cari dependensi 26px di router) | layout.rs:1486-1496, 1879-1919 | Ritme vertikal di semua diagram ber-cluster | Gap di dalam cluster = `GAP_L`; 4 gate dense-td tetap lulus |
| M9 | Note di state dan class, `«interface»`, komentar ER, cadangan ruang glyph ER | parser.rs:4885, 3128, 3149; er.rs; layout.rs:1895-1907 | Bug P0 #4, #8 | Korpus state/class/er memuat semua teks sumber; 0 glyph tertutup label |
| M10 | Opsi `theme` (neutral/mermaid) + baca init/frontmatter | `SvgOptions` scene.rs:3791-3806; style.rs | Kompatibilitas; memberi user pilihan tampilan | `%%{init:{"theme":"neutral"}}%%` menghasilkan node putih dengan satu warna tinta |
| M11 | Acceptance `narrow()` (area + panjang edge) + kandidat "wrap" pada rank yang sama | layout.rs:505, 593, 879-985 | fanout-frames | Narrowing tidak pernah menambah area >2% |
| M12 | Sequence: endpoint sadar aktivasi, peserta footer, stick figure untuk actor | seq.rs:322, 405-418, 650-660 | Paritas mermaid | Ujung kepala = tepi bar; footer tampil |
| M13 | Class: ranking urutan sumber + penempatan kartu multiplicity | parser.rs:3302-3307; class.rs:134, 325-345 | Bug P0 #5 | Parent di atas anak; 0 crossing di korpus class |
| M14 | Router architecture yang sadar port + obstacle | architecture.rs:204-206, 388-401 | Bug P0 #12 | 0 segmen menembus node; urutan baris tidak mengubah topologi |
| M15 | Hop semicircle di crossing orthogonal | casing.rs; emit | Garis putus dan dashed di graf padat | Garis tetap kontinu di dense-*/bench-75 |

### Big (L/XL)

| # | Apa | Di mana | Kenapa | Ukuran sukses |
|---|---|---|---|---|
| B1 | **Edge orthogonal di semua flowchart static** (terintegrasi router: chain dummy jadi polyline, `nudge`/`spread`, `GAP_L` per jumlah leg) | scene.rs:784-870, 3738; router.rs | Faktor visual terbesar (§2.1) | 100% edge flowchart orthogonal dan membulat; crossing bench-75 ≤ mermaid |
| B2 | Greedy FAS + network simplex dengan nesting edge + acyclicity tingkat cluster + penempatan node tanpa edge | layout.rs:989-1182 | bench-75, dense-td, dense-lr: arah LR, frame kosong, crossing | bench-75 ≥60/75 edge ke kanan; dense-lr landscape; frame tidak lebih lebar dari isinya + padding |
| B3 | Compound layout untuk `direction` subgraph/composite | scene.rs:1018 | state, flowchart | Composite state LR jadi satu baris (~459×109) |
| B4 | Bundling "comb" untuk fan (≥3 edge dari satu sisi) dan pita port hub | scene.rs:1460-1536; ports.rs | fanout-frames, dense-td hub | 0 crossing di dalam fan; port hub merata |
| B5 | Layout architecture yang sadar port (spatial map BFS) | architecture.rs:124-142, 251-342 | Topologi architecture | cache tepat di bawah svc; web dan mobile satu kolom |
| B6 | Layout mindmap berpusat ke parent (atau relaksasi gaya) | mindmap.rs:137-197 | Kanvas jarang | Ink ≥25%; rasio spoke max/min ≤2 |
| B7 | (Alternatif M3) `minlen=2` + dummy label ala dagre | layout.rs:989, 1396-1408 | Akar bersama label sempit dan off-centre | Label edge pendek selalu di run lurus |

**Urutan sprint yang disarankan:**

- Sprint 1: M0 → Q1+M1 → Q2 → Q3 → Q4.
- Sprint 2: Q5–Q9 → M3 → M2.
- Sprint 3: M4/M5 → B1.

**Bukti prototipe** (repo tidak disentuh):

- Fix subgraph advanced: `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rv_adv_skeptic/fm` (hasil di `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rv_adv_skeptic/fix2.png`)
- Lane back-edge: `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rv_be/fix_ABCD.png`
- lr sebelum/sesudah lane: `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rv_be/corp/lr.cmp.png`
- Gabungan fix stress: `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/stressdbg/xall7.png`
- Gap label + bias 0: `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rev_ll/acw.png`
- `H_WEIGHT` 1.0: `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/fmx_fanframes/expH.svg`
- Tier A/B token: `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/proto-out/` (file `*.four.png`)
- Side-by-side korpus: `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/cmp/out/` (file `<name>.side.png`)

---

## 6. Yang sudah lebih baik di flowmaid (jangan sampai regresi)

| Keunggulan | Bukti | Cara menjaganya |
|---|---|---|
| Chip label putih (rx 4, border `#d5d9ec`, padding) | Semua flowchart, class, state. Label mermaid berupa blok abu 50% tanpa padding dan sering ditembus garis | Tetap pakai chip saat label dipindah ke run lurus |
| Judul cluster rata kiri di atas chip, tidak pernah dicoret edge | subgraph, advanced, dense-td. Mermaid mencoret "Platform", "Event bus", dan lainnya | Chip judul tetap jadi obstacle router setelah B1 |
| Diamond kompak untuk label pendek | demo 102×65, lr, stress, fanout, back-edge (mermaid 171×171) | Batas aspek hanya untuk label panjang; jangan memakai diamond persegi ala mermaid |
| SVG portabel: `<text>` tanpa `foreignObject`, 5–10× lebih kecil, `role=img`, `aria-label`, `<title>` per node dan edge | Semua diagram | Wrap memakai `tspan`; tes render di rsvg/resvg |
| Node seukuran teks, kanvas kompak | dense-td 43% area, dense-lr 2.4× lebih kecil, bench-75 aspek 0.88 | `MIN_W` moderat (~96), jangan 152 |
| Menghormati `<br/>` dan tanpa wrap orphan | fanout-frames, dense-td, dense-lr, long-labels | Auto-wrap hanya untuk label tanpa break eksplisit; tidak memotong kata |
| Spine lurus dan simetris | demo, back-edge, fanout (mermaid punya jog Z) | Z-routing dan lane tidak boleh menambah jog pada edge yang sudah segaris |
| Self-loop berupa loop yang jelas | advanced, stress (self-loop mermaid rusak) | Ruang loop di sisi yang bebas, tanpa mengubah bentuk |
| Edge pendek yang langsung, tanpa detour perimeter | advanced Rollback→Platform, state 'invalid', dense-lr | Biaya router harus tetap menolak jalur perimeter |
| Port tersebar, tidak ada "black bar" atau blob kepala | bench-75, wide-labels | Distribusi merata ≥`PORT_SPACING` |
| Urutan sumber terjaga | Worker 1/2, mindmap searah jarum jam | Tie-break deklarasi (Q-items, B2) |
| Semantik dotted dan thick terjaga | dense-td (mermaid membuat 2 dotted jadi solid) | Test dash pattern tetap ada setelah `"2 3"` |
| Palet semantik per-shape | Dipuji di demo, lr, shapes, stress, fanout, dense-* | Jadikan tema; jangan dihapus |
| Edge `#44507a` lebih lembut (~8:1) | Audit global | Pertahankan di tema default |
| `<<choice>>`/`<<fork>>` state benar | Mermaid 12.2 menggambar kotak "decide"/"forked" | **Jangan** meniru bug mermaid demi paritas |
| Timeline menampilkan periode sebelum section pertama | Mermaid membuang 2002/2004/2005 | Pertahankan sebagai test |
| Sankey: ordering barycentre, label di atas ribbon dan terpusat (`dy=0.33em`), margin 24px, tooltip | 1 crossing vs 4 | Backward sweep hanya boleh menurunkan crossing |
| Sequence: autonumber bold inline, frame full-width sejajar dengan jarak 16px, kondisi di samping chip, tanpa shadow | Separuh area mermaid | Frame per-scope dijadikan opsi, bukan default |
| Pie, journey: palet kategorikal, separator tipis, wajah berwarna sesuai skor, legend terpusat | Lebih modern dari mermaid | Perbaiki kontras tanpa membuang warna |
| Header aksen class/ER/sequence | Lebih menonjol dari pastel mermaid | Gelapkan aksen atau pilih teks gelap untuk kontras; jangan ganti ke header pucat tanpa keputusan desain |
| Komentar `%%` di akhir baris di-strip (class, sequence) | Mermaid mencetaknya literal | Dokumentasikan sebagai divergensi yang disengaja |
| Ujung panah tepat di border | long-labels (mermaid menyisakan gap 4px) | Trim stroke (M1) dengan `refX` yang menjaga ujung tetap di border |