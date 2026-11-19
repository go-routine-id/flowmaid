#!/usr/bin/env bash
# instructions per route() call: (I(n=11) - I(n=1)) / 10 ; usage: icount.sh <binary> <file>
B=$1; F=$2
a=$(valgrind --tool=callgrind --callgrind-out-file=/dev/null $B $F 1 2>&1 | grep refs | awk '{gsub(",","",$4); print $4}')
b=$(valgrind --tool=callgrind --callgrind-out-file=/dev/null $B $F 6 2>&1 | grep refs | awk '{gsub(",","",$4); print $4}')
echo "$(basename $F .mmd) route_Minstr=$(( (b - a) / 5 / 1000000 ))"
