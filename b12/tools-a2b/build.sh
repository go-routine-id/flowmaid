#!/usr/bin/env bash
# build.sh <src-tree> <target-dir> <bindir>
#   Builds flowmaid's suite-on binaries (release, RUSTFLAGS='--cfg flowmaid_blocks') from <src-tree> with
#   CARGO_TARGET_DIR=<target-dir> and copies them into <bindir>:
#     flowmaid       the CLI (renders the 95 local inputs)
#     quality        examples/quality (the 332 + 32 direction copies, static and routed)
#     lib-tests      the lib unit-test binary       (cargo test --lib)
#     quality-tests  the tests/quality.rs binary    (cargo test --test quality)
#     BUILD-INFO     source tree, commit, dirty file count, rustc, time
#   The two test binaries read their fixtures from <src-tree> (CARGO_MANIFEST_DIR is compiled in), so they run
#   only while that tree exists. CARGO_BUILD_JOBS defaults to 2.
set -euo pipefail
src=$(cd "${1:?src tree}" && pwd); tgt=${2:?target dir}; bin=${3:?bindir}
mkdir -p "$tgt" "$bin"; tgt=$(cd "$tgt" && pwd); bin=$(cd "$bin" && pwd)
export RUSTFLAGS='--cfg flowmaid_blocks' CARGO_TARGET_DIR="$tgt" CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
log=$bin/build.log
( cd "$src" && cargo test --release --no-run --lib --test quality ) > "$log" 2>&1 || { tail -30 "$log" >&2; exit 1; }
( cd "$src" && cargo build --release --bin flowmaid --example quality ) >> "$log" 2>&1 || { tail -30 "$log" >&2; exit 1; }
lib=$(grep -o 'Executable unittests src/lib.rs ([^)]*)' "$log" | sed 's/.*(//; s/)$//' | tail -1)
qt=$(grep -o 'Executable tests/quality.rs ([^)]*)' "$log" | sed 's/.*(//; s/)$//' | tail -1)
[ -x "$lib" ] && [ -x "$qt" ] || { echo "build.sh: test executables not found in $log" >&2; exit 1; }
cp "$tgt/release/flowmaid" "$bin/flowmaid"; cp "$tgt/release/examples/quality" "$bin/quality"
cp "$lib" "$bin/lib-tests"; cp "$qt" "$bin/quality-tests"
{ echo "src=$src"
  echo "commit=$(git -C "$src" rev-parse HEAD 2>/dev/null || echo none)"
  echo "dirty_files=$(git -C "$src" status --porcelain 2>/dev/null | wc -l)"
  echo "rustflags=$RUSTFLAGS"
  echo "rustc=$(rustc --version)"
  echo "built=$(date -u +%FT%TZ)"; } > "$bin/BUILD-INFO"
echo "build.sh: $(grep -E 'Finished' "$log" | tail -1 | sed 's/^ *//'); binaries in $bin" >&2
