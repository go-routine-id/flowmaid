# flowmaid: resuming on Monday

> **Ringkasan (ID):** Fase B berhenti sementara setelah B8 atas permintaan user, lalu dirilis. Senin lanjut dari **B9** (compound layout), B10, B11, lalu c-leftovers. Semua bahan kerja ada di branch `backup/parity-workspace` (folder ini). Ikuti §3 kalau container baru, §2 kalau container lama masih hidup. User harus memberi aba-aba sebelum workflow jalan lagi.

Written 2026-11-04 by the session that ran phase B (session `cac93b13…`, claude.ai session `session_01JGDFJkgB3ttBJtcTtc6aL5`). §0 gets updated once B8 and the release are done.

## 0. State at pause (update when final)
| Thing | Value |
|---|---|
| Last release | **0.34.0** on `main` = `dd7c591` (PR #92). B0–B7, contributor JSON fix #78, hub-label perf fix `9c62b27`. Tag `v0.34.0` and `cargo publish` are done by the user (see §5). |
| Next release | **0.35.0** = 0.34.0 + B8 (layering). Planned right after B8 passes (see §0a). |
| `dev` | `9a78178` (B0–B7) until B8 merges. |
| Work branch | `ccr-ccc6d89a-5cxd7e`. B8 is in progress (verify#0 failed, fix#1 running at the time of writing). |
| Phase B workflow | Run `wf_513402ce-c52`, script `phaseB2.js`, task `wsgxt13tf`. It is stopped deliberately after B8 is pushed, on the user's request. |
| Hourly merge task | `trig_019VQH3TYG4ijXR8niHxK13z`. Disabled over the weekend; re-enable or recreate it on Monday (§4). |
| Backups | `backup/parity-workspace` (this folder), `backup/b8-layering-wip` (B8 WIP snapshot). The obsolete `backup/a1-class`, `backup/a1-class-wip` and `backup/a3-state` can be deleted by the user. |

### 0a. Pause procedure, as planned
1. B8 passes → the workflow pushes the work branch → stop the workflow before B9 edits anything.
2. PR work branch → `dev` (B8), wait for green CI, merge.
3. `release/0.35.0` from `main`, then merge `dev` into it.
   - In CHANGELOG, `[0.35.0]` holds only the B8 entries, above `[0.34.0]`. Take main's CHANGELOG and add the entries from `git diff d50c525..<B8 head> -- CHANGELOG.md`.
   - Keep the hub-label perf fix `9c62b27` when resolving `src/layout.rs`.
   - Bump the version and the README snippet, and add Highlights.
   - Refresh the README compare PNGs (`compare/pair.js`), and the benchmark if needed.
   - Run guard, open a PR to `main`, merge when CI is green.
4. PR `main` → `dev`, merge, then fast-forward the work branch to it. Monday's B9 then starts on everything, including the perf fix.
5. Disable the hourly task. Update §0 and HANDOFF.md, push this backup.

## 1. User rules (non-negotiable)
- Quality bar: *"yang penting kualitasnya tidak main-main dan premium"*. Go step by step and never skip a package (*"jangan loncat"*). A package passes only after independent verification, with up to 4 fix rounds; otherwise stop for review. Rare edge cases still block.
- After every passing package: push the work branch, open a PR to `dev`, wait for green CI (test, msrv, wasm), merge with a merge commit and `expectedHeadSha`.
- Push only to `ccr-ccc6d89a-5cxd7e`. Other branches (release/*, backup/*) need the user's explicit permission. Granted so far: release/0.34.0, release/0.35.0, backup/*.
- Commit trailers:
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`
  `Claude-Session: https://claude.ai/code/session_01JGDFJkgB3ttBJtcTtc6aL5`
  No model names anywhere else.
- Reply to the user in Indonesian. The user decides when the workflow stops and resumes (*"tunggu aba-aba aku"*).
- Never lose flowmaid's differentiators (PLAN.md invariants):
  - interactive scene / route / route_partial / hit_test with drag speed;
  - fold, to_mermaid, advance;
  - portable SVG, zero dependencies, MSRV 1.75, wasm, determinism;
  - the semantic palette stays the default.

## 2. Same container still alive (scratchpad present)
- If the workflow is merely stopped, resume it with `Workflow({scriptPath: "<scratchpad>/phaseB2.js", resumeFromRunId: "wf_513402ce-c52"})`. The cache replays B0–B8 and continues with B9.
- **Never edit `implPrompt`, `verifyPrompt` or `COMMON` before a resume**: the cache key is the prompt plus its options. If a restart interrupted a fix round, add a note scoped to that package and round only, like `RESTART_NOTE_B7`.
- Better alternative: run `phaseB-continue.js` with `{from: "b9-compound"}`. It does not depend on the old run's cache, and its `pre` is the current HEAD, which includes the release and the perf fix.

## 3. Fresh container: rebuild the workspace
```bash
NEW=<new scratchpad path from the system prompt>
OLD=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
git clone -b backup/parity-workspace https://github.com/go-routine-id/flowmaid "$NEW"
cd "$NEW" && grep -rlI "$OLD" --exclude-dir=.git . | xargs sed -i "s#$OLD#$NEW#g"
(cd mm && npm install)          # mermaid 12.2.0 (registry.npmjs.org is reachable)
rustup toolchain install 1.75 && rustup target add wasm32-unknown-unknown   # if missing
cd /home/user/flowmaid && git fetch origin && git checkout ccr-ccc6d89a-5cxd7e && git merge --ff-only origin/ccr-ccc6d89a-5cxd7e
"$NEW/tools/guard.sh" /home/user/flowmaid   # must print GUARD: ALL GREEN
```
Notes:
- Playwright and Chromium come preinstalled: `require('playwright')` works from any directory, so do not reinstall them.
- `tools/mmrender.js` and `tools/mmbench.js` load `mm/node_modules/mermaid/dist/mermaid.min.js`.
- Check disk with `df`. Each cargo `target/` dir is 1.5–2 GB; delete old ones when less than 3 GB is free.

Then, once the user gives the go:
```
Workflow({scriptPath: "<NEW>/phaseB-continue.js", args: {from: "b9-compound"}})
```
- It runs B9 → B10 → B11 → c-leftovers, then the wrap-up. Each package goes impl → verify → up to 4 fix rounds → push, or STOP with its commits left on the branch.
- If B9 had already started and left commits, also pass `pre: "<sha before B9>"` and either `resumeAt: "verify"` or `note: "<what is left>"`.
- Package specs live inside the script (`PKGS`). They cite report.md items and use the scratch files in this folder: rev_ll, rv_adv_skeptic, rv_be, verify/…, fmx_fanframes, proto-out and stressdbg are all included.

## 4. Hourly merge task (recreate on Monday if it is gone)
Create a scheduled task with cron minute 14, bound to the session, using this prompt:
> Hourly check-in for the user's rule "pastikan di push setiap selesai, push dan merge di dev branch". In /home/user/flowmaid: git fetch origin dev ccr-ccc6d89a-5cxd7e. If origin/ccr-ccc6d89a-5cxd7e has commits not in origin/dev (git log origin/dev..origin/ccr-ccc6d89a-5cxd7e), and no PR ccr-ccc6d89a-5cxd7e -> dev is already open, open one (summarise the passed package(s) from the commit messages and CHANGELOG, PR attribution footer), subscribe to it, and merge it (merge commit, expectedHeadSha) once CI test/msrv/wasm are green; then unsubscribe and tell the user in Indonesian which package is now in dev. Also glance at the Phase B workflow (journal of the current run): if a package was pushed or a verify verdict changed since the last report, give the user a short progress update; if nothing changed, say nothing. When phase B's workflow has finished (all packages through c-leftovers pushed and merged), delete this scheduled task.

The task needs a session; a new session means a new task. Delete it when phase B ends.

## 5. Release procedure (as done for 0.34.0)
1. Make a worktree `release/X` from `origin/main`, merge `origin/dev` into it, and resolve conflicts (CHANGELOG as in §0a).
2. Bump `Cargo.toml` and `Cargo.lock`. Update the README snippet `flowmaid = "X"`. Keep `exclude = ["docs/img/"]`.
3. CHANGELOG `## [X] - date` with a `### Highlights` list.
4. README "flowmaid next to mermaid":
   - Regenerate `docs/img/compare/*.png` with `compare/pair.js` (mermaid renders in `cmp/out`).
   - Re-run the benchmark with `compare/benchcmp-rel.sh` (`BENCH_BIN`, `OUT` env vars; result in `compare/bench/compare.md`) when layout changed.
   - The large-graph table comes from `cargo run --release --example bench`; see `compare/synth.txt`.
5. Run `tools/guard.sh <worktree>` and `cargo package --list` (expect `docs/img` absent). Commit "release: X — …", push `release/X` (needs permission), open a PR to `main`, and merge once CI is green.
6. **Tag + publish: the user does both.** The container's git proxy refuses tag pushes (HTTP 403), the GitHub MCP has no create-tag/release tool, and there is no crates.io token. Give the user:
   ```
   git fetch origin && git tag -a vX <merge sha> -m vX && git push origin vX && git checkout vX && cargo publish
   ```
   Or the GitHub Releases UI: new release, tag vX, target main.
7. Merge `main` back into `dev` with a PR, then bring the work branch up to date.

## 6. Open items / follow-ups (do not forget)
1. **Remaining phase B packages:** B9 compound layout (subgraph or composite `direction LR`, P0 #10); B10 all-orthogonal static edges (biggest visual factor); B11 crossing hops + comb bundling; c-leftovers (non-flowchart verifier leftovers, HANDOFF §4); then the wrap-up.
2. **Large-graph performance package** (user informed, not yet scheduled). Dense synthetic graphs: 1,000 nodes take about 3.3 s versus 1.8 s before phase B. Corpus diagrams take 0.04–48 ms. The README states this honestly. Propose it after phase B.
3. **Watch for quadratic loops in new code.** The B7 hub-label regression (6.5 s on a 1,000-edge labelled fan) was caught only by a wall-clock test that barely passed. When reviewing a package, consider asking the verifier to time `examples` on a 1,000-edge labelled fan and on the 1,000-node synthetic graph.
4. `scene::tests::colliding_label_placement_stays_near_linear` is a debug wall-clock test with a 20 s limit. It now takes about 2.3 s, so a failure means a real regression.
5. Known non-blocking issues to fold in later are in HANDOFF.md §4 (class fan-in crossings, state chips, journey/timeline wrap, gitGraph, ER, pie and journey colour wrap).
6. README compare images and the benchmark describe the release they were made for. Refresh them on each release that changes layout.
7. Once phase B is done, delete the hourly task and clean up old worktrees and target dirs.

## 7. Where things are in this folder
| Path | Contents |
|---|---|
| `HANDOFF.md` | Full chronological log of runs, restarts, merges and lessons. |
| `PLAN.md`, `report.md`, `perdiagram.json` | The parity plan and audit that every workflow prompt cites. |
| `phaseB2.js` | The live run's script. |
| `phaseB-continue.js` | Continue from a package in a fresh session. |
| `phaseB2.rescue.js`, `history/` | Older scripts and verifier outputs. |
| `tools/` | guard.sh, render.sh (3-way mermaid / baseline / now), perf.sh (drag budget vs `runs/baseline/perf.tsv`), fmrender.sh, mmrender.js, mmbench.js, benchcmp.sh, perfcmp.py, render3.js and render4.js. |
| `cmp/corpus`, `cmp/out`, `flowcorpus`, `runs/baseline`, `runs/bench` | Corpus, mermaid renders, perf baseline. |
| `compare/` | README comparison tooling and results (pair.js, benchcmp-rel.sh, bench/compare.md, synth*.txt). |
| `mm/package.json` | Run `npm install` for mermaid 12.2.0. |
| `rev_ll`, `rv_adv_skeptic`, `rv_be`, `fmx_fanframes`, `proto-out`, `stressdbg`, `verify/…`, `prototypes` | Evidence that the package specs reference. |
