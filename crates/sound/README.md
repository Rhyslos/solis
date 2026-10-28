# sound

**Layer 1 · Engine**

Sound ([decision 0056](../../docs/decisions/0056-sound-with-kira.md)): Kira wrapped, as `physics` wraps Rapier, so nothing else in Solis sees Kira's types.

## Purpose

Read sound files and play them, without ever changing the game.

- **Reading** (`read.rs`): WAV through `hound` and Ogg Vorbis through `lewton`, with Kira's own reader (`symphonia`, MPL-2.0) switched off.
  - Format by the file's first bytes, not its name.
  - Limits: files up to 256 MB, sample rates 8,000 to 192,000 Hz, up to 8 channels. Mono plays on both sides, stereo as it is, and more channels are mixed down to one.
  - Short sounds (up to 10 minutes) are decoded into memory. Longer ones are measured once and streamed from their file as they play.
  - Damage never crashes: a cut-off WAV keeps its whole frames, samples that aren't numbers become silence, and everything else is a plain error.
- **Overviews** (`overview.rs`): each sound's loudness over its length, at most 2,048 pairs of lowest and highest sample, made in the same pass as decoding, for drawing waveforms.
- **Buses** (`mixer.rs`, `buses.rs`): the project's buses in a tree under Master, each a Kira track with a volume control (its fader, game code's share, ducking, mute and solo, worked out by `mixer::levels` and `mixer::Ducker`) and then a meter.
- **The device** (`device.rs`): Kira's audio manager on the sound card, opened on a worker by the app. With no sound card, a silent device plays nothing and Solis carries on. Tests use Kira's mock device, so CI needs no sound card.
  - **Previews** of sound files play on an **Editor** track of their own, which the game's buses (S4) never touch.

## Must never

- Change game state, or be part of the checksum: sound is output only.
- Crash, or read past a limit, on a damaged or hostile file.
- Make the frame wait: files are read on workers, and Kira mixes on its own thread.
- Show Kira's types outside this crate.
- Know about the editor or the GPU.

## Depends on

`area` (its settings-file reader, for `.solsound` files) and `reflect` (asset IDs and values), Kira (`cpal` backend only), hound, lewton, ogg, log.

## Status

Phase 7, S1: reading WAV and Ogg (and measuring without keeping the samples, `measure_file`), overviews, Ogg header checks before `lewton` (`vorbis_check.rs`, after fuzzing found two files that made it ask for hundreds of gigabytes), the device, and the editor's preview. Phase 7, S2: sound events (`event.rs`: reading, writing, limits, and picking a play from sound's own random numbers) and previews varied in volume and pitch (`SoundDevice::preview_varied`). Phase 7, S3: the game's sounds (`game.rs`): one-shots and sounds on entities on Kira spatial tracks around a listener, paused with the game and faded out on Stop. Phase 7, S4: buses (`mixer.rs`: the project's `mixer.solmix`, levels and ducking; `buses.rs`: a Kira track per bus with a volume control and a meter). Phase 7, S5: loops (`Voice`: looping, a start part way through, counted by event) and choosing what's heard (`voices.rs`). Phase 7, S6: reverb zones (`reverb.rs`, and four reverb send tracks in `game.rs` that placed sounds send to, scaled by their buses).
