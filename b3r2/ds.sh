#!/usr/bin/env bash
# ds.sh <outdir> <files...>   -- static + routed SVG/PNG of each file from the working tree (copy)
set -euo pipefail
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
D=$S/b3r2/cp; O=$1; shift
bash $S/b3r2/sync.sh
printf '%s\n' "$@" > $S/b3r2/ds_list.txt
export DS_LIST=$S/b3r2/ds_list.txt DS_OUT=$O
(cd $D && CARGO_TARGET_DIR=$S/b3r2/target cargo test --release -q --lib dump_svgs::dump_svgs -- --exact --nocapture 2>&1 | grep -v "^running\|^test result\|^$\|^\.$" || true)
NODE_PATH=$(npm root -g) node $S/verify/b3-backedges/tools/svg2png.js $O/*.svg
