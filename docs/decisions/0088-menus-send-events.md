# 0088. Menus: buttons send events, and an entity shows its screens

- **Status:** Accepted (the project lead, 2026-11-05; builds on 0087, and on the game UI plan's choices 6 to 8, answered 2026-10-25)
- **Date:** 2026-11-05

## Problem

A screen can't change the game (decision 0087), but a menu's buttons must: Resume, Quit, Buy. And game code needs to open and close screens (the pause menu when Pause is pressed). The plan offered a button both `send` and `press` (an input action), and a `Screen` component whose `shown` game code changes, but game code can't change a built-in component's fields.

## Options

- Buttons **send an event** to an entity (chosen), **press an input action**, or both.
- Screens shown by **a method on the entity** (chosen: `showScreen(Screen.Pause)`), or by **an engine object** (`Screens:show(…)`) with no entity.

## Decision

- **A button sends an event:** `button "Resume" { send Resumed() to Ui:entity(); }`. Lust's own `send`, so nothing new in Lust. The event joins the **next tick's input** (with the keys pressed), so it's recorded in recordings, counted in their checksums, and a replay plays the same with no screen drawn. A screen's button may also change the screen's own fields (which tab is open).
- **Game code shows and hides screens with methods on the entity:** `showScreen(Screen.Pause)` and `hideScreen(Screen.Pause)` inside a component (`target:showScreen(…)` on another), with `screenShown(Screen.Pause)` to ask. The screens are project states, written by the editor like clips and sounds (`state Screen = [Hud, Pause];`), so a typo is a mistake before the game runs. They change the entity's `Screen` component (adding one if it has none for that screen), so open screens are saved, replayed and the same on every computer.
- The plan's other answers stand: pausing stops game time, not game code (`World:setGameSpeed(0)`); the menu actions (UI Move on the arrows, d-pad and left stick; UI Choose on Enter, Space and A; UI Back on Backspace and B; Pause on P and Start) are rebindable input actions, and while a screen that takes the focus is shown, the game's own actions wait; a screen can ask for the pointer (`view [pointer]`).

## Consequences

- No new Lust word, and no `press` from screens: an input action is pressed only by the player.
- M3 is built in three parts: buttons and events (with the pointer), focus and the menu keys, and the game's speed.
