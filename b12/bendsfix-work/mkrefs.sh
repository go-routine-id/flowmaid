#!/usr/bin/env bash
# Re-measures the five reference tables from the stored reference SVGs in b12/local/out:
#   ref-start.tsv        start commit 3abbfeb, static   (out/start/*.svg + out/t9bt-start/*.svg)
#   baseline-038.tsv     0.38.0 CLI, static             (out/fm038/*.svg + out/t9bt-038/*.svg)
#   ref-mermaid.tsv      mermaid 12.2                   (out/mermaid/*.mermaid.svg + out/t9bt-mm/*.mermaid.svg)
#   ref-start-routed.tsv start, quality --svg routed    (out/start/q/*.routed.svg)
#   ref-038-routed.tsv   0.38.0, quality --svg routed   (out/fm038/q/*.routed.svg)
# Row order and labels are the ones the live tables carry (92 rows in glob order, then the 3 t9 BT copies).
#
#   mkrefs.sh <blockmetrics> <outdir>
#
# Nothing is rendered: the SVGs are the stored renders, so a change of the yardstick re-measures exactly the
# drawings the tables were first made from. With the blockmetrics a table was made with, the output is
# byte-equal to it (checked 2026-11-17 with the pre-bends-fix blockmetrics on all five tables).
set -euo pipefail
BM=${1:?blockmetrics}; OUT=${2:?outdir}
HERE=$(cd "$(dirname "$0")" && pwd)
L=$HERE/../local
ST=$L/struct
mkdir -p "$OUT"
meas() { # label-or-empty out-file svgs...
  local lab=$1 f=$2; shift 2
  local args=(--struct-dir "$ST"); [ -n "$lab" ] && args+=(--label "$lab")
  node "$BM" "${args[@]}" "$@" 2>> "$OUT/mkrefs.warn" || true
}
: > "$OUT/mkrefs.warn"
two() { # label out-name dir1 glob1 dir2 glob2
  local lab=$1 name=$2 a b
  a=$(meas "$lab" x $3)
  b=$(meas "$lab" x $4)
  { printf '%s\n' "$a"; printf '%s\n' "$b" | tail -n +2; } > "$OUT/$name"
}
two start        ref-start.tsv    "$L/out/start/*.svg"            "$L/out/t9bt-start/*.svg"
two fm038        baseline-038.tsv "$L/out/fm038/*.svg"            "$L/out/t9bt-038/*.svg"
two ""           ref-mermaid.tsv  "$L/out/mermaid/*.mermaid.svg"  "$L/out/t9bt-mm/*.mermaid.svg"
meas start-routed x $L/out/start/q/*.routed.svg > "$OUT/ref-start-routed.tsv"
meas fm038-routed x $L/out/fm038/q/*.routed.svg > "$OUT/ref-038-routed.tsv"
wc -l "$OUT"/*.tsv >&2
