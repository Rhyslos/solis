# The UI sample: a HUD and a pause menu, start to finish

> A small game whose screens show everything game UI can do: a HUD that follows the game, a health bar, and a pause menu offering the player's settings, each drawn on the UI editor's canvas.

## When to use it
Open it when you're about to give your own game a HUD or a menu: it's the shortest working example of each part, and every file is meant to be read and changed. The [turret sample](first-game.md) is the place to start for game code itself; this one is about [screens](screens.md).

## How to use it
1. **Help > Make the UI sample…** (or **UI sample** among the launcher's examples), and choose an empty folder. The project opens on its area.
2. Press **Play**, then click the **Game panel** so the game has the keys.
   - **Space** (or a gamepad's **A**) fires the turret at the drone. The HUD in the bottom left counts the ammo and the hits, with a yellow bar that empties as you fire; the drone's health bar at the top follows its health.
   - **P** (or **Start**) pauses: the game stops and the pause menu shows. Move with the arrow keys or the stick, choose with **Space**, **Enter** or **A**, or click.
3. Try the menu:
   - **Text size** makes every screen's text a quarter bigger each time (back to 100% after 200%). The menu's buttons grow to fit it (they're at least 360 wide, `minWidth 360`, rather than exactly).
   - **Fullscreen** and **Resolution** say in the Console what the exported game would do; in the editor the game stays in the Game panel.
   - **Resume** carries on, and **Quit** closes the game (`Window:quit()`): while you play in the editor, play stops; in an [exported game](playtest-export.md), the game closes.
4. Press **Stop**, open the Library's `ui` folder and double-click **hud**: it opens on the canvas. Pick a piece and change it in Properties, or try **Preview values** to see the ammo bar empty.

**How its parts fit together:**

| File | What it does |
|---|---|
| `main.lst` | The game code: the **Turret** (fires on Jump, reloads), the drone's **Health**, and the **Pauser**, which opens the menu and changes the player's settings when the menu's buttons say so. |
| `ui/hud.lst` | The HUD, shown by the Turret's **Screen** component. It reads the turret's ammo and hits (`Ui:entity():turret()`); the bar is a `box` whose width is `300 * ammo / maxAmmo`, and while the turret reloads it stays empty, with `shadow` for a glow. The Screen component's **Glow** gives the whole HUD a soft light. |
| `ui/droneHealth.lst` | The drone's health bar, shown by the Drone's Screen component, reading its Health. |
| `ui/pauseMenu.lst` | The menu. It reads the settings (`Screens:textSize()`, `Window:fullscreen()`, `Window:resolution()`) to show them, and each button sends an event to the Game entity (`send BiggerText() to Ui:entity();`), since screens never change the game themselves. |
| `ui.solarea` | The area: the camera, the turret, the drone and the Game entity with the Pauser. |

**Why the buttons send events instead of changing the settings themselves:** a screen only reads the game. That keeps every change in game code, where recordings and replays see it, so the sample plays back the same from a recording, menu and all.

## Settings
The sample has no settings of its own. The **Screen** components on the Turret and the Drone have the screen's look ([Game screens](screens.md#settings)): try **Pixel size 4** on the Turret's for a pixel-art HUD.

## Shortcuts
| Action | Shortcut |
|---|---|
| Fire | `Space`, or the gamepad's `A` |
| Pause, and back | `P` or `Start` (`Backspace` or `B` too) |
| Move in the menu, choose | Arrow keys or the stick; `Space`, `Enter` or `A` |
| Switch fullscreen | `Alt + Enter` |

## Tips and common problems
- **Problem:** pressing Space does nothing. **Why:** the editor still has the keys. **Fix:** click the Game panel first; Esc gives the keys back.
- **Problem:** Fullscreen doesn't fill the screen. **Why:** while playing in the editor, the game stays in the Game panel. **Fix:** it's the exported game that changes its window; the Console says what it would do.
- **Problem:** after a big text size, the HUD's words reach past its panel. **Why:** a piece given a fixed width keeps it. **Fix:** leave sizes out where text grows, and check with **Text 150%** and **200%** on the canvas.

## Related
- [Game screens](screens.md): what a screen's file holds, the player's settings, and screens' looks.
- [The UI editor](ui-editor.md): drawing and changing screens on the canvas.
- [Your first game](first-game.md): the turret sample, for game code in all three languages.
- [Concepts](concepts.md): [screen](concepts.md#screen), [tie](concepts.md#tie), [player setting](concepts.md#player-setting), [focus](concepts.md#focus).
