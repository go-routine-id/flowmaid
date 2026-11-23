#!/bin/bash
# try.sh file.mmd cols rows -> prints plain capture
W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/work-flowcli
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/target-flowcli/release/flowcli
T="tmux -L flowtry -f $W/tmux.conf"
$T kill-server 2>/dev/null
$T new-session -d -s v -x $2 -y $3 -c $(dirname $1) "env TERM=xterm-256color COLORTERM=truecolor $B $(basename $1); sleep 100"
python3 -c 'import time;time.sleep(1.0)'
$T capture-pane -t v -p | sed 's/ *$//'
$T kill-server
