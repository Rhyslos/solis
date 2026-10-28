# sound/src

Sound (decision 0056): the Kira library does the playing and mixing, wrapped here so nothing else sees its types. WAV and Ogg Vorbis files are read here, within limits, never crashing on a damaged file.

**Start reading at** `read.rs`, then `device.rs`.

| File | What it does | Main items |
| --- | --- | --- |
| `lib.rs` | Lists the modules and what the crate offers. | |
| `read.rs` | Reads WAV (through `hound`) and Ogg Vorbis (through `lewton`) within limits. Short sounds are decoded into memory; long ones are measured once and streamed from their file. | `Clip`, `ClipInfo`, `Samples`, `Format`, `ReadError`, `read_file`, `measure_file` |
| `vorbis_check.rs` | Checks an Ogg Vorbis file's headers before `lewton` reads them, because a damaged header can make it ask for more memory than there is. | |
| `overview.rs` | A sound's loudness over its length, small enough to keep for every sound: the waveform the Library and Properties draw. | `Overview` |
| `device.rs` | The sound device on the sound card (or a test device, or none), and the editor's previews on a track of their own, varied in volume and pitch for sound events. | `SoundDevice` |
| `event.rs` | Sound events (`.solsound`): reading and writing them within limits, and picking each play (a take, its volume and pitch) from sound's own random numbers. | `SoundEvent`, `Pick`, `read`, `write`, `pick` |
| `game.rs` | The game's sounds on the device: played flat, at a place or on an entity, through their bus, around the listener (the game's camera); paused, stepped and stopped with the game. | `Placement`, `Spot`, `within_reach`, `SoundDevice::play_game_sound`, `SoundDevice::set_mixer` |
| `mixer.rs`, `mixer_tests.rs` | The project's buses (`mixer.solmix`): reading and writing them within limits into a sound tree under Master, editing it, each bus's level from its fader, mute, solo and game code's share, and ducking as it goes. Plain data and sums. | `Mixer`, `Bus`, `Ducking`, `Listening`, `levels`, `Ducker` |
| `buses.rs` | The buses on Kira's tracks: one track per bus, nested as they are, each with a volume control then a meter, so meters read after the fader. | `BusTree` (inside the crate) |
| `event_tests.rs` | Tests for sound events: round trips, mistakes refused with their line, and picks within their ranges, never the same take twice running. | |
| `tests.rs` | Tests for reading sound files, their overviews and the device, including the damaged files in [`../tests/data`](../tests/README.md). | |

**Read next:** the crate's [README](../README.md).
