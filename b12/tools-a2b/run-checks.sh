#!/usr/bin/env bash
# run-checks.sh <outdir> --commit <rev> [--repo <dir>]   build <rev> in a scratch worktree, run the checks, remove it
# run-checks.sh <outdir> --tree <dir>                    build the checkout <dir> as it stands (uncommitted changes too)
# run-checks.sh <outdir> --bin <bindir> [--src <dir>]    use binaries build.sh already made
#   options: --only local,gates,dirs (default all three)   --keep (keep the scratch worktree and target dir)
#
# Runs the three mandatory checks of critique.md section 4 step 0 for one build and stores everything a later
# compare.sh needs in <outdir>:
#   bin/      flowmaid, quality, lib-tests, quality-tests, BUILD-INFO (--commit / --tree)
#   local/    check 1: the 95 local inputs rendered, measured and judged by the LIVE judge (fmrun.sh, rejudge.sh)
#   gates/    check 2: suite-on lib + tests/quality.rs logs (trun.sh)
#   dirs/     check 3: the 332 + 32 direction copies, static and routed (qrun.sh)
#   RUN-INFO  what was run, and how long each step took
# --commit: `git -C <repo> worktree add --detach <SCRATCH>/wt-<sha> <rev>` (repo default /home/user/flowmaid; its
# own checkout is not touched), CARGO_TARGET_DIR=<SCRATCH>/target-<sha>; both are removed at the end unless --keep.
# --tree: CARGO_TARGET_DIR=<SCRATCH>/target-tree-<pid>, removed at the end unless --keep (the tree is never touched).
# --bin: the gate tests read their fixtures from the tree they were built from (BUILD-INFO src=, or --src).
# Env: CARGO_BUILD_JOBS (default 2), SCRATCH (default <this dir>/scratch). Keep > 4 GB free (df -h /).
set -uo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
out=${1:?outdir}; shift
mode=; rev=; repo=/home/user/flowmaid; tree=; bin=; src=; only=local,gates,dirs; keep=0
while [ $# -gt 0 ]; do
  case $1 in
    --commit) mode=commit; rev=$2; shift 2 ;;
    --repo) repo=$2; shift 2 ;;
    --tree) mode=tree; tree=$2; shift 2 ;;
    --bin) mode=bin; bin=$2; shift 2 ;;
    --src) src=$2; shift 2 ;;
    --only) only=$2; shift 2 ;;
    --keep) keep=1; shift ;;
    *) echo "run-checks.sh: unknown argument $1" >&2; exit 2 ;;
  esac
done
[ -n "$mode" ] || { sed -n '2,4p' "$0" | sed 's/^# //'; exit 2; }
SCRATCH=${SCRATCH:-$HERE/scratch}; mkdir -p "$SCRATCH" "$out"; out=$(cd "$out" && pwd)
info=$out/RUN-INFO; : > "$info"
log() { echo "$*" | tee -a "$info"; }
t0=$(date +%s); step() { local s=$(date +%s); "$@"; local r=$?; log "  $(basename "$1") $((  $(date +%s) - s )) s (exit $r)"; return $r; }
avail=$(df -Pk / | awk 'NR==2 {print int($4 / 1048576)}')
log "run-checks.sh $(date -u +%FT%TZ): outdir $out, mode $mode ${rev}${tree}${bin}, only $only; ${avail} GB free on /"
[ "$avail" -ge 4 ] || { log "run-checks.sh: less than 4 GB free on /, not building"; [ "$mode" = bin ] || exit 1; }
wt=; tgt=
cleanup() {
  [ "$keep" = 1 ] && { [ -n "$wt$tgt" ] && log "kept: $wt $tgt"; return; }
  if [ -n "$wt" ] && [ -d "$wt" ]; then git -C "$repo" worktree remove --force "$wt" && git -C "$repo" worktree prune; fi
  [ -n "$tgt" ] && rm -rf "$tgt"
  [ -n "$wt$tgt" ] && log "removed: $wt $tgt"
}
trap cleanup EXIT
case $mode in
  commit)
    sha=$(git -C "$repo" rev-parse --verify "$rev^{commit}") || exit 2
    s7=${sha:0:7}; wt=$SCRATCH/wt-$s7; tgt=$SCRATCH/target-$s7
    [ -e "$wt" ] && { echo "run-checks.sh: $wt exists (an earlier run with --keep?); remove it first" >&2; wt=; tgt=; exit 2; }
    git -C "$repo" worktree add --detach "$wt" "$sha" > /dev/null 2>&1 || { echo "run-checks.sh: worktree add failed" >&2; exit 1; }
    log "worktree $wt at $sha"
    step "$HERE/build.sh" "$wt" "$tgt" "$out/bin" || exit 1
    bin=$out/bin; src=$wt ;;
  tree)
    tree=$(cd "$tree" && pwd); tgt=$SCRATCH/target-tree-$$
    step "$HERE/build.sh" "$tree" "$tgt" "$out/bin" || exit 1
    bin=$out/bin; src=$tree ;;
  bin)
    bin=$(cd "$bin" && pwd); src=${src:-$(sed -n 's/^src=//p' "$bin/BUILD-INFO" 2>/dev/null)} ;;
esac
[ -f "$bin/BUILD-INFO" ] && sed 's/^/  build: /' "$bin/BUILD-INFO" | tee -a "$info"
rc=0
case ,$only, in *,local,*) step "$HERE/fmrun.sh" "$bin" "$out" || rc=1 ;; esac
case ,$only, in *,gates,*) step "$HERE/trun.sh" "$bin" "$out" "$src" || rc=1 ;; esac
case ,$only, in *,dirs,*) step "$HERE/qrun.sh" "$bin" "$out" || rc=1 ;; esac
log "run-checks.sh: done in $(( $(date +%s) - t0 )) s (exit $rc). Compare with: $HERE/compare.sh <baseline-outdir> $out"
exit $rc
