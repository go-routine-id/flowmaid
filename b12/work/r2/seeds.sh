#!/usr/bin/env bash
# seeds.sh [env...]: layered crossings for seeds 0..11
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
for n in infra-overview infra-overview-v2 perm-b-subgraphs-reversed perm-e-one-more-inner-edge blocks-overview blocks-overview-subgraphs-reversed dense-td dense-lr bench-75; do
  st=""
  for seed in 0 1 2 3 4 5 6 7 8 9 10 11; do
    v=$(env "$@" FM_XXSEED=$seed FM_XXDBG2=1 $S/b12/target-on/release/flowmaid $S/b12/local/in/$n.mmd -o /tmp/x.svg 2>&1 | grep "XXDBG start" | awk '{print $NF}')
    st="$st $v"
  done
  printf "%-28s %s\n" ${n:0:28} "$st"
done
