#!/usr/bin/env bash
# dspre.sh <outdir> <files...> -- static+routed renders from the pre-package tree (9a57363)
set -euo pipefail
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
D=$S/b3r2/pre; O=$1; shift
printf '%s\n' "$@" > $S/b3r2/dspre_list.txt
export DS_LIST=$S/b3r2/dspre_list.txt DS_OUT=$O
(cd $D && CARGO_TARGET_DIR=$S/b3r2/target_pre cargo test --release -q --lib dump_svgs::dump_svgs -- --exact --nocapture 2>&1 | grep -v "^running\|^test result\|^$\|^\.$" || true)
NODE_PATH=$(npm root -g) node $S/verify/b3-backedges/tools/svg2png.js $O/*.svg
