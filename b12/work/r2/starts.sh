#!/usr/bin/env bash
# starts.sh [env...] : per input, layered crossings per start + ordering ms
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
for n in infra-overview infra-overview-v2 perm-b-subgraphs-reversed perm-c-actors-reversed perm-e-one-more-inner-edge blocks-overview blocks-overview-subgraphs-reversed dense-td dense-lr bench-75; do
  out=$(env "$@" FM_XXDBG2=1 $S/b12/target-on/release/flowmaid $S/b12/local/in/$n.mmd -o /tmp/x.svg 2>&1)
  st=$(echo "$out" | grep "XXDBG start" | awk '{printf "%s ", $NF}')
  ms=$(echo "$out" | grep "order.run" | head -1 | awk '{print $NF}')
  printf "%-36s starts: %-34s ms %s\n" $n "$st" $ms
done
