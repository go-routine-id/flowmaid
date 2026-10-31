# flowmaid visual parity — execution plan (all sprints)

Repo: /home/user/flowmaid (Rust, zero deps). Branch: `ccr-ccc6d89a-5cxd7e` (based on main 5f318a0).
Goal: make flowmaid's output look as good as / better than mermaid.js 12.2.0 for every diagram type,
fixing every verified finding in the comparison report — WITHOUT losing what differentiates flowmaid.

Inputs (read the parts relevant to your work package):
- Full verified report (Indonesian): $S/report.md  (sections 2.x themes, 3 correctness bugs, 5 roadmap; item ids Q1..Q18, M0..M15, B1..B7)
- Per-diagram verified findings with root causes (JSON): $S/perdiagram.json  (field `verified[]` with root_cause/fix_sketch)
- Global style audit (markdown): $S/style.md
- Prototype evidence: $S/rv_adv_skeptic/ (subgraph fix), $S/rv_be/ (back-edge lanes), $S/rev_ll/ (label gap), $S/stressdbg/, $S/fmx_fanframes/, $S/proto-out/
  ($S = /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad)

## NON-NEGOTIABLE INVARIANTS (the user's explicit constraint: do not lose flowmaid's differentiating features)

1. **Interactive API keeps working and keeps its speed.** `scene::{scene, scene_sized, route, route_sized, route_partial,
   box_edge_bezier}`, `Scene::{hit_test, node_at, edge_at, cluster_at, hits_in_rect, nearest_node}` (pick must match render),
   `fold::scene_compact`, `to_mermaid` round-trip, json scene export, the whole `advance` module (render_advance_routed*,
   AdvanceScene picking, crossings), `quality` harness. Drag performance: per-file worst `drag_ms` and `route_ms` from
   `$S/tools/perf.sh` must not exceed max(1.25 × baseline, baseline + 1.0 ms) (timings are noisy on this 4-CPU box: re-run
   before concluding a regression); `route_partial` stays < 6 ms. Static `scene_ms` may grow at most 2× baseline per file.
2. **Scene geometry contract for hosts** (playground, desktop app, flowcli paint from it): `SceneEdge.bezier` +
   `SceneEdge.waypoints` semantics (hosts B-spline through waypoints — orthogonal paths must therefore be delivered as the
   router already does: corner-rounded, densely sampled points via `router::smooth`, so a host's spline reproduces them);
   first `Graph::edges.len()` scene edges index-parallel with `Graph::edges`, sub-edges after; node centres; cluster
   top-left. Public structs are `#[non_exhaustive]` → ADDING fields is fine; removing/renaming/changing meaning of pub
   items is NOT. No new public API without doc comments.
3. **Determinism**: byte-identical SVG across runs. **Zero dependencies.** **MSRV 1.75** (`cargo +1.75 check`).
   **wasm32** (`cargo check --lib --target wasm32-unknown-unknown`). `cargo clippy --all-targets -- -D warnings`. `cargo fmt`.
4. **Keep the things flowmaid already does better** (report §6): white padded label chips; cluster titles never struck by
   edges; compact diamonds for short labels; portable SVG (`<text>`, no foreignObject, small, role=img/aria/<title>);
   text-sized nodes & compact canvas; respecting `<br/>` and no orphan wraps; straight symmetric spines; clear self-loops;
   short direct edges (no perimeter detours); spread ports; source order preserved; dotted/thick semantics; the semantic
   per-shape palette as DEFAULT (add a neutral theme as an OPTION, never replace); correct `<<choice>>`/`<<fork>>`;
   timeline periods before first section; sankey barycentre ordering; sequence autonumber/frames; pie/journey palettes;
   stripping trailing `%%` comments. Do not copy mermaid's bugs for "parity".
5. **Tests**: every fix gets a regression test. Byte-exact snapshot tests / quality gates may be updated ONLY when the output
   change is intended — say which and why in the commit message. Never delete coverage, never `#[ignore]`, never loosen a
   gate to hide a regression. Gates that LOCK a defect (report §2.13: e.g. `MAX_ASPECT`/`WIDE_LABELS_CENTER_OFF` in
   tests/quality.rs, labels.rs `wide_lr_labels_step_beside_their_line_off_the_arrowhead`) may be revised, with the new
   value justified.
6. **Code style**: match the surrounding code (naming, comment density — this codebase explains *why* in comments,
   references constants by name). Keep duplicated rules in sync (e.g. back-edge bow rule exists in scene.rs AND
   layout.rs `drawn_breadth`; markers/cylinder are duplicated in advance.rs).
7. **Docs**: user-visible changes get a CHANGELOG entry under `## [Unreleased]` (Keep a Changelog style used in the file),
   README updates when behaviour/options change.

## Tools ($S/tools)
- `guard.sh [repo]` — fmt, clippy -D warnings, all tests, doctests, MSRV 1.75 check, wasm check. MUST print `GUARD: ALL GREEN` before you commit. (~2.5 min)
- `perf.sh <label> [repo]` — quality + timing (static & routed & drag) on the 14 flowchart corpus files, compared with baseline. Output `$S/runs/<label>/perf.tsv`.
- `render.sh <label> [repo]` — renders the 25-diagram corpus to `$S/runs/<label>/<name>.png` and `<name>.three.png` (mermaid | baseline | now). `ONLY=demo,lr render.sh ...` for a subset. VIEW the PNGs with the Read tool.
- `fmrender.sh <repo> <outdir> <files.mmd...>` and `node $S/tools/mmrender.js <outdir> <files.mmd...>` (run with `NODE_PATH=$(npm root -g)`) — render ad-hoc inputs with flowmaid / mermaid for comparison.
- Corpus sources: `$S/cmp/corpus/*.mmd`; mermaid reference renders `$S/cmp/out/<name>.mermaid.png`; baseline flowmaid `$S/runs/baseline/`.

## Commit rules
- Commit only when guard is green. Clear message: what/why, which report items (e.g. "Q1+M1"), which snapshots/gates changed and why.
- End every commit message with exactly these two lines (after a blank line):
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01JGDFJkgB3ttBJtcTtc6aL5
- Never push. Never rewrite history that is not your own work package's. Never mention model names in code/commits beyond that trailer.
