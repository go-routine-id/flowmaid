# flowmaid promo: cue sheet

Silent master `out/flowmaid-promo.mp4`, 1080x1350 @ 30 fps, 1080 frames (36.00 s), no audio track.
Generated from `src/flowmaid/timeline.ts` (SHOTS) and the per-shot `shots/<Name>.cues.ts` by `node scripts/export-cues.mjs`.
Frames are absolute (frame 0 = first frame); `time_s` = frame / 30; `mm:ss.ss` is the same time for editors.
Burned-in captions are also written as `out/captions.srt` (same windows as the table below).
Durations inside event / sound text are in film frames (30 f = 1 s); "clip f" / "src" numbers are frames of the
source clip, not of the film. In shot 1 (open, from frame 0) frame ranges in the text are absolute.

**Thumbnail:** frame 0 is the empty dotted canvas the opening draws on, so do not let LinkedIn pick the first
frame as the poster: upload `out/cover.png` (frame 1079, the final lockup with the VO1 line) as the custom thumbnail.

## Shots

| # | shot | from | to | duration | mode |
|---|------|------|----|----------|------|
| 1 | open | 0 (0.000 s) | 90 (3.000 s) | 90 f (3.00 s) | paper |
| 2 | render | 90 (3.000 s) | 228 (7.600 s) | 138 f (4.60 s) | paper |
| 3 | speed | 228 (7.600 s) | 341 (11.367 s) | 113 f (3.77 s) | paper |
| 4 | types | 341 (11.367 s) | 451 (15.033 s) | 110 f (3.67 s) | paper |
| 5 | browser | 451 (15.033 s) | 601 (20.033 s) | 150 f (5.00 s) | paper |
| 6 | flowcli | 601 (20.033 s) | 757 (25.233 s) | 156 f (5.20 s) | ink |
| 7 | dbx | 757 (25.233 s) | 907 (30.233 s) | 150 f (5.00 s) | ink |
| 8 | outro | 907 (30.233 s) | 1080 (36.000 s) | 173 f (5.77 s) | paper |

## Voice-over windows (burned-in captions)

Fit each recorded VO line inside its window (or move the window: `lead` / `tail` in `VO_LINES`, timeline.ts).

| VO | caption | from | to | on screen |
|----|---------|------|----|-----------|
| 1 | flowmaid renders Mermaid diagrams in pure Rust. | 4 (0.133 s) | 88 (2.933 s) | 2.80 s |
| 2 | Text in, clean SVG out. | 100 (3.333 s) | 224 (7.467 s) | 4.13 s |
| 3 | Over a hundred times faster than mermaid.js. | 238 (7.933 s) | 337 (11.233 s) | 3.30 s |
| 4 | Twelve diagram types, from flowcharts to sankey. | 353 (11.767 s) | 447 (14.900 s) | 3.13 s |
| 5 | In the browser, drag a node, and its edges follow. | 463 (15.433 s) | 595 (19.833 s) | 4.40 s |
| 6 | flowcli draws them in your terminal, and redraws when you save. | 623 (20.767 s) | 753 (25.100 s) | 4.33 s |
| 7 | In dbx, flowmaid lays out your database schema as an ERD. | 769 (25.633 s) | 899 (29.967 s) | 4.33 s |
| 8 | flowmaid is open source. | 929 (30.967 s) | 985 (32.833 s) | 1.87 s |
| 8 | Try it in your browser, or cargo add flowmaid. | 985 (32.833 s) | 1078 (35.933 s) | 3.10 s |

## Visual hits

| frame | time_s | mm:ss.ss | shot | event | suggested sound |
|-------|--------|----------|------|-------|-----------------|
| 0 | 0.000 | 00:00.00 | open | accent crosshair starts drawing (vertical 0-9, horizontal 8-18) | soft pen tick / quiet |
| 10 | 0.333 | 00:00.33 | open | first wordmark letter presses in (letterpress run 10-43, one glyph every 3 f) | soft letterpress thuds, light, accelerating |
| 28 | 0.933 | 00:00.93 | open | kicker "OPEN-SOURCE RUST CRATE" types (28-43) | light keyboard ticks 28-43 |
| 43 | 1.433 | 00:01.43 | open | last glyph lands - lockup complete | soft transition / tone bed, no hit |
| 46 | 1.533 | 00:01.53 | open | 1 s brand hold begins (static to 74) | tone bed continues, no hit |
| 76 | 2.533 | 00:02.53 | open | wordmark visibly lifts and docks into the header (dock 74-87, header rules draw 74-88) | short whoosh up |
| 87 | 2.900 | 00:02.90 | open | wordmark lands in the header slot | tiny click/tick |
| 92 | 3.067 | 00:03.07 | render | code pane draws its 1px frame (SVG pane follows at 6) | very soft line swish |
| 96 | 3.200 | 00:03.20 | render | hero.mmd typing starts (one line burst every 4.4 f, 44 f in all) | keyboard typing bed, 44 f (1.5 s) |
| 140 | 4.667 | 00:04.67 | render | code confirmed (pane rules turn accent, MERMAID tag filled) | short key press |
| 144 | 4.800 | 00:04.80 | render | submit: FLOWMAID tag filled, drip line runs to the root node, camera pull-back starts (72 f) | soft click + start of a slow pull-back whoosh |
| 150 | 5.000 | 00:05.00 | render | root node Request pops | soft pop (series start) |
| 162 | 5.400 | 00:05.40 | render | Auth? pops | soft pop |
| 170 | 5.667 | 00:05.67 | render | Services subgraph frame draws as an outline | very soft line swish |
| 174 | 5.800 | 00:05.80 | render | API Gateway pops with its "yes" chip | soft pop |
| 179 | 5.967 | 00:05.97 | render | 401 pops with its "no" chip | soft pop, quieter |
| 186 | 6.200 | 00:06.20 | render | Cache pops | soft pop |
| 191 | 6.367 | 00:06.37 | render | Database pops | soft pop, quieter |
| 198 | 6.600 | 00:06.60 | render | Response pops - tree complete (last arrow lands 5 f later) | soft pop, last and lowest |
| 204 | 6.800 | 00:06.80 | render | tree breathes 1.00-1.02-1.00 over 20 f, then static | gentle swell, no hit |
| 228 | 7.600 | 00:07.60 | seam | accent-line wipe render -> speed | soft swoosh, short (~0.5 s) |
| 244 | 8.133 | 00:08.13 | speed | footnote fades in with the readout and stays (geometric mean over 25 sample diagrams) | none |
| 256 | 8.533 | 00:08.53 | speed | mermaid.js 1x limit line snaps (halves slide apart) | small dry snap |
| 260 | 8.667 | 00:08.67 | speed | anticipation pull-back of the tape | soft tension / inhale |
| 265 | 8.833 | 00:08.83 | speed | tape sprints up the log scale (blurred), readout rolls 1 -> 122 | fast mechanical roll / riser, 22 f |
| 287 | 9.567 | 00:09.57 | speed | tape brakes and swings back once (122 -> 112 -> 113) | dull mechanical clunk |
| 299 | 9.967 | 00:09.97 | speed | 'FASTER THAN MERMAID.JS' rises under the readout (locked composition complete at the still point 10 f later; fully still to the cut) | none (under the lock hit) |
| 303 | 10.100 | 00:10.10 | speed | readout locks at ~113x on the FLOWMAID marker | firm impact, the speed hit |
| 341 | 11.367 | 00:11.37 | seam | accent-line wipe speed -> types | soft swoosh, short (~0.5 s) |
| 351 | 11.700 | 00:11.70 | types | blueprint: grid rules, then 12 wire diagrams draw on (32 f) | pencil / fine line scratch bed, 32 f |
| 389 | 12.967 | 00:12.97 | types | accent scan line starts top -> bottom (diagrams materialise) | scanner sweep, 30 f |
| 399 | 13.300 | 00:13.30 | types | row 1 materialised (flowchart, sequence, class) - counter rolls to 03/12 | light tick |
| 401 | 13.367 | 00:13.37 | types | row 2 materialised (state, ER, pie) - counter rolls to 06/12 | light tick |
| 405 | 13.500 | 00:13.50 | types | row 3 materialised (journey, git graph, mindmap) - counter rolls to 09/12 | light tick |
| 413 | 13.767 | 00:13.77 | types | row 4 materialised (timeline, sankey, architecture) | light tick |
| 419 | 13.967 | 00:13.97 | types | counter lands on 12/12 and turns accent - full wall holds to the cut | settle (soft, final) |
| 451 | 15.033 | 00:15.03 | seam | accent-line wipe types -> browser | soft swoosh, short (~0.5 s) |
| 473 | 15.767 | 00:15.77 | browser | accent lock corners contract onto the drag region (16 f) | soft focus click |
| 497 | 16.567 | 00:16.57 | browser | anticipation pull-back (6 f) | short inhale / reverse swell |
| 503 | 16.767 | 00:16.77 | browser | crash zoom 1x -> 2.12x (6 f, speed peak on the landing frame) | fast whoosh into a soft stop |
| 509 | 16.967 | 00:16.97 | browser | crash lands; cursor presses the Auth? node on the same frame (src 52); 3% bounce settles over 12 f | impact thump + mouse click |
| 517 | 17.233 | 00:17.23 | browser | drag starts: Auth? travels left, its 3 edges re-route every frame (clip src 60-112, 52 f) | subtle continuous drag texture, 52 f |
| 574 | 19.133 | 00:19.13 | browser | drag released (src 117), node toolbar pops back; still from here to the cut | mouse release click |
| 592 | 19.733 | 00:19.73 | seam | mode toggle knob slides to INK | mechanical toggle click (real switch, not a UI bleep) |
| 603 | 20.100 | 00:20.10 | seam | ink layer circle appears in the INK button (browser -> flowcli; full frame covered at 623) | airy whoosh ~0.7 s starts here, peaks 9 f later |
| 612 | 20.400 | 00:20.40 | seam | ink circle grows fastest (mid-reveal) | whoosh peak |
| 631 | 21.033 | 00:21.03 | flowcli | shell types the echo line (clip f10-38, 28 f) | keyboard typing, 28 f |
| 661 | 22.033 | 00:22.03 | flowcli | Enter pressed (clip f40) | key press |
| 667 | 22.233 | 00:22.23 | flowcli | flowcli --watch redraws, 8 -> 9 nodes (clip f46) | soft redraw blip / paper flick |
| 668 | 22.267 | 00:22.27 | flowcli | FREEZE: pane flash (3 f) + corner brackets, LIVE -> PAUSED | camera shutter / pause click |
| 677 | 22.567 | 00:22.57 | flowcli | marker circle draws round "Email buyer" (10 f) | marker scribble 10 f |
| 687 | 22.900 | 00:22.90 | flowcli | arrow shaft (6 f), then head (3 f) | short marker stroke |
| 691 | 23.033 | 00:23.03 | flowcli | annotation 'redrawn on save' + '8 -> 9 nodes' rise | none or soft pen |
| 703 | 23.433 | 00:23.43 | flowcli | hand underline (8 f) | short marker stroke |
| 711 | 23.700 | 00:23.70 | flowcli | ink fades, "+1" comment chip pins to the node | small pin tick |
| 719 | 23.967 | 00:23.97 | flowcli | unfreeze: status back to LIVE, clip ramps up | short rewind-forward whoosh |
| 725 | 24.167 | 00:24.17 | flowcli | click picks Backorder (clip f88; the rest of the diagram dims) | mouse click |
| 729 | 24.300 | 00:24.30 | flowcli | drag: Backorder moves up-right, edges re-route (clip f94-121, 15 f, braking) | subtle drag texture, 15 f |
| 744 | 24.800 | 00:24.80 | flowcli | drag lands (clip f121), hold to the end | mouse release click |
| 757 | 25.233 | 00:25.23 | seam | accent-line wipe flowcli -> dbx | soft swoosh, short (~0.5 s) |
| 784 | 26.133 | 00:26.13 | dbx | focus corners lock on the whole window (accent) | tiny mechanical focus tick (optional) |
| 787 | 26.233 | 00:26.23 | dbx | flash 1 - hard cut to photo B: 4-table ERD (clip f24 frozen) | camera shutter 1 |
| 806 | 26.867 | 00:26.87 | dbx | focus corners lock on the ERD panel | tiny mechanical focus tick (optional) |
| 809 | 26.967 | 00:26.97 | dbx | flash 2 - hard cut to photo C: zoomed ERD (clip f62 frozen) | camera shutter 2 |
| 824 | 27.467 | 00:27.47 | dbx | focus corners lock on the orders table | tiny mechanical focus tick (optional) |
| 827 | 27.567 | 00:27.57 | dbx | flash 3 - hard cut to photo D: orders close-up (clip f110 frozen) | camera shutter 3 (strongest) |
| 841 | 28.033 | 00:28.03 | dbx | HUD 'LAYOUT  flowmaid er::scene' rises (its rules started drawing 4 f earlier) | soft tick |
| 898 | 29.933 | 00:29.93 | seam | mode toggle knob slides to PAPER | mechanical toggle click (real switch, not a UI bleep) |
| 909 | 30.300 | 00:30.30 | seam | paper layer circle appears in the PAPER button (dbx -> outro; full frame covered at 929) | airy whoosh ~0.7 s starts here, peaks 9 f later |
| 918 | 30.600 | 00:30.60 | seam | paper circle grows fastest (mid-reveal) | whoosh peak |
| 921 | 30.700 | 00:30.70 | outro | first tile flies in (6 tiles, one every 3 f) | riser into the assembly, 28 f |
| 948 | 31.600 | 00:31.60 | outro | last tile lands (6 landings, one every 3 f, ending here) | optional soft thuds on landings, quieter each |
| 949 | 31.633 | 00:31.63 | outro | wordmark letterpress starts (tiles step back over 8 f) | impact / deep whoosh on the stamp (film peak) |
| 951 | 31.700 | 00:31.70 | outro | eyebrow OPEN SOURCE · GPL-3.0-OR-LATER types (18 f) | none (under the impact tail) |
| 965 | 32.167 | 00:32.17 | outro | accent rule grows under the wordmark (gutter lines shoot 6 f later) | soft shimmer or none |
| 977 | 32.567 | 00:32.57 | outro | `$ cargo add flowmaid` types (36 f, 2 f per char) | keyboard ticks, 36 f |
| 1007 | 33.567 | 00:33.57 | outro | links rise in (two lines, 6 f apart) | none |
| 1027 | 34.233 | 00:34.23 | outro | final lockup complete (caret off) - hold to the end, fully static for the last 1 s | silence / tail |
