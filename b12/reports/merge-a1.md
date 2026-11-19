# Merge of origin/dev into the step-1 branch (after checkpoint A1)

Workspace only, never committed. `R` = `/home/user/flowmaid`, branch `ccr-ccc6d89a-5cxd7e`. `M` =
`scratchpad/merge-a1` (work files: `inputs.txt`, `id-*.tsv`, `h/`, `ton-full.txt`, `sweep.txt`, `guard1.txt`,
`perf-*.tsv`, `rec/`, `msg.txt`; all build dirs and both worktrees removed).

**Result.** One merge commit `b5557e0` (parents `10e336d`, `f9aa9c0` = origin/dev), pushed fast-forward to
`ccr-ccc6d89a-5cxd7e` (`10e336d..b5557e0`). origin/dev is an ancestor of the branch, so a PR
`ccr-ccc6d89a-5cxd7e -> dev` squash-merges without conflicts. Author `rhyanz46`, the two trailer lines, no history
rewritten. Guard `GUARD: ALL GREEN` on the merged tree (`M/guard1.txt`).

## 1. Conflicts and how each was resolved

| file | what | resolution |
|---|---|---|
| `src/scene.rs` (textual conflict, `route_labelled`) | dev split `route_labelled` into `route_labelled` + `route_fitted` (bars fitted once; `compound::route` routes the rest of a split graph with `route_fitted`, and `route_partial_with`'s two fallbacks call `route_fitted` with the sizes it fitted). The package had added the test-only `levels::note_used(Entry::Route, ..)` just before `route_built`. | Dev's split kept verbatim. The `#[cfg(test)] note_used(Entry::Route, ..)` call moved to the top of `route_fitted`. Reason: the rest of a split graph now reaches `route_fitted` without `route_labelled`; with the record left in `route_labelled` it would be lost for the rest and `levels::tests` ("route() decides as scene() does", part by part) would see a part missing. In `route_fitted` every part `route()` draws is recorded once, exactly as before (blocks via `route_labelled` -> `route_fitted`, rest directly, non-split graphs via `route_labelled`). The partial fallbacks record `Entry::Route` as they did through `route_labelled`. |
| `src/levels.rs` (compile error after the merge) | dev derives `PartialEq` for `Graph` (and Node, Edge, Subgraph, SubEdge); the package's `Graph::block_links: Option<levels::BlockLinks>` had no `PartialEq`. | `BlockLinks` derives `PartialEq` (two `Vec<bool>`). Graph equality then includes the block links, which matches dev's doc ("equal graphs draw the same"). |
| `Cargo.toml` | dev: version 0.40.0; package: `[lints.rust] unexpected_cfgs` check-cfg for `flowmaid_blocks` | auto-merged, both kept |
| `src/compound.rs` | dev: bars fitted to each block's static frame (`bar_branches`, `GivenBox`); package: `any_part` | auto-merged, both kept; nothing in `any_part` touches dev's code |
| `src/layout.rs` | dev: retired `wrap_text`/`lines_at` and its test; package: the block layering/ordering | auto-merged; the package never used `wrap_text` |
| `src/model.rs` | dev: `PartialEq` derives and doc; package: `block_links` field | auto-merged (see levels.rs) |
| `src/scene.rs` (rest) | dev: `plain_crossings`, route_partial bar sizes, `bar_spans`/`bar_branches`, new tests | auto-merged |

Sync notes: the package adds no `Scene { .. }` literal (every literal in `src/` already has `plain_crossings`); both
`cargo check --all-targets` with the switch off and with `--cfg flowmaid_blocks` print no warning or error. CHANGELOG
entry under `[Unreleased]`. README unchanged (the merge did not need it).

No source file was taken "ours" or "theirs" wholesale.

## 2. Re-recorded references

- **`tests/fixtures/identity.tsv`** was recorded again from a build of **origin/dev itself** (`f9aa9c0`): a scratch
  worktree `M/wt/dev` with the package's `src/identity.rs` ported (`with_blocks(false, table)` -> `table()`, since dev
  has no switch; the last column, the block-mode class, cannot be computed on dev and was carried over from the
  recorded table, and the merged tree's own test checks it against its predicate) and the package's
  `tests/fixtures/blocks/` copied in, then `FLOWMAID_RECORD_IDENTITY=1 cargo test --lib identity`
  (`M/rec/identity-from-dev.tsv`, `M/rec/identity-port.rs`). **All 116 rows came out byte for byte as recorded at
  `3abbfeb`**: dev's 0.39.0/0.40.0 changes draw nothing different on any example or fixture. Only the table's header
  comment and the module doc of `src/identity.rs` change (they now say the table is re-recorded from dev's build at a
  merge, never from the merged tree). The merged default build passes it (`identity::everything_renders_as_before_with_blocks_off`,
  `levels::tests::the_permanent_identity_rows_are_the_fixtures_outside_block_mode`).
- **`BLOCK_CEILINGS`** (default-build ceilings recorded at the start): not touched; the default output of every fixture
  is unchanged, and `block_fixtures_are_not_drawn_worse` passes in the default build (guard). `blocks/turned-blocks`
  stays `(0, 0, 0, 4, 0), (0, 0, 0, 5, 0)`.
- Dev's `DEFECT_BASELINE`, `GAP_FLOORS`, `CEILINGS` etc.: kept as dev has them (the package's lines in
  `tests/quality.rs` merged without conflict).

## 3. Byte identity, default build (origin/dev `f9aa9c0` vs merged `b5557e0`)

Probe crate (`M/probe-dev`, `M/probe-mrg`, public API only): per input the FNV-1a hash of `render_svg` (or
`render_advance_text_svg`), and for a flowchart / state diagram of `to_svg(scene())`, `Debug(scene())`,
`to_svg(route())` at the scene's centres, `Debug(route())`, `to_svg(route_partial())` with the first node dragged
(7, 5), `Debug(route_partial())`.

| set | inputs | flowchart / state (geometry hashed) |
|---|---|---|
| `R/tests/fixtures/**`, `R/examples/*.mmd` | 116 | 104 |
| render.sh corpus `cmp/corpus` | 25 | |
| perf corpus `flowcorpus` | 17 | |
| stress inputs (verifier `xin`, `shuf`) | 11 | |
| generated bulk sets (`b8r1/bulkfc`, `bulkst`, `bulkcl`, `b8b/bulkin`, `verify/c-leftovers/sweep2`) | 11,926 | |
| local acceptance inputs | 92 | 92 |
| **all** | **12,187** | **2,412** |

**Differences: 0** (`cmp id-dev.tsv id-mrg.tsv`: identical; 0 panics; 142 inputs fail to parse on both alike).
CLI (`flowmaid in.mmd -o out.svg`, both release builds) over the 169 inputs of fixtures, examples, the render corpus,
the perf corpus and the stress inputs: 167 SVGs byte-identical, 2 advance-DSL examples fail identically on both (the
CLI does not take that DSL). The commit message gives the counts without the local family (12,095 / 2,320).

## 4. Block mode after the merge (`--cfg flowmaid_blocks`), before = `10e336d` built fresh

Everything is unchanged. Exact comparisons, both builds from source today (10e336d in `M/wt/a1`):

- **Probe in block mode** over the same 12,187 inputs: every flowchart / state input (2,412) draws the same
  `scene()`, `route()` and `route_partial()` SVG and geometry (Debug text compared without the `plain_crossings` field
  0.39.0 added). The 64 inputs that differ are class diagrams (`bulkcl` 46, c-leftovers `sweep2` 18): dev's 0.40.0
  class changes; on each the block-mode merged output equals origin/dev's.
- **Harness tables** (`examples/quality`, static + routed rows) on `tests/fixtures/{quality,blocks,notes,fans}`,
  `examples/`, the stress inputs and the local family: **258 rows, 0 differ** (timing columns ignored; `M/h/`).
- **Local judge `--due A1`**: 155 checks on 92 diagrams, **0 must-fail, 7 targets missed** (crossing targets, as A1);
  `metrics.tsv` identical to A1's own run (`local/out/r9-p4`) and to today's 10e336d build on all 92 rows (engine label
  aside). Crossings: analog 15, v2 14, perms 15 18 15 15 17, public fixture 6 (bt 7), bench-75 204, dense-lr 17,
  dense-td 49; breadths 3130 3119 3130 3153 3130 3130 3180 / 1142; `xframe_runs` 0 everywhere.
- **Section 7 rows of checkpoint-A1.md**, after (= before): analog 0 / 15 / 0 / 3130; v2 0 / 14 / 0 / 3119; perm-a, c, d
  0 / 15 / 0 / 3130; perm-b 0 / 18 / 0 / 3153; perm-e 0 / 17 / 0 / 3180; public fixture 0 / 6 (bt 7) / 0 / 1142;
  bench-75 0 / 204 / `fan_x` 4; dense-lr 0 / 17 / 0; dense-td 0 / 49 / 0.
- **nested-chain** (repository fixture, static): x_drawn 0, `lateral_x` 0, `fan_x` 0, defects 0.
- **Sweep** `blockcheck::a_generated_sweep_keeps_the_block_invariants --nocapture` (`M/sweep.txt`): passes; 200
  diagrams in block mode, 52 with a lane level, hard zeros before | as blocks 55 | 29 diagrams, 115 | 45 in all, blocks
  add hard zeros on the same 8 diagrams (flow 23 RL 0 -> 1; state 3, 4, 12, 13, 25, 27, 33 0 -> 2). Same as A1.
- No gate row moved, so nothing got worse than A1 or than the start. B11's hops and the c-leftovers changes moved no
  number: B11 was already in `3abbfeb`, and c-leftovers only changes class diagrams and bars of state composites laid
  out in their own direction under a drag, which no measured input reaches in a way that changes its drawing.

### Suite-on `cargo test --no-fail-fast` (`M/ton-full.txt`)

lib: **8 failed** (1,165 passed; A1: 1,152 passed, the 13 more are dev's new tests): `comb::tests::a_hubs_fan_is_a_comb_in_scene_and_route`,
`ports::tests::a_nearly_straight_pair_is_drawn_straight`, `render::tests::advanced_showcase_parses_and_renders`,
`scene::tests::a_composite_edge_to_its_own_state_drops_straight_from_the_border`,
`scene::tests::a_lane_stays_in_its_frames_in_route_and_on_a_drag_too`,
`scene::tests::far_apart_back_edges_and_frame_legs_square_up_alike_in_route`,
`scene::tests::note_links_hold_their_ends_and_corners_in_scene_route_and_drag`,
`scene::tests::note_links_match_between_scene_and_route`.
`tests/quality.rs`: **7 failed**: `a_one_px_drag_leaves_unrelated_edges_put`, `channels_keep_edges_apart_and_off_chips`,
`crossing_gaps_do_not_read_as_dashes`, `block_fixtures_are_not_drawn_worse`, `defect_baseline_gate`,
`edges_keep_off_frame_borders`, `quality_gate`. **The same names as A1, and the failure texts are identical** (diffed
`M/fail-a1.txt` against `M/fail-mrg.txt`: only thread ids and line numbers shifted by dev's added code). Every other
test binary passes.

## 5. Docs

- `docs/ROADMAP.md`: step 1 row "merged behind the switch, which is off: no drawing changes"; the "Open question"
  paragraph rewritten as answered, from the repository fixture `tests/fixtures/quality/dense-td.mmd` (harness,
  static): **49 drawn crossings in block mode against 66 in the default build** (default `fan_x` 1, block 0). It also
  says plainly that `route()` in block mode still draws more on it (**113 against 76**, routed row) until step 3.
  "Last updated 2026-11-16 (latest release: 0.40.0)". No internal names. The docs site renders the file with its own
  small Markdown reader; the new text uses only `code` and **bold**, no `|` inside table cells.
- `CHANGELOG.md` `[Unreleased]`: new `### Internal` with one line (subgraphs as blocks, step 1 of 4, behind an internal
  switch that is off: no drawing changes). No version written.
- README unchanged.

## 6. Guard and perf

- Guard (fmt, clippy -D warnings, tests, doctests, MSRV 1.75, wasm, rustdoc): **GUARD: ALL GREEN** on the tree that was
  committed (`M/guard1.txt`).
- `tools/perf.sh merge-a1` (default build, against the old `runs/baseline`): every file inside the limits A1 quotes
  except **dense-td `route_ms` 9.65 (limit 9.20)**. Noise, not the merge: the merged default code draws byte for byte
  what dev draws, and timed alternately (`M/perf-*.tsv`, two runs each) dev gives dense-td route 6.82 / **10.55** and
  the merged build 6.98 / 6.83; drag 13.86 / 13.33 (dev) and 13.56 / 16.55 (merged), limit 17.80. Other numbers of
  that run: bench-75 drag 20.53 (25.28), route 10.06 (15.76); dense-lr drag 11.12 (17.76), route 4.60 (8.03);
  fanout-frames drag 2.46 (3.44). One alternating run showed a bench-75 scene spike (59 ms merged, 38 ms in the other
  run and 38-41 ms on dev): the box, load about 1.6 while the run started.

## 7. What worries me (small)

- The class column of `identity.tsv` cannot be re-derived on dev (dev has no block-mode predicate); it was carried over
  and is checked only by the merged tree's own tests. That is what the column is for, but it is not "from dev".
- The Debug-geometry hash of the block-mode probe differs between 10e336d and the merge on every flowchart / state
  input until the new `plain_crossings` field is stripped from the text; with it stripped there is no difference. Any
  future probe that hashes `Debug(Scene)` across this merge must strip it too.
- The roadmap now prints the routed dense-td number (113 against 76). It is true and the page says step 3 builds it,
  but whoever reviews the PR may prefer the page to quote the static drawing only.
- Commit messages `f1879b1` and `32a3ebf` on this branch still carry local-family numbers (D5); they disappear with the
  planned squash into `dev`. The PR text and squash message must be written clean (`reports/reword-before-merge.md`).
- The perf run's dense-td `route_ms` over its limit: shown above to be load noise, but it was one run of perf.sh; the
  verifier may want a second clean run on a quiet box.
