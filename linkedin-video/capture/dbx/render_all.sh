#!/bin/bash
# render every caps/*.ansi to png/<name>.png at the given terminal size
W=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/work-dbx
COLS=${COLS:-120}; ROWS=${ROWS:-34}; FS=${FS:-15}; LH=${LH:-20}
PW=${PW:-1120}; PH=${PH:-752}
mkdir -p $W/html $W/png; rm -f $W/html/* $W/png/*
echo -n "[" > $W/list.json; first=1
for f in $W/caps/*.ansi; do
  n=$(basename $f .ansi)
  python3 $W/ansi2html.py $f $W/html/$n.html $COLS $ROWS $FS $LH $PW $PH dbx
  [ $first = 1 ] || echo -n "," >> $W/list.json; first=0
  echo -n "[\"$W/html/$n.html\",\"$W/png/$n.png\"]" >> $W/list.json
done
echo "]" >> $W/list.json
cd $W && node shoot.js list.json $PW $PH 2
