#!/bin/bash
# Start the two-pane tmux scene: left = plain shell, right = flowcli --watch
W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/work-flowcli
B=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/target-flowcli/release/flowcli
T="tmux -L flowvid -f $W/tmux.conf"
S=$W/shot
rm -f $S/*.layout $S/*.jsonl
cp $W/flow.pristine.mmd $S/flow.mmd
mkdir -p $S/bin; ln -sf $B $S/bin/flowcli
$T kill-server 2>/dev/null; python3 -c 'import time;time.sleep(0.3)'
$T new-session -d -s v -x ${COLS:-110} -y ${ROWS:-38} -c $S \
  "env -i PATH=$S/bin:/usr/bin:/bin HOME=$S TERM=xterm-256color COLORTERM=truecolor PS1='\[\e[32m\]\$\[\e[0m\] ' bash --norc --noprofile"
$T split-window -h -l ${RW:-60} -t v -c $S \
  "env -i PATH=$S/bin:/usr/bin:/bin HOME=$S TERM=xterm-256color COLORTERM=truecolor flowcli flow.mmd --watch; sleep 1000"
python3 -c 'import time;time.sleep(1.0)'
$T send-keys -t v.0 'clear; cat flow.mmd' Enter
python3 -c 'import time;time.sleep(0.6)'
$T send-keys -t v.0 C-l; python3 -c 'import time;time.sleep(0.3)'
$T send-keys -t v.0 'cat flow.mmd' Enter
python3 -c 'import time;time.sleep(0.6)'
