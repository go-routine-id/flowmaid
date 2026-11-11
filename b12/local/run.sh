#!/usr/bin/env bash
# Local acceptance runner for B12 (spec 4.3, 4.8). Never committed; the
# inputs in ./in hold the analog family, which must not enter the repository.
#
#   run.sh struct  [structdump]            (re)make ./struct/<name>.struct.json (default: B/start/structdump)
#   run.sh fm      <flowmaid-cli> <outdir> [label]   static rows  -> <outdir>/metrics.tsv
#   run.sh routed  <quality-example> <outdir> [label] routed rows -> <outdir>/metrics-routed.tsv,
#                                                     the harness table -> <outdir>/quality.tsv
#   run.sh mermaid <outdir>                           mermaid's rows -> <outdir>/metrics.tsv
#   run.sh judge   <outdir> [judge options...]        static judge against the three reference tables
#   run.sh judge-routed <outdir> [judge options...]
#
# IN=<dir> picks another input directory (default ./in), ONLY="a b" a subset of names.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
M=$HERE/../metrics
IN=${IN:-$HERE/in}
ST=$HERE/struct
mode=${1:-}
files() { if [ -n "${ONLY:-}" ]; then for n in $ONLY; do echo "$IN/$n.mmd"; done; else ls "$IN"/*.mmd; fi; }
need_struct() {
  local dump=${1:-$HERE/../start/structdump} f n
  mkdir -p "$ST"
  for f in $(files); do
    n=$(basename "$f" .mmd)
    if [ ! -s "$ST/$n.struct.json" ] || [ "${FORCE:-0}" = 1 ]; then
      "$dump" "$f" > "$ST/$n.struct.json" || { echo "run.sh: structdump failed for $n" >&2; rm -f "$ST/$n.struct.json"; }
    fi
  done
}
case "$mode" in
  struct) FORCE=1 need_struct "${2:-}"; ls "$ST" | wc -l >&2 ;;
  fm)
    cli=${2:?cli}; out=${3:?outdir}; label=${4:-fm}; mkdir -p "$out"; need_struct
    svgs=()
    for f in $(files); do
      n=$(basename "$f" .mmd)
      msg=$("$cli" "$f" -o "$out/$n.svg" 2>&1) || { echo "run.sh: $cli failed on $f: $msg" >&2; exit 1; }
      svgs+=("$out/$n.svg")
    done
    "$M/blockmetrics" --struct-dir "$ST" --label "$label" "${svgs[@]}" > "$out/metrics.tsv" 2> "$out/metrics.warn" || true
    echo "wrote $out/metrics.tsv ($(grep -c WARN "$out/metrics.warn" || true) warnings in metrics.warn)" >&2 ;;
  routed)
    q=${2:?quality example}; out=${3:?outdir}; label=${4:-fm-routed}; mkdir -p "$out/q"; need_struct
    dir=$IN
    if [ -n "${ONLY:-}" ]; then dir=$out/q-in; rm -rf "$dir"; mkdir -p "$dir"; for f in $(files); do cp "$f" "$dir/"; done; fi
    "$q" "$dir" --svg "$out/q" > "$out/quality.tsv" 2> "$out/quality.err" || { echo "run.sh: quality failed, see $out/quality.err" >&2; exit 1; }
    "$M/blockmetrics" --struct-dir "$ST" --label "$label" "$out"/q/*.routed.svg > "$out/metrics-routed.tsv" 2> "$out/metrics-routed.warn" || true
    "$M/blockmetrics" --struct-dir "$ST" --label "${label%-routed}" "$out"/q/*.static.svg > "$out/metrics-static.tsv" 2> "$out/metrics-static.warn" || true
    echo "wrote $out/metrics-routed.tsv, $out/quality.tsv" >&2 ;;
  mermaid)
    out=${2:?outdir}; mkdir -p "$out"; need_struct
    NODE_PATH=${NODE_PATH:-$(npm root -g)} node "$HERE/../../tools/mmrender.js" "$out" $(files) >&2
    "$M/blockmetrics" --struct-dir "$ST" "$out"/*.mermaid.svg > "$out/metrics.tsv" 2> "$out/metrics.warn" || true
    echo "wrote $out/metrics.tsv" >&2 ;;
  judge)
    out=${2:?outdir}; shift 2
    "$M/judge" "$out/metrics.tsv" --baseline "$M/baseline-038.tsv" --start "$M/ref-start.tsv" --mermaid "$M/ref-mermaid.tsv" --thresholds "$M/thresholds.json" "$@" ;;
  judge-routed)
    out=${2:?outdir}; shift 2
    "$M/judge" "$out/metrics-routed.tsv" --baseline "$M/ref-038-routed.tsv" --start "$M/ref-start-routed.tsv" --mermaid "$M/ref-mermaid.tsv" --thresholds "$M/thresholds-routed.json" "$@" ;;
  *) sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac
