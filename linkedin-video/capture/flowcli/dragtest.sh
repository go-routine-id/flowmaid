#!/bin/bash
W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/work-flowcli
T="tmux -L flowvid -f $W/tmux.conf"
$W/setup.sh
$T send-keys -t v.0 "echo '    D --> N[Email buyer]' >> flow.mmd" Enter
python3 -c 'import time;time.sleep(0.8)'
m(){ $T send-keys -t v.1 -l $'\e[<'"$1;$2;$3$4"; python3 -c 'import time;time.sleep(0.12)'; }
X0=31; Y0=30; X1=$1; Y1=$2
m 0 $X0 $Y0 M
for i in 1 2 3 4 5 6 7 8; do m 32 $((X0 + (X1-X0)*i/8)) $((Y0 + (Y1-Y0)*i/8)) M; done
m 0 $X1 $Y1 m
$T capture-pane -t v.1 -p | sed 's/ *$//' | sed -n 3,37p
