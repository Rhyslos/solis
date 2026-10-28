# Checking a project before updating

> Opens a project without the editor, reads every file, checks the game's code and plays back recordings, and says whether anything changed.

## When to use it
Before you update Solis on a project that matters (your own game), or in a workflow that tries each new build on it before you do. It answers one question: **does this build still open my project and play my game the same?** If the answer is no, don't update yet: the report says what changed.

The samples Solis makes (the turret sample, the benchmark and the playground) are checked this way by Solis's own tests on every change, so a build that breaks them never ships.

## How to use it
1. **Keep a few recordings as checks.** Play your game while recording its input (**Play > Play and record inputs**, see [Input](input.md)), then press **Stop**. The recording is saved in the project's `recordings` folder. Move the ones you want checked into `recordings/checks/`. A good check is a minute of ordinary play: walking, using things, saving.
2. **Run the check** from a command prompt, with the build you're about to use:

   ```text
   solis --check-project "C:\Games\My farm"
   ```

3. **Read the report.** It's printed, and kept in the project as `.solis/check-report.txt` (on Windows, Solis has no window to print to, so open that file). Each line says `ok`, `note` (fine, but worth knowing: an older file that was upgraded as it's read) or `FAIL`. The last line says whether the project checks out.

The check opens the project the way the editor does, so it also writes the engine's description files (`assets/solis/`) and any missing [sidecars](concepts.md#projects), as opening it in the editor would. It never changes your areas, code or other files.

## What it checks
| Step | What fails |
|---|---|
| Files | Any area, assembly, sound event, Mixer, input actions, recording, save game, sidecar or project file that this build can't read. An older one that's upgraded as it's read is a note, not a failure. |
| Game code | Any mistake in the game's Lust or Lua. A call to a name that was renamed is a note. |
| Recordings | Each recording in `recordings/checks/` is played back in the area it was recorded in. If the game comes out differently from the same input on any tick, it fails, with the tick. It also fails, saying so, if the area isn't as it was when recorded (record it again), or if it was made before recordings said their area. Saves the game makes while it plays back are kept by the check, never written into your project. |

## Tips and common problems
- **Problem:** a recording fails that used to pass, and you changed the area or the code since. **Why:** the check compares the game with what it did when recorded; your change makes it play differently, which is what it's for. **Fix:** record the check again after your change.
- **Problem:** a recording fails with only a Solis update in between. **Why:** the new build plays your game differently. **Fix:** keep using the build you had (the last ten are on the Releases page; see [Installing and updating](installing-and-updating.md)) and report it, with the recording.
- **Problem:** "its area isn't as it was when this was recorded". **Why:** the area was changed and saved since, or the recording was made with unsaved changes (the check reads the saved area; Solis warns when you start recording with unsaved changes). **Fix:** save the area, then record the check again.
- **Problem:** "it doesn't say which area it was recorded in". **Why:** recordings made before this version didn't name their area. **Fix:** record it again.

## Related
- [Input](input.md): recording and replaying play.
- [Backups and upgrades](backups-and-upgrades.md): what happens to older files.
- [Installing and updating](installing-and-updating.md).
