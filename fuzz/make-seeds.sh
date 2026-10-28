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
# Sound files: the sound crate's Ogg test files, and small WAVs of each
# kind it reads (fuzz_targets/sound_file.rs).
mkdir -p "$seeds/sound_file"
cp crates/sound/tests/data/*.ogg "$seeds/sound_file/"
python3 - "$seeds/sound_file" <<'PY'
import math, struct, sys
def wav(name, rate, channels, bits, fmt=1, frames=200):
    width = bits // 8
    data = bytearray()
    for i in range(frames * channels):
        v = math.sin(i / 7)
        if fmt == 3: data += struct.pack('<f', v)
        elif bits == 8: data += bytes([int(128 + v * 100)])
        elif bits == 24: data += int(v * 8_000_000).to_bytes(3, 'little', signed=True)
        else: data += int(v * (2 ** (bits - 2))).to_bytes(width, 'little', signed=True)
    head = struct.pack('<4sI4s4sIHHIIHH4sI', b'RIFF', 36 + len(data), b'WAVE', b'fmt ', 16,
                       fmt, channels, rate, rate * channels * width, channels * width, bits,
                       b'data', len(data))
    open(f"{sys.argv[1]}/{name}.wav", 'wb').write(head + data)
wav('mono_8bit', 22050, 1, 8)
wav('stereo_16bit', 44100, 2, 16)
wav('mono_24bit', 48000, 1, 24)
wav('stereo_32bit', 48000, 2, 32)
wav('stereo_float', 44100, 2, 32, fmt=3)
PY
echo "Seeds in $seeds:"
for d in "$seeds"/*/; do echo "  $(basename "$d"): $(ls "$d" | wc -l) files"; done
