#!/usr/bin/env bash
# The fuzzers must build the same library versions the program ships (the
# review of November 2026).
#
# Usage: fuzz-lock-check.sh            compare the two lock files
#        fuzz-lock-check.sh --locked   the same, then ask Cargo whether a
#                                      build would change fuzz/Cargo.lock
#        fuzz-lock-check.sh --fix      bring fuzz/Cargo.lock into step
#
# fuzz/ is a workspace of its own, so it has its own lock file, and nothing
# in Cargo keeps that file in step with the main Cargo.lock. Left alone, the
# two drift apart, and the weekly fuzzing vets versions of libraries that
# are not the ones in the program.
#
# The comparison: every package fuzz/Cargo.lock names that the main
# Cargo.lock also names must be at a version the main one has, from the
# same place. Packages only the fuzzers use (libfuzzer-sys and what it
# needs) are left alone, and so are packages the main lock has that the
# fuzzers don't build. It needs no Cargo, so it can run before anything
# else.
#
# --locked adds what the comparison can't see: a package the fuzzers now
# need that their lock file doesn't name yet, or one it names that nothing
# uses any more. `cargo fuzz` has no --locked of its own, and its build
# would quietly rewrite the file, taking the newest version of whatever it
# lacks, so Cargo is asked first. Nothing is built, and little is fetched:
# the registry's index, and the one library that comes from a git
# repository.
#
# --fix copies the main lock over the fuzzers' and lets Cargo take out what
# the fuzzers don't use and add what only they use (at its newest version).
# Commit the fuzz/Cargo.lock it leaves.
#
# Prints every package that is out of step and how to fix it; exits 1 then.
set -euo pipefail

cd "$(dirname "$0")/../.."
main_lock=Cargo.lock
fuzz_lock=fuzz/Cargo.lock
mode="${1:-}"

# One annotation on the run's page in CI (GitHub reads %0A as a new line),
# plain lines anywhere else.
fail() {
    local message="$1"
    if [ "${GITHUB_ACTIONS:-}" = "true" ]; then
        message=${message//'%'/'%25'}
        message=${message//$'\n'/'%0A'}
        echo "::error title=The fuzzers' lock file is out of step::$message"
    else
        echo "$message" >&2
    fi
    exit 1
}

# "name version place" for every package in a lock file. A package of the
# repository's own has no place; it is written "here".
packages() {
    awk -F'"' '
        function flush() {
            if (name != "") print name, version, (place == "" ? "here" : place)
            name = version = place = ""
        }
        /^\[/ { flush(); in_package = ($0 == "[[package]]") }
        in_package && /^name = / { name = $2 }
        in_package && /^version = / { version = $2 }
        in_package && /^source = / { place = $2 }
        END { flush() }
    ' "$1"
}

# One line for each package that is out of step; nothing when all is well.
differences() {
    awk '
        NR == FNR {
            known[$1] = 1
            has[$1 " " $2 " " $3] = 1
            place[$1 " " $2] = $3
            versions[$1] = versions[$1] sep[$1] $2
            sep[$1] = " and "
            next
        }
        !($1 in known) || (($1 " " $2 " " $3) in has) { next }
        ($1 " " $2) in place {
            printf "%s %s: the fuzzers take it from %s, the program from %s\n", $1, $2, $3, place[$1 " " $2]
            next
        }
        { printf "%s: the fuzzers build %s, the program %s\n", $1, $2, versions[$1] }
    ' <(packages "$main_lock") <(packages "$fuzz_lock") | LC_ALL=C sort -u
}

case "$mode" in
"" | --locked | --fix) ;;
*)
    echo "Usage: $0 [--locked | --fix]" >&2
    exit 2
    ;;
esac

for lock in "$main_lock" "$fuzz_lock"; do
    [ -f "$lock" ] || fail "$lock is missing, so the fuzzers' libraries can't be checked against the program's."
done

if [ "$mode" = "--fix" ]; then
    before="$(mktemp)"
    cp "$fuzz_lock" "$before"
    cp "$main_lock" "$fuzz_lock"
    if ! cargo update --workspace --manifest-path fuzz/Cargo.toml; then
        cp "$before" "$fuzz_lock"
        rm -f "$before"
        fail "Cargo could not finish $fuzz_lock (its message is above), so the file is as it was."
    fi
    rm -f "$before"
fi

found="$(differences)"
if [ -n "$found" ]; then
    count="$(wc -l <<<"$found" | tr -d ' ')"
    if [ "$mode" = "--fix" ]; then
        # Something only the fuzzers use asks for a newer version than the
        # main lock has, so Cargo could not keep the main one.
        advice="Still out of step after the fix: a library only the fuzzers use needs a newer version of these than $main_lock has. Update them in the main lock first (cargo update -p <name>), then run this again."
    else
        advice="To fix, run tools/ci/fuzz-lock-check.sh --fix and commit $fuzz_lock."
    fi
    fail "$fuzz_lock is out of step with $main_lock in $count package(s), so the fuzzers would not build what the program ships:
$found
$advice"
fi

if [ "$mode" = "--locked" ]; then
    cargo update --workspace --locked --manifest-path fuzz/Cargo.toml ||
        fail "A build of the fuzzers would change $fuzz_lock (Cargo's message is above): the fuzzers need a package the file doesn't name, or it names one nothing uses now. To fix, run tools/ci/fuzz-lock-check.sh --fix and commit $fuzz_lock."
fi

echo "The fuzzers build the same library versions as the program ($fuzz_lock is in step with $main_lock)."
