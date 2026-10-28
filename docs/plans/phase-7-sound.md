# Phase 7 plan: Sound (draft)

The working plan for Phase 7 (ROADMAP.md, "Phase 7: Sound"; DESIGN.md §11). Phase 7 is sound only since [decision 0059](../decisions/0059-2d-first.md); animation's events come with 2D's frame animation (Phase 9). Animation has its own plan; the two meet at **animation events** (see "Where sound meets animation"). This draft lists the choices still to make, as questions for the project lead, then a plan for every milestone, so a later session can carry on where this one stopped. Once the questions are answered, the answers move into "Choices made before any code" with their decision numbers, and milestones are ticked off here and in the README tracker as they are built.

**Gate:** a character walks and runs, its footsteps (played by game code, decision 0059) each sound a little different, the music dips while someone speaks, steps get quieter and move left and right as the camera moves away and around, and **walking into a reverb zone** (a cave, a hall) makes them echo.

## Answers (the project lead, 2026-10-25)

Every choice below was answered with the recommendation: Kira now and Steam Audio later; WAV and Ogg through permissive readers; one `.solsound` file per event; buses Master > Music, Effects, Voice, Ambience, UI with Voice ducking Music; natives that play and stop and give nothing back, loops through `AudioSource`; reverb by where the listener is; loops come back on load, one-shots don't; 64 sounds at once; no new keys (buttons only). They are [decision 0056](../decisions/0056-sound-with-kira.md).

## The order of Phase 7's milestones

S1, S2, S3 (built), then S4, S5, S6, S8, and S7 last. Animation's A1–A6 left this phase (decision 0059): A4, A5 and A6's frame events go to Phase 9's frame animation, the rest to Phase 16.

## Choices to make before any code

Each question has a recommended answer first, then the alternatives, in plain words.

1. **Which sound library?**
   - **Recommended: Kira alone for Phase 7** (`kira` 0.12.5, MIT or Apache-2.0, released 2026-09-26). It already has everything the roadmap lists: buses (Kira calls them tracks), effects on a bus (reverb, filter, compressor, delay, EQ, volume), sounds placed in 3D around a listener that get quieter with distance and pan left and right, smooth fades, and long files played as they're read (streaming). It's pure Rust, so nothing extra to install or ship.
   - Kira plus **Steam Audio** now (through the `audionimbus` 0.16.0 bindings, MIT or Apache-2.0; Steam Audio itself is Apache-2.0, version 4.8.1). Steam Audio adds realistic sound through walls, sound bouncing off the level, and "headphone 3D" (HRTF, where you can hear whether a sound is above or behind you). But it's a C library: a `phonon.dll` ships with every game, building needs Clang, and Solis would have to write the glue that feeds Steam Audio's output into Kira (the bindings stop at the maths). It's also outside the roadmap's Phase 7 list. **Recommended: later**, as its own decision, when occlusion or headphone 3D is wanted. DESIGN.md §4 and §11 name Steam Audio, so this choice needs a decision record saying "Kira now, Steam Audio later".
   - **Fyrox's sound library** (`fyrox-sound` 1.0.1, MIT): has HRTF and reverb built in, but is made for the Fyrox engine and has a smaller following. **oddio** (0.7.4, MIT or Apache-2.0): light and made for many sounds, but no release since 2023. **Writing our own mixer** on top of `cpal` (the sound-card library, 0.18.2, Apache-2.0, which Kira uses underneath): full control, months of work for what Kira already does.

2. **Which audio file formats, and which code reads them?** This one matters because of the licence rules.
   - Kira's usual file reader is `symphonia` (0.6.1), which is **MPL-2.0**: a "weak copyleft" licence that `deny.toml` doesn't allow (Solis keeps every dependency permissive, so games can use any licence). Kira can be used without it: Solis decodes the file itself and hands Kira the samples.
   - **Recommended: WAV and Ogg Vorbis**, read with `hound` (WAV, 3.5.1, Apache-2.0) and `lewton` (Ogg Vorbis, 0.10.2, MIT or Apache-2.0), with Kira's symphonia switched off. WAV for short effects straight from a sound editor, Ogg for music and anything long (a tenth of the size). Between them they cover what game sound tools export. `lewton` has had no release since 2021, so it gets Solis's own fuzz tests before it's trusted.
   - Allow MPL-2.0 for symphonia alone (one line in `deny.toml` with a reason): gives MP3 and FLAC too, with no extra code. MPL only asks that changes to symphonia's own files be shared, which Solis wouldn't make, but it breaks "everything permissive".
   - Also add FLAC or MP3 through other permissive readers later, if a game maker asks.

3. **Where does a sound event live?**
   - **Recommended: a small text file per event**, `footstep.solsound`, Lust-style like areas (decisions 0015 and 0020), made with **Asset > New sound event** in the Library and edited in Properties when selected: which clips, the volume and pitch variation, the bus, 3D or not, how far it carries, how many may play at once. A file per event merges in Git and gets an asset ID like any asset, so renaming it breaks nothing.
   - One sound bank file holding every event in the project: one place to look, but every change touches the same file (merge conflicts).
   - Settings stored on the audio file's own `.solmeta`: no new file kind, but one clip can't be in two events, and "Footstep" with five clips has no single home.

4. **Which buses come ready-made, and how does ducking work?**
   - **Recommended:** a project file, `mixer.solmix`, starting with **Master** and under it **Music**, **Effects**, **Voice**, **Ambience** and **UI**. Game makers add their own (Engines, Weapons). Ducking rules are written on a bus: "when **Voice** plays, **Music** goes down 8 dB, over 0.2 s, and comes back over 0.6 s after the last voice ends". One ready-made rule: Voice ducks Music.
   - The four buses DESIGN.md names (music, effects, voice, UI) with no Ambience: simpler, but wind and rain end up in Effects, where an explosion's ducking would also dip them.
   - Ducking by a compressor listening to another bus ("sidechain"), as music producers do: smoother, but Kira has no sidechain, so Solis would build it, and it's harder to explain.

5. **What can game code do with sound?** Natives in a new `solis/sound` file, with names that can't clash with Lust's reserved words (as `saveGame` and `loadGame` avoided `save` and `load`):
   - **Recommended:** `playSound(event)` (not placed: UI clicks, music stings), `playSoundAt(event, place)`, `playSoundOn(event, entity)` (follows the entity), `stopSoundsOn(entity)` (with a short fade), and `setBusVolume(bus, volume)` for an options menu. **None of them gives anything back** (see "Sound never changes the game"). Looping sounds (an engine hum, a campfire) come from an **`AudioSource` component** (DESIGN.md §5 lists it): an event, playing or not, volume; game code switches it on and off by setting its fields.
   - Also give back a "sound number" from `playSound…` so a single sound can be stopped or have its pitch changed (an engine revving): more control, but the number has to be made the same way on every PC whether or not there's a sound card, and game code is tempted to ask "is it still playing?", which can't be allowed.
   - Also allow `soundParameter(entity, name, value)` (Wwise-style "RTPC": pitch follows a ship's speed): powerful, but better with adaptive music (DESIGN.md §11, item 4, not in Phase 7).

6. **What does a reverb zone listen to?**
   - **Recommended: where the listener is** (the game's camera). A `ReverbZone` component (a box or a ball, with a soft edge in metres, a preset: small room, hall, cave, sewer, outdoors; and how strong). While the listener is inside, everything heard gets that reverb; across the soft edge it fades from one to the other. This is how Unity's and Unreal's simple reverb zones work, it's cheap (a few reverb buses in all, not one per sound), and it's what the gate asks for when the camera follows the character in.
   - Per sound: each sound gets the reverb of the zone it is in, so a gunshot inside a cave echoes even heard from outside. More realistic, costs a send per sound per zone; better done later with Steam Audio.

7. **What's in a save game?** Decision 0038 says a save holds everything, but sound is not game state.
   - **Recommended:** nothing extra. `AudioSource` components are saved with every other component, so loops and music started by them come back on load; one-shot sounds in flight (a footstep, a shot) are not saved, and bus volumes set by game code are kept with the game's settings, not the save. Loading fades the old sounds out and starts the loaded game's loops.
   - Save every playing sound and where it was up to: a load sounds seamless, but a save then depends on the sound card, and the save's checksum can't include it.

8. **How many sounds at once?**
   - **Recommended: 64 heard at once** (a project setting, 16 to 256), the most important chosen each tick: by a priority on the event, then by loudness at the listener. Sounds too far away to hear are never started. Each event also has its own limit (8 laser shots at once by default) and the same event asked for many times in one tick plays a few copies, a little louder, instead of hundreds. A loop out of earshot is kept "virtual" (counted, not mixed) and comes back in at the right point.
   - 32 (lighter, as on older consoles) or 128 (richer battles, more mixing time). The number only changes how busy a big battle sounds.

9. **Controls (CONTRIBUTING rule 17).** Sound adds no game keys, but the editor gains a few things to press:
   - **Recommended:** in the Library, a ▶ button on a sound or sound event's tile and in its Properties plays it (pressing again stops); **no new keys**. In the Game panel, a speaker button mutes the game's sound. The Mixer panel's faders are dragged like any slider; double-click puts one back to 0 dB (as in most audio programs).
   - Also **Space** to preview the selected sound in the Library (as in Unreal's and Unity's browsers, and the Windows file previewer). Space isn't used by the Library now; it's Jump only while the game has the input.

## Where things stand

- **No sound at all yet.** New projects already put `*.wav` and `*.ogg` in Git LFS (`asset_db::project`), but the asset database knows only models and assemblies (`asset_db::AssetKind`), and the Library shows audio files as "Other".
- **The tick hands out its results** in `Workbench::run_game` (`crates/editor/src/play.rs`): what game code said, spawned, saved and rebound is gathered on the host (`PlayWorld`) during the tick and handled after it, in order. Sound requests fit exactly there: one more list on the host.
- **Natives** are listed in `script::describe::NATIVES` and written as `solis/*.lst` files; `solis/sound.lst` joins them.
- **The camera** (`area::builtin::Camera`) and drawing between ticks (`World::camera_between`, `World::placed_between`) give the listener and every sound's place smoothly, every frame.
- **Animation events** don't exist yet; the animation plan defines them.

## Sound never changes the game

Sound must be heard and never felt by the simulation (decisions 0033 and 0036), so a recording replays and a networked game stays in step with sound on, off, or no sound card:

- **One way only:** game code asks for sounds; the request lands at the end of the tick, in the fixed order (entity, component, the order asked: decision 0033); then the sound crate plays them. No native gives anything back, and no component's field is ever written by the sound system.
- **Sound's own randomness:** picking a clip and its pitch and volume uses a random stream of its own, made from the game's seed, the tick and the request's place in the order. It never draws from game code's random numbers, so adding a footstep can't change what an enemy does. (The same game then also sounds the same each replay, which helps chase bugs.)
- **Not in the checksum:** nothing of sound is in `World::checksum` or the save's sum.
- **Tests:** the same recording with a real sound device, the test ("mock") device, and none at all gives the same checksums tick by tick.

## Safety (CONTRIBUTING rules 8 to 10)

- **Files read get fuzz tests:** WAV, Ogg, `.solsound` and `.solmix` are read within limits and never crash on damage. Limits: an audio file up to 256 MB; 1 or 2 channels (more are mixed down; 3D sounds play mono); sample rates 8,000 to 192,000; a clip held in memory up to 10 minutes, longer ones streamed; decoded sound in memory up to 512 MB in all, with a message past it; up to 32 clips in an event.
- **The frame loop never waits:** files are read and decoded on workers; Kira mixes on its own thread, and the frame only sends it messages without waiting. Opening the sound card happens on a worker when Solis starts.
- **No sound card is fine:** if none is found, or it's unplugged mid-game, the game plays on silently and says so once in the Console; Solis tries again when a device appears.
- **Game code can't flood it:** up to 256 sound requests a tick (more are dropped and said once); a NaN place is refused at the native, as `moveTo` does; an unknown event name is said once, like an unknown assembly.

## The shape of the code

| Crate | Layer | What it gains | Milestone |
|---|---|---|---|
| `sound` (new) | 1 | Kira wrapped (as `physics` wraps Rapier): the device, buses, voices, listener, reverb zones; reading WAV and Ogg; `.solsound` and `.solmix` files | S1–S6 |
| `asset_db` | 1 | Asset kinds `Sound` (WAV, Ogg) and `SoundEvent` (`.solsound`) | S1, S2 |
| `area` | 0 | Built-in components `AudioSource` and `ReverbZone` | S5, S6 |
| `script` | 1 | `solis/sound` natives; the host's list of sound requests | S3 |
| `editor` | 2 | Handing each tick's requests to `sound`; the listener and sound places each frame; preview; the sound event in Properties; the Mixer panel; mute | S1–S6 |
| `app` | 3 | Opening the sound device on a worker; the Library's audio thumbnails (a waveform) | S1 |

`deny.toml` stays as it is with the recommended formats; CI's Linux machine needs the ALSA headers (`libasound2-dev`) for `cpal` to build, and Kira's Linux-only realtime/D-Bus features stay off.

## Milestones

### S1. Sound files and the sound device

- The `sound` crate: Kira 0.12 with symphonia off; WAV through `hound` and Ogg through `lewton`, decoded on workers into Kira's samples; long files streamed.
- The app opens the sound device on a worker; no device is not an error. Tests use Kira's mock device, so CI needs no sound card.
- The Library knows sound files: a waveform tile, length and channels in Properties, and preview (question 9) on an **Editor** bus of its own, never touched by the game's buses.
- Tests: normal files of every supported kind read; edge cases (silence, one sample, 192 kHz, 8-bit, mono and stereo); damaged files refused (fuzz, thousands each); the limits; preview starts and stops. Decision records for questions 1 and 2.

### S2. Sound events as assets

- `.solsound` files (question 3): clips (by asset ID), volume (dB, with a ± range), pitch (semitones, ± range), bus, 2D or 3D, near and far distance and the falloff curve between them (realistic, linear or none), how many at once, priority, looping.
- **Asset > New sound event**, and dropping several audio files on it adds them as clips. Properties edits it; ▶ previews it, picking a different clip and pitch each press, so "nothing sounds repeated" can be heard at once.
- Hot reload: changing a `.solsound` or one of its clips is heard on the next play.
- Tests: round trip (write, read, the same); damaged files refused (fuzz); the picker never plays the same clip twice in a row when it has more than one; variation stays inside its ranges; the same seed, tick and order picks the same clip.

### S3. Game code plays sounds, in 3D

- The natives (question 5) in `solis/sound.lst`, gathered on the host during the tick and handed to `sound` after it, in order (beside saves in `run_game`).
- **3D:** the listener is the Game panel's camera; each sound's place and the listener's are set every frame from the places drawn between ticks, so a passing ship pans smoothly at 144 Hz. Volume falls off between the event's near and far distance and the sound pans by its direction (Kira's spatial tracks). Solis is Z-up; the listener's facing is turned into Kira's convention in one place, tested.
- Pause pauses the game's sounds, Play carries on, Stop fades them out; Step plays the step's sounds. After a stall, only the last few ticks' one-shots play, not a pile-up.
- Lust, Lua and Rust components all reach the same calls; the manual's game-code pages show all three.
- Tests: "Sound never changes the game" (above); a sound to the right is louder on the right; far sounds are quiet and beyond the far distance silent; requests land in the same order on 1 core and 4; the limits and a NaN place.

### S4. Buses, ducking and the Mixer panel ✓

Built as mocked up and approved (2026-10-28): the Mixer is a tab beside the Console, a bus shows in Properties, mute and solo aren't saved, and `setBusVolume` takes a share of the fader (0 to 1) for an options menu. Meters are read after each bus's volume (a volume control, then a meter, on its track).


- `mixer.solmix` (question 4), read within limits (fuzz); buses as Kira tracks under Master, ducking rules carried out by Solis (fades on the ducked bus).
- **The Mixer panel** (Window > Mixer), **mocked up for the project lead first** (CLAUDE.md), using the shared sliders and Properties layout (decision 0050): a fader, mute and solo for each bus, and a level meter while sound plays (the meter asks for frames while it moves, decision 0026). Fader changes are saved to `mixer.solmix`; changes game code makes with `setBusVolume` last for the play.
- The Game panel's mute button.
- Tests: round trip; a bus's volume and mute reach the sounds on it; Voice ducks Music and Music comes back after the last voice; nested buses multiply.

### S5. Loops, voice limits and the scale target ✓

Built (2026-10-28). Game code can't set a built-in component's fields, so an `AudioSource` is switched with a native, `setSoundPlaying(entity, playing)`; its state is in the world's checksum and in saves. Heard at once is `voices` in `mixer.solmix`, set at the top of the Mixer. Kira makes and reuses its own voices, so there's no pool of Solis's own. The Profiler's Sound line is the frame before's (the app works sound out after the editor draws).


- The `AudioSource` component: an event, playing, volume; it follows its entity, starts at Play and on spawn, stops on despawn.
- Choosing what's heard (question 8): each tick, the sounds asked for and the loops near enough are ranked and the best 64 play; far loops go virtual; one event asked many times in a tick plays a few copies, louder. A pool of voices made once at Play, so a busy tick makes nothing new.
- Scale: 2,000 units each with an engine loop and 20,000 projectiles each starting a sound once, in the scale check (decision 0011): choosing and placing must stay under 0.5 ms a tick, and a new Profiler line, **Sound**, shows it.
- Tests: the right 64 are chosen; a virtual loop comes back at the right point; despawned entities' sounds stop; save and load bring loops back (question 7).

### S6. Reverb zones ✓

Built (2026-10-28): four reverb sends, each zone heard taking one (smallest first, so the smaller wins). Only sounds placed in the world send to them; a sound's send is scaled by its bus and the buses above it (sends leave a sound before its bus in Kira), and the sends' own volume is Master's, so faders, ducking, mute and solo reach the echoes.


- The `ReverbZone` component (question 6): box or ball, soft edge, preset, strength; drawn as an outline when selected, like colliders.
- A few reverb buses (up to 4 zones heard at once); each tick the zones around the listener set how much of each everything sends. Overlapping zones: the smaller one wins.
- Tests: inside, the reverb's level is full; outside, none; across the edge it fades smoothly; the presets read and write round trip.

### S8. Listeners and who hears what

Added by [decision 0058](../decisions/0058-listeners-and-who-hears-what.md), built after S6.

- The `Listener` component: on a camera or character; with none, the game's camera hears. Each listener hears from its own place and facing.
- **Heard by** on a sound event: Everyone (default), Only who caused it, or Groups (names a Listener belongs to). `.solsound` gains `heard_by`; files without it are heard by everyone.
- Properties for both are **mocked up for the project lead first**.
- Tests: with no Listener the camera hears; two listeners in different groups each hear only their groups' sounds; "only who caused it" reaches only that listener; the same recording plays the same game whoever hears what.

### S7. The gate's level ✓

Built (2026-10-28) before S8, while S8's mock-up waited for the project lead. The sounds are made in code (`crates/app/src/playground_sounds.rs`, CC0): four footsteps, music, a radio's wordless chatter and a drip. Footsteps come from ground covered (a stride of 0.8 m), so running steps faster.


Built last. (Sounds from animation events moved to Phase 9's frame animation, decision 0059.)

- Footsteps are played by the playground's game code, from the walk input and a step timer that speeds up when running.
- **Help > Make the playground…** (Phase 6) gains footsteps, a music loop, a talking radio (Voice ducking Music) and a cave with a reverb zone, using a small set of sounds made for Solis (licensed CC0, listed in `licenses/`).
- Tests: footsteps play at walk and run speed and vary; the playground plays the same from a recording, sound on or off.

### End of the phase

- The review (decision 0027): three reviewers, fixes in pull requests by kind, the scale check with all three levels.
- The gate, tried by the project lead on their PC, with headphones and speakers.

## Where sound meets animation

Built with 2D's frame animation (Phase 9) and 3D animation (Phase 16), decision 0059. The animation plan owns animation events; sound needs only this from them:

- **Events are found in the tick**, by the animation's own clock (game time, not the frame), so they are deterministic and the same on every PC. An event fires on the tick its marker is crossed, so a footstep is at most 1/60 s (17 ms) from the exact frame, below what people notice; if needed, the sound can be started that fraction later within the frame.
- **When blending** between walk and run, each animation's events fire only while it counts for more than half the blend, so a blend doesn't play two footsteps for one step.
- **An event can carry a sound event's name** (played with no code), and is also delivered to game code as a typed event, so a game can choose the sound itself (gravel or metal underfoot).

## Sources

- Kira 0.12.5, MIT or Apache-2.0: [crates.io](https://crates.io/api/v1/crates/kira), [docs.rs](https://docs.rs/kira/latest/kira/), [effects](https://docs.rs/kira/latest/kira/effect/index.html), [tracks and spatial tracks](https://docs.rs/kira/latest/kira/track/index.html), [features](https://raw.githubusercontent.com/tesselode/kira/main/crates/kira/Cargo.toml), [sound data without symphonia](https://docs.rs/kira/latest/kira/sound/static_sound/struct.StaticSoundData.html)
- symphonia 0.6.1, MPL-2.0: [docs.rs](https://docs.rs/crate/symphonia/latest)
- hound 3.5.1, Apache-2.0: [crates.io](https://crates.io/api/v1/crates/hound); lewton 0.10.2, MIT or Apache-2.0: [docs.rs](https://docs.rs/crate/lewton/latest)
- cpal 0.18.2, Apache-2.0: [crates.io](https://crates.io/api/v1/crates/cpal)
- Steam Audio 4.8.1, Apache-2.0: [GitHub](https://github.com/ValveSoftware/steam-audio); audionimbus 0.16.0, MIT or Apache-2.0: [docs.rs](https://docs.rs/crate/audionimbus)
- fyrox-sound 1.0.1, MIT: [crates.io](https://crates.io/api/v1/crates/fyrox-sound); oddio 0.7.4, MIT or Apache-2.0: [crates.io](https://crates.io/api/v1/crates/oddio)
