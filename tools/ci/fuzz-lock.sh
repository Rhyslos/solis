#!/usr/bin/env bash
# Writes the fuzzers' lock file (fuzz/Cargo.lock) from the main Cargo.lock,
# so the fuzzers build the library versions the program ships (the review
# of November 2026).
#
# Usage: fuzz-lock.sh        (from anywhere; it takes no arguments)
#
# fuzz/ is a workspace of its own, so Cargo wants a lock file there, and
# nothing in Cargo keeps a second lock file in step with the first. Kept in
# the repository, it drifted: the weekly fuzzing vetted versions of 15
# libraries that were not the ones in the program. So it isn't kept
# (fuzz/.gitignore). There is one lock file, the main one, and this script
# writes the fuzzers' from it before every build of them: a copy of the
# main lock, from which Cargo takes out what the fuzzers don't build and to
# which it adds what only they use (libfuzzer-sys and arbitrary, pinned to
# exact versions in fuzz/Cargo.toml).
#
# Run it as often as you like: it starts from the main lock every time,
# whatever fuzz/Cargo.lock was there. Nothing is built, and little is
# fetched (the registry's index, and the one library that comes from a git
# repository); with no network it uses what is already downloaded.
#
# Exits 1, saying why and leaving no fuzz/Cargo.lock, when Cargo can't work
# the file out, or when the result has another version of a package than
# the main lock (something only the fuzzers use needs a newer one).
set -euo pipefail

cd "$(dirname "$0")/../.."
main_lock=Cargo.lock
fuzz_lock=fuzz/Cargo.lock

# One annotation on the run's page in CI (GitHub reads %0A as a new line),
# plain lines anywhere else. A lock file that isn't right is never left.
fail() {
    local message="$1"
    rm -f "$fuzz_lock"
    if [ "${GITHUB_ACTIONS:-}" = "true" ]; then
        message=${message//'%'/'%25'}
        message=${message//$'\n'/'%0A'}
        echo "::error title=The fuzzers' lock file couldn't be written::$message"
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

# The fuzzers' packages set against the main lock's. `differ` gives a line
# for each package both name that the fuzzers have at a version, or from a
# place, the main lock doesn't; `own` gives the libraries only the fuzzers
# use.
compare() {
    awk -v want="$1" '
        NR == FNR {
            known[$1] = 1
            has[$0] = 1
            versions[$1] = versions[$1] sep[$1] $2
            sep[$1] = " and "
            next
        }
        !($1 in known) { if (want == "own" && $3 != "here") print $1, $2; next }
        want == "differ" && !($0 in has) {
            printf "%s: the fuzzers would build %s, the program builds %s\n", $1, $2, versions[$1]
        }
    ' <(packages "$main_lock") <(packages "$fuzz_lock") | LC_ALL=C sort -u
}

if [ $# -gt 0 ]; then
    echo "Usage: $0   (no arguments: it writes $fuzz_lock from $main_lock)" >&2
    exit 2
fi
[ -f "$main_lock" ] || fail "$main_lock is missing, so there is nothing to write $fuzz_lock from."

# Cargo keeps every version the copy names that the fuzzers need, drops the
# packages they don't, and adds the ones missing. Tried again without the
# network if that fails, which works when the libraries are downloaded.
resolve() {
    cp "$main_lock" "$fuzz_lock"
    cargo update --quiet --workspace --manifest-path fuzz/Cargo.toml "$@" 2>&1
}
if ! said="$(resolve)"; then
    if ! said_offline="$(resolve --offline)"; then
        fail "Cargo couldn't work out $fuzz_lock from $main_lock. It said:
$said
And without the network:
$said_offline"
    fi
    echo "No network: $fuzz_lock was worked out from the libraries already downloaded."
fi

found="$(compare differ)"
if [ -n "$found" ]; then
    fail "Written from $main_lock, $fuzz_lock would still have other versions than the program ships:
$found
Something only the fuzzers use (fuzz/Cargo.toml) needs a newer version of these than $main_lock has. Update them in the main lock first (cargo update -p <name>), or pin the fuzzers' library to a version that accepts the main lock's."
fi

count="$(packages "$fuzz_lock" | wc -l | tr -d ' ')"
own="$(compare own | paste -sd, - | sed 's/,/, /g')"
echo "Wrote $fuzz_lock from $main_lock: $count packages, every one the program also builds at the program's version. Only the fuzzers use: ${own:-nothing}."
