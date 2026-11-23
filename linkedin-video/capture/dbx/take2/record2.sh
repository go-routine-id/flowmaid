#!/bin/bash
# Take 2: drive dbx 0.5.10 in tmux (100x29) and capture every state as
# ANSI (for rendering) and plain text (for check.py).
#   g -> ERD, mouse drag pans it (pre-zoom), Ctrl+B folds the tree,
#   '-' zooms to 0.80x, k k k / h h pan, '.' x3 selects orders, hold.
export COLS=100 ROWS=29 SOCK=dbxvid2
source /tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/work-dbx/take2/env2.sh
C=$W2/caps
rm -rf $C; mkdir -p $C
cap() { capa $C/$1; }
# SGR mouse reports (1-based coords); args are 0-based col,row
mdown() { $T send-keys -t v -l $'\e'"[<0;$(($1+1));$(($2+1))M"; }
mdrag() { $T send-keys -t v -l $'\e'"[<32;$(($1+1));$(($2+1))M"; }
mup()   { $T send-keys -t v -l $'\e'"[<0;$(($1+1));$(($2+1))m"; }

start; w 1.5
keys Enter; w 1.0            # connect to the saved 'shop-demo' SQLite connection
keys Enter; w 0.6            # expand the 'main' database node
w 4.0                        # connect toasts gone
cap 01-explorer
keys g; w 0.5                # generate the ERD (flowmaid layout), default zoom
cap 02-erd
# mouse drag on the ERD canvas (unfolded canvas = cols 33..98, rows 4..24):
# press at (92,23), a fast move left 48 cells, then up 13 rows one row at a time
mdown 92 23; w 0.05
mdrag 44 23; w 0.3
cap 03-drag-00
for i in $(seq 1 13); do mdrag 44 $((23 - i)); w 0.15; cap $(printf '04-drag-%02d' $i); done
mup 44 10; w 0.3
cap 05-drag-done
keys C-b; w 0.4              # fold the explorer tree: ERD gets the full width
cap 06-fold
keys -; w 0.4                # zoom: 1.00x -> 0.80x (diagram larger)
cap 07-zoom
for i in 1 2 3; do keys k; w 0.3; cap 08-k$i; done
for i in 1 2; do keys h; w 0.3; cap 09-h$i; done
for i in 1 2 3; do keys .; w 0.3; cap 10-sel$i; done
w 1.5; cap 11-hold
stop
