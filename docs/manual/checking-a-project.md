# Checking a project before updating

> Opens a project without the editor, reads every file, checks the game's code (its screens too), plays back recordings and, if you ask, plays the game for a moment, and says whether anything changed.

## When to use it
Before you update Solis on a project that matters (your own game), or in a workflow that tries each new build on it before you do. It answers one question: **does this build still open my project and play my game the same?** If the answer is no, don't update yet: the report says what changed.

The samples Solis makes (the turret sample, the benchmark and the playground) are checked this way by Solis's own tests on every change, so a build that breaks them never ships.

## How to use it
1. **Keep a few recordings as checks.** Play your game while recording its input (**Debug › Play and record inputs**, see [Input](input.md)), then press **Stop**. The recording is saved in the project's `recordings` folder. Move the ones you want checked into `recordings/checks/`. A good check is a minute of ordinary play: walking, using things, saving.
2. **Run the check** from a command prompt, with the build you're about to use:

   ```text
   solis --check-project "C:\Games\My farm"
   ```

3. **Play it for a moment too** (a [smoke check](concepts.md#smoke-check)), if you like: add `--smoke` and how many [ticks](concepts.md#tick) to play (60 is a second).

   ```text
   solis --check-project "C:\Games\My farm" --smoke 120
   ```

   The game's [start level](concepts.md#start-level) is played for that long with nothing pressed, just as **Play** would play it, screens and all. Anything the [Console](console.md) says that's an error or a warning fails the check, with the tick it was said on. It's the quickest way to know the game *starts*: code can be free of mistakes and still stop on its first tick, or a screen can fail as it opens.
4. **Read the report.** It's printed, and kept in the project as `.solis/check-report.txt` (on Windows, Solis has no window to print to, so open that file). Each line says `ok`, `note` (fine, but worth knowing: an older file that was upgraded as it's read) or `FAIL`. The last line says whether the project checks out.

The check opens the project the way the editor does (with its input actions, 2D settings and Mixer), so it also writes the engine's description files (the `solis` folder) and any missing [sidecars](concepts.md#projects), as opening it in the editor would. It never changes your levels, code or other files.

## What it checks
| Step | What fails |
|---|---|
| Files | Any level, module, sound event, Mixer, input actions, 2D settings, tile map, tileset, Atlas Crop file, animator, clips file, recording, save game, sidecar or project file that this build can't read. An older one that's upgraded as it's read is a note, not a failure. |
| Game code | Any mistake in the game's Lust or Lua, and its [screens](screens.md) started as **Play** starts them: a screen that can't start is a mistake in its file. A call to a name that was renamed is a note. |
| Recordings | Each recording in `recordings/checks/` is played back in the level it was recorded in, with the project's sprite sheets, tilesets and tile maps read as the editor reads them (so a 2D game's walls are there). If the game comes out differently from the same input on any tick, it fails, with the tick. It also fails, saying so, if the level isn't as it was when recorded (record it again), or if it was made before recordings said their level. Saves the game makes while it plays back are kept by the check, never written into your project. |
| Playing (only with `--smoke`) | The game's start level, played for the ticks asked with no input. Each error or warning the Console says fails, once, with the first tick it was said on and how often (`tick 1, warning: The screen Hud stopped: …`). What opening the level says counts too, as "as the level opened". The start level is the one the game was last exported to start in ([Playtesting](playtest-export.md)), or else the project's first level by name. A project without levels is a note. |

## Tips and common problems
- **Problem:** `--smoke` fails with a warning you expected (a sound card missing on a build machine, say). **Why:** every warning counts, since most mean something's wrong. **Fix:** put right what it says, or run the check without `--smoke`.
- **Problem:** the check says "the game's screens couldn't start". **Why:** a screen's code stops as it opens, so on **Play** none of the screens would show. **Fix:** the line names the screen's file and line: put that right. The editor's [Problems](problems.md) panel shows the same mistake when the code is checked.
- **Problem:** a recording fails that used to pass, and you changed the level or the code since. **Why:** the check compares the game with what it did when recorded; your change makes it play differently, which is what it's for. **Fix:** record the check again after your change.
- **Problem:** a recording fails with only a Solis update in between. **Why:** the new build plays your game differently. **Fix:** keep using the build you had (the last ten are on the Releases page; see [Installing and updating](installing-and-updating.md)) and report it, with the recording.
- **Problem:** "its level isn't as it was when this was recorded". **Why:** the level was changed and saved since, or the recording was made with unsaved changes (the check reads the saved level; Solis warns when you start recording with unsaved changes). **Fix:** save the level, then record the check again.
- **Problem:** the check takes a couple of seconds longer after you've just made a file. **Why:** a new file gets its sidecar (the small file beside it holding its ID) once it has been still for two seconds, so a file Git is still writing isn't given a wrong ID. The check waits for it, so the file is checked with its ID in the same run. **Fix:** nothing: it's on purpose.
- **Problem:** "it doesn't say which level it was recorded in". **Why:** recordings made before this version didn't name their level. **Fix:** record it again.

## Related
- [Playing a game from the command line](playing-from-the-command-line.md): `--play` with a script of input
- [Input](input.md): recording and replaying play.
- [Backups and upgrades](backups-and-upgrades.md): what happens to older files.
- [Installing and updating](installing-and-updating.md).
