# flowmaid: resuming on Monday (START HERE)

> **Ringkasan (ID) — DILANJUTKAN 2026-11-10 ~15:10 UTC (yang berlaku sekarang):** user bilang "mari kita lanjutkan saja!", jadi jeda dibatalkan dan fase B jalan lagi di sesi yang sama. Urutan: **B11 → c-leftovers → rilis 0.39.0 → B12 (subgraph sebagai blok, §0a)**. Keadaan terkini ada di **§0** dan **§0b**; bagian lain yang bertanda HISTORICAL jangan diikuti. Semua bahan kerja ada di branch `backup/parity-workspace` (folder ini).

## 0. State at the 2026-11-10 pause (CURRENT — read this first)
| Thing | Value |
|---|---|
| Releases | 0.36.0 `main` b2d6d0c (PR #99), 0.37.0 `main` 8e3553b (PR #102), **0.38.0 `main` 56b0bdd (PR #108, merged 2026-11-10 03:2x UTC; includes contributor PR #105)**. The user tags and publishes each (§7 step 7); tags `v0.36.0`, `v0.37.0`, `v0.38.0` are on origin (checked 2026-11-10); crates.io had 0.37.0 at 04:29 UTC, so ask once whether `cargo publish` of 0.38.0 is done. 0.38.0 went through four review rounds (rvrel38/, wf_224da1be-a8f, wf_54ad3715-c76): 18 → 9 → 5 → 2 findings, all fixed. |
| Phase B | Done through **B10**. Next: **B11** (`b11-hops-comb`: M15 semicircle hops at crossings + B4 comb bundling / even hub port band), then **c-leftovers** (narrowed class/state spec; repro files restored from wsbackup/verify), then the wrap-up. |
| B11 WIP | Started before the pause and cut off by a container restart: `backup/b11-hops-comb-wip` **662eea3** (also a local `git stash` in /home/user/flowmaid, gone if the container is new). It never passed verification. The workflow re-implements B11 from scratch; look at the WIP only for ideas. |
| Work branch | `ccr-ccc6d89a-5cxd7e` = `dev` = **dc05e8c** (PR #109 main → dev merged 2026-11-10 03:27 UTC; contains main 56b0bdd = 0.38.0). Clean, nothing unpushed. The B11 WIP stash in /home/user/flowmaid is only a copy of `backup/b11-hops-comb-wip` 662eea3. |
| Workflow | See **§0b** for the live run. Old runs `wf_513402ce-c52` and `wf_5b4ccf67-c1e` no longer exist; do **not** resume them. |
| Hourly task | `trig_019VQH3TYG4ijXR8niHxK13z`, bound to session `session_01JGDFJkgB3ttBJtcTtc6aL5` (see §0b for whether it is enabled and its prompt). A NEW session must not re-enable it (it would wake the old session): delete it (delete_trigger) and create its own with the §5 template. |
| Contributor PR #105 | arifintahu, fixes #106 (a `\|` inside a double-quoted edge label). User chose to ship it in 0.38.0: merged into `release/0.38.0` as 003c2a7 (CHANGELOG entry under [0.38.0] › Fixed, credited); thank-you comment posted. Done: PR #105 shows merged and issue #106 is closed. |
| Follow-ups | §8 (large-graph perf package), §11 (c-polish candidates from the README review), and §11b (2 open items from the 0.38.0 reviews; review files in `rvrel38/`). New issues filed by the user on 2026-11-10 for hosts (flowcli): **#110** fallible `route` for class/ER, **#111** `route_partial` for ER/class/mindmap, **#112** hit-testing for mindmap/sequence/pie/journey/timeline/sankey, **#113** measure-aware `route_partial` + compact fold. Not scheduled yet: offer them to the user as a parallel track (non-flowchart modules, like c1-nonflow). |

### 0a. USER DECISION 2026-11-10 ~04:40 UTC — new package B12 "subgraphs as blocks", AFTER B11
The user compared flowmaid 0.38.0 with mermaid 12.2 in the flowmaid-web playground on their private architecture diagram (`01-overview.mmd`, folder ARCHITECTURE-CURRENT; screenshots in the session) and said mermaid is "masih jauh lebih rapi". Diagnosis:
- mermaid (very likely the ELK layout, since its edges are orthogonal; to be confirmed from the source) lays every subgraph out as ONE BLOCK and stacks the blocks in flow order: users → external services (one row) → DR → PROD → DEV/STAGING; edges into the external-services block come up from below.
- flowmaid ranks each node on its own: the "Layanan pihak luar" frame spans the whole drawing height (DNS/Google on top, Telegram middle, OpenAI/Azure bottom); DR/PROD/STAGING frames sit side by side, so long horizontal runs cross the PROD frame (SMTP, pull image, dump gagal, tunnel SSH); ~8 parallel lines leave GitHub/DNS through one corridor with labels far from both ends.
- B9 (compound.rs) already lays a subgraph out on its own, but only one with `direction` and no edge across its border.
Decisions: **order = B11 → c-leftovers → release 0.39.0 → B12** ("Minggu depan, setelah B11"; option text said the plan B11 → c-leftovers → 0.39.0 stays, then this package). **Scope = default for ALL diagrams with subgraphs** (not only `flowchart-elk` / `layout: elk`); diagrams without subgraphs unchanged.
B12 sketch (write its spec into phaseB-continue.js before running): each subgraph laid out recursively as a block (members' internal edges only), blocks placed as single nodes in the parent layering with block-level cycle breaking (more out-edges → earlier), cross-block edges routed orthogonally between blocks with ports on block borders, labels near their edge's middle; keep route()/route_partial()/picking consistent and the drag budget; class/ER untouched; state composites follow the same rule (mermaid/ELK does). Measure on the corpus + a sanitized copy of the user's diagram (crossings, frame strikes, area, label-to-edge distance) against 0.38.0 and against mermaid renders.
TODO with the user: ask for the text of `01-overview.mmd` (they did not paste it yet). It holds private infrastructure (IPs, domains): never commit it as is — commit only a copy with names/IPs replaced.

### 0b. Live state after the 2026-11-10 resume (CURRENT)
- `main` = b9e0ab9 (the user's README commit "crate link" on top of 0.38.0 56b0bdd); synced into dev by PR #114; work branch fast-forwarded to dev before B11 started (shas in HANDOFF.md).
- Run: `phaseB-continue.js` with `{from: "b11-hops-comb", until: "c-leftovers", noWrap: true, note: …}` (run id and task id in HANDOFF.md's last lines). `until` / `noWrap` are new args (2026-11-10): this run ends after c-leftovers' push without the Wrap-up; B12 gets its own run later, from a work branch synced with dev, and that run does the Wrap-up.
- PLAN.md's commit trailer is now `Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>` + the same Claude-Session line (the session's attribution reminder changed on 2026-11-10).
- At `push:b11-hops-comb`: integrate B11 → dev (§10). At `push:c-leftovers`: integrate, release **0.39.0** (§7; `release/0.39.0` is covered by the user's "release at every big package"), main → dev, then fast-forward the work branch to dev (the run has ended), then B12.
- B12: design/spec work goes to `<S>/b12/` (SPEC.md); add a `b12-blocks` entry to PKGS that points at it, then run `{from: "b12-blocks"}`.

**Resume steps as written at the pause (kept for a new session; `<S>` = this workspace folder, rebuilt per §6 in a new session or container):**
1. `cd /home/user/flowmaid && git fetch -q origin && git checkout ccr-ccc6d89a-5cxd7e && git merge --ff-only origin/dev` (push if it moved).
2. `Workflow({scriptPath: "<S>/phaseB-continue.js", args: {from: "b11-hops-comb"}})` (in a fresh container: §6 first, and update `BR` / the commit trailer per §3).
3. At `push:b11-hops-comb`, integrate B11 → dev (§10). c-leftovers runs next; at its push, integrate and release **0.39.0** = B11 + c-leftovers (§7).
4. Then **B12 subgraphs as blocks** (§0a, user decision): write its spec, get the user's go, run it, release.
5. Then propose the §11 "c-polish" package and the §8 large-graph performance package to the user.

Written 2026-11-04 by the session that ran phase B (CLI session `cac93b13…`, claude.ai `session_01JGDFJkgB3ttBJtcTtc6aL5`). §2 records the state at the pause and gets updated as the pause completes.

> **[HISTORICAL, superseded by §0/§0b; those runs no longer exist, the narrowed c-leftovers spec is already in phaseB-continue.js, do NOT stop a run after push:b11] Update 2026-11-09:** resumed. Live run `wf_5b4ccf67-c1e` (task `wyf69sgs1`, script phaseB-continue.js, `{from: "b8b-leftovers"}`): B8b (B8 leftovers) → B9 → B10 → B11 → c-leftovers. The hourly task is re-enabled. To resume after a restart in this session, use `Workflow({scriptPath: "<S>/phaseB-continue.js", resumeFromRunId: "wf_5b4ccf67-c1e"})`, and don't edit prompts before resuming.

> **[HISTORICAL, superseded by §0/§0b; those runs no longer exist, the narrowed c-leftovers spec is already in phaseB-continue.js, do NOT stop a run after push:b11] Update 2026-11-09 (parallel):** a second track runs on branch `ccr-ccc6d89a-5cxd7e-par` (worktree `<S>/wt/par`, script `phaseB-par.js`, run `wf_e7e056b0-fbb`, package `c1-nonflow`).
> - Its CHANGELOG text is in `<S>/par/CHANGELOG-c1.md`; add it to CHANGELOG at the next release.
> - The main track's c-leftovers was narrowed to class/state. Stop the main run after `push:b11-hops-comb`, then resume it with `resumeFromRunId` so the narrowed spec is picked up.

> **[HISTORICAL, superseded by §0/§0b; those runs no longer exist, the narrowed c-leftovers spec is already in phaseB-continue.js, do NOT stop a run after push:b11] 0.36.0 released 2026-11-09:** `main` = `b2d6d0c` (PR #99, B8b + c1). The user tags `v0.36.0` and runs `cargo publish`. B9 is running; its PR to dev will conflict on CHANGELOG, so integrate it through branch `ccr-ccc6d89a-5cxd7e-par` (§10).

## 1. First five minutes: find out where things stand
Run this as one Bash call:
```bash
cd /home/user/flowmaid && git fetch -q origin --tags && \
git log --oneline -6 origin/dev && echo --- && git log --oneline -4 origin/ccr-ccc6d89a-5cxd7e && echo --- && \
git ls-remote origin 'refs/heads/backup/*' 'refs/heads/release/*' && git tag -l 'v0.3*' && git status --short | head
```
**Expected (2026-11-10):** `origin/dev` and `origin/ccr-ccc6d89a-5cxd7e` at or after `dc05e8c`, tags through `v0.38.0`, `backup/b11-hops-comb-wip` = `662eea3`. Then follow **§0 / §0b**. If the work branch has commits that dev lacks, a package is in progress or stopped: read HANDOFF.md's last lines before running anything. **The A–D table below is HISTORICAL (the 2026-11-04 B8 pause); do not use it** — path A would re-run B9.

Historical paths:

| Situation | Path |
|---|---|
| `origin/dev` has the B8 merge (a "Merge pull request … ccr-ccc6d89a-5cxd7e" after `f6c8e82`, with B8 commits such as `feat(layout): narrowing keeps a step…` and `fix(layout): edge-less subgraph members…`) | **A** |
| B8 commits are on `origin/ccr-ccc6d89a-5cxd7e`, which means they passed verification and were pushed, but `dev` lacks them | **B** |
| B8 exists only on `backup/b8-layering-wip`, so it never passed verification | **C** |
| Commits for B9 or later are on the work branch or on a `backup/b9-*` branch | **D** |

**A. B8 is in dev: start B9.** Fast-forward the work branch to `origin/dev` (`git checkout ccr-ccc6d89a-5cxd7e && git merge --ff-only origin/dev`; push it if it moved). Once the user gives the go, run `phaseB-continue.js` with `{from: "b9-compound"}` (§4).

**B. B8 was pushed but is not in dev.** The PR to dev conflicts in `CHANGELOG.md` because dev already has the 0.34.0 release (PR #93 synced main into dev). Fix it on the work branch:
1. `git merge origin/dev`.
2. Resolve CHANGELOG: keep dev's `## [0.34.0]` section unchanged, and put B8's entries under `## [Unreleased]` above it. B8's entries are the hunks `git diff d50c525 <B8 head> -- CHANGELOG.md` adds.
3. `src/layout.rs` merged cleanly in a test. If it does conflict, keep the `hub_label_room` speedup (fan built once, `s_curve_blend`, `FanCurves`) from 9c62b27.
4. Run guard, commit the merge, push, open a PR to dev, wait for green CI, merge. Then continue with A.

**C. B8 never passed: rebuild it on top of dev, then verify.**
```bash
cd /home/user/flowmaid && git checkout ccr-ccc6d89a-5cxd7e && git merge --ff-only origin/dev   # d50c525 is in dev, so this fast-forwards
PRE=$(git rev-parse HEAD)
git cherry-pick d50c525..origin/backup/b8-layering-wip    # B8 commits; resolve CHANGELOG as in B (B8 entries under [Unreleased])
```
Then run the workflow with:
```
{from: "b8-layering", pre: "<PRE>", resumeAt: "verify"}
```
Pass `note` instead of `resumeAt` when the B8 work looks unfinished. For context, verify#0's findings and fix#1's report are in `history/b8-verify0.json`. Ignore `history/b8-full-layering-wip.patch`, `b8-layout-full.rs` and `b8-rank.rs`: they are early drafts and must not be applied.

**D. B9 started by accident.** Save it (`git branch backup-b9-wip HEAD`, then push to `backup/b9-wip`). Reset the work branch to the B8 head, and follow A. The B9 attempt may be thrown away: the workflow re-implements B9 from scratch.

## 2. State at the 2026-11-04 B8 pause (HISTORICAL, superseded by §0; do not act on it)
| Thing | Value |
|---|---|
| 0.34.0 | `main` `dd7c591` (PR #92); tag `v0.34.0` and crates.io published by the user. |
| B8 | **Passed** verify#1 (2026-11-04 13:39 UTC). Pushed `4381c24`. The work branch then merged dev (`65decf7`: CHANGELOG with B8 under [Unreleased]). Merged to dev via **PR #94** → dev `2bd1114`. Verdicts: `history/b8-verify0.json`, `history/b8-verify1.json`. |
| 0.35.0 | `release/0.35.0` `57e1981` → **PR #95 merged** → `main` = `3430984`. Tag `v0.35.0` and `cargo publish` are still **to be done by the user** (§2a). |
| Workflow | Run `wf_513402ce-c52` **stopped** right at `push:b8-layering` (B9 never started). Next package: **b9-compound**, i.e. path A in §1. |
| Hourly task | `trig_019VQH3TYG4ijXR8niHxK13z` **disabled** (bound to the old session; delete it and create a new one in a new session, §5). |
| Work branch | `ccr-ccc6d89a-5cxd7e` = dev = `9dc3292` (contains main `3430984` = 0.35.0). Clean, nothing unpushed. |
| Backups | `backup/parity-workspace` (this folder) and `backup/b8-layering-wip` (now merged; can be deleted). The obsolete `backup/a1-class`, `backup/a1-class-wip` and `backup/a3-state` can go too (the user decides). |

### 2a. Release 0.35.0 follow-through (HISTORICAL)
- PR #95 merged 2026-11-04 14:06 UTC → `main` = **`3430984`** (0.35.0).
- Tag `v0.35.0` + `cargo publish`: **the user** does it (`git fetch origin && git tag -a v0.35.0 3430984 -m "flowmaid 0.35.0" && git push origin v0.35.0 && git checkout v0.35.0 && cargo publish`).
- main → dev sync: **PR #96 merged** → `dev` = **`9dc3292`**; the work branch `ccr-ccc6d89a-5cxd7e` was fast-forwarded and pushed (= `9dc3292`). **So Monday is path A: start B9 from `9dc3292`.**

## 3. Rules (from the user, plus this session's practice)
- Quality bar: *"yang penting kualitasnya tidak main-main dan premium"*. Go step by step and never skip a package (*"jangan loncat"*).
  - A package passes only after an independent verification, with up to 4 fix rounds; otherwise stop and review with the user. Rare edge cases still block.
- After every passing package: push the work branch, open a PR to `dev`, wait for green CI (test, msrv, wasm), merge with a merge commit and `expectedHeadSha`.
  - The workflow itself only pushes. The PR and merge come from the hourly task (§5) or are done by hand.
- The user decides when the workflow stops and resumes (*"tunggu aba-aba aku"*).
- Reply in Indonesian. No model names in commits, PRs or code, except the attribution trailer and PR footer the session's system reminder prescribes (copied into PLAN.md's commit rules).
- Never lose flowmaid's differentiators (PLAN.md invariants):
  - interactive scene / route / route_partial / hit_test with drag speed;
  - fold, to_mermaid, advance;
  - portable SVG, zero dependencies, MSRV 1.75, wasm, determinism;
  - the semantic palette stays the default.
- **New session identity:**
  - *Commit trailer.* Use the attribution lines the new session's system reminder gives; they replace `Claude-Session: …session_01JGDFJkgB3ttBJtcTtc6aL5`. Also update the trailer line in `PLAN.md`, which every agent prompt cites. Editing PLAN.md does not touch the prompt text, so it is safe.
  - *Branch.* Push to the branch the new session designates. If it is not `ccr-ccc6d89a-5cxd7e`, create it from that branch, which carries all the history, and change `const BR = …` in `phaseB-continue.js`. PRs then go from the new branch to `dev`.
  - *Pushing elsewhere.* release/* and backup/* need the user's explicit permission. Granted so far: release/0.34.0, release/0.35.0, backup/*, and the integration branch `ccr-ccc6d89a-5cxd7e-par` (2026-11-09). release/0.36.0–0.38.0 were pushed under the §10 decision "release at every big package", which also covers release/0.39.0. Tags: never (the proxy refuses them; the user tags).

## 4. Running the workflow
- **Fresh session or fresh container:** `Workflow({scriptPath: "<S>/phaseB-continue.js", args: {from: "<package>", pre?: "<sha>", resumeAt?: "verify", note?: "…"}})`.
  - Package ids, in order: `b8-layering`, `b8b-leftovers`, `b9-compound`, `b10-orthogonal` (all done), `b11-hops-comb`, `c-leftovers`, later `b12-blocks`, then the wrap-up. Args `until: "<pkg>"` and `noWrap: true` end a run early.
  - With no args it starts at b8, so always pass `from`.
  - Each package goes impl → verify → up to 4 fix rounds → push, or STOP with its commits left on the branch.
- **Resuming:** no old run can be resumed (`wf_513402ce-c52` ended at the B8 pause; `wf_5b4ccf67-c1e` was lost in the 2026-11-10 restart). Only a run started by this same session can be resumed with `resumeFromRunId: "<its id>"`, and never edit `implPrompt`, `verifyPrompt`, `COMMON` or `PKGS` before that. After a new session or container, use `{from: "<pkg>", pre: "<sha before the package>", resumeAt: "verify"}` when that package's commits are already on the branch.
- **Stopping cleanly after a package:** the script starts the next package seconds after the push. To pause, watch the run's journal for `"label":"push:<pkg>"` and stop the task at once. If needed, finish the push by hand (`git push -u origin <branch>`). Path D covers a late stop.
- **Back up WIP** to `backup/<pkg>-wip` at each fix-round commit if the container might go away.

## 5. Hourly merge task (recreate in a new session)
1. Delete `trig_019VQH3TYG4ijXR8niHxK13z`; it wakes the old session.
2. Create a task bound to the new session, at cron minute 14, with this prompt (fill in the run id and journal path):
> Hourly check-in for the user's rule "pastikan di push setiap selesai, push dan merge di dev branch". In /home/user/flowmaid: git fetch origin dev <work branch>. If origin/<work branch> has commits not in origin/dev, and no PR <work branch> -> dev is already open, open one (summarise the passed package(s) from the commit messages and CHANGELOG, PR attribution footer), subscribe to it, and merge it (merge commit, expectedHeadSha) once CI test/msrv/wasm are green; if it conflicts, merge origin/dev into the work branch and resolve (CHANGELOG: new entries under [Unreleased]) only between packages, never while a package is in progress; then unsubscribe and tell the user in Indonesian which package is now in dev. Also glance at the Phase B workflow (run <runId>, journal ~/.claude/projects/-home-user-flowmaid/<cli-session>/subagents/workflows/<runId>/journal.jsonl): if a package was pushed or a verify verdict changed since the last report, give the user a short progress update; if nothing changed, say nothing. When phase B's workflow has finished (all packages through c-leftovers pushed and merged), delete this scheduled task.
3. Disable it while the workflow is paused, because every firing costs tokens.

## 6. Fresh container: rebuild the workspace
Run as **one** Bash call, because variables and `cd` do not carry over between calls:
```bash
NEW=<scratchpad path from the new system prompt>; OLD=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
TMP=$(mktemp -d) && git clone -q -b backup/parity-workspace https://github.com/go-routine-id/flowmaid "$TMP/ws" && \
mkdir -p "$NEW" && cp -a "$TMP/ws/." "$NEW/" && cd "$NEW" && \
grep -rlI "$OLD" --exclude-dir=.git . | xargs sed -i "s#$OLD#$NEW#g" && \
(cd mm && npm install --silent) && \
(rustup toolchain list | grep -q 1.75 || rustup toolchain install 1.75) && rustup target add wasm32-unknown-unknown && \
chmod +x "$NEW"/tools/*.sh && echo READY
```
Then run `"$NEW/tools/guard.sh" /home/user/flowmaid`. It must print `GUARD: ALL GREEN` on the branch you start from.

Notes:
- Playwright and Chromium are preinstalled; never reinstall them.
- The sed leaves about 40 tracked files modified in `$NEW` (39 on 2026-11-10). That is fine, and is also fine if the backup is pushed again from there.
- Check disk with `df`: each cargo `target/` is 1.5–2 GB. Delete old ones when less than 3 GB is free.

## 7. Release procedure (as done for 0.34.0–0.38.0; next: 0.39.0)
1. **Branch.** Make a worktree `release/X` from `origin/dev` (dev contains main since PR #93). Check that main is an ancestor: `git merge-base --is-ancestor origin/main origin/dev`.
2. **Version.** `Cargo.toml` and `Cargo.lock` → X. Keep `exclude = ["docs/img/"]`.
3. **CHANGELOG.** Rename `## [Unreleased]` to `## [X] - date`, add a new empty `## [Unreleased]` above it, and add a `### Highlights` list at the top of `[X]`.
4. **README.** Update every version mention: the `flowmaid = "0.34"` snippet, and "flowmaid 0.34.0" in the comparison intro and image alt texts (README lines ~14–18).
   - **Images.** `FM_VERSION=X node compare/pair.js $S/cmp/corpus <tree>/target/release/flowmaid $S/cmp/out <empty scratch dir> <tree>/docs/img/compare advanced back-edge class demo er sequence state subgraph`. Do this whenever layout changed; B8 does change it.
   - **Benchmark.** Build with `cargo build --release --example bench --bin flowmaid` in `<tree>`, then `BENCH_BIN=<tree>/target/release/examples/bench OUT=$S/compare/bench compare/benchcmp-rel.sh <tree>`. Copy the table from `compare/bench/compare.md` into README "### Next to mermaid" and update the "about N× faster" lines.
   - **Large-graph table.** From `cargo run --release --example bench` (see `compare/synth.txt`).
5. **Checks.** Run `tools/guard.sh <tree>` and `cargo package --list` (no `docs/img`). Look at the 1,000-edge labelled fan timing (should be about 0.4 s release).
6. **Ship.** Commit "release: X — …", push `release/X`, open a PR to `main`, merge when CI is green.
7. **Tag and publish: the user does both.** The container's git proxy refuses tag pushes (HTTP 403), the GitHub MCP has no tag/release tool, and there is no crates.io token. Give the user:
   ```
   git fetch origin && git tag -a vX <merge sha> -m "flowmaid X" && git push origin vX && git checkout vX && cargo publish
   ```
8. **Sync back.** Open a PR `main` → `dev` (or the user does it, as with #93), then fast-forward the work branch.

## 8. Open items and follow-ups (do not forget)
1. **Phase B (B8–B10 done, 0.35.0–0.38.0; left: B11, c-leftovers, 0.39.0, B12, wrap-up). Original list:** B8 (if not done), then B9 compound layout (subgraph/composite `direction LR`, P0 #10), B10 all-orthogonal static edges (biggest visual factor), B11 crossing hops + comb bundling, c-leftovers (non-flowchart verifier leftovers, HANDOFF §4), and the wrap-up.
2. **Large-graph performance package.** The user knows about it; not yet scheduled. 1,000-node dense synthetic graphs take about 3.3 s versus 1.8 s before phase B; corpus diagrams take 0.04–48 ms. The README states this honestly. Propose it after phase B.
3. **Watch for quadratic code in new packages.** The B7 hub-label regression (6.5 s on a 1,000-edge labelled fan) was caught only by a wall-clock test that barely passed. After each package, time a 1,000-edge labelled fan (`python3 -c "print('flowchart TB'); [print(f'  H -->|label {i}| T{i}') for i in range(1000)]"`) and the 1,000-node synthetic graph against the previous release.
4. `scene::tests::colliding_label_placement_stays_near_linear` (debug, 20 s limit) takes about 2.3 s on branches that contain 9c62b27. A branch without that fix (B8 WIP before the dev merge) takes 16–26 s and may fail under load. That is not B8's fault; merge dev in.
5. Known non-blocking issues: HANDOFF.md §4 (class fan-in crossings, state chips, journey/timeline wrap, gitGraph, ER, pie and journey colour wrap).
6. When phase B is done: delete the hourly task, clean up old worktrees and target dirs, and suggest the user delete the obsolete backup branches.
7. **B8 leftovers (from verify#1; fold into B9 or a B8 follow-up):**
   - *Incomplete r1 #1.* A box member that is a source but links out of the box, and every member of a box that narrowing stacked or wrapped, still sit on layer 0 beside the hub. Generated case f182 goes from 0 to 7 static defects with 4 frame strikes; repros in s4 and s10.
   - *Suggested fix:* compute a box's first row from the members that have an edge coming in from outside, and pull its source members and loose parts down to that row, ignoring rank-only hub links.
   - *Skipped on drag-budget grounds:* network simplex with nesting edges, and the "frames no wider than their contents" item that depends on it; greedy ELS FAS (two-walk DFS met the 66/75 target instead); seating parts on their innermost nested box.
   - *Not done:* `dense-lr` is still portrait.
   - Details are in `history/b8-verify1.json`.

## 9. What is in this folder
| Path | Contents |
|---|---|
| `MONDAY.md` | This file. |
| `HANDOFF.md` | Chronological log of runs, restarts, merges and lessons. Its header and §1, §2 and §5 are from 2026-10-31 and outdated; its §6–§8 log is current. |
| `PLAN.md`, `report.md`, `perdiagram.json`, `style.md` | The parity plan and audit that every workflow prompt cites. |
| `phaseB-continue.js` | Continue from any package (use this). |
| `phaseB2.js` | The live run's script (only for `resumeFromRunId` in the same container). |
| `phaseB.draft.js`, `phaseB2.rescue.js`, `history/` | Older scripts, verifier outputs, `b8-verify0.json`. |
| `tools/` | guard.sh, render.sh (3-way mermaid / baseline / now), perf.sh (drag budget vs `runs/baseline/perf.tsv`), fmrender.sh, mmrender.js, mmbench.js, benchcmp.sh, perfcmp.py, render3.js. `render4.js` (hard-coded playwright path) and `ixcheck/` are unused. |
| `cmp/corpus`, `cmp/out`, `flowcorpus`, `runs/baseline`, `runs/bench` | Corpus, mermaid renders, perf baseline. |
| `compare/` | README comparison tooling and results (pair.js, benchcmp-rel.sh, bench/compare.md, synth*.txt). |
| `mm/package.json` + lock | Run `npm install` for mermaid 12.2.0. |
| `rev_ll`, `rv_adv_skeptic`, `rv_be`, `rv`, `rev`, `rv_shapes`, `rv_denselr_skeptic`, `sk`, `tlx`, `t`, `fm`, `cmp/zff`, `fmx_fanframes`, `proto-out`, `proto/src`, `stressdbg`, `prototypes`, `verify/…`, `para.png`, `exp1.png` | Evidence cited by the package specs, report.md and perdiagram.json. |
| `archdoc/` | The user's RFC-001 architecture PDF sources and output (chat + Claude Code bridge). Not part of flowmaid. |
| `README.md` | Old restore notes; superseded by §6. |

## 10. Release at every big package (user decision 2026-11-09)
Plan: **0.36.0** = B8b + c1-nonflow (whichever is in dev), **0.37.0** = B9, **0.38.0** = B10, **0.39.0** = B11 + c-leftovers. The user tags and runs `cargo publish` for each.

**Boundary procedure (revised 2026-11-09 03:00 UTC — never stop the main run).**
When a package's push lands on `origin/ccr-ccc6d89a-5cxd7e`, the hourly task (or I) do the following. Meanwhile the main workflow simply continues with the next package.
1. Try PR `ccr-ccc6d89a-5cxd7e` → dev.
   - If it is mergeable, merge it once green.
   - If it conflicts (after a release, usually only CHANGELOG), use the **integration branch** `ccr-ccc6d89a-5cxd7e-par` (permission granted):
     1. `git fetch origin`, then check out `ccr-ccc6d89a-5cxd7e-par` in a scratch worktree.
     2. `git merge --ff-only origin/dev` (it is an ancestor of dev; if not, use a normal merge).
     3. `git merge <package push sha>` (the sha, not the moving branch tip, so a later package's commits are not swept in).
     4. Resolve CHANGELOG: take dev's sections and add the package's new entries under `## [Unreleased]`. The entries come from `git diff <pre> <pkg sha> -- CHANGELOG.md`, where pre is the package's `pre:` sha in the journal.
     5. Run guard, push, PR → dev, and merge once green.
   - Never touch the work branch while a package is in progress.
2. Release the next version from dev (§7):
   - Version bump.
   - CHANGELOG `[X]` + Highlights. For 0.36.0, also add `par/CHANGELOG-c1.md` and its README follow-ups.
   - README mentions, compare PNGs (`FM_VERSION=X`), re-measured tables.
   - Guard, then PR → main, merge.
   - PR main → dev, merge.
   - Give the user the merge sha.
3. Bring the work branch up to date with dev only when the main run is stopped or finished (end of phase B, or a user pause). Until then it lacks the release commits, which is harmless.
4. Deferred cleanup: delete `layout::wrap_text` / `lines_at` and `wrap::RETIRED_GREEDY_WRAP` (dead since c1-nonflow) in the final integration at the end of phase B.

**PAR track** (c1-nonflow): it ends by itself after its push. PR `ccr-ccc6d89a-5cxd7e-par` → dev, merge (no CHANGELOG conflict: its text is in `par/CHANGELOG-c1.md`).

## 11. Follow-ups found by the README review (2026-11-09, run wf_ced7df68-569; result in rvreadme/result.json)
Engine work, not in the 0.37.0 release (README now states the current behaviour). Candidate "c-polish" package after phase B:
1. Parser: recognise mermaid 12.2 headers as explicit "not supported yet": short `xychart`, `block`, `requirement`, `info`, `ishikawa(-beta)`, `eventmodeling`, `venn-beta`, `treeView-beta`, `usecase-beta`, `wardley-beta`, `cynefin-beta`, `railroad*-beta`, `agentflow-beta` (today: flowchart text / '-beta' edge error). Then README can say "every" again.
2. Compound (B9 parity gap): a `direction` block whose members have no links among them is NOT laid out on its own (flowmaid: row in TB/LR); mermaid lays it out in its own direction (column for LR in TB). Decide + implement.
3. flowmaid theme: `style` fill without `color` keeps the dark ink on a dark fill (mermaid themes re-ink via Palette::ink_on_fill). Re-ink in the flowmaid theme too?
4. Flowchart edge labels: `<br/>` is drawn as a space (mermaid breaks the line).
5. Sequence: mermaid's two-way `<<->>` / `<<-->>` and half-arrows are parse errors.
6. Advance SVG: `<br/>` in edge labels and lane titles drawn on one line (chip 18px high).
7. Advance JSON README example: the label chip covers the arrowhead and the diamond clips the label text.
8. Advance text DSL README example: `a --> b` and `c --> b:top` share the last 12.5 px into b and their arrowheads coincide.
9. Contrast: chip/ER-row hairlines and amber accent #d99114 on white are under 3:1 (documented exceptions).

### 11b. Follow-ups from the 0.38.0 release reviews (rvrel38/, wf_224da1be-a8f, wf_54ad3715-c76)
1. **SceneEdge::bezier fallback ends drift off the path's ends** (docs now say so; no SVG effect, the painter draws waypoints). 25 scene() / 49 route() edges over 17k corpus files have an end >3 px off (fork→join in f2j_lr.mmd: 105 px; q18 `n12 ==> n8`: 53 px). Cause: ports::move_end / turn_end move bezier[pi] only `if same(e.bezier[pi], old)`, and ortho.rs:731 re-smooths a route() channel's waypoints without touching bezier. Fix (not just translating controls, which keeps a 45° tangent): at the very end of edgepass::finish_held, for flowchart/state edges with waypoints only, set `bezier = [w0, w0 + k*(w1-w0), wn + k*(w(n-1)-wn), wn]` with k capped by the leg length; then confirm class/ER byte-identical (class.rs:403/415 read bezier) and flowchart SVGs unchanged. Then the README/CHANGELOG "usually but not always at the path's ends" wording can be tightened.
2. Optional regression test: a 401+-edge drawing with a `direction` block laid out on its own — the rest part's edges keep curves only when the rest alone is over 400 (per-part cap).

## 12. USER DECISION 2026-11-09 17:20 UTC — B10 is the last package this week (DONE 2026-11-10)
**Status:** B10 integrated (PR #107 → dev 0862621); 0.38.0 released (PR #108 → main 56b0bdd); main → dev sync PR #109 → dev dc05e8c; work branch fast-forwarded to dc05e8c and pushed; hourly task disabled; workflow not running.
"setelah b10 selesai kita cukup untuk minggu ini! cukup rilis dulu b10! minggu depan kita lanjutkan".
- At push:b10-orthogonal: TaskStop wyf69sgs1 (do not let B11 run). Integrate B10 → dev (README hand-port into the new layout), release 0.38.0, main → dev, ff work branch to dev, back up, disable the hourly task trig_019VQH3TYG4ijXR8niHxK13z (its prompt was rewritten for this).
- NEXT WEEK resume: Workflow({scriptPath: <scratchpad>/phaseB-continue.js, args: {from: "b11-hops-comb"}}) on the work branch = dev; then c-leftovers (narrowed class/state spec, repro files restored from wsbackup/verify); release 0.39.0 = B11 + c-leftovers; then propose the §11 "c-polish" package.
