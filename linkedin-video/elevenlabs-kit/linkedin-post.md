# LinkedIn post: flowmaid video

## English (recommended; the video is in English)

flowmaid in 36 seconds: Mermaid diagrams rendered in pure Rust (voice and sound effects by elevenlabs.io)

I've been building flowmaid, a Mermaid diagram engine in Rust with zero dependencies. Mermaid text in, clean SVG out.

What the video shows:
- 12 diagram types: flowchart, sequence, class, state, ER, pie, user journey, git graph, mindmap, timeline, sankey and architecture
- About 113x faster than mermaid.js (geometric mean over 25 sample diagrams)
- It runs in the browser as WebAssembly: drag a node and its edges re-route live
- flowcli, my terminal diagram viewer built on flowmaid, redraws the diagram every time you save the file (--watch)
- dbx, my terminal database client, uses flowmaid to lay out your database schema as an ERD

Try it in your browser: https://opensource.go-routine.com/flowmaid-web/
Code: https://github.com/go-routine-id/flowmaid (GPL-3.0-or-later)
Rust: cargo add flowmaid

Feedback and issues on GitHub are welcome.

#rustlang #OpenSource #WebAssembly #Mermaid #Diagrams #DevTools

## Bahasa Indonesia (alternatif)

flowmaid dalam 36 detik: diagram Mermaid dirender dengan Rust murni (voice-over dan efek suara: elevenlabs.io)

Saya sedang mengembangkan flowmaid, mesin diagram Mermaid yang ditulis dengan Rust, tanpa dependensi. Input-nya teks Mermaid, output-nya SVG yang rapi.

Yang ada di video:
- 12 jenis diagram: flowchart, sequence, class, state, ER, pie, user journey, git graph, mindmap, timeline, sankey, dan architecture
- Sekitar 113x lebih cepat daripada mermaid.js (rata-rata geometrik dari 25 contoh diagram)
- Berjalan di browser sebagai WebAssembly: geser sebuah node, garis-garisnya langsung ikut menyesuaikan
- flowcli, viewer diagram di terminal yang saya buat dengan flowmaid, menggambar ulang diagramnya setiap kali file disimpan (--watch)
- dbx, klien database di terminal buatan saya, memakai flowmaid untuk menampilkan skema database sebagai ERD

Coba di browser: https://opensource.go-routine.com/flowmaid-web/
Kode: https://github.com/go-routine-id/flowmaid (GPL-3.0-or-later)
Rust: cargo add flowmaid

Silakan kirim masukan atau buka issue di GitHub.

#rustlang #OpenSource #WebAssembly #Mermaid #Diagrams #DevTools
