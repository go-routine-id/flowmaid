# flowmaid LinkedIn promo: production brief (hard constraints)

Mode: video-shotcraft **autonomous free creation** (`references/pipeline.md`), chosen by the owner.
Skill repo (read-only reference): `../shotcraft/repo` (SKILL.md, references/, demos/, assets/lib/, gallery/api/library.json).
Remotion project (this folder): Remotion 4.0.484, React 19. The old template comp `AiflPromo` stays as is.
New work goes in `src/flowmaid/` and registers a new composition `FlowmaidPromo` in `src/Root.tsx`.

## Format
- 1080 x 1350 (4:5, LinkedIn feed), 30 fps, about 34-36 s. Language: English.
- Deliverable is SILENT (no audio track). The owner adds an ElevenLabs voice-over and their own sound effects.
  So: no Freesound, no BGM. Instead export a cue sheet (`out/cues.csv` + `out/cues.md`) with every
  visual hit (frame, time, shot, action, suggested sound type) from the same `SHOTS` timeline.
- Captions: burn in the voice-over lines (below) as on-screen captions, styled with the design tokens, driven by one
  `CAPTIONS` table derived from `SHOTS` (the owner's VO will be fitted later, so timing must be easy to move).
- LinkedIn mobile overlays cover the bottom ~10% (y > 1215): keep important content above it.
- Render: `npx remotion render src/index.ts FlowmaidPromo out/flowmaid-promo.mp4 --browser-executable=/opt/pw-browsers/chromium_headless_shell-1243/chrome-headless-shell-linux64/chrome-headless-shell`
  (remotion.media is blocked; the same flag works for `npx remotion still`). Output H.264, yuv420p.
  Add `--muted` or make sure no audio track is written.

## Visual language: flowmaid-web (the product's own playground design). Do NOT invent another skin.
Source of truth: `../fmweb/index.html` (CSS vars) and the approved style frames `../fw/style.html`, `../fw/fx.html`
(look at `../fw/style-frames.png` and `../flowmaid-effects-preview.mp4`).
- Two modes. Paper (light, the default): `--bg:#efece5 --panel:#e7e3da --mass:#ddd8cc --ink:#15140f --dim:#55534a
  --muted:#86826f --line:#e2ded4 --line2:#cbc5b6 --accent:#8839ef --accent-soft:rgba(136,57,239,.10)
  --canvas:#f5f3ee --dot:#d3cdbf`. Ink (dark, used for the terminal scenes): `--bg:#17171a --panel:#1e1e22
  --mass:#26262b --ink:#eae8ee --dim:#cdccd2 --muted:#908f96 --line:#26262b --line2:#35353d --accent:#cba6f7
  --canvas:#141417 --dot:#2b2b31`.
- Square corners, 1px rules, NO shadows, NO gradients. "Structure comes from lines."
- Fonts (Google Fonts, load with @remotion/google-fonts or a <link>): Archivo 900 (display, letter-spacing -.045em),
  Inter 400/500/600 (body), JetBrains Mono 400/500/600 (labels, code; labels uppercase .07em-.16em tracking).
- Canvas: dotted whiteboard (`--canvas` + `--dot` dots, 28px grid). Header strip like the playground
  ("flowmaid" in Archivo 900 + accent eyebrow "PURE RUST · ZERO DEPS" + INK | PAPER mode toggle), footer strip in mono
  uppercase muted. Scene labels as accent tags (`01 RENDER`).
- Code uses the playground's syntax colours (paper): header kw #8839ef, kw #1d5ad6, dir #0e6f7c, str #2d7a3e,
  node text #6b5300, arrows #b54a12, punctuation #55534a.
- The mode switch (toggle slides PAPER -> INK and a circle grows out of the button) is the approved transition into the
  terminal scenes; switch back to PAPER for the end card.
- No personal branding (no "~/AS", no "#![arian]", no ariansaputra.com). No emoji. No AI tropes
  (no blue-purple gradients, glow swarms, left-border cards).

## Assets (all real, all in `public/`)
- `public/clips/web.mp4` 1590x1064, 156 f: the real flowmaid-web playground (Paper theme). Code pane on the left
  (x < 730 px), canvas on the right. A cursor drags the "Auth?" node left from f53 (press) to f117 (release); its 3 edges
  re-route live every frame. Toolbar hides ~f153. Diagram ink bbox in the source: x 774-1517, y 98-905.
- `public/clips/flowcli.mp4` 2044x1620, 173 f (source f24-f196): real flowcli in tmux. Left pane: shell typing
  `echo '    D --> N[Email buyer]' >> flow.mmd` (src f34-f62, Enter f64); right pane: flowcli --watch redraws
  (src f70: 8 -> 9 nodes); click picks Backorder (src f112), drag in 10 steps (src f118-f145), click on canvas clears
  (src f175). Mp4 frame = source frame - 24.
- `public/clips/dbx.mp4` 2240x1504, 133 f, ALREADY EDITED: f0-17 explorer, f18-29 4-table ERD (orders, products,
  order_items, payments), f30-132 zoomed 0.80x + pan + select `orders` (highlight at mp4 f~76). HARD CUTS at mp4 f18
  and f30 (cover them with a cut/transition). Never show other dbx frames: the raw take has overprinted labels.
- `public/svg/*.svg`: flowmaid 0.40 renders with a transparent background: hero.svg (the flowchart below),
  advanced (flowchart), sequence, class, state, er, pie, journey, gitgraph, mindmap, timeline, sankey, architecture.
- `public/hero.mmd`: the Mermaid source typed in the opening.
- `public/frames/*.png`: stills of the clips (for posters / covers).
- Flowcharts in hero.svg: edges are `<path fill="none" marker-end=...>`, white chip rects are labels.

## Claims you may make (verified; do not add others, do not show a wasm size)
- flowmaid renders Mermaid diagrams in pure Rust. Mermaid text in, clean SVG out. Zero dependencies.
- 12 diagram types: flowchart, sequence, class, state, ER, pie, user journey, git graph, mindmap, timeline, sankey,
  architecture.
- About 113x faster than mermaid.js (geometric mean over 25 sample diagrams; say it on screen as a footnote).
- Runs in the browser as WebAssembly (flowmaid-web). Drag a node and its edges follow (re-route live).
- flowcli: diagrams in your terminal, built on flowmaid; `--watch` redraws when the file is saved; drag re-routes.
- dbx: a terminal database client; it lays out its ERD with flowmaid (`flowmaid er::scene`).
- Open source, GPL-3.0-or-later. `cargo add flowmaid`. Links: opensource.go-routine.com/flowmaid-web (playground),
  github.com/go-routine-id/flowmaid (code).

## Voice-over (the owner records it later with ElevenLabs; scene timing should leave room for each line)
1. flowmaid renders Mermaid diagrams in pure Rust.                       (~2.7 s)
2. Text in, clean SVG out.                                               (~2.2 s)
3. Over a hundred times faster than mermaid.js.                          (~2.9 s)
4. Twelve diagram types, from flowcharts to sankey.                      (~2.9 s)
5. In the browser, drag a node, and its edges follow.                    (~4.3 s)
6. flowcli draws them in your terminal, and redraws when you save.       (~4.5 s)
7. In dbx, flowmaid lays out your database schema as an ERD.             (~4.5 s)
8. flowmaid is open source. Try it in your browser, or cargo add flowmaid. (~5.5 s)

## Engineering rules (from the skill)
- One source of truth: `SHOTS` (from/duration per shot); CAPTIONS, CUES and transition points derive from it and are exported.
- Deterministic: no Date.now / Math.random / argless new Date; seeded PRNG only.
- Each shot: 2 acceptance frames rendered with `npx remotion still ... --frame=N` into `out/qa/` and looked at.
- Follow `references/aesthetic-rules.md`. Read the chosen card in full and its exact demo TSX before implementing.
- Keep disk use small (about 5 GB free): delete scratch renders you no longer need.
