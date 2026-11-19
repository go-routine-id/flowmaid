# Merge of origin/dev (0.40.1) into the step-2 branch (after the first part of step 2 and the four block-mode fixes)

Workspace only, never committed. `R` = `/home/user/flowmaid`, branch `ccr-ccc6d89a-5cxd7e`. `M` =
`scratchpad/merge-a2/m` (my work files; the rest of `scratchpad/merge-a2/` is the independent checker's and was not
touched). `T` = `scratchpad/b12/tools-a2b`.

**Result.** One merge commit **`0f31509`** (parents `15b5383` = pre-merge head, `dbe37ec` = origin/dev), pushed
fast-forward to `ccr-ccc6d89a-5cxd7e` (`15b5383..0f31509`). origin/dev is an ancestor of the branch, so a PR
`ccr-ccc6d89a-5cxd7e -> dev` merges without conflicts. Author `rhyanz46`, the two trailer lines, no history rewritten.
**GUARD: ALL GREEN** on the committed tree (`M/guard1.txt`). Checkpoint A2 is not declared met (nothing here claims it).

## 1. Conflicts and resolutions

`git merge --no-ff origin/dev` (merge-base `7b0f29d`, the squashed step 1; dev gained `8da7ace`, `802410f`, `22c8174`,
`e9dc009`, `dbe37ec`).

| file | what | resolution |
|---|---|---|
| `tests/fixtures/identity.tsv` (the only textual conflict) | dev re-recorded 8 rows for 0.40.1 (route() hashes of `blocks/p12-state`, `p14-empty-and-self`, `ratio2x4`, `ratio3`, `ratio4`, `ratio4mask`, `notes/near-drag`; the route_partial hash of `blocks/turned-blocks`): their routed drawings reach past the top-left margin, which #76 now keeps instead of shifting. The branch added the rows of its two new fixtures (`blocks/nested-wide-title`, `blocks/twoway-noted`). | Re-recorded from a build of origin/dev (section 2): dev's table plus the two rows. |
| `src/scene.rs` (auto-merged) | dev: `Scene::origin`, `Scene::routed`, `try_route`, `Fitted`/`fit_top_left`, `top_left_canvas`, `view_shift`, routed viewBox, new tests. Branch: `Scene::block_layout`, `SceneCluster::bold_title`, block-mode frames in route()/route_partial(), title measures. | Both kept. All four `Scene { .. }` literals carry `origin`, `routed`, `block_layout`; `route_built` sets `block_layout: blocks` and dev's `origin: (0,0), routed: false`; `route_partial_with` keeps dev's `sc.origin = base.origin` after the branch's `keep_inner_room` (switch-on only); `try_route` wraps `route`, so the branch's block path is reached through it unchanged. |
| `src/compound.rs` | dev: `join` grows the canvas from each part's origin, new routed-canvas test. Branch: `block_layout: false` in `join` and `part_base`. | Both kept. |
| `src/ortho.rs` | dev: `right = sc.origin.0 + sc.width` (jog walls, `TitleSides::of`). Branch: `title_head_measured(.., right, inked)`, `axis_ends`, `TitleSide` 6-tuple. | Both kept: the branch's measure takes dev's right edge. |
| `src/edgepass.rs` | dev: `refit` sets `routed` and grows the routed canvas up/left. Branch: `Ctx::axis_ends`, block-layout tests. | Both kept (dev's early return for `Routed && static_fit` precedes the unchanged static path). |
| `src/architecture.rs`, `src/gitgraph.rs` | dev: `origin`, `routed`. Branch: `bold_title`, `block_layout`. | Both kept. |
| `CHANGELOG.md`, `README.md`, `docs/ROADMAP.md`, `Cargo.toml` (0.40.1), `Cargo.lock`, `src/class.rs`, `src/er.rs`, `src/mindmap.rs`, `src/parser.rs`, `tests/fallible_route.rs` | dev only | dev's text kept exactly; my doc edits on top (section 5). |

No source file was taken from one side wholesale. `cargo check --all-targets` clean after the merge (default build);
the suite-on build (`T/build.sh`) clean.

Read and left as is (test-only, harmless today): `src/blockcheck.rs` (branch, `#[cfg(test)]`) passes `sc.width` as the
canvas right edge to `title_head` / `fit_cluster_title` in four places. Since dev's `Scene::origin` the right edge of
a routed scene is `origin.0 + width`; the checks there run on laid-out scenes (origin 0), so nothing changes now.

## 2. Re-recorded references

- **`tests/fixtures/identity.tsv`**: recorded from a build of **origin/dev itself** (`dbe37ec`), never from the merged
  tree: scratch worktree `M/wt-dev` (removed), the branch's two new fixture files copied in, `FLOWMAID_RECORD_IDENTITY=1
  cargo test --lib identity` with its own target dir (removed). dev's `src/identity.rs` is byte-identical to the
  merged one (step 1 is in dev), so the class column is computed on dev too. **All 116 of dev's rows came out byte for
  byte as dev has them; the two new rows came out exactly as the branch had recorded them** (`M/identity-from-dev.tsv`).
  The merged table = dev's + 2 rows. On dev, with the new table, `identity::everything_renders_as_before_with_blocks_off`
  and `levels::tests::the_permanent_identity_rows_are_the_fixtures_outside_block_mode` pass; on the merged default
  build both pass too.
- Nothing else re-recorded. Dev did not touch `tests/quality.rs`; the branch's lines there merged without conflict.
  No ceiling, floor or baseline changed. The package's block rows did not move in block mode (section 4: every gate
  message and every stdout table identical apart from the one new test).

## 3. Default build: byte identity, merged tree vs origin/dev

Probe crates (`M/probe-dev`, `M/probe-mrg`, public API only, release, no cfg): per input FNV-1a of `render_svg` (or
`render_advance_text_svg`), and for every flowchart / state diagram of `to_svg` + `Debug` of `scene()`, `route()` at the
scene's centres, `route_partial()` with the first node dragged (7, 5), `route_partial()` with the middle node dragged
(-40, 23), and `route()` at that drag. The merged probe strips `, block_layout: false` **and `, bold_title: false`**
from the Debug text (both fields are the branch's; a `true` would still show).

Inputs: every `.mmd` under the scratchpad (outside `private/`) and the repository's `examples/` and `tests/`, deduplicated
by content: **29,512** (the 118 repository files under their repository paths).

| | all | without the local acceptance family |
|---|---|---|
| inputs | 29,512 | 29,400 |
| flowchart / state with geometry hashed | 9,734 | 9,624 |
| other diagram types rendered | 19,444 | 19,442 |
| fail to parse, alike on both | 334 | 334 |
| **differences** | **0** | **0** |
| panics | 0 | 0 |

`cmp M/id-dev.sorted.tsv M/id-mrg.tsv`: identical.

CLI (`flowmaid in.mmd -o out.svg` and `--compact 600`, plus `--advance-text` for the advance examples), release
builds of both: the 118 repository inputs, `cmp/corpus`, `flowcorpus` and 3,000 random inputs of the set above:
**3,160 inputs (3,074 without the local family), 6,323 runs, 0 SVG differences** (`M/cli-{dev,mrg}.tsv`, columns 1-4;
the stderr column differs only by the output path the CLI echoes). 65 inputs fail to parse on both with identical
messages; `b8r1/synth5000.mmd` hits the 120 s timeout on both.

The guard's own identity tests (`identity.rs`, `levels.rs`) pass on the merged default build.

## 4. Block mode after the merge vs before (the three mandatory checks)

Baseline re-run (the implementer's `T/cand-15b5383` was not trusted): `T/run-checks.sh M/base-15b5383 --commit 15b5383`
(fresh worktree + target, removed). It equals both `T/cand-15b5383` and the checker's `merge-a2/cand-15b5383` on all
three checks (no worse, no informational line). Candidate: `T/run-checks.sh M/cand-merge --tree R` on the merged tree
before the commit (BUILD-INFO shows `commit=15b5383 dirty_files=17`: the staged merge; the committed sources are the
same files). `T/compare.sh M/base-15b5383 M/cand-merge` -> `M/compare-merge-vs-15b5383.out`:

| check | result |
|---|---|
| 1 cells (live judge) | static 444/444 cells, must-fail 86 -> 86, worse 0, no informational line; routed 62/62, 37 -> 37, worse 0. **no worse** |
| 2 panics | tests 1227 / 1234 (7 new from dev), failing 14 -> 15. The 14 old failures: same tests, same message lines, same stdout tables. **WORSE (1): `scene::tests::drag_past_top_left_keeps_every_position` (dev's new test) fails in block mode.** The other 6 new dev tests pass in block mode (`class::..drag_past_top_left..`, `class::..stray_dot..`, `compound::a_routed_block_layout_canvas_holds_its_drawing`, `er::..drag_past_top_left..`, `scene::drag_inside_keeps_the_origin`, `scene::routed_offset_under_print_precision_is_written_as_before`). |
| 3 dirs (332 + 32, static and routed) | every verdict column identical (static defects 41, routed 169, x_drawn 1283 / 1997, ...), +0/-0 everywhere. **no worse** |

**Cause of the one WORSE line.** Panic: `Some("cart service") route: node 10 to (-60.0, 162.5): -28.8 -0.75`, from
`assert!((vy - (miny - MARGIN)).abs() <= 0.05)` (MARGIN = 28). In block mode `blocks/blocks-overview-lr` with
`cart service` dragged to x = -60 draws its top at y = -0.75, so the viewBox correctly starts at -28.75, which the SVG
writes as `-28.8` (0.1 px precision). The test's tolerance is exactly half that step; `|-28.8 - (-28.75)|` is
`0.05000000000000071` in f64, just over 0.05. Diagnostic in a scratch copy only (`M/dbg`, removed): with `0.05 + 1e-9`
in that test's three viewBox asserts, the whole test passes in block mode, i.e. every kept position, origin, far side,
drawn centre and pick it checks holds. It is a rounding-boundary flaw in dev's test that the block layout's geometry
happens to hit; not a drawing fault and not caused by the merge resolution. The test is dev's released code and was
not changed (no loosening). It passes in the default build (guard).

Also run, block mode, merged tree: `tests/fallible_route.rs` (dev's new integration test, not part of the three checks):
5/5 pass (`M/fallible-route-on.txt`).

## 5. Docs

- `docs/ROADMAP.md`: "Last updated 2026-11-18 (latest release: 0.40.1)". Step 2 row: first part merged behind the
  switch, which is off: frames sized to their titles, room for labels in the gaps between blocks, edge ends kept clear
  of frame titles; no drawing changes; drawing the frames tighter round their content waits until after step 3,
  because it depends on the paths step 3 draws under a drag. Step 3 row: "next". **Also** (beyond the table) the
  dense-diagram paragraph, whose present-tense routed number had gone stale: "After step 1 it drew fewer: 49 ... and
  after the first part of step 2, 42. Redrawn by `route()` ... it still has more, 107 against 76, until step 3."
  Measured on `tests/fixtures/quality/dense-td.mmd`, harness `x_drawn`: block mode 42 static / 107 routed
  (`cand-merge/bin/quality`); default build 66 / 76 (fresh `cargo build --example quality`). No internal codes, no
  `|` inside cells.
- `CHANGELOG.md` `[Unreleased]`: `### Internal` with one line: "Subgraphs as blocks, step 2 of 4, first part (frames
  sized to their titles, room for labels in the gaps between blocks), behind an internal switch that is off: no
  drawing changes."
- README unchanged.

## 6. Guard

`CARGO_BUILD_JOBS=2 nice -n 19 tools/guard.sh R` on the merged tree: fmt, clippy -D warnings, all tests, doctests,
MSRV 1.75, wasm, rustdoc: **GUARD: ALL GREEN** (`M/guard1.txt`). Commit message: no local-family name or number
(counts given without that family), no model name outside the trailer.

## 7. What worries me

1. **Disk.** 2.9 GB free on `/` after the guard (the repository's `target/` is about 12 GB). `run-checks.sh` refuses to
   build below 4 GB, so the next package's mandatory checks will be refused until space is freed (old artifacts in
   `R/target`, e.g. `cargo clean` there, or old scratch dirs). I removed everything I built (dev worktree, all scratch
   target dirs, the debug copy); `M` holds 143 MB of results and binaries.
2. **The new block-mode failure** (`drag_past_top_left_keeps_every_position`) enters the suite-on failing list (15 now).
   Step 3's baseline is this merge, so it will not show as new there, but it should not be forgotten: the fix belongs in
   dev's test (compare with the value as written, e.g. `view_origin(miny - MARGIN)`, or allow float noise above half a
   print step), which is a call for the lead / user because the rules forbid me to touch a test to make it pass. The
   same flaw can hit the default build on any drawing whose top or left lands on a .x5 boundary.
3. Debug-geometry probes comparing dev with this branch must strip `bold_title: false` as well as `block_layout:
   false` (my first merged probe run did not and showed spurious Debug differences; SVG hashes were equal all along).
4. The block-mode candidate was built from the staged working tree, not from a checkout of `0f31509`; the sources are
   the same files, but a verifier who wants a commit build should run `T/run-checks.sh <out> --commit 0f31509`.
5. Commit messages earlier on the branch may still carry verifier input names; the squash into dev and its PR text
   must be written clean, as at step 1 (`reports/reword-before-merge.md`).
6. The lead's autopush loop (`tools/autopush.sh`, backup branch `backup/b12a2-merge-wip`) was running during the merge
   and pushed working-tree snapshots there; the work branch itself I pushed by hand after the guard.
