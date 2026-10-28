#!/usr/bin/env bash
# Breaking changes must be labelled (Phase 8, C5; decision 0053).
#
# Usage: breaking-check.sh <base commit> <head commit>
#
# A pull request is breaking when it
#   - adds a new version of a file format: a new tests/fixtures/<format>/v<N>/
#     folder (a format's every version keeps an example there, and a test
#     fails without one), or
#   - takes something away from what game code can call: a line removed or
#     changed in docs/game-code-interface.txt (lines added are not breaking).
# A breaking pull request must add a line under "### Breaking" in the
# Unreleased part of CHANGELOG.md, saying what changed and what to do.
# Prints why, and exits 1, when one is missing.
set -euo pipefail

base="$1"
head="$2"
reasons=()

# New format versions: a format whose newest example folder (v<N>) is newer
# than before. A format's first examples are new, not breaking.
newest() {
    git ls-tree -r --name-only "$1" -- tests/fixtures/ |
        sed -nE 's#^tests/fixtures/([^/]+)/v([0-9]+)/.*#\1 \2#p' |
        sort -k1,1 -k2,2n | awk '{ newest[$1] = $2 } END { for (f in newest) print f, newest[f] }' |
        sort
}
while read -r format version; do
    before="$(newest "$base" | awk -v f="$format" '$1 == f { print $2 }')"
    if [ -n "$before" ] && [ "$version" -gt "$before" ]; then
        reasons+=("a new version of a file format: $format, from $before to $version")
    fi
done < <(newest "$head")

# Lines taken away from what game code can call.
if git cat-file -e "$base:docs/game-code-interface.txt" 2>/dev/null; then
    gone="$(git diff "$base" "$head" -- docs/game-code-interface.txt |
        grep -E '^-[^-#]' || true)"
    if [ -n "$gone" ]; then
        reasons+=("something taken away from what game code can call:")
        while IFS= read -r line; do
            reasons+=("    ${line#-}")
        done <<<"$gone"
    fi
fi

if [ ${#reasons[@]} -eq 0 ]; then
    echo "Not a breaking change."
    exit 0
fi

# Lines this pull request adds under "### Breaking" in the Unreleased part.
breaking_lines="$(git show "$head:CHANGELOG.md" | awk '
    /^## / { unreleased = ($0 ~ /^## Unreleased/); section = ""; next }
    /^### / { section = $0; next }
    unreleased && section == "### Breaking" && NF { print }
')"
added="$(git diff "$base" "$head" -- CHANGELOG.md | grep -E '^\+[^+]' | cut -c2- || true)"
labelled=false
while IFS= read -r line; do
    if [ -n "$line" ] && grep -qxF -- "$line" <<<"$breaking_lines"; then
        labelled=true
        break
    fi
done <<<"$added"

echo "This pull request is breaking:"
printf '  %s\n' "${reasons[@]}"
if $labelled; then
    echo "It says so under \"### Breaking\" in CHANGELOG.md."
    exit 0
fi
echo "::error::This pull request is breaking but CHANGELOG.md has no new line under \"### Breaking\" in Unreleased. Say what changed, that older files still open (their upgrader), or what game code must change, and until when an old name keeps working."
exit 1
