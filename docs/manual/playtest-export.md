# Exporting for playtesting

> Makes a folder holding your game, ready to zip and send to a friend, who plays it without the editor.

## When to use it

When you want someone else to try your game: a friend, a tester, or yourself on another computer. They get a folder with a program named after your game. Double-clicking it starts the game straight away, filling its window, with no launcher, no editor and nothing to install.

It's a **playtest** build, made in seconds, because the game's Lust and Lua code runs as it does when you press Play. A finished release with an installer comes later (Phase 18). Until then, a game's own Rust code isn't built into an export.

## How to use it

1. Open your project, and save the area your game starts in.
2. Choose **File > Export for playtesting…** (or search for it).
3. Check the settings (see the table below). They start from your project's name and the open area, and next time from what you chose last.
4. Click **Export**. The window and the [Console](console.md) say where it went. Click **Open folder** to see it.
5. Zip that folder (in Windows Explorer, right-click it, then **Send to > Compressed (zipped) folder**) and send the zip.

Your friend unzips it (all of it, somewhere they can write, such as their Documents) and double-clicks **`<Game>.exe`** inside. Double-clicking it inside the zip doesn't work: Windows takes only the program out, without the game beside it.

**What's in the folder, and why:**

| File | What it is |
|---|---|
| `<Game>.exe` | The program, named after your game. It's the same program as the editor: finding `playtest.txt` beside itself, it plays instead. |
| `playtest.txt` | The game's name, the area it starts in, and its first window. |
| `game/` | Your project's files: code, areas, sprites, sounds, screens. The editor's own things stay out: its cache, saves, recordings, backups and Git's history. |

**Where the player's things go:** everything that's the player's goes in their own folder, `Saved Games/<Game>` on Windows (next to other games' saves):

- `saves/`: the game's [save games](save-games.md);
- `settings.txt`: the player's [settings](concepts.md#player-setting) (text size, UI size, fullscreen, window size), kept from one start to the next;
- `log.txt`: warnings and mistakes the game reported, like the Console's (past 1 MB it starts again, keeping the one before as `log.old.txt`);
- `crash log.txt`: written if the program stops unexpectedly. It says what happened and the last things the game said. Ask your playtesters to send it to you.

**Quitting:** give your pause menu a Quit button that calls `Window:quit()` (see [Game code](game-code.md)). The game closes at the end of that tick. While you play in the editor, play stops instead, and the Console says the exported game would close. The window's close button always works too.

## Settings

| Setting | What it does |
|---|---|
| Game's name | The program's name (`Farm.exe`), the window's title, and the player's saves folder (`Saved Games/Farm`). Characters Windows can't use in file names are taken out. Keep it the same between builds, so saves carry over. |
| Starts in | The area the game opens in. |
| Into the folder | Where the export goes. Each export gets a new dated folder, so older builds stay. A folder that already has files in it isn't used, and nor is one inside your project. Click **…** to choose. |
| Window | **Windowed** or **Fullscreen**, and the window's size, when the game first starts. After that, the player's own choice (in your game's menu, or with Alt+Enter) is kept. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Switch the exported game between fullscreen and a window | `Alt + Enter` |

In an exported game, every key is the game's, Esc included.

## Tips and common problems

- **Problem:** the game says "The game can't start: its code has mistakes." **Why:** the exported code doesn't compile. **Fix:** fix the mistakes in the Problems panel, then export again.
- **Problem:** "already has files in it". **Why:** exports never mix with, or replace, anything. **Fix:** choose a new folder, or empty that one.
- **Problem:** the game says "The game's files can't be read", or "The game stopped". **Why:** the `game` folder is missing or damaged, or it's somewhere the game can't write (the first start writes a cache into `game/`). **Fix:** unzip the whole export again, somewhere like Documents. The message names the log with more.
- **Problem:** the export is big. **Why:** it holds the program and everything in the project's assets, used or not. **Fix:** move files the game doesn't use out of the assets.
- **Problem:** Windows warns about the program when your friend opens it. **Why:** it isn't signed, as programs from big companies are. **Fix:** they choose **More info > Run anyway**. Send builds only to people who trust you.
- **Problem:** saves from an older build are gone. **Why:** the game's name changed, so the saves folder did too. **Fix:** keep the name the same.
- **Problem:** Rust components of your own don't run. **Why:** building a game's own Rust code comes with the full export (Phase 18). **Fix:** use Lust or Lua for now.

## Related

- [Screens](screens.md): the player's settings and a pause menu.
- [The UI sample](ui-sample.md): a pause menu with Quit, ready to export.
- [Save games](save-games.md).
- [Concepts](concepts.md): [player setting](concepts.md#player-setting), [playtest](concepts.md#playtest).
