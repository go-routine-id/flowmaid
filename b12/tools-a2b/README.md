# tools-a2b: the three mandatory checks, baseline B against candidate C

`T` = `/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/tools-a2b`,
`B12` = its parent directory.

The skeptic's review (`B12/design-a2b/critique.md`, section 4 step 0 and section 5) found that three of five studies
hid regressions because they compared must-fail line counts and failing test names. From then on every step of
b12a2b is judged by three checks, each one command, against a stored baseline:

1. **cells**: per-cell no-worse on the 95 local inputs, judged by the LIVE judge;
2. **panics**: the messages of every suite-on gate, not only which tests fail;
3. **dirs**: the 332 + 32 direction copies, static and routed, copy by copy.

The scripts are the skeptic's (`B12/design-a2b/critic/`, originals kept in `T/orig-critic/`), made to take
arguments instead of fixed paths and to use the live judge in `B12/metrics` (the corrected bends counter, installed
2026-11-17; see `B12/metrics/README.md`, "Bends fix"). The baseline for 7122a53 is `T/base-7122a53/`.

## Baseline B against candidate C

The candidate is a commit:

```sh
T=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/tools-a2b
$T/run-checks.sh $T/cand-<sha> --commit <sha>     # scratch worktree + build + the three checks, then removes both
$T/compare.sh $T/base-7122a53 $T/cand-<sha>        # the report; exit 0 only when all three say "no worse"
```

The candidate is a working tree with uncommitted changes:

```sh
$T/run-checks.sh $T/cand-wip --tree /path/to/checkout   # builds that tree as it stands (it is not touched)
$T/compare.sh $T/base-7122a53 $T/cand-wip
```

The candidate is binaries you already built with `T/build.sh <src-tree> <target-dir> <bindir>`:

```sh
$T/run-checks.sh $T/cand-x --bin <bindir> [--src <src-tree>]   # the gate tests need the tree they were built from
```

A new baseline, for example once a step has landed and becomes the base of the next:

```sh
$T/run-checks.sh $T/base-<sha> --commit <sha>
```

Options: `--only local,gates,dirs` runs a subset (compare.sh then reports the missing check as WORSE: it cannot
read it); `--keep` keeps the scratch worktree and target dir (`T/scratch/wt-<sha>`, `T/scratch/target-<sha>`);
`--repo <dir>` builds from another repository (default `/home/user/flowmaid`). `CARGO_BUILD_JOBS` defaults to 2,
`SCRATCH` to `T/scratch`. run-checks.sh refuses to build with less than 4 GB free on `/`.

`--commit` runs `git -C /home/user/flowmaid worktree add --detach T/scratch/wt-<sha> <sha>` and removes the worktree
(`git worktree remove --force`, `git worktree prune`) at the end: the repository's own checkout, branch and index are
never touched.

After the live judge changes (a new counter, new tables or thresholds), re-judge the stored runs before comparing.
No rebuild is needed, because the renders are stored:

```sh
$T/rejudge.sh $T/base-7122a53 && $T/rejudge.sh $T/cand-<sha> && $T/compare.sh $T/base-7122a53 $T/cand-<sha>
```

The checks one by one (the same scripts that run-checks.sh and compare.sh call):

| check | run (writes into `<outdir>`) | compare |
|---|---|---|
| 1 cells | `fmrun.sh <bindir> <outdir>` (renders, then `rejudge.sh <outdir>`) | `python3 cells.py <B> <C>` |
| 2 panics | `trun.sh <bindir> <outdir> [<src-tree>]` | `python3 panics.py <B> <C> [test ...]` |
| 3 dirs | `qrun.sh <bindir> <outdir>` | `python3 qcmp.py <B> <C>` |

Every compare script takes `--quiet` (only counts and the worse lines) and exits 1 when something is worse.

## Expected run time and disk

Measured on this machine for 7122a53:

| step | time |
|---|---|
| build (`build.sh`, fresh target dir), CARGO_BUILD_JOBS=3 | 140 s (about 3.5 min at 2 jobs) |
| check 1: render 95 inputs + `quality --svg` + measure + judge | 15-17 s |
| check 2: lib tests (1193) + tests/quality.rs (15), 4 threads | 35 s |
| check 3: `quality` over 332 + 32 copies | 15 s |
| `compare.sh` | under 5 s |

All in, `run-checks.sh --commit` took 210 s at 3 jobs (expect about 4.5 minutes at the default 2). The target dir is about 60 MB and is removed. A run
directory holds about 36 MB (the binaries included).

## What each check checks, and what "no worse" means

### 1. cells: the local inputs, cell by cell (`cells.py`)

**Inputs.** `B12/local/in/*.mmd` (95: the analog family, the public fixture and its permutations, the block
fixtures, the dense corpus and the t9 BT copies) are drawn by the candidate's CLI (static) and by its `quality --svg`
(routed twins). They are measured by the live `B12/metrics/blockmetrics` and judged by `B12/local/run.sh judge`
(`--due A2`, `--due A1`, `--due A2 --all`) and `judge-routed --all`, with the live reference tables and
`thresholds*.json`.

**Cells.** A cell is one rule line of the judge: input, metric and rule. The static table holds every rule due by
A2 (444 cells); the routed table holds the routed rules (62 cells).

**Verdict.** "No worse" means every cell that is a MUST FAIL in either run is in one of these classes:

| class | meaning | counts as |
|---|---|---|
| NEW | must-fail in C, not in B | worse |
| WORSE | must-fail in both, and the value moved further to the wrong side of its bound | worse |
| CHANGED | must-fail in both, and a string value (`bands`) or NA changed | worse |
| MISSING | a cell of B that C's judge did not print | worse |
| GONE | must-fail in B only | better |
| BETTER | must-fail in both, and the value moved toward its bound | better |

So "89 -> 85 must-fail lines" with one cell getting worse inside a failing line is WORSE.

**Informational, not part of the verdict.** Target-missed lines that are new, gone or worse; and "drift": passing
cells whose value moved the wrong way while staying within the must.

**Output.** One line per table, then the worse cells and, without `--quiet`, the better and informational ones:

```
[local static A2] 444 / 444 cells; must-fail 89 -> 89; worse 0 (new 0, worse inside failing 0, ...)
[local routed] 62 / 62 cells; must-fail 37 -> 37; worse 0 ...
cells: no worse
```

### 2. panics: suite-on gate messages (`trun.sh`, `panics.py`)

**What runs.** `trun.sh` runs the suite-on (`--cfg flowmaid_blocks`) lib tests and `tests/quality.rs` with
`--test-threads=4 --show-output` from the source tree, using the test binaries that build.sh copied.

**What is compared, per test.**
- **Status.** A test that newly fails is worse. A baseline test that C no longer runs is MISSING, which is also
  worse. A new test that fails counts as newly failing. A test that newly passes is better.
- **Message lines.** This applies to every test that fails in both runs: `quality_gate` (per fixture and mode,
  `x > ceiling` / `< floor`), `defect_baseline_gate` (static and routed fields over their ceilings),
  `block_fixtures_are_not_drawn_worse` (per block fixture and mode), `a_one_px_drag_leaves_unrelated_edges_put`
  (drag jumps, per drag), `channels_keep_edges_apart_and_off_chips`, `crossing_gaps_do_not_read_as_dashes` and the
  failing lib tests.
  - Each panic message is compared line by line, after the thread ids, source line numbers and backtraces are
    stripped.
  - A line found only in C is NEW, which is worse: a new violation row. An example is
    `blocks/nested-cases static: defects = 1 > ceiling 0`.
  - When a line of C matches a line of B apart from its numbers, the two are a pair. The pair is WORSE when the
    first number that differs grew, or shrank on a `< floor` line; otherwise it is better. An example is
    `routed lat_backtrack 3856.27 > ceiling 425` becoming `4753.93 > ceiling 425`.
  - Names that contain digits, such as `bench-75` or `p01-iso`, stay text.
- **Stdout (informational).** The captured stdout of every test that printed one in both runs is compared cell by
  cell. This covers `quality_gate`'s full table (file, mode, column), `defect_baseline_gate`'s field list (including
  the fields still under their ceilings) and the blockcheck sweeps. Timing columns are skipped.

**Verdict.** "No worse" means no test newly fails or is missing, and no message line is NEW or WORSE.

**Output.** The header line `[gates] tests 1208 / 1208; failing 14 -> 14` is followed by the worse lines and ends
with `panics: no worse` or `panics: WORSE (n)`.

### 3. dirs: the direction copies (`qrun.sh`, `qcmp.py`)

**What runs.** `examples/quality` (suite-on) over two sets of copies, frozen in `T/inputs/`:
- `inputs/dirs`: 332 copies, every flowchart and state fixture of `tests/fixtures/{blocks,quality,notes}` drawn TD,
  BT, LR and RL. They are generated by `B12/work/a2i/tools/mkdirs.py` and checked equal to a fresh generation from
  7122a53's fixtures.
- `inputs/dirsx`: 32 copies, the 8 examples with subgraphs in the four directions.

Each copy gives a static and a routed row with the harness's full column set.

**Verdict columns** (the skeptic's set, critique.md 1.3), compared per copy and mode:
- `defects`, `x_drawn`, `lateral_x`, `fan_x`, `edge_chip`, `side_ends`: a copy is worse when its value grows;
- `min_edge_gap`: a copy is worse when it drops by more than 0.5 px;
- a copy and mode that C no longer measures is MISSING, which is worse.

**Verdict.** "No worse" means no copy is worse in any verdict column, in either mode, in either set.

**Informational.** For every other harness column the output gives the total and how many copies went up or down,
with the first worse copies. Higher is worse, except `min_node_gap` and `chip_gap_min`. Descriptive columns
(`nodes`, `width`, `ink`, the idiom counts and similar) carry no direction. Timings are skipped.

**Output.** One line per set and mode, for example:

```
[dirs routed] 332 copies  defects 169->169 (+0/-0)  x_drawn 1997->1997 (+0/-0)  lateral_x 418->418 (+0/-0)
  fan_x 183->183 (+0/-0)  edge_chip 78->78 (+0/-0)  side_ends 361->361 (+0/-0)  min_edge_gap down 0
```

(`+n/-m` = copies worse / better.) Each line is followed by `WORSE <column> (n): <copy> a->b; ...` lists, and the
output ends with `dirs: no worse` or `dirs: WORSE (n)`.

### The overall verdict

`compare.sh` ends with:

```
VERDICT cells:  no worse
VERDICT panics: no worse
VERDICT dirs:   no worse
VERDICT: no worse
```

and writes the whole report to `<C>/compare-vs-<basename of B>.txt`.

- **"VERDICT: no worse"** (exit 0) is the bar: C is no worse than B on any judged cell, gate message or direction
  copy.
- **Any WORSE line** must be fixed, or taken to the lead as an explicit decision; it must never be reported as a net
  gain. Improvements show as GONE, BETTER, `newly passing`, `- gone` and `-m` counts.

## The baseline `T/base-7122a53/`

7122a53 (`fix(blocks): steps whose rooms do not meet ...`, the head of b12a2-geometry), built suite-on in the scratch
worktree `T/wt` with `CARGO_TARGET_DIR=T/target` and CARGO_BUILD_JOBS=3. Worktree and target dir are removed.

| part | contents | cross-check |
|---|---|---|
| `bin/` | `flowmaid`, `quality`, `lib-tests`, `quality-tests`, `BUILD-INFO`, `build.log` | |
| `local/` | `svg/` (95), `q/` (95 static + 95 routed SVGs), `metrics*.tsv`, `judge-A2.txt` (89 must-fail), `judge-A1.txt` (0), `judge-A2-all.txt`, `judge-routed-all.txt` (37 must-fail) | the 95 SVGs are byte-equal to `B12/local/out/fr4-split`; `judge-A2.txt` / `judge-A1.txt` are byte-equal to `B12/local/out/fr4-split-bendsfix/`, which is byte-equal to `B12/bendsfix-work/dry-fr4-split/judge-A2.txt` and `B12/design-a2b/bends/data/judge-corrected-A2.txt` |
| `gates/` | `lib.txt`, `quality.txt`, `failing.txt` (14: 8 lib, 6 quality), `results.txt` | `failing.txt` equals `B12/work/a2i/ton-split.fail`; panics.py against `ton-split.txt`: no message differs |
| `dirs/` | `dirs.tsv`, `dirsx.tsv` (full tables) | the 7 columns of the skeptic's qrun equal `B12/work/a2i/qd-split.tsv` on all 332 copies x 2 modes (static defects 41, routed 169, routed fan_x 183, ...) |
| `SUMMARY.txt`, `RUN-INFO` | the numbers above; what ran and how long | |

Two runs of each check on the same binaries give identical reports, with no informational line either.

**Re-running the baseline.** `bin/flowmaid` and `bin/quality` run anywhere, so `fmrun.sh` and `qrun.sh` work at any
time. `bin/lib-tests` and `bin/quality-tests` read their fixtures from `T/wt` (CARGO_MANIFEST_DIR is compiled in).
To re-run the baseline's gates, re-create that worktree, run the tests, then remove it:
`git -C /home/user/flowmaid worktree add --detach T/wt 7122a53`, then `T/trun.sh T/base-7122a53/bin T/base-7122a53`,
then `git -C /home/user/flowmaid worktree remove --force T/wt`. Alternatively, run
`run-checks.sh T/base-7122a53 --commit 7122a53` again; it rebuilds.

**Checked once.** `run-checks.sh --commit 7122a53` (a second, independent build) compared against this baseline
gives "VERDICT: no worse" with no informational line, in 210 s at CARGO_BUILD_JOBS=3, and removes its worktree and
target dir (`T/validation/selftest-commit.log`, `selftest-commit-vs-base.txt`).

**Known catches.** The checks catch the regressions the skeptic found (reports in `T/validation/`):
- the squeeze prototype's renders (`design-a2b/critic/svg/S`) give 12 NEW and 22 WORSE cells under the live judge,
  including analog `lbl_crowd` 4 -> 8, `lateral_x` 1 -> 2/3, `bends_max` 6 -> 8/9 and public `bends_max` 4 -> 6;
- its test log (`critic/t/S.txt`) gives 2 newly failing tests and 11 worse gate-message lines, including
  `defect_baseline` routed `lat_backtrack` 3856 -> 4754 and `quality_gate` dense-td routed `edge_node_pairs` 1 -> 2.

## Not covered here

- **Timing and perf**: route_ms, scene_ms and the drag benchmark. The `dfr_own` / `dfr_far` columns need `--drag`
  and are 0 here.
- **The bulk corpora** (`bulkst`, `bulkfc`, `all`, `stress`) of the regress study.
- **Diagrams without subgraphs**, which must stay byte-identical.
- **Pictures.**

Those stay the implementer's and the verifier's own duty, as in the spec.

## Files

| file | what |
|---|---|
| `run-checks.sh` | the driver: build (commit / tree) or take binaries, then the three checks into `<outdir>` |
| `compare.sh` | B against C: the three compares and the verdict |
| `build.sh` | `<src-tree> <target-dir> <bindir>`: suite-on release build of the CLI, `examples/quality` and the two test binaries |
| `fmrun.sh`, `rejudge.sh`, `cells.py` | check 1 |
| `trun.sh`, `panics.py` | check 2 |
| `qrun.sh`, `qcmp.py`, `inputs/` | check 3 |
| `orig-critic/` | the skeptic's scripts as they were (hard-coded paths, old judge), and `qcol.sh` |
| `base-7122a53/` | the baseline |
| `validation/` | the checks run on known cases: the squeeze prototype (cells, panics), injected changes (dirs), `ton-split.txt` (panics) |
| `scratch/` | worktrees, target dirs and test runs; safe to delete when no run is active |
