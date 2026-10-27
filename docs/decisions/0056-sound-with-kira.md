# 0056. Sound: Kira, WAV and Ogg, sound events, and sound that never changes the game

- **Status:** Accepted
- **Date:** 2026-10-27 (answers given 2026-10-25)

## Problem

Phase 7 brings sound (ROADMAP.md; DESIGN.md §11):
- sound events with random clips, pitch and volume;
- buses and ducking;
- 3D falloff;
- reverb zones.

Each part has a choice to make. The Phase 7 sound plan ([docs/plans/phase-7-sound.md](../plans/phase-7-sound.md)) asked nine questions with recommendations and alternatives.

## Decision

The project lead took every recommendation:

1. **The sound library is Kira** (0.12, MIT or Apache-2.0), wrapped in a new `sound` crate, as `physics` wraps Rapier. Steam Audio (through walls, bouncing, headphone 3D) may come later.
2. **WAV and Ogg Vorbis**, read with `hound` (Apache-2.0) and `lewton` (MIT or Apache-2.0), with Kira's own reader (`symphonia`, MPL-2.0) switched off, so every dependency stays permissive. `lewton` hasn't had a release since 2021, so Solis's fuzz tests vet it before it's trusted.
3. **One `.solsound` file per sound event**, Lust-style text like areas (decisions 0015 and 0020):
   - clips;
   - volume and pitch, each with a range;
   - bus;
   - 2D or 3D;
   - near and far distances;
   - how many at once;
   - priority;
   - looping.

   Made with **Asset > New sound event**, edited in Properties.
4. **Buses:** Master, with Music, Effects, Voice, Ambience and UI under it. Voice ducks Music. Saved in `mixer.solmix`, and edited in a Mixer panel.
5. **Game code plays and stops sounds; nothing comes back** (`solis/sound`). Loops are started by an `AudioSource` component.
6. **Reverb zones work by where the listener is:** a `ReverbZone` (box or ball, soft edge, preset, strength) gives everything heard its reverb while the camera is inside.
7. **Saves:** nothing extra. `AudioSource` components come back with their entities; one-shot sounds in flight don't.
8. **Up to 64 sounds heard at once** (a project setting, 16 to 256), chosen each tick by priority and loudness. Far loops are kept "virtual" and resume at the right point.
9. **No new keys:** a ▶ button previews sounds and sound events; a speaker button mutes the game; double-click resets a fader.

**Sound never changes the game.** It is the rule all of the above serves:
- requests go one way, landing at the end of the tick in the fixed order (decision 0033);
- sound's randomness is its own stream, made from the seed, the tick and the request's place;
- nothing of sound is in the checksum.

The same recording gives the same checksums with a real sound card, a test device, or none.

## Consequences

- **New dependencies:** `kira`, `hound` and `lewton` join the dependency register (decision 0052), walled into `sound`, with exit plans.
- **Linux CI needs the ALSA headers** (`libasound2-dev`) for Kira's sound-card layer to build.
- **New file kinds:** `.solsound` and `.solmix` join RENAMING.md's list of extensions in the milestones that make them.
- **The Mixer panel is a new editor panel,** so it's mocked up for the project lead before it's built (CLAUDE.md, "UI is mocked up first").
- **Its faders, meters and Properties layout** use the shared pieces (decision 0050).
