## flowmaid 0.32 vs mermaid.js 12.2.0: default visual style audit

### What "mermaid default" means in 12.2.0

The comparison only holds if the baseline is right, so I read the defaults from `mm/node_modules/mermaid/dist/mermaid.js`:

- **Theme and look.** The top level says `theme:"default"`, `look:"classic"`, `layout:"elk"` (lines 6432-6455). The `flowchart` block overrides the theme and look to `theme:"redux-color"`, `look:"neo"` (lines 5855-5872). Its other values are `nodeSpacing 50`, `rankSpacing 50`, `curve "basis"`, `padding 15`, `wrappingWidth 120`, `minNodeWidth 120` and `diagramPadding 8`.
- **Curve.** The `curve:"basis"` setting does not take effect. The ELK layout path sets `layoutEdge.curve = "rounded"` (line 160730), and that draws with `generateRoundedPath(…, 5)` (line 53123). So mermaid 12's flowchart edges are **orthogonal with rounded corners of about 5 px**. This is not the old dagre spline look.
- **Font.** The theme font is `"Recursive Variable"`, which mermaid does not ship. The headless Chrome render fell back to Arial, which is the same face flowmaid's `Helvetica, Arial` stack resolves to. The text in the side-by-side images is therefore a fair comparison.

### What makes flowmaid look worse, ranked from the side.png images

1. **Edge shapes are inconsistent and loose.** Most of the "jelek" impression comes from here, not from colour. Flowmaid uses three kinds of edge in one drawing:
   - free cubic curves between adjacent ranks, with small S-hooks around labels (the demo's yes/no edges);
   - B-splines through layout waypoints;
   - occasional rounded orthogonal paths from the router (the "no" edge in fanout).

   Back-edges become wide arcs or V-loops (demo E→B, lr Redis→Service, state "retry"). Mermaid draws everything orthogonal, and every arrowhead meets the node side at a right angle.
2. **Arrowheads are small, and they balloon on thick edges.** On a normal edge the head is about 10.7×9.5 px and narrow; on a `==>` edge it grows to about 21×19 px (demo D→F, lr →Database, advanced Gateway→Job Queue). Mermaid's head is a fixed 10.5×12.8 px.
3. **Plain labels never wrap.** The long-labels diamond is about 1080×75 px, and wide-labels comes out 1847 px wide. Mermaid wraps every node label at 120 px.
4. **Nodes are tight and uneven.** Heights are 38 px, multi-line text has a 17 px line height (1.2), and widths hug the text, so rows look ragged. Mermaid uses 45 px heights and a 152 px minimum width, which reads as a regular grid.
5. **The drawing is flat.** Mermaid's neo look puts a hard offset shadow on nodes and clusters; flowmaid has none.
6. **Spacing rhythm is off.** Rank gaps are 60 px (mermaid ELK: about 40). Inside clusters they grow to 86 px even where no frame edge is crossed (subgraph: Auth→User DB). The 28 px canvas margin is heavy (mermaid: 8).

**Where flowmaid is already better (keep these):**
- White edge-label chips (mermaid uses a grey 50% block with no padding).
- Left-aligned cluster title on a chip (mermaid centres the title on the frame edge, and edges strike through it, e.g. "Platform" in subgraph.side.png).
- The semantic shape colours.
- Solid accent headers in class, ER and sequence diagrams.
- Pie and journey charts.
- State `<<choice>>`/`<<fork>>` are drawn correctly; mermaid 12.2 draws `decide`/`forked` as plain boxes because they are declared after use.

### Global style tokens, side by side (flowchart)

| Token | mermaid 12.2.0 default (neo / redux-color) | flowmaid now (file:line) | Makes flowmaid look worse? |
|---|---|---|---|
| Font family | `"Recursive Variable", arial, sans-serif` (renders as Arial) | `Helvetica, Arial, sans-serif`, scene.rs:3914 | No. Same face in practice. Keep, because the width table is calibrated to Helvetica (layout.rs:173). |
| Node font size | 14px | 14, scene.rs:2802 | No |
| Edge label font size | 14px | 14, inherited (scene.rs:3956-3973) | No |
| Cluster title | 14px, normal weight, `#000`, centred on the top edge | 12px bold `#6a7086`, left, on a chip (layout.rs:176, scene.rs:2846-2851) | Slightly small. The style itself is better than mermaid's. |
| Font weight | Normal everywhere | Normal (nodes), bold (cluster titles) | No |
| Line height | 1.5 → 21px | `LINE_H` 17, layout.rs:166 | **Yes.** Multi-line labels look cramped. |
| Text colour | `#28253D` | `#232840`, scene.rs:51 | No (same) |
| Node fill | `#ffffff`, all shapes | Pastel per shape, style.rs:21-79 | No. This is flowmaid's identity. |
| Node stroke | `#28253D` | Saturated per-shape hue, style.rs:21-79 | No |
| Node stroke width | 2px | 1.6, scene.rs:2979 (also hard-coded at 3037, 3052-3053, 3111) | Mild. Slightly faint. |
| Corner radius | rect 0, rounded 12, stadium h/2 | rect 3, rounded 9, stadium h/2, scene.rs:2984-2986 | No |
| Edge colour | `#000000` | `#44507a`, scene.rs:50 | No. Softer, still about 8:1 contrast. Keep. |
| Edge width (normal / thick) | 2 / 3.5px | 1.7 / 3.4, scene.rs:2441-2442 | Mild. |
| Dotted pattern | `2 2` (real dots); dashed `3` | `"5 4"` (reads as dashed), scene.rs:2444 | Minor |
| Arrowhead | Filled triangle 11.5×14 viewBox, drawn 10.5×12.8px, `userSpaceOnUse`, line stops 4px short of the tip | `M0 1 L9 5 L0 9` in a 10×10 viewBox, `markerWidth 7` in stroke-width units: about 10.7×9.5px, refX 8.5, line runs under the tip (scene.rs:2805-2808, 2447) | **Yes.** Small and narrow. The 1.7px line blunts the tip. |
| Thick edges scale the arrowhead? | No (`userSpaceOnUse`) | **Yes**: 2× (about 21×19px) | **Yes.** Very visible. |
| Edge label box | `rgba(204,204,204,.5)`, no padding, no border, 21px tall | White, `rx 4`, 1px `#d5d9ec` border, 20px tall, text + 14px (scene.rs:3960-3962, 52; layout.rs:158-160) | No. Flowmaid is better. |
| Cluster fill / stroke | Palette cycles per cluster (`#FDF4FF`/`#E879F9`, `#F0FDFA`/`#2DD4BF`, `#FFF7ED`/`#FB923C`…); 1px, rx 0; fallback `#F9F9FB`/`#BDBCCC` | One tint for all: `#f7f8fd` / `#c9cfe8`, 1.4px, rx 8, scene.rs:2823-2826 | Mild. Nested clusters can't be told apart. |
| Cluster padding | About 24 sides, about 48 top (21 title + pad) | `SUB_PAD` 14, `SUB_HEADER` 26 (scene.rs:997-999); `FRAME_PAD` 14, `CLUSTER_HEADER` 26 (layout.rs:71-74) | No |
| Node padding / size | 16 horizontal, 12 vertical; label box min 120 wide → node 152×45; 2 lines → 66 tall | `PAD_X` 16, `BASE_H` 38, `MIN_W` 54 (layout.rs:48-52) | **Yes.** Cramped, ragged widths. |
| Wrapping | Every node label at 120px | Markdown labels only, 175px (markdown.rs:28-31); plain labels never | **Yes.** Huge flat shapes. |
| Diamond | A square rotated 45° (171px for "Valid?") | `w=(tw+24)·1.6`, `h=1.7·BASE_H` (layout.rs:405) | No for short labels (more compact than mermaid); **yes** for long ones (no aspect cap). |
| Node spacing | 50 (measured 50) | `GAP_B` 50, layout.rs:56 | No |
| Rank spacing | 50 configured; ELK gives about 40 (24 inside clusters) | `GAP_L` 60 (layout.rs:57); 86 inside clusters | **Yes.** Stretched, uneven rhythm. |
| Default curve | Orthogonal, ~5px rounded corners, all edges | `basis`: cubic / B-spline (model.rs:189, scene.rs:3738), mixed with router paths smoothed by `smooth()` (router.rs:2567); `rounded` exists only on request (scene.rs:350) | **Yes, the biggest factor.** |
| Canvas padding | `diagramPadding` 8 | `MARGIN` 28 (scene.rs:33, layout.rs:58) | Mild (extra whitespace when embedded) |
| Drop shadow | `feDropShadow dx=4 dy=4 stdDev=0 opacity .06` on nodes and clusters (small variant 2/2) | None | **Yes.** Looks flat. |
| Background | `#ffffff` | `#ffffff`, opt-out (scene.rs:3936) | No |

### Other diagram types

| Diagram | mermaid default | flowmaid now | Verdict |
|---|---|---|---|
| class / ER | Tinted fill and coloured border per box, bold dark header, 14px | Solid accent header with white bold text, 13px base (class.rs:205, 247; er.rs:196, 232) | Flowmaid looks better. Content is missing, though: class drops `«interface»`, the note, and the ` : ` before return types; ER drops the comment column. |
| sequence | Tinted actor boxes, black 1px lifelines, yellow notes | Solid accent actor boxes, dashed lifelines, numbered bold, amber notes (seq.rs:511, 551) | Flowmaid is cleaner. Keep. |
| state | Neo boxes; composite `direction LR` honoured | Composite drawn top-down with big gaps; note missing | Layout problem, not a token problem |
| mindmap | Large pastel shapes, strong root | Saturated pills with white text; pale root `#c9cfe0` (mindmap.rs:38) | Root is a weak focal point. Give it an accent fill. |
| gitgraph | Thick lanes coloured per branch, orthogonal elbows | Every branch edge 2px `#44507a` (gitgraph.rs:336); diagonal swoops cross commit labels | **Worse.** Use branch accent colours at 3-4px with rounded elbows. |
| pie / journey / timeline | Neon pastels and bugs (mermaid's timeline drops periods) | Accent palette, white % labels | Flowmaid is better. |
| architecture | Big blue icon tiles | One flat row of 13px boxes with emoji glyphs; edges hug node and frame borders | Worse layout. Not a token issue. |

### Problems that aren't tokens but dominate the "ugly" impression

- **advanced:** the `Dev` node is placed inside the CI frame, and the `Dev → CI` edge runs off the top of the canvas (`d="M 656.5 68.0 … 656.5 28.0"`). The "Async Workers" frame is never drawn: the SVG has 2 cluster rects for 3 subgraphs.
- **lr:** the Redis→Service back-edge dips into a V-loop below the drawing.
- **fanout:** the "no" back-edge threads up between Worker 2 and Worker 3, through the fan.
- **subgraph:** every rank inside the cluster is 86 px apart, against 60 px outside.

### Recommended default-theme change set

**Tier A: token changes only. Prototyped and safe.**

| # | Change | Where |
|---|---|---|
| 1 | `EDGE_WIDTH` 1.7 → **2.0**; `EDGE_WIDTH_THICK` 3.4 → **3.5** | scene.rs:2441-2442 |
| 2 | Arrow marker → `viewBox="0 0 10 10" refX="9" refY="5" markerUnits="userSpaceOnUse" markerWidth="11" markerHeight="11"`, path `M 0 0 L 10 5 L 0 10 z`. Set `ARROW_MARKER_UNITS` 7.0 → **5.5** so `ARROW_MARKER_PX` stays 11 and the label clearance in labels.rs:92 still matches. Thick edges then keep the same head. Optional polish: put the tip exactly on the border (`refX=10`) and stop the stroke about 3 px short, as mermaid does, so the tip stays sharp. | scene.rs:2447, 2805-2808 |
| 3 | Node stroke default 1.6 → **1.8**, including the hard-coded 1.6 values | scene.rs:2979, 3037, 3052-3053, 3111 |
| 4 | Soft shadow on node shapes only (not text, edges or clusters): `<filter id="fm-shadow-<DefsKey>" x="-10%" y="-10%" width="130%" height="140%"><feDropShadow dx="0" dy="2" stdDeviation="1.5" flood-color="#232840" flood-opacity="0.14"/></filter>`. Namespace the id the same way markers are. Add an `SvgOptions` opt-out for flat output. | scene.rs node style (2976-2980) and defs |
| 5 | `MARGIN` 28 → **16** (leaves room for the shadow) | scene.rs:33, layout.rs:58 |
| 6 | `BASE_H` 38 → **42**; `LINE_H` 17 → **20**; `MIN_W` 54 → **96** | layout.rs:50, 166, 52 |
| 7 | `GAP_L` 60 → **50** (mermaid's `rankSpacing`); keep `GAP_B` 50 | layout.rs:57 |
| 8 | Clusters: stroke 1.4 → **1.0**; title 12 → **13px**, bold → **600**. Alternate fill by nesting depth, **`#f7f8fd` / `#eef1fa`**. Keep rx 8 and the left-aligned title chip. | scene.rs:2823-2851, layout.rs:176 |
| 9 | Optional: dotted dash `"5 4"` → **`"2 3"`** so dotted edges look dotted, as in mermaid | scene.rs:2444 |
| Keep | `EDGE_COLOR #44507a`, `TEXT_COLOR`, edge-label chip, semantic shape palette, corner radii, font stack | — |

**Tier B: sizing (needs layout tests updated).**

10. Run the existing greedy wrap from markdown.rs (175 px) on **plain** node labels too, not just ``"`…`"`` strings. Wrap diamond text at **120 px**, and cap the diamond's aspect with `h = max(1.7·BASE_H, w/2)` at layout.rs:405.
11. Reserve cluster header and padding only on the ranks that actually cross a frame boundary, so inner gaps drop from 86 px back to `GAP_L`.

**Tier C: edge shape. This is the biggest single improvement.**

12. Use one family of edge shapes by default.
    - Prototype B draws adjacent-rank edges as a rounded Z with radius 6. The cubic's control points are already `[p0, (x0,m), (x3,m), p3]`, so this is nearly free, and it already matches mermaid 12 (demo yes/no edges, perpendicular heads).
    - Simply turning spline or back-edge control points into Z-steps does **not** work: it makes large detours (fanout, column B). Long edges and back-edges have to use the router's own orthogonal paths (`router.rs` lattice + `smooth`).
    - If you'd rather keep curves as flowmaid's identity, at least force a straight stub of 12 px or more, perpendicular to the node side, at both ends, and route back-edges along the side channel instead of in wide arcs.

### Prototype evidence

I copied the source to the scratchpad and applied Tier A items 1-7 there; items 8 and 9 were not prototyped. Column B also uses an env-var hack for the orthogonal edges. The repo itself is untouched. Snapshot tests will need updating if these values land.

Generated images and SVGs:
- `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/proto-out/{demo,fanout,subgraph,state,lr,advanced}.four.png`: mermaid / flowmaid now / A (tokens) / B (tokens + orthogonal)
- `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/proto-out/*.three.png`: mermaid / now / A
- `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/proto-out/*.proto.svg` and `*.ortho.svg`

Supporting files:
- Patched source: `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/proto/src` (scene.rs, layout.rs)
- Render script: `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/tools/render4.js`

What the prototype showed:
- **Column A is clearly crisper.** Thick-edge heads are normal size, nodes are taller and more regular, and the shadow adds depth without clashing with the pastel palette. In lr, the different node sizes also made the router drop the V-loop, but that was a side effect and isn't guaranteed.
- **Column B** matches mermaid's look on adjacent-rank edges but makes fanout worse. That supports doing Tier C through the router rather than in the painter.