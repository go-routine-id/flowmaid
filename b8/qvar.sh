#!/usr/bin/env bash
# run quality tests + casing gates under env toggles; print failures briefly
cd /home/user/flowmaid
echo "=== $*"
env "$@" timeout 900 cargo test --release --test quality 2>&1 | grep -E "> ceiling|> [0-9]+ drag|< floor|routed lbl|xgap|static [0-9]|routed [0-9]|panicked" | grep -v "^thread" | head -30
env "$@" timeout 900 cargo test --release --lib -- casing:: scene::tests::route_lanes scene::tests::note_links scene::tests::far_apart scene::tests::a_wide_strokes scene::tests::lr_colliding 2>&1 | grep -E "FAILED|test result"
