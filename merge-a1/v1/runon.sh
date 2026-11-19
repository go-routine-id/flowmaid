#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/merge-a1/v1
( nice -n 19 $V/probe-dev/target/release/vprobe mid $V/rest-big.txt > $V/restB-dev.tsv 2>/dev/null; nice -n 19 $V/probe-head/target/release/vprobe mid $V/rest-big.txt > $V/restB-head.tsv 2>/dev/null; touch $V/restB.done ) &
( nice -n 19 $V/probe-a1on/target/release/vprobe fullon $V/all-small-0.txt > $V/on-a1.0.tsv 2>/dev/null; touch $V/on-a1.0.done ) &
( nice -n 19 $V/probe-headon/target/release/vprobe fullon $V/all-small-0.txt > $V/on-head.0.tsv 2>/dev/null; touch $V/on-head.0.done ) &
wait
( nice -n 19 $V/probe-a1on/target/release/vprobe fullon $V/all-small-1.txt > $V/on-a1.1.tsv 2>/dev/null; touch $V/on-a1.1.done ) &
( nice -n 19 $V/probe-headon/target/release/vprobe fullon $V/all-small-1.txt > $V/on-head.1.tsv 2>/dev/null; touch $V/on-head.1.done ) &
( nice -n 19 $V/probe-a1on/target/release/vprobe midon $V/all-big.txt > $V/onB-a1.tsv 2>/dev/null; touch $V/onB-a1.done ) &
( nice -n 19 $V/probe-headon/target/release/vprobe midon $V/all-big.txt > $V/onB-head.tsv 2>/dev/null; touch $V/onB-head.done ) &
wait
