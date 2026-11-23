#!/bin/bash
# probe2.sh OUT : capture explorer, unfolded ERD, folded ERD at COLS x ROWS
source /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/work-dbx/take2/env2.sh
o=$W2/probe/$1
start; w 1.5
keys Enter; w 1.0
keys Enter; w 0.6
capa $o-a-tree
keys g; w 0.8
capa $o-b-erd
keys C-b; w 0.5
capa $o-c-fold
w 3.5
capa $o-d-fold-clean
stop
