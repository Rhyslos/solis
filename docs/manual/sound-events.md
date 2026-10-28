# Sound events

> A sound event is what your game plays, such as "footstep": a few recordings (takes) and how much to vary them, so the same sound never sounds copied.

## When to use it
- **For anything heard more than once:** footsteps, gunshots, doors, coins. A real footstep never sounds exactly the same twice; playing one recording over and over sounds mechanical very quickly. An event picks a different take each time, and changes its [pitch and volume](concepts.md#sound) a little.
- **To set how a sound is heard in one place:** how loud, whether it comes from a place in the world or is heard the same everywhere, how far it carries, and which [bus](concepts.md#sound) it belongs to. Game code plays the event by name ([Game code: Sounds](game-code.md#sounds)), so changing the event changes every place that plays it.
- A single sound you only ever play once, such as one piece of music, still works best as an event with one take: the event holds its settings.

## How to use it
1. **Make one:** in the **Library**, **Import > New sound event**. It's made in the folder shown, called "New sound event", and shown in Properties. Rename it like any file (right-click > **Rename…**), say to `footstep`.
2. **Add takes:** drag sound files from the Library onto the event's tile, or into the **Takes** list in Properties. Five to eight takes of a footstep is plenty. The tile shows how many it has (×5).
3. **Listen:** press **Play** in Properties, or point at the event's tile and click its **▶**. Each press picks a different take, pitch and volume, never the same take twice in a row, and the line under Play says what it picked. Press it a few times to hear the variety.
4. **Change how it sounds:** edit the settings below in Properties. Each change is saved to the event's `.solsound` file as soon as you let go of the mouse. To take a take out, click the **×** at the end of its row (the sound file itself stays in the project).

## Settings
| Setting | What it does |
|---|---|
| Volume | How loud, in decibels (dB): 0 is as recorded, −6 about half as loud, +6 about twice. The second number is how much each play may be louder or quieter: 1 to 3 dB is subtle and natural. |
| Pitch | How high or low, in semitones (st): 12 is an octave up. The second number is how much each play may differ: 0.5 to 2 st keeps footsteps fresh without sounding odd. |
| Bus | Which [bus](mixer.md) it plays through (Music, Effects, Voice, Ambience, UI, or one of your own), so its fader, ducking and a game's options turn it up or down with the others. |
| Loop | Plays again from the start when it ends: an engine hum, a campfire, music. |
| Placement | **In the world**: it comes from a place, quieter far away and from its direction (a footstep, a door). It plays as one channel (mono), mixed down from stereo if need be, so it can come from one side. **Everywhere**: heard the same wherever you are, as recorded, in stereo if the file is (music, the interface). Each take in the list says whether its file is mono or stereo. |
| Full until, Silent past | For sounds in the world: full volume up to the first distance, silent past the second, in metres. A footstep might be 2 m and 30 m; an explosion 10 m and 300 m. |
| Fade | How it gets quieter between the two distances. **Realistic** drops quickly at first, as sound does outdoors; **Even** drops steadily; **None** stays full until the far distance. |
| At once | The most plays of this event heard at the same time; more are skipped. 8 is a good start; a machine gun might want 4. When game code asks for one event many times in one tick (a volley of a hundred shots), up to 3 copies play, a little louder, instead of a hundred. |
| Heard by | Who hears it: **Everyone**, **Only who caused it** (the listener on the entity that played it), or **Groups** (listeners in one of its groups). See [Listeners and who hears what](listeners.md). |
| Priority | When more sounds want to play than the project's **Heard at once** (in the [Mixer](mixer.md)), **High** ones are heard first, then the loudest where you are; **Low** ones go first. |

## Sounds that keep playing

An engine's hum, a campfire, a radio: add an **AudioSource** to the entity (Properties > Add component > AudioSource), and type the sound event's name in **Sound event**, its file in assets without `.solsound` (`sfx/engine`). It plays over and over while the game runs, following the entity, from Play (or when game code makes the entity) until the entity goes. **Playing** switches it off and on, and game code does the same with `setSoundPlaying` ([Game code: Sounds](game-code.md#sounds)). **Volume** makes it louder or quieter on top of the event's own.

Thousands of them are fine: only the most important are heard at once (the [Mixer](mixer.md)'s **Heard at once**), and the rest rest. A resting one isn't mixed at all, but it keeps its place in the sound: walk up to a campfire that was resting and it carries on where it would have been, not from the start. A save game keeps whether each was on or off.

## Tips and common problems
- **Problem:** Play says "it has no takes yet". **Why:** the event has no sound files in it. **Fix:** drag sound files onto it from the Library.
- **Problem:** a take shows as an ID with "(missing)". **Why:** that sound file was deleted, or its `.solmeta` file was lost, so the event can't find it. Plays skip it. **Fix:** click its × and drag the file in again.
- **Problem:** Properties says Solis couldn't read the event. **Why:** the `.solsound` file was edited by hand and has a mistake, which the message points to by line. **Fix:** correct that line in a text editor, or delete the file and make the event again.
- A `.solsound` file is plain text, one setting per line, so Git shows exactly what changed and merges two people's changes cleanly. A value out of range (a volume of 99, say) is brought into range when read, and a note in Properties says so.
- Pressing Play in the editor plays on its own, so you hear the event even while the game's sound is turned down.

## Related
- [Sounds](sounds.md)
- [Projects and the Library](projects-and-the-library.md)
- [Concepts: Sound](concepts.md#sound)
