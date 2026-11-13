# 0110: One rule for the engine's folder

- **Status:** Accepted (the second review pass's N-14; replaces how [0072](0072-lust-alpha-59-and-the-engine-folder.md) finds the folder)
- **Date:** 2026-11-13

## Problem

Three rules said which folder of a project's code is the engine's (`solis/`, where Solis writes the files describing the engine, the data files' classes, the project's names, the kit and the screens' readers):

- the compiler took the top folder of any file whose first line said it described the engine ([0072](0072-lust-alpha-59-and-the-engine-folder.md)'s option 2), so a game file anywhere that copied that line made its folder the engine's and stopped counting as the game's code;
- the app writes the files into `branding::CODE_FOLDER`;
- the screens skipped paths starting with `solis/`, and the screen editor wrote `fetch "solis/screen"` into a user's screens, both by hand.

Renaming the program would have broken screens, and the rules could disagree about a file.

## Options

1. **Keep the first line** as the rule and make the screens use it too: they would need the files' text wherever a path is asked about.
2. **The branding's folder everywhere** (recommended): `area::lust_data::ENGINE_FOLDER`, which a test in the app keeps equal to `branding::CODE_FOLDER`. The reason for 0072's option 2 (the script crate can't reach the app's branding) went when [0081](0081-what-solis-makes-is-lust.md) put the folder's name in `area`, which the script crate uses.

## Decision

**Option 2.** `script::describe::in_engine_folder` says whether a path is in the engine's folder and `engine_folders` gives it as Lust's host folder; the compiler, the check for a game's own code, the screens and the screen editor (through `script::screens::readers_module`) all read `ENGINE_FOLDER`. A file in that folder is the engine's whatever it says; a file elsewhere is the game's, whatever its first line says.

## Consequences

- What Solis writes doesn't change while the program is called Solis: screens still fetch `"solis/screen"`, so screens on disk read as before.
- The description files' first line no longer decides anything; `ENGINE_FILE` only says what the files are.
- A file a user puts in `solis/` is the engine's (the manual says to keep one's own code outside it, as Solis writes that folder again on every open).
- RENAMING.md lists the places that follow the folder's name.
