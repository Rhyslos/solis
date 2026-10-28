# Sounds

> Put WAV and Ogg sound files in your project, see their waveforms in the Library, and listen to them before you use them.

## When to use it
- **To bring sounds into a project:** footsteps, doors, music, birdsong. Solis reads two kinds of [sound file](concepts.md#sound): **WAV**, which keeps every sample as it is (right for short effects straight from a sound editor), and **Ogg Vorbis**, about a tenth of the size (right for music and anything long). Most sound programs save both; if yours makes MP3 or FLAC, save as Ogg instead.
- **To check a sound before using it:** listen to it, and see how long it is, whether it's mono or stereo, and its [sample rate](concepts.md#sound).

Sounds are played through [sound events](sound-events.md): an event holds one or more sound files (takes) and how to vary them. Game code plays them in the game ([Game code: Sounds](game-code.md#sounds)); sounds placed in the level and the mixer arrive in the next updates of Phase 7.

## How to use it
1. **Add a sound:** copy a `.wav` or `.ogg` file into the project's assets folder, or a folder inside it, from Explorer. It appears in the **Library** within a moment, with a teal stripe and a picture of its waveform: how loud it is from start to end.
2. **Listen to it:** point at its tile and click the **▶** in its corner. Click **■** to stop. Only you hear it: what you hear in the Library is never affected by the game's own volumes.
3. **See what it holds:** click its tile. **Properties** shows a larger waveform with a line where it's playing, **Play** and **Stop**, and its details (below). Pick something in the area again and Properties shows that instead.

Right-click a sound for **Play**, **Show in Explorer**, **Rename…** and **Delete**, as with any file.

## Settings
Properties shows these about a sound (they come from the file, so change them in your sound program):

| Detail | What it means |
|---|---|
| Length | How long it plays: milliseconds, seconds, or minutes and seconds. |
| Channels | **Mono** is one sound; **stereo** is two, left and right. A file with more (surround sound) is mixed down to one. |
| Sample rate | How many times a second the sound was measured. 44,100 Hz and 48,000 Hz are usual; 8,000 Hz to 192,000 Hz are read. |
| Format | WAV or Ogg Vorbis. |
| Kept | **From memory** for sounds up to ten minutes long (at the highest quality, 192 kHz, up to about a minute and a half), ready to play at once. Longer ones are **streamed**: played from their file a piece at a time, so a long piece of music doesn't fill the memory. |

## Tips and common problems
- **Problem:** the Console says "No sound: couldn't open the sound card". **Why:** Solis found no [sound card](concepts.md#sound) it could use: none plugged in, or another program holding it. **Fix:** plug in speakers or a headset; Solis tries again every 30 seconds and says "Sound is on." when it finds one. Everything else works meanwhile, and ▶ says there's nothing to play on.
- **Problem:** a sound's tile shows a speaker instead of a waveform, and Properties says Solis couldn't read it. **Why:** the file is damaged, isn't really WAV or Ogg Vorbis, or is outside what's read (more than 256 MB, more than 8 channels, or a sample rate outside 8,000 to 192,000 Hz). Solis checks every sound file carefully, so a broken one is refused instead of crashing. **Fix:** save it again from your sound program, as WAV or Ogg Vorbis.
- **Problem:** an MP3 or FLAC file shows as a plain file. **Why:** Solis reads WAV and Ogg Vorbis only, so every library it uses can stay free for any game to use. **Fix:** save it as Ogg Vorbis (for music) or WAV (for short effects).
- Double-clicking a sound shows it in its folder rather than opening another program, as for other files Solis doesn't edit itself.

## Related
- [Projects and the Library](projects-and-the-library.md)
- [The Console](console.md)
- [Concepts: Sound](concepts.md#sound)
