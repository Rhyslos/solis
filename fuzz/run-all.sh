#!/usr/bin/env bash
# Runs every fuzz target (or those TARGETS names) for SECONDS_EACH each (default 600), LANES at a time
# (default 2), starting from fuzz/seeds and keeping what each finds in
# fuzz/corpus/<target>. Crashes land in fuzz/artifacts/<target>. A target
# with no folder in fuzz/seeds starts from nothing.
# Run from the repository's top folder. Needs cargo-fuzz and a nightly
# compiler (or RUSTC_BOOTSTRAP=1 with stable, as some machines must).
set -uo pipefail
seconds="${SECONDS_EACH:-600}"
lanes="${LANES:-2}"
# The fuzzers' lock file, written from the main one so they build the
# library versions the program ships (fuzz/README.md, The lock file).
tools/ci/fuzz-lock.sh || exit 1
cd fuzz
targets="${TARGETS:-$(cargo fuzz list)}"
cargo fuzz build -O >/dev/null 2>&1 || { echo "build failed"; exit 1; }
run() {
    local t="$1"
    mkdir -p "corpus/$t" "artifacts/$t"
    # The fuzzer stops at once if handed a folder that isn't there.
    local folders=("corpus/$t")
    if [ -d "seeds/$t" ]; then folders+=("seeds/$t"); fi
    cargo fuzz run -O "$t" "${folders[@]}" -- \
        -max_total_time="$seconds" -timeout=45 -rss_limit_mb=4096 \
        -max_len=1048576 -print_final_stats=1 > "artifacts/$t.log" 2>&1
    local status=$?
    local execs
    execs=$(grep -m1 "stat::number_of_executed_units" "artifacts/$t.log" | awk '{print $2}')
    echo "$t: exit $status, ${execs:-?} runs, $(ls "corpus/$t" | wc -l) in corpus, $(ls "artifacts/$t" | wc -l) found"
}
export -f run
export seconds
printf '%s\n' $targets | xargs -P "$lanes" -I{} bash -c 'run {}'
