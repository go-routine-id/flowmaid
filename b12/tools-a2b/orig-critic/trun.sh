#!/usr/bin/env bash
# trun.sh <label> [ENV=val ...]: suite-on lib + quality tests of the critic stack under env -> t/<label>.txt/.fail
C=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/b12/design-a2b/critic
lab=$1; shift; mkdir -p $C/t
lib=$(grep -o 'unittests src/lib.rs ([^)]*)' $C/build.log | sed 's/.*(//; s/)//' | tail -1); q=$(grep -o 'tests/quality.rs ([^)]*)' $C/build.log | sed 's/.*(//; s/)//' | tail -1)
cd $C/wt && { env "$@" timeout 1500 $lib --test-threads=4 ; env "$@" timeout 1500 $q --test-threads=4 ; } > $C/t/$lab.txt 2>&1
grep -E "^test .* FAILED$" $C/t/$lab.txt | sed 's/^test //; s/ \.\.\. FAILED//' | sort > $C/t/$lab.fail
echo "$lab: $(grep 'test result' $C/t/$lab.txt | tr '\n' ' ')"
