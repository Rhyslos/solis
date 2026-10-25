#!/usr/bin/env bash
# Writes the fuzzers' starting files into fuzz/seeds/<target>/ from real
# ones: a save, a recording, input actions and an area made by the editor's
# tests, the sample models and their cache entries and sidecars, and the
# samples' game code. Run from the repository's top folder.
set -euo pipefail
seeds="$(pwd)/fuzz/seeds"
mkdir -p "$seeds"
export SOLIS_FUZZ_SEEDS="$seeds"
cargo test --locked -q -p editor write_fuzz_seeds -- --ignored >/dev/null
cargo test --locked -q -p asset_db write_fuzz_seeds -- --ignored >/dev/null
mkdir -p "$seeds/game_code" "$seeds/area_file" "$seeds/project_file"
for f in samples/*/assets/main.lst; do
    cp "$f" "$seeds/game_code/$(basename "$(dirname "$(dirname "$f")")").lst"
done
# Lust, a 0x01 byte, then Lua (fuzz_targets/game_code.rs).
{ cat samples/turret/assets/main.lst; printf '\001'; cat samples/turret/assets/sparks.lua; } \
    > "$seeds/game_code/turret-with-lua.lst"
cp samples/turret/assets/turret.solarea "$seeds/area_file/"
printf 'name = "Harbour"\nformat = 1\n' > "$seeds/project_file/harbour.solis"
# GitHub's answer about a release, a 0x01 byte, then the checksum file
# (fuzz_targets/update_answer.rs).
mkdir -p "$seeds/update_answer"
repo="https://github.com/Rhyslos/solis/releases/download/build-57"
{ printf '{"tag_name":"build-57","draft":false,"prerelease":false,"assets":['
  printf '{"name":"solis-setup.exe","browser_download_url":"%s/solis-setup.exe","size":1234},' "$repo"
  printf '{"name":"solis-setup.exe.sha256","browser_download_url":"%s/solis-setup.exe.sha256","size":82}]}' "$repo"
  printf '\001%064d  solis-setup.exe\n' 0; } > "$seeds/update_answer/release.json"
echo "Seeds in $seeds:"
for d in "$seeds"/*/; do echo "  $(basename "$d"): $(ls "$d" | wc -l) files"; done
