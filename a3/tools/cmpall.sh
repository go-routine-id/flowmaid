#!/usr/bin/env bash
# Build the a3 worktree, render the corpus, list files that differ from runs/a3-head (== baseline for flowcharts).
S=/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad
R=$S/wt/a3-state
(cd $R && cargo build --release -q --bin flowmaid) || exit 1
mkdir -p $S/a3/now
for f in $S/cmp/corpus/*.mmd; do n=$(basename $f .mmd); $R/target/release/flowmaid $f > $S/a3/now/$n.svg 2>/dev/null; cmp -s $S/a3/now/$n.svg $S/runs/a3-head/$n.svg || echo "DIFF $n"; done
# also quality fixtures and examples
for f in $R/tests/fixtures/quality/*.mmd $R/examples/*.mmd; do n=$(basename $f .mmd); d=$(basename $(dirname $f)); mkdir -p $S/a3/now/$d $S/a3/head/$d; $R/target/release/flowmaid $f > $S/a3/now/$d/$n.svg 2>/dev/null; if [ -f $S/a3/head/$d/$n.svg ]; then cmp -s $S/a3/now/$d/$n.svg $S/a3/head/$d/$n.svg || echo "DIFF $d/$n"; fi; done
echo done
