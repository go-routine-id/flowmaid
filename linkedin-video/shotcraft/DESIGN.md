# flowmaid LinkedIn promo: design spec (video-shotcraft, autonomous free creation)

Director stages 0-3 of `references/pipeline.md`, written for the implementers. BRIEF.md is the hard
constraint; where a shot card or skill default disagrees with the brief, the brief wins and the
deviation is listed in section 10. All frame numbers come from `src/flowmaid/timeline.ts`
(single source of truth); if this file and the code ever disagree, the code wins and this file
is updated.

Composition: `FlowmaidPromo`, 1080 x 1350 (4:5), 30 fps, 1080 frames = 36.00 s, silent master.
(Fixer round 1 re-timed shots 3, 4, 5 and 8; see section 14. All numbers below are current.)

---

## 0. Product brief

| item | decision |
|------|----------|
| Product | flowmaid, an open-source Rust crate that renders Mermaid diagrams in pure Rust: Mermaid text in, clean SVG out, zero dependencies. |
| Proof points (allowed claims only) | 12 diagram types; about 113x faster than mermaid.js (geometric mean over 25 sample diagrams, shown as a footnote); runs in the browser as WebAssembly (flowmaid-web), drag a node and its edges re-route live; flowcli draws diagrams in the terminal and `--watch` redraws on save; dbx lays out its ERD with flowmaid (`flowmaid er::scene`); GPL-3.0-or-later; `cargo add flowmaid`; links opensource.go-routine.com/flowmaid-web and github.com/go-routine-id/flowmaid. Nothing else (no wasm size, no version numbers, no driver lists). |
| Audience | Developers and tech leads scrolling LinkedIn on a phone, mostly muted until they tap. |
| Use | Organic LinkedIn post (post copy already drafted in `../post.md`). |
| Tone | Engineering dev-tool, precise, calm, confident. "Structure comes from lines." |
| Format | 1080x1350, ~36 s, English, burned-in captions, silent master + cue sheet (owner adds ElevenLabs VO and own SFX). |
| Data safety | All footage is the author's own demo data (shop-demo SQLite, sample flowcharts); nothing personal; no personal branding. |

## 1. Requirement -> execution decisions

| requirement (brief) | execution |
|---|---|
| 1080x1350 @ 30 fps, ~34-36 s | 8 shots, 1080 f (36.00 s), see section 7. |
| Silent master | No `<Audio>` anywhere; every `<OffthreadVideo>` is `muted`; render with `--muted`. |
| Cue sheet from the same SHOTS | Each shot keeps local cues in `shots/<Name>.cues.ts`; `src/flowmaid/cues.ts` maps them through SHOTS and adds seam cues; `node scripts/export-cues.mjs` writes `out/cues.csv` + `out/cues.md` (verified working). |
| Captions = VO lines, one CAPTIONS table from SHOTS, easy to move | `VO_LINES` in timeline.ts (`lead`/`tail`/`parts` per line) -> `CAPTIONS`; rendered by `lib/Caption.tsx` (`VoCaption`) in Main. Move a caption by editing its `lead`/`tail`. |
| LinkedIn overlay covers y > 1215 | Caption bottom edge at frame y 1196; shot content lives in SAFE (frame y 136..1036); nothing important below 1196. Footer strip (1296..1350) is decorative only. |
| flowmaid-web design tokens, two modes | `src/flowmaid/tokens.ts` (PAPER / INK / SYNTAX / FONT / layout / motion). `Chrome.tsx` reproduces the playground header, dotted canvas and footer per mode. |
| Square corners, 1px rules, no shadows/gradients/glow | Enforced in lib (Pane, ClipCard, Caption). Every card's glow / shadow / grain / vignette / light-band / gold-dust is removed (section 10). The only CSS gradient used is the playground's own 1.6px dot tile. |
| Fonts Archivo 900 / Inter / JetBrains Mono | Google Fonts is unreachable from the render box, so the same families are loaded from local files in `public/fonts/` (Archivo 900/800 = Google Fonts TTFs; Inter, JetBrains Mono from the system) via FontFace + delayRender (`loadFlowmaidFonts`). Verified in rendered stills. |
| Real assets as hero footage | `clips/web.mp4`, `clips/flowcli.mp4`, `clips/dbx.mp4` are the heroes of shots 5-7, framed with `lib/ClipCard.tsx` inside a `lib/Pane.tsx`; `svg/*.svg` are the real renders for shots 2, 4, 8; `frames/*.png` feed the outro tiles. |
| dbx hard cuts at f18 / f30, never show other frames | Shot 7 uses only dbx.mp4 frames; it never plays across f18/f30 (each change of frame happens under a pane flash = beat-cut grammar). |
| Mode switch into Ink terminal scenes and back to Paper for the end card | Seam 5->6: PAPER->INK (knob slides 9 f, Ink layer grows as a circle from the INK button over 22 f). Seam 7->8: INK->PAPER (same grammar, circle from the PAPER button). Implemented once in Main/Chrome from TRANSITIONS. |
| One motion technique is the hero only once | Section 5: eight different hero techniques. The accent-line wipe (house seam from fx.html) and the pane line-draw are the brand's connective tissue, not heroes. |
| Hold >= 1 s on final wordmark/CTA; rests after batch motion | Brand hold 30 f in shot 1; outro lockup complete at local 120 (caret off), static from 143, shot ends 173 (53 f hold, last 30 f fully static). Speed readout fully still 32 f before its cut; types wall holds 32 f with 12/12 (30 f fully still after the scan line). Batch motions (cascade, wall scan, tile fly-in) all end with >= 15 f rest. |
| Deterministic | No Date/Math.random; seeded hash/mulberry32 only (`lib/helpers/rand.ts`); video remaps are pure functions of the frame. |
| Never touch skill repo / git repos | Everything lives in this folder; lib components are copies with origin notes. |

## 2. Visual direction (stage 1)

Single direction, no alternatives explored: the brief mandates the product's own playground skin
and the owner already approved style frames (`../fw/style-frames.png`, `../fw/style.html`) and an
effects preview (`../fw/fx.html`, `../flowmaid-effects-preview.mp4`). A new HTML styleframe would
only duplicate them, so stage-1 styleframes are skipped with that reason; the approved frames are
the reference. Direction name: **"the playground, narrated"** - every scene is the flowmaid-web
page itself (header, dotted canvas, framed panes with drawn 1px rules), and the film moves through
it the way a person would.

### 2.1 Colour tokens (tokens.ts)

| token | Paper (default) | Ink (terminal scenes) |
|---|---|---|
| bg | #efece5 | #17171a |
| panel | #e7e3da | #1e1e22 |
| mass | #ddd8cc | #26262b |
| ink | #15140f | #eae8ee |
| dim | #55534a | #cdccd2 |
| muted | #86826f | #908f96 |
| line | #e2ded4 | #26262b |
| line2 | #cbc5b6 | #35353d |
| accent | #8839ef | #cba6f7 |
| accent-soft | rgba(136,57,239,.10) | rgba(203,166,247,.12) |
| canvas | #f5f3ee | #141417 |
| dot | #d3cdbf | #2b2b31 |

Syntax (paper): header kw #8839ef, kw #1d5ad6, dir #0e6f7c, str #2d7a3e, node text #6b5300,
arrows #b54a12, punctuation #55534a. The SVG renders keep flowmaid's own default theme colours
(they are the product output).

### 2.2 Type

Archivo 900, letter-spacing -.045em (display, wordmark). Inter 400/500/600 (captions, body).
JetBrains Mono 400/500/600 (labels uppercase, .07em labels / .16em eyebrows; code).
Sizes: wordmark 150-168 px (hero), header wordmark 44 px; captions Inter 600 52 px; scene label
22 px mono; tag chips 20 px; pane headers 17 px; must-read secondary text >= 28 px; URLs 32 px.

### 2.3 Layout (1080x1350)

- Header 0..96 (wordmark, accent eyebrow "PURE RUST · ZERO DEPS", INK|PAPER toggle at x 846..1024),
  footer 1296..1350 ("PURE RUST · ZERO DEPS · WEBASSEMBLY" ... "GPL-3.0-OR-LATER").
- Canvas 96..1296 (dotted, 28 px grid). Shots draw in CANVAS-LOCAL coordinates (0,0 = frame (0,96)).
- SAFE content box (canvas-local): x 56..1024, y 40..940 (frame 136..1036). Scene label row at canvas y 44.
- Caption band (canvas-local) 954..1100 (frame 1050..1196). LinkedIn overlay line at frame y 1215.
- Gutter G = 56.

### 2.4 Motion personality tokens

Brand on the two axes: energy low-mid, tone serious -> pipeline preset **professional trust**
(fintech/enterprise/B2B): main duration ~21 f, entrance bezier(0,0,0.2,1), overshoot 1.0 (no
bounce on fades/pushes), squash 0. Tuned with the approved fx.html easings:

| token | value | use |
|---|---|---|
| ease.quiet | cubic ease-out | secondary fades/rises |
| ease.expo | exponential ease-out | lines drawing, masks, knob |
| ease.inout | cubic in-out | wipes, circle, camera moves |
| ease.enter | bezier(0,0,0.2,1) | default entrances (~21 f) |
| mask rise | translateY 110% -> 0, expo 12-16 f | headlines, captions, labels |
| pane draw | 4 rules in turn (expo per segment) then fill/header fade, 22 f | every framed surface |
| wipe | 16 f inout, 2 px accent line on the edge | same-mode seams |
| mode switch | knob 9 f inout, then circle 22 f inout from the target button, radius = 1.1 x farthest corner | Paper<->Ink |
| overshoot | none, except real landings (outro tiles, card rule y1 > 1) and a <= 3 % camera bounce (crash zoom), <= 8 % tape brake | |
| holds | brand >= 30 f, key info >= 30 f, batch rest >= 15 f | R1/R2 |

Self-check, three words for the motion: precise, drawn, unhurried. Matches "engineering dev-tool".

## 3. Feature -> shot mapping (stage 2)

| feature (claim) | shot | hero technique (card) | why this card |
|---|---|---|---|
| Brand / "pure Rust" | 1 open | letterpress wordmark + typewriter kicker + 1 s hold (brand-ink-open) | name first, low-energy start |
| Mermaid text in, clean SVG out | 2 render | text becomes a structure: edges lead nodes in a rank cascade (canvas-materialize-moves / diagram-cascade) | "from one text, a tree grows", exactly the product |
| ~113x faster than mermaid.js | 3 speed | fixed pointer, scale moves: log tape sprints from 1x to 113x and brakes (gauge-readout-moves / tape-scroll-fixed-pointer) | "the number is a visible distance"; instrument look = engineering |
| 12 diagram types | 4 types | blueprint grid draws, accent scan materialises the 12 real renders (wall-reveal-moves / wireframe-draw-on) | non-displacement batch reveal, "structure from lines" |
| Browser / WebAssembly, drag re-routes live | 5 browser | lock brackets + 6 f crash zoom into the live drag (crash-zoom-punch, bounce variant) | single "look at this" before a real interaction |
| flowcli: terminal, --watch redraw (+drag) | 6 flowcli | flow -> freeze on the redraw -> marker annotation -> unfreeze (speed-ramp-freeze / freeze-annotate) | stops the clip exactly on "redraws when you save" |
| dbx: ERD via flowmaid er::scene | 7 dbx | three shutter flashes, each a hard cut to a tighter crop (beat-cut-moves / paparazzi-flash) | turns the clip's unavoidable hard cuts into intentional beats |
| Open source, cargo add, links | 8 outro | group photo: each feature's representative tile flies into its slot around the lockup (outro-group-photo-launch) | recap + CTA, film's structural peak |
| Paper <-> Ink | seams 5->6, 7->8 | mode-switch circle from the toggle (theme-switch-moves / palette-theme-ripple, approved fx variant) | the product's own INK|PAPER toggle is the cause |

Each hero technique appears once. The house wipe (fx.html) is used on the four same-mode seams;
pane line-draws are the brand's entrance for framed surfaces (style, not hero).

## 4. Chosen cards (verified in `gallery/api/library.json`)

Skill repo root: `../shotcraft/repo`. Gallery preview mp4s are NOT present locally
(`gallery/media` holds only a poster; fetching is not allowed), so the demo TSX is the only
motion reference; nobody may claim a match to a gallery clip.

| shot | card (category) | style key | card file | exact demo TSX |
|---|---|---|---|---|
| 1 open | brand-ink-open (opening) | brand-ink-open | references/shots/opening/brand-ink-open.md | demos/typography/brand-ink-open/BrandInkOpen.tsx |
| 2 render | canvas-materialize-moves (interaction) | diagram-cascade | references/shots/interaction/canvas-materialize-moves.md | demos/interaction/canvas-materialize-moves/DiagramCascadeBuild.tsx |
| 3 speed | gauge-readout-moves (data) | tape-scroll-fixed-pointer | references/shots/data/gauge-readout-moves.md | demos/data/gauge-readout-moves/TapeScrollFixedPointer.tsx |
| 4 types | wall-reveal-moves (ui-entrance) | wireframe-draw-on | references/shots/ui-entrance/wall-reveal-moves.md | demos/ui-entrance/wall-reveal-moves/WireframeDrawOn.tsx |
| 5 browser | crash-zoom-punch (camera) | crash-zoom-punch (bounce variant) | references/shots/camera/crash-zoom-punch.md | demos/camera/crash-zoom-punch/CrashZoomReal.tsx |
| 6 flowcli | speed-ramp-freeze (rhythm) | freeze-annotate | references/shots/rhythm/speed-ramp-freeze.md | demos/rhythm/speed-ramp-freeze/FreezeAnnotateReal.tsx |
| 7 dbx | beat-cut-moves (rhythm) | paparazzi-flash | references/shots/rhythm/beat-cut-moves.md | demos/rhythm/beat-cut-moves/PaparazziFlash.tsx |
| 8 outro | outro-group-photo-launch (outro) | outro-group-photo-launch | references/shots/outro/outro-group-photo-launch.md | demos/outro/outro-group-photo-launch/OutroGroupPhotoLaunch.tsx |
| seams 5->6, 7->8 | theme-switch-moves (interaction) | palette-theme-ripple | references/shots/interaction/theme-switch-moves.md | demos/interaction/theme-switch-moves/PaletteThemeRipple.tsx (implemented in Main.tsx with the approved fx.html timing; already done) |
| captions | (asset) Caption | - | assets/lib/Caption.tsx | adapted -> src/flowmaid/lib/Caption.tsx |
| clip framing | (asset) ClipCard | - | demos/README.md "真实视频素材" | assets/lib/ClipCard.tsx -> src/flowmaid/lib/ClipCard.tsx (+ RemapVideo); reference demo demos/ui-entrance/clipcard-looping/ClipCardLooping.tsx |
| shared easing/blur | (fixture) Polish | - | - | demos/_fixtures/Polish.tsx -> src/flowmaid/lib/Polish.tsx |

Sequence template: references/sequences/promo-energy-arc.md (section 6).

## 5. Energy arc (promo-energy-arc filled in)

| slot | share | shots | energy |
|---|---|---|---|
| (1) brand open | 8 % | 1 open | low (letterpress + hold) |
| (2) single hero | 13 % | 2 render (the product's atomic unit: text -> diagram, slowest, most crafted) | mid |
| (3) feature climb | 60 % | 3 speed (mid-high hit) -> 4 types (mid, calm reveal) -> 5 browser (high spike, then calm drag) -> mode switch -> 6 flowcli (mid, freeze = breath) -> 7 dbx (high, three shutter hits) | alternating |
| (4) launch close | 17 % | 8 outro (group photo + CTA, structural peak, long hold) | peak then rest |

Breathing cards: no separate paper-title-cards. Deliberate deviation from the template's 2-4
title cards: the captions already carry one line per scene and a 36 s LinkedIn cut cannot spare
55 f cards; breathing comes from in-shot holds (speed lock 23 f + held frame, types rest, browser
post-drag hold, flowcli freeze 51 f, dbx photo hold 80 f).
Fixer round 1: speed lock 32 f fully still, types 12/12 wall 32 f, browser full view 30 f before the crash.

## 6. Storyboard (stage 3)

| # | time | shot | key motion |
|---|------|------|-----------|
| 1 | 0.00-3.00 | Brand open (Paper, header absent) | Accent crosshair draws and fades; "flowmaid" letterpresses glyph by glyph (Archivo 900); kicker "OPEN-SOURCE RUST CRATE" types; full lockup holds 1 s; wordmark shrinks and docks into the header slot while the header/footer rules draw in. |
| 2 | 3.00-7.60 | Text in, SVG out | Camera 1.25x on the hero.mmd pane; code types line by line in playground colours; pane confirms (accent); a drip line runs to the SVG pane and hero.svg grows rank by rank, edges drawn 10 f before their nodes, while the camera pulls back to show both panes; tree breathes once. |
| 3 | 7.60-11.37 | ~113x faster | Wipe in. Readout + footnote "Geometric mean over 25 sample diagrams." come in together; log speed tape parked at "mermaid.js 1x" behind a fixed accent pointer; the limit line breaks, anticipation, 22 f sprint to 122x with speed blur, one brake swing, locks at 113x; readout "~113x", label "FASTER THAN MERMAID.JS"; 32 f fully still. |
| 4 | 11.37-15.03 | 12 diagram types | Wipe in. 3x4 blueprint grid draws its rules and wire versions of the 12 real renders (1.2 s); short breath; accent scan line sweeps top->bottom and materialises flowchart ... sankey, architecture; counter 00/12 -> 12/12; the real wall holds 32 f. |
| 5 | 15.03-20.03 | flowmaid-web (browser, WebAssembly) | Wipe in. Real playground clip in a pane; full view holds 30 f while accent corners lock on the diagram region; anticipation, 6 f crash zoom with 3 % bounce; the real drag of "Auth?" plays in close-up, three edges re-route live; hold; knob slides to INK. |
| T | 20.03 | Mode switch PAPER->INK | Ink layer (with Ink chrome) grows as a circle out of the INK button over 22 f. |
| 6 | 20.03-25.23 | flowcli (Ink) | Real tmux clip: echo appends a node, Enter, --watch redraws (8->9 nodes); FREEZE (pane flash, corners, LIVE->PAUSED); rest dims, marker circle around "Email buyer", arrow, annotation "redrawn on save"; unfreeze ramps through the drag; hold. |
| 7 | 25.23-30.23 | dbx (Ink) | Wipe in. Live explorer; flash 1 -> photo of the 4-table ERD; flash 2 -> tighter photo of the zoomed ERD; flash 3 -> close-up photo of orders and its two relationship connectors (payments just outside the edge, see 8.7); 80 f hold with HUD "LAYOUT  flowmaid er::scene"; knob slides to PAPER. |
| T | 30.23 | Mode switch INK->PAPER | Paper layer grows as a circle out of the PAPER button. |
| 8 | 30.23-36.00 | Open source / CTA | Crane settle; six feature tiles fly into slots (01 render, 02 speed, 03 diagram types / 04 flowmaid-web, 05 flowcli, 06 dbx); tiles step back; "flowmaid" letterpress; eyebrow "OPEN SOURCE · GPL-3.0-OR-LATER"; accent rule; `$ cargo add flowmaid` types; both links rise; static hold to the end. |

## 7. Frame-level timeline (1080x1350 @ 30 fps)

| shot | from | duration | content |
|------|------|----------|---------|
| open | 0 | 90 | brand-ink-open; header handoff 74-88 (HEADER_HANDOFF); VO1 caption 4-88 |
| render | 90 | 138 | diagram-cascade; VO2 caption 100-224 |
| speed | 228 | 113 | wipe-in 228-244; tape; lock 303, fully still 309-341; VO3 caption 238-337 |
| types | 341 | 110 | wipe-in 341-357; wireframe-draw-on; scan 389-419, 12/12 at 419; VO4 caption 353-447 |
| browser | 451 | 150 | wipe-in 451-467; full view to 497; crash 503-509 + drag; knob slide 592-601; VO5 caption 463-595 |
| flowcli | 601 | 156 | circle-in 601-623; freeze-annotate; VO6 caption 623-753 |
| dbx | 757 | 150 | wipe-in 757-773; paparazzi-flash; knob slide 898-907; VO7 caption 769-899 |
| outro | 907 | 173 | circle-in 907-929; group photo + CTA; VO8a 929-985, VO8b 985-1078 |
| total | | 1080 | 36.00 s |

Seam rule (Main.tsx): every seam is at the incoming shot's `from`; the incoming is revealed over
its first 16 f (wipe) or 22 f (mode circle); the outgoing is shown FROZEN on its last frame
underneath. So: start entrances around local 4-10, keep the last 9 f before a mode seam calm (the
knob is sliding), and make the last frame of every shot a good still.

### Voice-over room

| VO | line | est. | shot length | caption on screen |
|----|------|------|-------------|-------------------|
| 1 | flowmaid renders Mermaid diagrams in pure Rust. | 2.7 s | 3.00 s | 2.80 s |
| 2 | Text in, clean SVG out. | 2.2 s | 4.60 s | 4.13 s |
| 3 | Over a hundred times faster than mermaid.js. | 2.9 s | 3.77 s | 3.30 s |
| 4 | Twelve diagram types, from flowcharts to sankey. | 2.9 s | 3.67 s | 3.13 s |
| 5 | In the browser, drag a node, and its edges follow. | 4.3 s | 5.00 s | 4.40 s |
| 6 | flowcli draws them in your terminal, and redraws when you save. | 4.5 s | 5.20 s | 4.33 s |
| 7 | In dbx, flowmaid lays out your database schema as an ERD. | 4.5 s | 5.00 s | 4.33 s |
| 8 | flowmaid is open source. / Try it in your browser, or cargo add flowmaid. | 5.5 s | 5.77 s | 1.87 + 3.10 s |

Every shot is longer than its VO line. Lines 6-8 have captions slightly shorter than the VO
estimate because captions wait for the mode circle to finish; the VO itself may start during the
circle. Retime with `lead`/`tail` in `VO_LINES` once the real VO exists.

## 8. Per-shot specs (canvas-local coordinates; frames are shot-local unless "abs")

Common: every shot starts with `SceneLabel` (lib/Pane.tsx) at canvas y 44 except open and outro:
`01 RENDER`, `02 SPEED`, `03 DIAGRAM TYPES`, `04 FLOWMAID-WEB`, `05 FLOWCLI`, `06 DBX`.

### 8.1 open (0-90) - brand-ink-open
Canvas: dotted Paper, header/footer hidden until 74 (Chrome handles them).
- 0-9 vertical, 8-18 horizontal crosshair stroke (accent, 3 px, butt caps, 64 px) centred above the
  wordmark; fades 24-34.
- Wordmark "flowmaid", Archivo 900 168 px, -.045em, ink, centred (frame y ~ 600). Glyph i: delay
  10 + 3i, 12 f, scale 1.6 -> 1 (origin centre-bottom) + blur 6 -> 0 + opacity; accent 2 px glint dash
  under each glyph at its landing (card). Last glyph lands 43.
- Kicker "OPEN-SOURCE RUST CRATE", mono 26 px .16em, accent, preceded by a 40 px accent rule;
  types from 28 at 0.7 f/char; accent block cursor blinks 2 f, stops at 74.
- Hold 46-76: everything static (R1).
- 74-88 handoff (OpenOverlay, frame coords): kicker/crosshair gone by 78; wordmark moves and
  scales (ease.inout) to the header slot: left edge x 56, vertical centre y 48, 44 px. At 88 it
  must sit exactly where `Chrome` draws its wordmark (Chrome shows its own from abs 88; the
  overlay Sequence ends at 88). The canvas-local OpenShot renders the crosshair + kicker; the
  wordmark itself lives in OpenOverlay for the whole shot (so it can leave the canvas clip).
- Acceptance: abs 60 (lockup in hold), abs 84 (wordmark mid-dock, header rule drawing).

### 8.2 render (90-228) - diagram-cascade
- Code pane: Pane x 56, y 104, w 420, h 820, title "HERO.MMD", tag "MERMAID", fill panel, draw 2-24.
  Code from public/hero.mmd with the fx.html token colouring (tokens.SYNTAX), JetBrains Mono 22 px,
  line-height 35 px, padding 22 px.
- SVG pane: Pane x 500, y 104, w 524, h 820, title "HERO.SVG", tag "FLOWMAID", fill canvas (dots),
  draw 6-28. hero.svg (371x581) inlined, fitted ~1.24x and centred.
- Camera (wraps both panes): 0-54 scale 1.25 with the code pane centre at canvas (540, 490);
  54-126 pull back to identity (ease.inout). The tree grows in the space the camera reveals.
- Typing 6-50: line i types over ~4 f starting 6 + 4.4 i (chars revealed left->right), accent caret.
- 50 confirm: code pane rules turn accent, tag filled (tagActive). 54 submit: svg pane tag active;
  accent 1 px orthogonal drip line from the code pane's right edge (line-1 height) to the Request
  node, drawn 54-62 (expo), fades 110-120.
- Cascade (hero.svg structure: nodes are `<g><title>Name</title>`, edges are `<path marker-end>`
  with `<title>A → B</title>`): Request 60; Auth? 72; API Gateway 84, 401 89; Cache 96,
  Database 101; Response 108. Each edge draws 16 f (dashoffset, expo) starting 10 f before its
  target node; marker shown when the stroke is ~complete; yes/no chips pop with their edge end;
  "Services" frame + title draw as outline at 80. Node pop: scale from the top-edge midpoint 0.6 -> 1
  + opacity over ~10 f, spring({damping 20, stiffness 190}) (<= 4 % overshoot).
- 114-134 breath: tree group 1 -> 1.02 -> 1 around the tree's visual centre (not screen centre);
  134-137 static (and frozen during the next wipe).
- Acceptance: abs 130 (local 40, typing, camera 1.25x), abs 216 (local 126, tree complete, both panes).

### 8.3 speed (228-341) - tape-scroll-fixed-pointer
- SceneLabel right side: mono "LOG SCALE" muted.
- Tape: Pane x 600, y 104, w 424, h 820, title "RENDER SPEED", tag "LOG", fill panel. Log scale,
  300 px per decade; ticks at 1..9 x 10^k: majors 1x/10x/100x (120 px, 2 px ink, label mono 28 px),
  minors (60 px, 1 px dim), small labels 2x 5x 20x 50x 200x (mono 20 px muted). Tape scrolls
  vertically inside the pane body; larger values above.
- Fixed pointer at the pane body's vertical centre: accent triangle (16 px) on the pane's left edge
  + 2 px accent line across the pane. "MERMAID.JS" baseline: dashed 1 px line + label at 1x;
  "FLOWMAID" accent marker at 113x (appears as it reaches the pointer).
- Readout (left, x 56..560): label "RENDER SPEED" mono 22 px dim above; number Archivo 900 ~200 px,
  ink, text = round(v) followed by an accent "×" (U+00D7); prefixed "~" once locked ("~113×");
  tabular digits. Under it, at the lock: "FASTER THAN MERMAID.JS" mono 24 px muted (mask rise 71-81).
  Footnote at canvas y 838-920: "Geometric mean over / 25 sample diagrams." Inter 400 32 px (Q11 aux
  size) in --dim, fading in with the readout 16-28 and on screen for the rest of the shot (~2.8 s).
- Value (v, log units), constants in `shots/b/beats.ts`: 0-12 parked 1.0 (tape fades in 10-20,
  pointer 11-23); 12-28 creep to 1.12 (swift); 28-34 limit line breaks (two halves slide 8 px apart
  and fade); 32-37 anticipation to 0.95; 37-59 sprint to 122 (bezier .55,0,.25,1) with vertical
  SpeedBlur on the tape (cap 14 px); 59-75 brake: one visible swing, settle 113 (residual < 0.05 px at
  75), locked constant from 75; the "~" and the FLOWMAID marker settle 75-81. 81-113: every pixel
  still (32 f; card ">= 30f 全锁死", R1).
- Acceptance: abs 276 (local 48, mid-sprint with blur), abs 328 (local 100, locked ~113x + footnote).

### 8.4 types (341-451) - wireframe-draw-on
- SceneLabel right side: counter mono 22 px "00/12" -> 03, 06, 09, 12 as rows materialise; final accent.
- Grid x 56..1024, y 100..940: 3 cols (322.7) x 4 rows (210). Cell: header strip 40 px (label mono
  19 px uppercase + index "01".."12"), 1 px rule; body = SVG contained, 12 px padding.
- Order: 01 FLOWCHART (svg/hero.svg; advanced.svg rendered as a pale 67 px strip, fixer round 1), 02 SEQUENCE, 03 CLASS, 04 STATE, 05 ER, 06 PIE,
  07 JOURNEY, 08 GIT GRAPH, 09 MINDMAP, 10 TIMELINE, 11 SANKEY, 12 ARCHITECTURE (public/svg/*.svg).
- Blueprint 6-42 (staggered groups, pathLength=1 dashoffset; `TYPES_BEATS`): outer frame 6-30; column
  rules 9 + 2c (20 f); row rules 11 + 2r (20 f); cell header rules 14 + 1.5i (12 f); wire version of each
  real SVG (fills none, strokes --muted 1 px, text hidden; inject pathLength="1" per shape) cell i from
  12 + 1.1 i, 18 f. (First cut: 10-64; compressed ~0.7x in fixer round 1 so the real renders hold.)
  Labels muted. Blueprint and materialised layers are the same SVGs at the same transform, so they
  align by construction (card pitfall).
- 42-48 breath. 48-78 scan: materialised layer (real SVGs, ink labels, accent indices) clipped
  `inset(0 0 (1-s)*100% 0)` over the grid box; a 2 px accent horizontal line rides the edge (fades in
  46-50, out 76-80). Same `s` drives line and clip. Counter lands on 12/12 and turns accent at 78.
  78-110: the real wall holds 32 f (30 f fully still after the line; card >= 15 f, R1 >= 1 s), +16 f
  held under the next wipe.
- Acceptance: abs 377 (local 36, blueprint late), abs 441 (local 100, all 12 materialised, 12/12).

### 8.5 browser (451-601) - crash-zoom-punch (bounce)
- SceneLabel right: accent tag "WEBASSEMBLY".
- Pane x 56, y 160, w 968, h 696 (header 48 + body 648), title
  "OPENSOURCE.GO-ROUTINE.COM/FLOWMAID-WEB", tag "WASM", draw 2-24.
- ClipCard src clips/web.mp4 (1590x1064), width 966 x 646, `map: f => clamp(f - 6, 0, 134)`
  (press src 52-53 = local 58-59, release src 117 = local 123; src 118-134 are identical in the
  close-up, so from local 124 the view is still; src 135+ (cursor leaves, toolbar hides) never shown).
- Camera (source px, constants in `shots/c/browserCam.ts`): 0-16 zoom 1, centre clip; 16-46 creep
  1 -> 1.03: the full view holds 30 f after the wipe (card ">= 30f 建立全景"). 22-38 four accent
  L-corners (24 px arms, 2 px) contract from 150 px outside to 14 px outside the target rect (they
  live in source coordinates and ride the camera). 46-52 anticipation (zoom x0.97).
  52-58 crash, log-space strong ease-in (EASE.exit) to Z_peak = 2.06 x 1.03 (left edge pinned).
  58-70 damped-cosine settle to Z = 2.06 (one visible rebound, <= 3 %). Motion blur only 52-58
  (time-sampled sub-frame layers of the same frozen video frame, echo spacing <= 6 px). Corners
  fade 70-78. (First cut: crash at 40, full view only ~18 f; moved 12 f later in fixer round 1.)
- Q2 cap: on-screen source scale <= 1.30 (966/1590 x 2.13 = 1.29).
- 59-123 drag in close-up (creep 1 -> 1.01); 123-141 settled hold; 141-150 calm (knob slides).
- Removed from the card (brief): ember stage, back-tilt, exposure flash, periphery darkening,
  rim light, right-column readout. Kept: pre-lock, anticipation, 6 f ease-in push, small bounce.
- Acceptance: abs 489 (local 38, full view + locked corners), abs 541 (local 90, zoomed, mid-drag).

### 8.6 flowcli (601-757) - freeze-annotate (Ink)
- SceneLabel right: mono muted "TERMINAL".
- Pane x 56, y 104, w 968, h 815 (body 766), title "FLOWCLI — FLOW.MMD", status at header right
  "● LIVE" / "❚❚ PAUSED" (mono 16 px), tag "--WATCH", draw 8-30 (under the circle reveal).
- ClipCard src clips/flowcli.mp4 (2044x1620), width 966 x 765, map:
  0-20 src 0 (fade in 12-20); 20-67 src = local - 20 (typing src 10-38, Enter 40, redraw 46);
  67-118 src 47 FROZEN (51 f >= 45); 118-146 smootherstep src 47 -> 121 (avg ~2.6x; the drag src
  94-121 plays in the fast-to-settling half); 146-156 src 121.
- Freeze at 67 (instant): pane-body flash INK.ink 0.9 -> 0 over 3 f; accent L-corners snap in over
  8 f at the body corners (inset 10 px); status -> PAUSED. 70-76 everything except the target dims
  (4 INK.bg rects at 0.5). Target = new node "Email buyer", source bbox x 1372-1590, y 1418-1500.
- 76-86 marker: accent superellipse around the target (pad 18 px, 3.5 px, 1.08 turns, open end,
  feTurbulence displacement scale 6, fixed seed); 86-92 arrow shaft, 92-95 head, pointing from the
  annotation to the circle. 90-102 annotation in the empty lower-left of the clip (card px ~ (60, 520)):
  "redrawn on save" Inter 600 44 px INK.ink + mono 24 px dim "8 -> 9 nodes"; 102-110 hand underline.
  110-118 ink fades; a small accent "+1" chip stays pinned at the node's top-right to the end.
  118 status -> LIVE.
- Acceptance: abs 696 (local 95, frozen with circle + annotation), abs 751 (local 150, drag released).

### 8.7 dbx (757-907) - paparazzi-flash (Ink)
- SceneLabel right: mono muted "TERMINAL DATABASE CLIENT".
- Pane x 56, y 120, w 968, h 698 (body 650), title "DBX — SHOP-DEMO", tag "SQLITE", draw 2-24.
- ClipCard src clips/dbx.mp4 (2240x1504), width 966 x 648. Only these source frames are ever shown:
  - A live 0-30: src = floor(local x 0.6) (0..17, explorer); slow push 1 -> 1.02.
  - flash 1 at 30 -> photo B: src 24 frozen (4-table ERD + "ERD generated" toast), crop 1.0 (whole window).
  - flash 2 at 52 -> photo C: src 62 frozen (zoomed ERD: orders, payments, shipments), crop ~1.3 centred on the
    three tables' bounding box (approx src (1060, 560)).
  - flash 3 at 70 -> photo D: src 110 frozen (orders highlighted), crop 1.9 on the orders table and its
    two relationship connectors (src x 28-1207, y 200-991; orders spans ~x 136-1000, y 296-586 at src
    110; payments starts at x ~1280 and sits just outside the right edge); 70-150 push 1 -> 1.015 with
    the top edge fixed. The top edge sits in the empty rows between dbx's "[ ERD: main ]" tab row (which
    carries a map-emoji glyph, ~35 px at this zoom) and the ERD frame title, so no emoji is held on
    screen (fixer round 1; first cut started at y 131.5 and showed the tab row).
  - No crop edge may cut through a text row (card pitfall); measure on the frozen frames.
- Flash: pane-body only (not the whole frame, R4), INK.ink at 0.95 -> 0 over 4 f ease-out; on the flash
  frame +/-2 px seeded jitter of the body. Shutter settle after each cut: 1.03 -> 1 and -16 px -> 0
  over 6 f. Photos: saturate 0.9 / contrast 1.05.
- Focus corners (accent L, 2 px) 8 f before each flash contract from 1.3x to the next crop rect,
  muted until 3 f before the flash, then accent (lock).
- HUD row under the pane at canvas y ~846 (1 px line2 rules above/below): "LAYOUT" mono 22 px muted +
  "flowmaid er::scene" mono 30 px INK.ink; mask rise at 84.
- 141-150 calm (knob slides to PAPER).
- Acceptance: abs 797 (local 40, photo B after flash 1), abs 867 (local 110, close-up + HUD).

### 8.8 outro (907-1080) - outro-group-photo-launch (Paper)
- Group layer (tiles + lockup): 0-40 crane `perspective(1400px) rotateX(4deg -> 0) scale(1.06 -> 1)`
  (ease.inout), then push +0.035 from 36 to 143 (inout), fully static 143-173.
- Six tiles 280x200 (Pane-like, header 36 px with mono 17 px label, 1 px line2, no shadow):
  top row y 40: T1 x 56 "01 RENDER" (svg/hero.svg on dots), T2 x 400 "02 SPEED" ("~113x" Archivo 900
  72 px + "VS MERMAID.JS" + "GEO MEAN · 25 DIAGRAMS", the claim's qualifier, also on the cover),
  T3 x 744 "03 DIAGRAM TYPES" (2x2 mini grid: sequence, pie, mindmap, sankey);
  bottom row y 700: T4 x 56 "04 FLOWMAID-WEB" (frames/web-drag.png, canvas part),
  T5 x 400 "05 FLOWCLI" (frames/flowcli-drag.png, right pane), T6 x 744 "06 DBX" (frames/dbx-erd.png, ERD).
  Numbers run 01-06 in reading order and the names are the in-film scene labels (fixer round 1; the
  first cut had 01 02 04 / 03 05 06 and short names).
- Fly-in: cue 14 + 3i (order T1, T3, T5, T2, T4, T6), 12 f each, bezier(0.34, 1.4, 0.44, 1) (card rule:
  landing y1 > 1, kept), start offsets 360-500 px from outside (top row from above/sides, bottom row
  from below/sides), flight rotation +/-8 deg -> 0 (settle at 0: grid-true, within the card's +/-5),
  scale 1.12 -> 1, SpeedBlur 0.3 x px/f capped 22 during flight only. Landing (cue+12..+21): border
  line2 -> ink -> line2 and a 1.5 % press (replaces the contact shadow).
- 42-50 step back: tiles opacity 0.88, saturate 0.92.
- Lockup (centred, canvas y 270..680): eyebrow y 280 "—— OPEN SOURCE · GPL-3.0-OR-LATER" mono 22 px
  accent, types 44-62; wordmark y ~320 "flowmaid" Archivo 900 150 px, glyph delay 42 + 1.8 i, 8 f
  letterpress (same recipe as shot 1); 58-70 accent 2 px x 240 px rule grows under it, 1 px line2
  extensions shoot to the gutters 64-72 and fade 72-78.
- 64-80 cargo chip draws (Pane, fill bg, x 250..830, y 520, h 72); 70-106 `$ cargo add flowmaid`
  types at 2 f/char (mono 40 px, accent "$", block caret, caret stops at 120).
- 100-112 links (mask rise, 6 f stagger): "opensource.go-routine.com/flowmaid-web" mono 32 px accent,
  "github.com/go-routine-id/flowmaid" mono 32 px dim, centred at y ~615 / ~657.
- 120-173 hold (last 30 f absolutely static).
- Acceptance: abs 937 (local 30, tiles landing, crane), abs 1077 (local 170, final static lockup).

## 9. Engineering contract (scaffold, already built and proven)

```
src/Root.tsx                     AiflPromo (unchanged) + FlowmaidPromo (1080x1350, 30 fps, props {guides})
src/flowmaid/tokens.ts           PAPER/INK/SYNTAX/FONT/layout/motion tokens, loadFlowmaidFonts()
src/flowmaid/timeline.ts         SHOTS (durations once), TRANSITIONS, HEADER_HANDOFF, knobAt, VO_LINES, CAPTIONS, cue(), TRANSITION_CUES
src/flowmaid/cues.ts             aggregates shots/*.cues.ts + seam cues -> CUES
src/flowmaid/Chrome.tsx          ModeLayer: header/toggle/canvas/footer per mode (isolation: isolate)
src/flowmaid/Main.tsx            Sequences, held outgoing frames, wipe line, mode circles, OpenOverlay, captions
src/flowmaid/lib/                Pane + SceneLabel, ClipCard + RemapVideo + clipToCard, Caption (VoCaption),
                                 DigitRoll, Polish (bezier/EASE/ramp/mix/velocity/SpeedBlur), helpers/rand, helpers/motion, Placeholder
src/flowmaid/shots/<Name>.tsx    one component per shot (currently labelled placeholders)
src/flowmaid/shots/<Name>.cues.ts  local cues per shot (planned values; update to what you render)
src/flowmaid/shots/<pkg>/        private helpers of a work package (a/, b/, c/, d/)
public/fonts/                    local Archivo / Inter / JetBrains Mono files
scripts/export-cues.mjs          -> out/cues.csv, out/cues.md
```

Rules for implementers:
1. Own only your shot files, their `.cues.ts`, and `src/flowmaid/shots/<pkg>/`. Do not edit tokens,
   timeline, Chrome, Main, lib, Root. If you need a lib change, copy the piece into your private
   folder. If a duration must change, report it; do not edit SHOTS.
2. Draw in canvas-local coordinates; `useCurrentFrame()` is shot-local. Use `palette(SHOTS.<id>.mode)`
   or PAPER/INK; never a hex that is not in tokens (SVG renders and clips excepted).
3. Every video: `ClipCard` (or `RemapVideo`) with `muted`; output->source remaps via `map`. Verified:
   `map` returns the exact source frame (RemapVideo = Freeze + OffthreadVideo).
4. z-index is contained per layer (Chrome uses `isolation: isolate`), so it is safe inside a shot.
5. No Math.random / Date; seeded hash only. No shadows, gradients (except the dot tile), glow,
   grain, vignette, rounded corners, emoji, personal branding.
6. Read the card in full and the exact demo TSX before writing; keep the card's pitfall/"命门"
   parameters unless this spec explicitly adapts them for the brief.
7. QA: render your 2 acceptance frames:
   `npx remotion still src/index.ts FlowmaidPromo out/qa/<shot>-f<abs>.png --frame=<abs> --browser-executable=/opt/pw-browsers/chromium_headless_shell-1243/chrome-headless-shell-linux64/chrome-headless-shell`
   (add `--props='{"guides":true}'` to see SAFE / caption band / LinkedIn line), look at them,
   crop-zoom text to check sharpness. Delete scratch renders.
8. `npx tsc --noEmit -p .` must pass.
9. Full render (integration): `npx remotion render src/index.ts FlowmaidPromo out/flowmaid-promo.mp4 --muted --browser-executable=...` (H.264, yuv420p defaults), then `node scripts/export-cues.mjs`.

Scaffold proof (looked at): `out/qa/scaffold-f0280.png` (speed placeholder in Paper chrome, VO3
caption, fonts loaded) and `out/qa/scaffold-f0604.png` (mid mode switch: Ink layer growing from
the INK button over the held browser frame). A temporary test also confirmed `RemapVideo` shows
web.mp4 source f53 exactly and that ClipCard camera zoom works; that test composition was removed.
A z-index leak between layers during the circle reveal was found in the first f604 still and fixed
(`isolation: isolate`).

## 10. Deliberate deviations (aesthetic-rules / cards), with reasons

| rule / card | deviation | reason |
|---|---|---|
| theme-switch-moves "<= 1 theme switch per film" | two switches (into Ink, back to Paper) | brief mandates both; it is the product's own INK|PAPER toggle, same grammar both times |
| theme-switch B: strong ease-out, white-hot ring, coral glow, echo ring | fx.html timing (inout 22 f), no ring/glow | approved effects preview; brief: no glow |
| promo-energy-arc: 2-4 paper-title-cards | none | 36 s budget; captions carry one line per scene; holds give breath |
| brand-ink-open: serif wordmark, amber, exit = fade | Archivo 900, accent, exit = docks into header slot | brand tokens; Q9 (land in a real slot) |
| diagram-cascade: 0.95-1.2 f/char typing; spring with ~15 % overshoot; aurora breathing light | ~4.4 chars/f line bursts; <= 4 % overshoot; no light | 191-char code is decorative (approved fx typed it in 75 f); motion tokens; no glow |
| tape-scroll: porcelain look, ~17 % brake overshoot, linear tape | Paper tokens, ~8 % overshoot, LOG tape | brand; professional trust; 1x->113x honest on one tape |
| wireframe-draw-on: left->right glowing scan, warm afterglow | top->bottom 2 px accent line, no afterglow | reading order "from flowcharts to sankey"; no glow/gradients |
| crash-zoom-punch: ember stage, tilt, exposure flash, darkened periphery, 2.4-2.8 zoom | flat pane, no flash/darkening, zoom 2.13 | brief look; Q2 source-resolution cap (web.mp4 is 1590 px wide) |
| freeze-annotate: lifted target card, porcelain stage | rest dimmed instead of lifting (real video); Ink tokens | footage is a video, not cards |
| paparazzi-flash: full-frame white flash, grain/vignette | pane-level INK.ink flash, no grain | R4 budget (no frame-level hits at all); brief |
| outro-group-photo-launch: stage light, gold dust, light band, contact shadows, settled tilt +/-5 deg, no caption | none of those; tiles settle at 0 deg; VO8 caption shown | brief (no glow swarms, no shadows); grid-true layout; brief requires captions. Energy peak carried by structure + crane + fly-in only (Q8 trade-off, recorded) |
| Q11 caption >= 56 px | 52 px | 52 px on a 1080-wide frame = 4.8 % of width vs 56/1920 = 2.9 %; above the rule's intent |
| lib Caption (22 px mono strip) | replaced by VoCaption 52 px Inter | it is the narrative subtitle |
| Fonts via Google Fonts | local font files | network blocked; same families |
| Gallery reference clips | not consulted | not present locally; demo TSX is the motion reference |
| workbench manifest (pipeline stage 5.9) | not written | out of brief scope; add later if the owner wants the workbench |
| wireframe-draw-on B: draw-on groups ~30 f each, ~62 f in all | blueprint 6-42 (~36 f, groups 12-24 f) | the real renders are the point of the shot: they now hold 32 f instead of ~20 f, and the faint blueprint no longer outlasts them (review, fixer round 1) |
| storyboard "close-up of orders/payments" (dbx photo D) | close-up of orders + its two relationship connectors; payments just outside the edge | orders + payments span src x 135-2060 (1925 px); a 1.4-1.5 crop shows ~1545 px, so keeping both would cut payments' text rows (card pitfall: no crop edge through a text row). Photo C (1.17) is the layout view; D is the detail. |
| emoji in dbx UI | dbx's own "[ ERD: main ]" tab carries a map-emoji glyph; it shows small (<= 20 px) in the live explorer and photos B/C, but photo D's 80 f hold starts below the tab row | it is real product UI in a real recording; the brief's "No emoji" is honoured for everything we draw, and the magnified one is cropped out |
| Q11 two text states (render code pane) | hero.mmd code is read at 27.5 px during typing (camera 1.25x, ~2 s), then sits at 22 px (identity) as the "text in" reference next to the tree | not enlarged: its 27-character lines already fill the 420 px pane (26-28 px would overflow), and ending the pull-back at 1.1 pushes the SVG pane off the canvas or into the caption band. Recorded, not fixed (review nit) |
| crash-zoom-punch / speed / types holds | (none any more) | fixer round 1 met the card holds: browser full view 30 f, speed fully still 32 f, types wall 32 f |

## 11. Work packages (parallel)

| pkg | shots | frame ranges (abs) | files owned |
|-----|-------|--------------------|-------------|
| A | 1 open, 2 render | 0-90, 90-228 | shots/Open.tsx, Open.cues.ts, Render.tsx, Render.cues.ts, shots/a/ |
| B | 3 speed, 4 types | 228-341, 341-451 | shots/Speed.tsx, Speed.cues.ts, Types.tsx, Types.cues.ts, shots/b/ |
| C | 5 browser, 6 flowcli | 451-601, 601-757 | shots/Browser.tsx, Browser.cues.ts, Flowcli.tsx, Flowcli.cues.ts, shots/c/ |
| D | 7 dbx, 8 outro | 757-907, 907-1080 | shots/Dbx.tsx, Dbx.cues.ts, Outro.tsx, Outro.cues.ts, shots/d/ |

Integration after the packages: type-check, full render with `--muted`, extract every acceptance
frame plus each seam (abs 228-244, 341-357, 451-467, 592-623, 757-773, 898-929) from the mp4,
regenerate cues, then the independent final review (final-review.md + aesthetic-rules.md) by a
clean-context reviewer.

## 12. Risks

- web.mp4 resolution limits the browser close-up (source scale 1.29x on screen): text in the
  playground will be slightly soft at the zoom; do not raise Z.
- flowcli unfreeze plays the drag at ~2.6x average; if it reads as a glitch, end the ramp at src 96
  and hold (drop the drag) rather than slowing the freeze.
- dbx photo crops must avoid cutting text mid-row/mid-word; measure on frozen frames 24 / 62 / 110.
- Caption 2-3 lines may graze the SAFE bottom (frame 1036) by a few px; shots should keep their
  lowest important content above canvas y ~930.
- No gallery preview clips available locally: fidelity to the cards rests on the demo TSX only.

## 13. Integration (done)

- Shared change requests: packages A and B reported none. The integrator received no
  shared_change_requests from C or D (C's report was cut off; D's report was not passed on). No
  shot file needed a tokens / timeline / Chrome / lib change to build; `npx tsc --noEmit -p .` passes.
- Shared changes made at integration:
  - `tokens.ts`: the font loader's delayRender gets a 120 s timeout and fails the render
    (`cancelRender`) instead of silently falling back to system fonts. `remotion.config.ts`:
    `setDelayRenderTimeoutInMilliseconds(120000)`. Reason: the first full render failed at frame 582
    because one tab's font load passed the 30 s default.
  - `Main.tsx` / `Root.tsx`: `cover` prop. A still of the last frame (1078 in the first cut, 1079 now) with `{"cover":true}` shows the final
    lockup with the VO1 line in the caption box. This is `out/cover.png`.
  - `scripts/export-cues.mjs`: writes `out/cues.csv` (frame,time_s,shot,event,suggested_sound),
    `out/cues.md` and `out/captions.srt` (exact CAPTIONS windows). `cues.ts` re-exports FPS.
  - Shot cue texts: shot-local frame ranges inside the event/sound text are rewritten as durations,
    so every number in the absolute cue sheet reads correctly.
- Render (final): `npm run render:flowmaid`, which is the brief's command plus `--muted --color-space=bt709`.
  Without `--color-space=bt709` the JPEG frame pipeline writes `yuvj420p` (full range). With it, the
  output is H.264 High, `yuv420p`, tv range, BT.709, 1080 frames (1079 in the first cut), with no audio stream.
  Cover: `npm run cover:flowmaid`. Cues: `npm run cues:flowmaid`.

## 14. Fixer round 1 (after the independent review)

Timeline: speed 105 -> 113 f, outro 180 -> 173 f (its hold had 68 f; now 53 f with the last 30 f
static), total 1080 f = 36.00 s. Everything after shot 3 moved +8 f; outro ends at 1080.

| review item | change |
|---|---|
| speed: lock + footnote held ~10-15 f; footnote 28 px muted, ~0.5 s | run 6-7 f earlier + 8 f longer shot; footnote 32 px --dim in with the readout (16-28) and on for ~2.8 s; "~"/marker/claim settle by 81; every pixel still 81-113 (32 f) |
| types: 12/12 wall held ~20 f, blueprint ~2 s | blueprint 6-42 (was 10-64), scan 48-78 (was 70-100); counter lands on 12/12 and turns accent at 78; wall holds 32 f; cell 01 = hero.svg |
| cues off by > 2 f | mode-circle cues derived from the reveal (first visible at cut + 2, fastest at cut + 11); row 4 and counter-accent split (counter cue = COUNTER_DONE); blueprint bed on the first clearly visible stroke (BLUEPRINT_VISIBLE); browser cues derived from browserCam constants |
| outro tiles out of reading order, short names, unqualified ~113x on the cover | 01 02 03 / 04 05 06 with scene-label names; "GEO MEAN · 25 DIAGRAMS" under "VS MERMAID.JS" |
| dbx photo D | top edge below the emoji tab row; crop otherwise unchanged (deviation recorded in section 10) |
| browser full view ~18 f | crash 12 f later (52-58); full view 16-46 = 30 f; clip capped at src 134 so the tail stays still |
| render code at 22 px after the pull-back | not changed; recorded in section 10 |
| frame 0 blank poster | no picture change: `out/cues.md` tells the owner to upload `out/cover.png` (frame 1079) as the custom thumbnail |

Cover: `npm run cover:flowmaid` now stills frame 1079 (= TOTAL - 1).
