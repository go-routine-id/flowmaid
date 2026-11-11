#!/usr/bin/env python3
"""Generate the two script variants for running B12 and c-leftovers side by side
(user, 2026-11-11: "harusnya bisa paralel kan?").

  phaseB-b12.js      B12 in the main repo (work branch), with the shared-machine note
  phaseB-parleft.js  c-leftovers in the worktree <S>/wt/par on branch ccr-ccc6d89a-5cxd7e-par

phaseB-continue.js itself is NOT changed: the live B11 run (wf_5d440b00-741) resumes from it.
Re-run this after any edit of phaseB-continue.js.
"""
import sys
S = '/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad'
src = open(f'{S}/phaseB-continue.js').read()

def rep(s, a, b, count=1):
    if s.count(a) != count:
        sys.exit(f'expected {count} x {a[:70]!r}, found {s.count(a)}')
    return s.replace(a, b)

COMMON_END = "so check examples/state.mmd and the state corpus too.`"

# ---------- B12 in the main repo ----------
b12_note = (
    "so check examples/state.mmd and the state corpus too.\n"
    "SHARED MACHINE (4 CPUs): a second package (class/state leftovers) runs at the same time in another worktree "
    "(${S}/wt/par, branch ccr-ccc6d89a-5cxd7e-par). Never touch that worktree or that branch. Its builds run under "
    "`nice`, so yours have priority, but a timing taken while it compiles or measures can still be off: before perf.sh "
    "or any other timing check `pgrep -c rustc` and `cat /proc/loadavg`; if another build is running, wait for it "
    "(up to 10 minutes, look every 30 s), take every timing at least twice, and call a timing regression or choose a "
    "time-derived constant only from runs that agree. Disk is tight: run `df -h /` before a large build; under 4 GB "
    "free, delete scratch target directories you made yourself and say so in your report.`"
)
def js(note):
    body = note[:-1]            # all but the closing backtick of the template literal
    return body.replace('`', '\\`') + '`'
b12 = rep(src, COMMON_END, js(b12_note))
b12 = rep(b12, "name: 'parity-phase-b-continue'", "name: 'parity-phase-b-b12'")
b12 = rep(b12, "the lead releases 0.40.0 after this package passes", "the lead releases it after this package passes, under the next free version number (0.40.0 or 0.41.0)")
open(f'{S}/phaseB-b12.js', 'w').write(b12)

# ---------- c-leftovers in the par worktree ----------
par_note = (
    "so check examples/state.mmd and the state corpus too.\n"
    "SHARED MACHINE (4 CPUs) — THIS IS THE PARALLEL TRACK: package B12 (\"subgraphs as blocks\") runs at the same time "
    "in the main repo /home/user/flowmaid on branch ccr-ccc6d89a-5cxd7e. Never touch that directory or that branch; "
    "your repo is the git worktree ${REPO} on branch ${BR}. B12 has priority on the machine, so be polite: run every "
    "build, test, clippy, doc, guard.sh and render.sh as `CARGO_BUILD_JOBS=2 nice -n 19 <command>`. Timings are the "
    "exception: a timing taken while the other track compiles is worthless. For perf.sh or any timing, first build "
    "under nice (`CARGO_BUILD_JOBS=2 nice -n 19 cargo build --release --examples` in ${REPO}), then wait until no other "
    "build runs (`pgrep -c rustc` is 0 and the 1-minute load in /proc/loadavg is under 1.0; look every 30 s for up to "
    "10 minutes; if it never gets quiet, measure anyway and say so), run the timing WITHOUT nice, measure before and "
    "after in the same quiet window, twice, and call a regression only when both runs agree. Disk is tight: run "
    "`df -h /` before a large build; under 4 GB free, delete scratch target directories you made yourself and say so "
    "in your report.`"
)
par = rep(src, COMMON_END, js(par_note))
par = rep(par, "const REPO = '/home/user/flowmaid'", f"const REPO = '{S}/wt/par'")
par = rep(par, "const BR = 'ccr-ccc6d89a-5cxd7e'", "const BR = 'ccr-ccc6d89a-5cxd7e-par'")
par = rep(par, "name: 'parity-phase-b-continue'", "name: 'parity-phase-b-parallel-leftovers'")
par = rep(par, "Work in the main repo ${REPO} on branch ${BR} (cd there).", "Work in the git worktree ${REPO} on branch ${BR} (cd there; it is a separate checkout of the same repository).")
par = rep(par, "Package C-leftovers (main track).", "Package C-leftovers (parallel track: it runs beside package B12).")
old_start = "  - For these two block-placement items: package B12"
i = par.index(old_start); j = par.index("\n", i)
if par.count(old_start) != 1: sys.exit('block-placement sentence not unique')
par = par[:i] + (
    "  - For these two block-placement items: package B12 (\"subgraphs as blocks\", ${S}/b12/SPEC.md) is being built "
    "right now in the main repo and reworks how every subgraph with a link across its border is placed. Do NOT fix "
    "them here: skip both with that reason (a legitimate skip; the lead re-checks them once B12 has landed). For the "
    "same reason keep your state and compound changes small and local: do not restructure src/layout.rs, "
    "src/compound.rs or the frame code in src/scene.rs beyond what an item needs, because B12 edits the same files "
    "and the two branches are merged afterwards."
).replace('`', '\\`') + par[j:]
open(f'{S}/phaseB-parleft.js', 'w').write(par)
print('written: phaseB-b12.js, phaseB-parleft.js')
