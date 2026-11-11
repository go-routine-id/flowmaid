# Checkpoint S: corridor spike (B12.2)

Workspace only, never committed. `B` = the b12 workspace directory, `R` = the repository.

**Gate: met.** `route_ms` of the corridor spike on the analog at the experiment's block positions is 1.93 ms and
1.98 ms on two runs of the spike's own harness, against the limit 2.88 ms (4.6: max(1.25 x baseline, baseline + 1.0)
from B12.0's lower `perf.sh` run of the starting release, 1.88 ms; the spec's provisional 2.9 is replaced by it).
No stop rule fired. B12.3 starts.

## 1. What it ran on, and the command lines

- Tree: `B/spike/tree` = `git archive` of `bd7069a` (B12.1) plus the throwaway spike, as a patch in
  `B/spike/spike.patch` (scene.rs: `spike_corridors` and its call in `route_built`; edgepass.rs / quality.rs /
  router.rs: a stage profiler and two debug dumps behind environment variables). Nothing of it is in the repository;
  B12.2 has no commit.
- Block positions: `B/spec/tree` (the experiment of appendix A) built as `B/spike/posdump-exp`, run with
  `FM_B12=2 FM_B12_LVL=1 FM_B12_STARTS=7`; it prints each node's centre and size from its static scene into
  `B/spike/pos/<name>.pos`.
- Harness: `B/spike/spike.rs` (an example in the spike tree; binary kept as `B/spike/spike-bin`): parses the input,
  calls `route_sized` with the dumped centres and sizes, prints `route_ms` as the fastest of `ROUTE_RUNS` (5) calls,
  the way `quality::measure_source` times `route`, the worst and mean `drag_ms` over `quality::drag_frames`, and the
  harness measures of the routed scene. `B/spike/spike-base` is the same harness built on the unmodified `bd7069a`:
  today's `route()` at the same positions.
- Commands, twice in a row (`B/spike/results.txt` holds the output):
  `spike-base B/local/in/<n>.mmd B/spike/pos/<n>.pos` and `spike-bin B/local/in/<n>.mmd B/spike/pos/<n>.pos --svg ..`
  for `<n>` in the analog, v2, bench-75, dense-td, dense-lr.
- Machine: 4 CPUs shared with a second package. No `rustc` was running during the runs; the load average was 3.1 to
  3.2 (the other package's benchmark processes) and did not fall in the ten minutes I waited, so the numbers are
  taken under that load. The two runs agree to 0.05 ms on the gated number.

## 2. The table against 4.6

`route_ms` / worst `drag_ms`, run 1 | run 2. "today" = unchanged `route()` at the experiment's positions.

| input | limit `route_ms` | today `route_ms` | spike `route_ms` | limit `drag_ms` | today `drag_ms` | spike `drag_ms` |
|---|---|---|---|---|---|---|
| analog (gated) | 2.88 | 3.46 \| 3.17 | **1.93 \| 1.98** | 5.69 | 6.61 \| 6.62 | 6.85 \| 6.05 |
| v2 (information) | 3.97 | 4.18 \| 4.11 | 3.55 \| 4.76 | 7.64 | 8.83 \| 8.57 | 6.71 \| 6.60 |
| bench-75 | 15.76 | 9.98 \| 9.38 | 12.06 \| 11.78 | 25.28 | 21.61 \| 17.36 | 18.17 \| 18.44 |
| dense-td | 9.20 | 8.81 \| 8.49 | 5.71 \| 6.18 | 17.80 | 15.21 \| 14.96 | 12.98 \| 10.81 |
| dense-lr (information) | 8.03 | 6.94 \| 9.47 | 5.17 \| 4.98 | 17.76 | 10.56 \| 16.89 | 12.26 \| 9.20 |

- The analog's `drag_ms` is reported, not gated (4.1): the spike is over its limit on both runs (6.85, 6.05 against
  5.69) and so is today's `route()` at these positions (6.61, 6.62). The mean drag frame falls from 3.49 | 3.59 to
  2.82 | 2.83 ms. Which frames are the worst was not looked into; the spike has no R6 (the kept edges of a drag) and
  falls back to today's builders whenever two ends overlap along the flow. B12.13 has to bring the worst frame under
  5.69.
- v2 is not gated and is on both sides of its limit (3.55, 4.76): `repair` still searches there (1.59 and 2.14 ms of
  the stage profile below): the single greedy sweep leaves overlaps there that the router takes up again.
- bench-75 is slower with the spike (12.06 | 11.78 against 9.98 | 9.38) and inside its limit. Second cause as the
  spec expects: the experiment's positions give its seven bands gaps of 60 to 125 px between member boxes (one of
  288 px), less the frames' padding, for 75 edges; `repair` is 7.5 ms of the 12 there, so most corridors are
  searched again.

Band gaps at the experiment's positions (`B/spike/bandgaps.py`, between the member boxes of consecutive bands along
the flow, px; a frame's padding and title row come off each end):

| input | gaps |
|---|---|
| analog | 122, 158, 166 |
| v2 | 122, 166, 142, 139 |
| bench-75 | 60, 100, 288, 88, 125, 63 |
| dense-td | 217, 303, 309 |
| dense-lr | 72, 64, 241, 63, 76 |

Stage profile of one `route()` call with the spike (ms, run 1 | run 2):

| input | repair | relabel | nudge | comb | ortho | assign |
|---|---|---|---|---|---|---|
| analog | 0.47 \| 0.48 | 0.37 \| 0.37 | 0.26 \| 0.26 | 0.18 \| 0.18 | 0.03 \| 0.03 | 0.07 \| 0.07 |
| v2 | 1.59 \| 2.14 | 0.60 \| 0.65 | 0.51 \| 0.70 | 0.09 \| 0.09 | 0.04 \| 0.05 | 0.06 \| 0.08 |
| bench-75 | 7.62 \| 7.45 | 0 | 1.70 \| 1.66 | 2.03 \| 2.02 | 0.08 \| 0.08 | 0.12 \| 0.12 |
| dense-td | 2.01 \| 2.13 | 0.79 \| 0.86 | 0.68 \| 0.69 | 0.48 \| 0.55 | 0.48 \| 0.47 | 0.13 \| 0.13 |
| dense-lr | 2.14 \| 2.10 | 0.64 \| 0.63 | 0.51 \| 0.50 | 0.33 \| 0.34 | 0.08 \| 0.08 | 0.10 \| 0.10 |

Today's `route()` on the analog at the same positions spends 1.50 ms in `repair` (9 edges searched); with corridors
0.47 ms (2 edges searched: one node hit of a chain into the last band, one overlap of two returns in one column).
That is where the gate is won: a corridor that is already clear costs `repair` only its scoring.

## 3. Runs through a foreign frame (routed)

Harness `edge_frame_x` / `edge_frame_pairs` and block-metrics `xframe_runs` on the routed SVGs (`B/spike/bm`):

| input | today: edges / pairs / runs | spike: edges / pairs / runs |
|---|---|---|
| analog | 0 / 0 / 0 | 0 / 0 / 0 |
| v2 | 0 / 0 / 0 | 0 / 0 / 0 |
| bench-75 | 0 / 0 / 0 | 0 / 0 / 0 |
| dense-td | 1 / 2 / 1 | 0 / 0 / 0 |
| dense-lr | 3 / 9 / 6 | 1 / 2 / 2 |

`edge_node_x` is 0 everywhere, before and after. dense-lr keeps one edge through two foreign frames (cause not
looked into); B12.13's target there is "not above the static value + 2".

## 4. What the drawing looks like (information; the spike is not judged on quality)

| input | `x_drawn` today -> spike | `lateral_x` | struck chips | crowded chips | `bends_max` |
|---|---|---|---|---|---|
| analog | 25 -> 35 | 7 -> 3 | 4 -> 7 | 6 -> 10 | 10 -> 12 |
| v2 | 24 -> 27 | 10 -> 6 | 1 -> 4 | 5 -> 10 | 12 -> 13 |
| bench-75 | 315 -> 391 | 21 -> 17 | 0 -> 0 | 0 -> 0 | 12 -> 17 |
| dense-td | 118 -> 159 | 38 -> 9 | 2 -> 5 | 5 -> 11 | 17 -> 14 |
| dense-lr | 37 -> 71 | 9 -> 3 | 4 -> 5 | 5 -> 9 | 11 -> 12 |

The spike draws more crossings than today's `route()` and crowds more chips. Both follow from what it leaves out, not
from the corridor idea: its rows and columns are handed out first come, first served, with no nesting by end order
(the static chains of B12.4 and the ordering of B12.5 give `route()` that order through the scene's centres, R4), and
it reserves no label slot (G3). Lateral border crossings fall on every input because entries go through flow-facing
sides. Pictures: `B/spike/<name>.base.svg` and `B/spike/<name>.spike.svg` (PNG for the analog).

## 5. What the spike builds, and what B12.13 has to add

Built (`spike_corridors`, called in `route_built` before the edge loop, for real edges and stand-in sub-edges):

- R2's classes from `Tree::join` and the F1 boxes of both ends, with `FLOW_MIN_GAP`: downstream with a gap, upstream,
  or overlapping along the flow (left to today's builders).
- R3: one greedy sweep along the flow from source to target. Obstacles are the other units of the joining level and
  the units beside the holder in each end's chain, title chips included. A blocked column sidesteps in the gap before
  the obstacle, to the side with the least sideways travel towards the target; rows and columns come from a registry
  of free ones kept in true flow coordinates. Upstream edges are swept mirrored.
- R5's entries in outline: flow-facing sides, a free stretch of the side beside the title chip, else a displaced entry
  with a jog between the title row and the member; a free pin at mid-side for an edge that ends on a block.
- The waypoints go into the edge stages as bare corners on the axes (`ports::assign` keeps them, `ortho` has nothing
  to do: 0.03 ms on the analog).

Not built, and the reasons the numbers above are a floor for speed and a ceiling for nothing else:

- R1 (frames from the shared frame function), R4 (order of tracks in a gap from the ends' order), R6 (kept edges and
  frames in `route_partial`), R7, R8; no label slots; no nesting of rows; no second pass when the sweep leaves an
  overlap, so `repair` still finds work on v2, the dense inputs and bench-75.
- The registry of taken rows and columns is a linear scan; its cost was not profiled apart from the rest of the
  builder (the whole call, builder included, is the `route_ms` above).

Guidance for B12.13 from the measurements: (1) the saving is all in `repair`, so the corridor builder must hand over
tracks that are already apart by `TRACK_PITCH`; an overlap costs a search each. (2) The worst drag frame is decided by
the frames where an end leaves its band; classify per frame and keep the fallback cheap. (3) The gaps G1 reserves
decide bench-75: with the experiment's gaps the corridors collide, so the routed budget there can only be judged after
B12.9.

## 6. Musts and stop rules

- Gate S: `route_ms` on the analog within 2.88 ms on two runs: 1.93, 1.98. Met.
- Stop rule "over the limit on the analog on both runs": not fired.
- Reported, not gated: analog `drag_ms` 6.85 | 6.05 against 5.69 (today's `route()` at the same positions: 6.61 |
  6.62); v2 `route_ms` 3.55 | 4.76 against 3.97.
