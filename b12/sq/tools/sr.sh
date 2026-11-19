#!/usr/bin/env bash
# sr.sh <bindir> <outdir> <file.mmd>...: static (CLI) and routed (quality --svg) SVGs, plus a static|routed pair png
set -euo pipefail
bin=$1; out=$2; shift 2; mkdir -p "$out/in" "$out/q"
for f in "$@"; do cp "$f" "$out/in/"; done
for f in "$@"; do n=$(basename "$f" .mmd); "$bin/flowmaid" "$f" -o "$out/$n.static.svg" >/dev/null 2>&1; done
"$bin/quality" "$out/in" --svg "$out/q" > "$out/q.tsv" 2>/dev/null
for f in "$@"; do n=$(basename "$f" .mmd); cp "$out/q/$n.routed.svg" "$out/$n.routed.svg";
  NODE_PATH=$(npm root -g) node /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/squeeze/pair.js "$out/$n.pair.png" "$out/$n.static.svg" static "$out/$n.routed.svg" routed 1; done
