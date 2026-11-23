#!/bin/bash
# Drive dbx inside tmux and capture each state as ANSI (final take).
source /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/work-dbx/env.sh
rm -rf $W/caps; mkdir -p $W/caps
w() { python3 -c "import time;time.sleep($1)"; }
start; w 1.5
cap 01-picker
keys Enter; w 1.0            # connect to the saved SQLite connection
keys Enter; w 0.6            # expand the 'main' database node
cap 02-tables
w 3.5; cap 03-tables-clean   # toasts gone
keys g; w 1.0                # generate the ERD (flowmaid layout)
cap 04-erd
keys C-b; w 0.5              # fold the explorer tree: ERD gets the full width
cap 05-erd-full
w 3.5; cap 06-erd-full-clean
keys -; w 0.4; cap 07-zoom   # zoom in (0.80x)
for i in 1 2 3 4 5 6 7 8; do keys l; w 0.25; cap 08-pan-$i; done
keys 0; w 0.4; cap 09-reset
for i in 1 2 3; do keys .; w 0.35; cap 10-sel$i; done
w 1.0; cap 11-sel-settled
