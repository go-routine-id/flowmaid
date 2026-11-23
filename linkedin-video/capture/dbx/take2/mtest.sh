#!/bin/bash
source /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/work-dbx/take2/env2.sh
# SGR mouse: press/drag/release at 0-based (col,row)
mdown() { $T send-keys -t v -l $'\e'"[<0;$(($1+1));$(($2+1))M"; }
mdrag() { $T send-keys -t v -l $'\e'"[<32;$(($1+1));$(($2+1))M"; }
mup()   { $T send-keys -t v -l $'\e'"[<0;$(($1+1));$(($2+1))m"; }
start; w 1.5
keys Enter; w 1.0
keys Enter; w 0.6
w 4.0
keys g; w 0.6
capa $W2/mt/a-erd
mdown 90 22; w 0.1
mdrag 38 22; w 0.3
capa $W2/mt/b-jump
for r in 21 20 19 18 17 16 15 14 13 12 11 10 9; do mdrag 38 $r; w 0.06; done
w 0.3
mup 38 9; w 0.3
capa $W2/mt/c-dragged
keys C-b; w 0.4
capa $W2/mt/d-fold
keys -; w 0.4
capa $W2/mt/e-zoom
stop
