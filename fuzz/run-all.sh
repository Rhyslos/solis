#!/usr/bin/env bash
# Runs every fuzz target for SECONDS each (default 600), LANES at a time
# (default 2), starting from fuzz/seeds and keeping what each finds in
# fuzz/corpus/<target>. Crashes land in fuzz/artifacts/<target>.
# Run from the repository's top folder. Needs cargo-fuzz and a nightly
# compiler (or RUSTC_BOOTSTRAP=1 with stable, as some machines must).
set -uo pipefail
seconds="${SECONDS_EACH:-600}"
lanes="${LANES:-2}"
cd fuzz
targets=$(cargo fuzz list)
cargo fuzz build -O >/dev/null 2>&1 || { echo "build failed"; exit 1; }
run() {
    local t="$1"
    mkdir -p "corpus/$t" "artifacts/$t"
    cargo fuzz run -O "$t" "corpus/$t" "seeds/$t" -- \
        -max_total_time="$seconds" -timeout=20 -rss_limit_mb=4096 \
        -max_len=1048576 -print_final_stats=1 > "artifacts/$t.log" 2>&1
    local status=$?
    local execs
    execs=$(grep -m1 "stat::number_of_executed_units" "artifacts/$t.log" | awk '{print $2}')
    echo "$t: exit $status, ${execs:-?} runs, $(ls "corpus/$t" | wc -l) in corpus, $(ls "artifacts/$t" | wc -l) found"
}
export -f run
export seconds
printf '%s\n' $targets | xargs -P "$lanes" -I{} bash -c 'run {}'
