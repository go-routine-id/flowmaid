#!/bin/bash
# probe.sh NZOOM  -> big pane capture of the whole ERD at zoom 1/1.25^NZOOM
source /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/work-dbx/take2/env2.sh
n=$1
start; w 1.5
keys Enter; w 1.0
keys Enter; w 0.6
keys g; w 1.0
keys C-b; w 0.5
for i in $(seq 1 $n); do keys -; w 0.2; done
w 3.5
capa $W2/probe/z$n
stop
