#!/usr/bin/env bash
V=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/rel0402/verify-1-api
cd "$V" || exit 1
tail -n +7 rp_files.txt | split -l 12 - rpb2/batch_
ls rpb2/batch_?? | nice -n 15 xargs -P 2 -I{} sh -c 'timeout 900 ./rp/target/release/rp $(cat {}) > {}.out 2> {}.err; echo "exit $?" >> {}.out'
echo ALLDONE
