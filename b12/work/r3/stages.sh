#!/usr/bin/env bash
# stages.sh [env...]: per-stage x_drawn/edge_node/edge_frame/hard on the key inputs, final only compact
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12
for f in /home/user/flowmaid/tests/fixtures/blocks/blocks-overview.mmd $B/local/in/infra-overview.mmd $B/local/in/infra-overview-v2.mmd $B/local/in/perm-b-subgraphs-reversed.mmd $B/local/in/perm-e-one-more-inner-edge.mmd /home/user/flowmaid/tests/fixtures/quality/dense-td.mmd /home/user/flowmaid/tests/fixtures/quality/dense-lr.mmd /home/user/flowmaid/tests/fixtures/quality/bench-75.mmd /home/user/flowmaid/tests/fixtures/quality/fanout-frames.mmd /home/user/flowmaid/examples/advanced.mmd; do
  printf "%-28s " $(basename $f .mmd | cut -c1-27)
  env "$@" FM_XXDBG4=1 FM_XXDBG2=1 $B/target-dbg/release/flowmaid $f -o /tmp/x.svg 2>&1 | awk '/XXDBG start/ {if(!done){l=l $NF " "}} /stage builder/ {done=1} /XXDBG stage/ {if(!seen[$3]++){gsub(":","",$3); s=s $3 " " $5 "/" $9 "/" $11 "/" $13 "  "}} END{print "L[" l "] " s}'
done
