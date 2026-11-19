#!/usr/bin/env bash
# evalsets.sh <bindir> <outdir>: quality over the verifier's sets (rnd, tw, nr, nt, sw72, i3) -> <outdir>/<set>.tsv
set -uo pipefail
W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/a2c/fr2
bin=$1; out=$2; mkdir -p $out
for s in rnd tw nr nt sw72 i3; do timeout 900 $bin/quality $W/sets/$s > $out/$s.tsv 2>/dev/null & done; wait
