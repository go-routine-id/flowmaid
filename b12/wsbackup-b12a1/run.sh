#!/usr/bin/env bash
# run.sh - render the B12 inputs with one engine and measure them with blockmetrics.
#
#   run.sh fm <flowmaid-cli> <outdir> [label]   render in/*.mmd with that CLI, write <outdir>/<name>.svg and <outdir>/metrics.tsv
#   run.sh mermaid <outdir>                     the same with mermaid 12.2 (default config, ../../tools/mmrender.js)
#   run.sh struct [structdump-binary]           (re)write struct/<name>.struct.json for every input (default bin/structdump-038)
#
# IN=<dir> overrides the input directory (default: in/ next to this script).
# A missing struct/<name>.struct.json is made on the fly with bin/structdump-038.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
IN=${IN:-$HERE/in}
mode=${1:-}
need_struct() {
  local dump=${1:-$HERE/bin/structdump-038} f n
  mkdir -p "$HERE/struct"
  for f in "$IN"/*.mmd; do
    n=$(basename "$f" .mmd)
    if [ ! -s "$HERE/struct/$n.struct.json" ] || [ "${FORCE:-0}" = 1 ]; then
      "$dump" "$f" > "$HERE/struct/$n.struct.json" || { echo "run.sh: structdump failed for $n (not a flowchart/state diagram?)" >&2; rm -f "$HERE/struct/$n.struct.json"; }
    fi
  done
}
case "$mode" in
  fm)
    cli=${2:?usage: run.sh fm <flowmaid-cli> <outdir> [label]}; out=${3:?outdir}; label=${4:-fm}
    mkdir -p "$out"; need_struct
    for f in "$IN"/*.mmd; do
      msg=$("$cli" "$f" -o "$out/$(basename "$f" .mmd).svg" 2>&1) || { echo "run.sh: $cli failed on $f: $msg" >&2; exit 1; }
    done
    "$HERE/blockmetrics" --struct-dir "$HERE/struct" --label "$label" "$out"/*.svg > "$out/metrics.tsv"
    echo "wrote $out/metrics.tsv" >&2 ;;
  mermaid)
    out=${2:?usage: run.sh mermaid <outdir>}
    mkdir -p "$out"; need_struct
    NODE_PATH=${NODE_PATH:-$(npm root -g)} node "$HERE/../../tools/mmrender.js" "$out" "$IN"/*.mmd >&2
    "$HERE/blockmetrics" --struct-dir "$HERE/struct" "$out"/*.mermaid.svg > "$out/metrics.tsv"
    echo "wrote $out/metrics.tsv" >&2 ;;
  struct)
    FORCE=1 need_struct "${2:-$HERE/bin/structdump-038}"
    ls "$HERE/struct" >&2 ;;
  *) sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac
