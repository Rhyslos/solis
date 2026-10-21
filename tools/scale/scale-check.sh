#!/usr/bin/env bash
# The scale check (decision 0011): times the editor and the viewport against
# their targets, and prints a table for the pull request.
#
# Not part of the normal test run: run it at the end of every phase (as
# part of the gate) and whenever the project lead asks. It builds in
# release mode, as the editor ships.
#
# Usage:
#   tools/scale/scale-check.sh
#
# Today it times one frame of the editor's interface and one viewport frame
# at 1920x1080. From Phase 4 it grows into the scale target in DESIGN.md §2
# (2,000 units and 20,000 projectiles at 60 frames a second). The viewport
# time depends on the graphics card, so it is reported, not judged.
set -euo pipefail

failed=0
run() {
    local out
    if ! out=$(cargo test --release --locked -q "$@" -- --ignored --nocapture --test-threads=1 2>&1); then
        failed=1
    fi
    grep -E "Scale check|SLOW|panicked|skipping" <<<"$out" || true
}
run -p editor scale_check
run -p render --test gpu scale_check
if [ "$failed" -ne 0 ]; then
    echo "Something missed its target (marked SLOW above), or a check failed."
    exit 1
fi
echo "Scale check passed."
