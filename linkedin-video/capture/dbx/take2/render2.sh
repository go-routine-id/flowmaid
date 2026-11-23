#!/bin/bash
# render take2/caps/*.ansi -> take2/png/<name>.png (2240x1504 @ DSF 2)
W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/work-dbx
W2=$W/take2
COLS=100; ROWS=29; FS=18; LH=24; PW=1120; PH=752
mkdir -p $W2/html $W2/png; rm -f $W2/html/* $W2/png/*
echo -n "[" > $W2/list.json; first=1
for f in $W2/caps/*.ansi; do
  n=$(basename $f .ansi)
  python3 $W/ansi2html.py $f $W2/html/$n.html $COLS $ROWS $FS $LH $PW $PH dbx
  [ $first = 1 ] || echo -n "," >> $W2/list.json; first=0
  echo -n "[\"$W2/html/$n.html\",\"$W2/png/$n.png\"]" >> $W2/list.json
done
echo "]" >> $W2/list.json
cd $W2 && node $W/shoot.js $W2/list.json $PW $PH 2
