#!/usr/bin/env bash
# Turns a saved `cargo test` output into one GitHub annotation listing the
# failed tests and why, so the reason shows on the run's page without
# downloading the whole log. Usage: failed-tests.sh test-output.txt
set -u
out=${1:-test-output.txt}
[ -f "$out" ] || exit 0
summary=$(sed 's/\x1b\[[0-9;]*m//g; s/\r//g' "$out" \
  | grep -A6 -E "^---- .* stdout ----" \
  | grep -v -E "^note: run with|^stack backtrace" \
  | head -80)
[ -n "$summary" ] || summary="No test failures found; the build itself may have failed."
summary=${summary//'%'/'%25'}
summary=${summary//$'\n'/'%0A'}
echo "::error title=Failed tests::$summary"
