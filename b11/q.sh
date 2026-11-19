#!/usr/bin/env bash
# q.sh <label> [cols] : quality over flowcorpus -> b11/q_<label>.tsv, compare with q_pre.tsv
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
cd /home/user/flowmaid && cargo build --release --example quality 2>&1 | grep -E "^(warning|error)" -A 14 | head -60
./target/release/examples/quality $S/flowcorpus > $S/b11/q_$1.tsv 2>/dev/null
python3 $S/b11/cols.py $S/b11/q_$1.tsv ${2:-fan_x,fan_uneven,edge_edge_x,x_drawn,end_cross,overlap_pairs,tight_pairs,defects,lbl_edge,hug_runs,band_pairs,lat_wiggles} $S/b11/q_${3:-pre}.tsv | grep -E "bench|dense|fanout|TOTAL|file|->"
