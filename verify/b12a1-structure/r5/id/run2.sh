#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/verify/b12a1-structure/r5
cd $V/id
nice $V/tp-pre/release/ident $(cat part-ac2) > pre-part-ac2.tsv 2> pre-part-ac2.err &
nice $V/tp-now/release/ident $(cat part-aa.f) > now-part-aa.tsv 2> now-part-aa.err &
nice $V/tp-now/release/ident $(cat part-ab.f) > now-part-ab.tsv 2> now-part-ab.err &
wait
nice $V/tp-now/release/ident $(cat part-ac.f) > now-part-ac.tsv 2> now-part-ac.err
echo ID-DONE
