# Advance mode: terminals, sub-elements, and a crossing-free router

Status: **approved 2026-09-05** (D1–D7). Not yet implemented. Targets `src/advance.rs`.

## Goals

1. **Zero edge crossings whenever a crossing-free orthogonal routing exists**; when none exists, the residual crossings sit where they cost least.
2. **Edges reference connection points by id** at three levels — node, named anchor, sub-element — composably.
3. **Sub-elements** (compartments / cells / pins) are first-class: they have ids, rects, anchors, and can be edge endpoints.
4. Everything stays orthogonal, deterministic (same input → byte-identical SVG), zero-dependency, MSRV 1.75, wasm32-clean, and additive over today's DSL/JSON.

## 1. Terminal references

A *terminal* is one end of an edge. Grammar (text DSL):

```
terminal := node ('.' element)* ('@' anchor | ':' side)?
```

| Form | Meaning |
|---|---|
| `a` | node `a`, router picks the side |
| `a:right` | node `a`, right side, centre offset — exists today |
| `a@out` | named anchor `out` declared on `a` |
| `a.cpu` | sub-element `cpu` of `a`, router picks an exposed side |
| `a.cpu:left` | sub-element `cpu`, left side |
| `a.cpu@pin1` | named anchor on the sub-element |
| `a.cpu.core0` | nested sub-element |

`.` descends, `@` names an anchor, `:` picks a side; `@`/`:` are terminal and mutually exclusive. Ids may therefore not contain `.` or `@` — validated at parse with a line-numbered error. (`:` was already reserved by side syntax.)

JSON mirrors it: `"from": "a.cpu@pin1"` is accepted as a string; a structured form `{"node":"a","element":["cpu"],"anchor":"pin1"}` is also accepted for programmatic callers.

### Model

```rust
pub struct Anchor      { pub id: String, pub side: AdvanceSide, pub offset: f64 } // 0..=1 along the side
pub struct SubElement  { pub id: String, pub label: String, pub shape: Shape,
                         pub anchors: Vec<Anchor>, pub elements: Vec<SubElement>,
                         pub layout: ElementLayout, pub style: NodeStyle }
pub enum   ElementLayout { Column, Row }                     // how children stack inside
pub struct EdgeEnd     { pub node: String, pub path: Vec<String>, pub at: Option<AnchorRef> }
pub enum   AnchorRef   { Side(AdvanceSide), Named(String) }

// additive fields
AdvanceNode  { …, pub anchors: Vec<Anchor>, pub elements: Vec<SubElement>, pub layout: ElementLayout }
AdvanceEdge  { …, pub from_end: EdgeEnd, pub to_end: EdgeEnd }   // from/to/from_side/to_side kept, derived
```

The four sides are built-in anchors at offset 0.5, so `a:right` and a declared `anchor r right 0.5` resolve identically.

### Declaring anchors and sub-elements (text DSL)

```
cpu[CPU] {
  anchor out bottom 0.5
  core0[Core 0]
  core1[Core 1] {
    anchor irq right
  }
  layout column
}

cpu.core1@irq --> mem.bank0
cpu@out       --> bus:top
```

`anchor <id> <side> [offset 0..=1]` (offset `0.5` when omitted); `layout column` is the default, `row` lays children side by side. Comments are whole lines only, as elsewhere in the DSL. A block may also be written on one line — `core1[Core 1] { anchor irq right }` — and such blocks nest.

Parser: a line that is a node declaration and ends with `{` opens a **node block**; `lane … {` still opens a lane block. One block stack, each frame tagged lane-or-node, so `}` closes the right one. Inside a node block only `anchor`, `layout`, `style`, and element declarations are legal.

### Sub-element layout

Deterministic, coordinate-free: children stack as **compartments** (`column`, default) or side by side (`row`). The parent grows to fit; nesting recurses. No free placement in v1 — that would reintroduce the coordinate bookkeeping the DSL exists to avoid.

### Resolution → point + exit direction

`resolve(EdgeEnd) -> (x, y, dir)`:

- node: a point on the node rect's boundary; side auto = router chooses among the four.
- anchor: its point on its host rect.
- sub-element: a point on the **sub-element's** rect, which lies inside the parent. The edge first travels in `dir` straight to the parent's outer boundary — a *lead* inside the endpoint's own node, which is permitted — and normal routing starts there.

**Exposed-side rule.** A sub-element side may be used only if it touches the parent's boundary; in a column, `left`/`right` always are, `top` only for the first child, `bottom` only for the last. An interior side is a parse-time error (`"a.core0:bottom is not an exposed side"`) rather than a lead that pierces a sibling.

## 2. The router

### Channel grid

After layout (unchanged — lanes, node placement, barycentre ordering all stay), build a grid from:

- x-lines: lane boundaries; every node's left/right edge ± clearance; the midline of every horizontal gap
- y-lines: every node's top/bottom edge ± clearance; the midline of every vertical gap

Grid vertices are the intersections. A vertex inside a node rect is blocked — except vertices inside an edge's own endpoint nodes, so leads and ported exits are legal. Routing on this grid is orthogonal by construction, and it fixes today's two defects: cross-lane / ported edges no longer pass through blockers, and a ported edge no longer passes back through its own source.

### Per-edge A\*

```
cost(path) = length
           + B · bends
           + X · crossings_with_already_routed_edges
           + S · grid_segments_shared_with_other_edges
```

`B` keeps paths clean, `S` stops two edges lying on top of each other (overlap is not a crossing, but reads as one), `X` is the crossing penalty and is what the next section escalates. Ties break on a fixed order (edge index, then direction order), which keeps output byte-identical.

The first step out of a terminal is forced to its exit direction, so an anchor on the right really leaves rightwards.

**Ports are hard constraints.** Given `d:right --> b:top` for two side-by-side nodes, the shortest path that honours both is a loop — right, up, left, down — and that is what the router draws. It never overrides a declared side to find a shorter route; a port the router may ignore is not a port. Without ports (`d --> b`) the router picks the sides itself and the same pair gets a single straight segment. So the length of a ported path is the cost of the declaration, not of the router.

### Negotiated rip-up-and-reroute

```
order  ← edges by Manhattan distance, shortest first
X      ← X0
for iter in 1..=MAX_ITERS:
    route every edge in order with A*            (already-routed edges are obstacles-with-cost)
    if crossings(all) == 0: break
    X ← X · ESCALATION
    move the edges involved in a crossing to the front of `order`
```

This is PathFinder-style negotiated congestion routing, the standard approach in EDA. Per edge, once `X` dominates, A\* returns a crossing-free path whenever one exists on the grid given the others; the rip-up lets earlier edges move out of the way of later ones.

### What is actually guaranteed

> Zero crossings whenever a crossing-free orthogonal routing exists **on the channel grid** and is found within `MAX_ITERS`. When none exists, the remaining crossings sit where the escalated cost made them cheapest.

Two honest limits: negotiated routing is a strong heuristic, not a proof of global optimality; and the grid decides which paths exist at all. A finer grid finds more paths at more cost — exposed as `config router_grid coarse|fine`.

The scene reports `crossings: usize`, so tests and the UI can assert or display it.

## 3. Scene, hit-testing, painters

```rust
AdvanceSceneNode { …, pub elements: Vec<AdvanceSceneElement>, pub anchors: Vec<AdvanceSceneAnchor> }
AdvanceSceneElement { id, label, x, y, w, h, style, elements, anchors }
AdvanceSceneAnchor  { id, x, y, side }
AdvanceSceneEdge    { …, from_point: (f64,f64), to_point: (f64,f64) }
AdvanceScene        { …, crossings: usize }
```

Hit-testing gains `element_at` (innermost) and `anchor_at(tolerance)`; `hit_test` order becomes Anchor → Element → Node → Edge → Lane. `scene_to_json` carries all of it, so `flowmaid-web` can paint compartments and snap drags to anchors.

## 4. Compatibility

| Surface | Change |
|---|---|
| Text DSL / JSON | Additive only; every existing input parses identically |
| Ids | `.` and `@` become reserved in ids — a diagram using them gets a clear error |
| Node / lane geometry | Unchanged |
| Edge paths | **Change** for cross-lane and ported edges (they gain obstacle avoidance) and, with the new router, for same-lane too. Determinism is kept; snapshots shift |
| `from_side` / `to_side` on scene edges | Kept, derived from `EdgeEnd` |
| Dependencies / MSRV / wasm32 | Unchanged |

Decision needed: the new router becomes the default (recommended — it is the point) versus opt-in behind `config router grid` for one release.

## 5. Phases

| Phase | Delivers | Router |
|---|---|---|
| **P1 Terminals** | `Anchor`, `SubElement`, `EdgeEnd`; DSL + JSON parsing; resolution with the exposed-side rule; compartment rendering; scene + hit-testing | Today's, fed resolved points and excluding the endpoint's own node — which alone fixes the ported-through-own-node defect |
| **P2 Grid router** | Channel grid, A\* with bends + obstacles; replaces the same-lane / cross-lane / ported routers | Zero through-box for every edge kind |
| **P3 Negotiation** | Crossing cost, rip-up-and-reroute, `crossings` in scene; a planar test suite asserting 0 | Zero crossings where possible |
| **P4 Ship** | README, docs site, `examples/advance_terminals.mmd`, CHANGELOG, minor bump | — |

One PR and one independent review per phase.

## 6. Out of scope (design accommodates, not built)

- **Nets** — a net is a set of terminals sharing a trunk. `EdgeEnd` already gives each terminal a resolved point; a net router would be a Steiner-tree variant on the same grid. Later, if wanted.
- Free-coordinate sub-elements.
- Curved / diagonal paths.

## 7. Decisions to confirm

| # | Decision | Recommendation |
|---|---|---|
| D1 | Reference syntax | `.` descend, `@` anchor, `:` side |
| D2 | New router default or opt-in | Default; minor bump; CHANGELOG notes the visual change |
| D3 | Sub-element layout | Column (compartments) default, `row` optional; no free coordinates |
| D4 | Interior-side anchors on sub-elements | Parse error, not a piercing lead |
| D5 | Guarantee wording | As in §2 — precise about the grid and the iteration bound |
| D6 | Nets | Out of scope now; model already accommodates |
| D7 | A port that makes the path much longer | Hard constraint — honoured at any cost; without a port the router picks the shortest sides |
