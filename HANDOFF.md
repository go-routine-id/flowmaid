# Handoff — flowmaid mermaid-parity (state at 2026-10-31 15:50 WIB)

Read this first on Monday 2026-11-02 07:05 WIB (scheduled resume). The user writes Indonesian; reply in Indonesian.

## 1. Where things stand

| Ref | Commit | Content |
|---|---|---|
| `main` | a2a7dc7 (tag `v0.33.0`, published on crates.io 2026-10-31) | 0.33.0: ER, sequence, gitGraph, pie/journey/timeline, sankey/mindmap, architecture reworked; advance negotiated routing |
| `dev` | 8469531 | 0.33.0 + everything below (contains `main` and the tag) |
| `ccr-ccc6d89a-5cxd7e` (work branch) | 8469531 | identical to `dev` (fast-forwarded) — start phase B from here |
| `release/0.33.0` | 8af0e58 | done, merged |
| `backup/parity-workspace` | this branch | scratch workspace (plan, report, tools, references, this file). Never merge into the crate. |
| `backup/a1-class`, `backup/a1-class-wip`, `backup/a3-state` | — | obsolete (content is in dev); the session's git proxy refuses branch deletion (HTTP 403) → the user deletes them via the GitHub UI |

In `dev` but not yet released (→ 0.34.0): class (hierarchy, notes, «interface», generics, direction, hyphen/backtick names, two-way relations + lollipop), state (note cards, compact choice, fork/join), follow-ups (state note link static == route, `ClassNote::new`, any-depth mindmaps), interactive (edge picking = painted stroke, ~4.5× faster hit_test; marquee exact; finite routing for any node sizes), `bench -- files` example, clean rustdoc. CHANGELOG `[Unreleased]` documents all of it, including **Breaking** items.

## 2. Monday plan

1. `git fetch`; confirm work branch == `dev`; if the container is new, restore this workspace into the session scratchpad (see README.md here; mermaid: `npm i mermaid@12.2.0` into `mm/`; baseline renders in `baseline/`; tools expect `S=…/scratchpad`).
2. **Phase B (flowchart core)** — run `phaseB.draft.js` (12 packages B0…B11, serial in the main repo, each implement → independent verify → ≤2 fix rounds → push if pass, else park on `failed/<id>` and reset). Order in the script: B0 metrics, B1 arrows/stroke, B2 subgraph+bbox, B3 back-edge lanes, B4 polish, B5 theme option, B6 outline snap+wrap, B7 labels/ports, B8 layering, B9 compound direction, B10 all-orthogonal, B11 hops/comb.
   - Open user decision: if token budget is limited, reorder to B1 → B3 → B10 → B2 first (biggest visual impact). User has not answered — ask once at start, default to script order.
   - The user's token budget is limited ("Claude Ultra", ran low on Saturday): keep agents focused, report progress, stop cleanly if agents fail on quota.
3. **Phase C** (user's explicit request: "update readme, pasang screenshot perbandingan dengan mermaid serta performance benchmark"):
   - README: side-by-side mermaid vs flowmaid screenshots for ~8 representative diagrams (flowchart demo/advanced/back-edge, class, er, sequence, state, gitgraph, architecture…), small PNG/WebP under e.g. `docs/img/compare/`, generated from the FINAL code (render.sh / render3.js make mermaid|baseline|now; produce mermaid|flowmaid pairs).
   - Benchmark table: `tools/benchcmp.sh` → `runs/bench/compare.md` (flowmaid `bench -- files` vs mermaid 12.2.0 `mmbench.js` in headless Chromium), plus method notes and interactive numbers (drag/route_partial from perf.sh). First (pre-phase-B) table: `runs-bench/compare.md` — geomean 131× faster, SVG 1.5–24× smaller; dense flowcharts only 9–16× (bench-75 39 ms vs 355 ms) → watch phase B does not slow them.
   - Update the existing README "Performance" section (old synthetic table) rather than duplicating.
4. **Release 0.34.0**: `release/0.34.0` branch (Cargo.toml/Cargo.lock bump, CHANGELOG `[Unreleased]` → `[0.34.0] - <date>`, README dependency snippet) like 0.33.0 (8af0e58); then on the user's request PR → dev/main. User tags + `cargo publish` (no crates.io token here).

## 3. Invariants (PLAN.md has the full list)
- Never lose the interactive features (scene/route/route_partial/hit_test/edge_at/hits_in_rect/fold/to_mermaid/advance); drag budget per file: drag_ms/route_ms ≤ max(1.25× baseline, baseline + 1 ms) vs `baseline/perf.tsv`; route_partial < 6 ms; static scene_ms ≤ 2× baseline. Current headroom is tight on bench-75 (worst drag 20–23 ms vs 25.3 limit).
- Painter-only effects (arrow trims, hops, gaps) stay out of SceneEdge geometry; geometry changes go into scene AND route; pick == render (use `scene::edge_path_d` / `edge_stroke_width`, see 0b63338).
- Zero deps, MSRV 1.75 (0 warnings), wasm32, determinism, `cargo doc -D warnings` clean (fixed 70d09f7 — keep it clean), semantic palette stays default (themes only as option).

## 4. Known non-blocking issues to fold into phase B (from verifiers)
- State note links: elbow ends can differ 7–18 px between static and route (port spreading differs) → fix with B7 (ports); one remaining nested-composite case (verify/fu/v2/dump/g26.mmd) where static elbows and route refuses.
- Class: plain 6-source fan-in has 4 crossings (unlabelled gap one jog lane short for even counts); 5+ self-loops beside a close sibling run under it; `A.B` namespace names unsupported.
- State: twins3 'two' chip half over 'three' stroke; wide LR choice chip touches c->A start.
- Journey/timeline: greedy wrap leaves orphan words; `wrap_text` ignores `<b>/<i>` across wraps.
- gitGraph: merge into a farther free lane drawn like a fork; `order: 0` tie rule differs from mermaid.
- ER: LR duplicate relation pair draws an X; 'employs' label near neighbouring line in cycles.
- Pie 17+ slices colour wrap; journey 11+ actors colour repeat.

## 5. Process rules the user expects
- Push only to `ccr-ccc6d89a-5cxd7e`; other branches (backup/release) only with permission. Don't push unverified commits to the work branch (stop hook will nag — explain, push after verification).
- Create PRs only when asked; when the user says "merge ke dev": PR work branch → dev, wait for green CI, merge commit with expectedHeadSha, unsubscribe.
- Commit trailer: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` + `Claude-Session: https://claude.ai/code/session_01JGDFJkgB3ttBJtcTtc6aL5`. No model names elsewhere.
- Pending offers to the user: review/help merge contributor PR #78 (JSON nesting depth cap, targets main).
