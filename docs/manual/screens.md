# Game screens: a HUD and menus in Lust

> A screen (the HUD, a pause menu, a shop) is a Lust file that says what's on it, drawn over the game while it plays and reading the game's values as it goes.

## When to use it

Use a screen for anything the player reads on top of the game: the day and time, money, an energy or health bar, an ammo count. A screen only shows the game; it can't change it, so it can never make the game play differently on another computer or in a replay.

A screen can also have **buttons** (a pause menu's Resume, a shop's Buy). A button doesn't change the game either: it sends an [event](game-code.md#events), and game code decides what happens, in the next tick. That keeps a menu as fair as the keys: what was clicked is kept in a [recording](play-mode.md) like a key press, and plays back the same.

A menu can be used with the keys or a controller too: the arrow keys (or the d-pad) move a ring between its buttons, and Enter (or A) chooses.

For now screens are written by hand in Lust. The UI editor, where you draw them, comes later and edits the same files.

## How to use it

1. **Make the file.** In the Library, make a folder called `ui` inside your assets, and a Lust file in it named after the screen: `ui/hud.lst` is the screen **Hud**. Its name starts with a letter and holds only letters and numbers.
2. **Write the screen.** The file holds an `assembly` with the screen's name and a `view`, and passes it. The view uses Lust's own pieces (the same ones as Lust's windows):

   ```
   fetch "main":Clock;

   assembly Hud {
       has {
           int coins = 3;
       }

       view [width 1920, height 1080] {
           stack [width fill, height fill, bottom, left, pad 48] {
               column [gap 8] {
                   text "Day " + Clock.day.text() + ", " + Clock.hour.text() + ":00" [size 32, bold, white];
                   progress Clock.energy [to 100, width 300, height 20, green];
               }
           }
       }
   }

   pass Hud;
   end
   ```

   - `[width 1920, height 1080]` is the size you design at. On other screens everything grows or shrinks with the screen's **height**, and a wider screen (ultrawide) gets more room at the sides.
   - To pin something to a corner, put it in a `stack` as big as the screen (`width fill, height fill`) and say where: `bottom`, `top`, `left`, `right`, `center`, `middle`, with `pad` for the distance from the edge.
3. **Show it.** Add a **Screen** component to an entity (the player, or a "Game" entity), and pick **Hud** in its **Screen** field. **Shown** turns it on and off; **Order** decides which of several screens is on top.
4. **Press Play.** The screen is drawn over the Game panel, and changes with the game: after every tick it reads the game's values again.

**What a screen can read:**

- the game's [game-wide objects](game-code.md) (`Clock.hour`, `Money.amount`);
- its own fields (`coins` above): the screen's memory;
- **the entity it's shown for** (the one whose Screen component shows it): `Ui:entity()`, and a copy of any of that entity's [Lust components](game-code.md), with a reading method named after the component: `Ui:entity():health()` gives its `Health`. Fetch `Ui` from `solis/screen` and the component from its file:

  ```
  fetch "solis/screen":Ui;
  fetch "main":Health;

  assembly DroneHealth {
      view {
          Health health = Ui:entity():health();
          text "Drone " + health.hp.text() + " / 20" [size 26, bold, white];
          progress health.hp [to 20, width 360, height 18, red5];
      }
  }

  pass DroneHealth;
  end
  ```

  The copy is the component as the last tick left it: its methods work (`health:fraction()`), and changing it changes nothing in the game. The turret sample (**Help > Make the sample game…**) has this health bar over its drone.

  **Lua and Rust components, and Solis's own** (Transform, Sprite, Camera, AudioSource and the rest) are read the same way. Solis describes each one to screens as a Lust component of its fields, so fetch it from `solis/screen` instead of from a file of yours:

  ```
  fetch "solis/screen":[Ui, Transform, Sparks];

  assembly Where {
      view {
          Transform t = Ui:entity():transform();
          Sparks s = Ui:entity():sparks();
          text "x " + t.position.x.text() + ", sparks " + s.shown.text() [white];
      }
  }

  pass Where;
  end
  ```

  Their fields are copies, as Lust components' are; they have no methods. A field Lust can't hold (a model or other asset, or one named like one of Lust's own words) is left out.

**Buttons.** A `button` has its words and, in `{ }`, what a click does: usually `send` an event to the entity the screen is shown for. Declare the event in game code, fetch it in the screen, and handle it in a component on that entity:

```
-- ui/pauseMenu.lst
fetch "solis/screen":Ui;
fetch "main":[Resumed, Quit];

assembly PauseMenu {
    view {
        stack [width fill, height fill, center, middle] {
            column [gap 12, background gray9, pad 32, round 12] {
                text "Paused" [size 40, bold, white];
                button "Resume" [width 240, blue] { send Resumed() to Ui:entity(); }
                button "Quit" [width 240, hover:[background red7]] { send Quit() to Ui:entity(); }
            }
        }
    }
}

pass PauseMenu;
end
```

```
-- main.lst
fetch "solis/project":Screen;

event Resumed();
event Quit();

component Pauser {
    on Resumed() {
        hideScreen(Screen.PauseMenu);
    }
}

pass Pauser, Resumed, Quit;
end
```

- A button is drawn light grey with rounded corners; a colour word (`blue`, `green`, `red5`) colours it, and its words turn white or dark to stay readable. `disabled` greys it out and stops clicks.
- Under the pointer it darkens a little, or takes the styles in its `hover:[…]`.
- The `{ }` may also change the screen's own fields (a tab picked, say). It can't change the game: send an event for that.
- What a button sends reaches the game at the start of the next tick, before the components' `update`s.

**Showing and hiding screens from game code.** Every entity can show one screen at a time. Its calls take the screen by name, from `solis/project.lst` (fetch `Screen` from `"solis/project"`):

| Call | What it does |
|---|---|
| `showScreen(Screen.PauseMenu)` | Shows that screen on the entity's Screen component, at the end of the tick. An entity without one is given one. |
| `hideScreen(Screen.PauseMenu)` | Hides it, at the end of the tick, if the entity shows that screen. |
| `screenShown(Screen.PauseMenu)` | Whether the entity shows that screen, as the tick started. |

In Lua they're `solis.entity.showScreen(self.me, "PauseMenu")`, `solis.entity.hideScreen(…)` and `solis.entity.screenShown(…)` (the screen's name as text), and a button's event arrives in `on_Resumed`. In Rust, through the component's host: `cx.host().show_screen(cx.me(), "PauseMenu", true)` (`false` hides it) and `cx.host().screen_shown(cx.me(), "PauseMenu")`.

Use another entity (a "Menus" entity) for a second screen shown at the same time. Which screens are open is part of the game: it's in save games and recordings.

**The mouse while a screen has buttons.** While a screen with buttons is shown, the pointer stays visible over the Game panel, even after you click the game to give it the keys, so you can click the buttons. A click on a button goes to the button, not to the game; a click anywhere else gives the game the mouse and keys as usual. When the last screen with buttons goes, the pointer is held by the game again.

**With the keys or a controller.** Once the game has the keys, **MenuMove** (the arrow keys, the d-pad or the left stick) moves the focus to the nearest button that way, shown with a blue ring; held down, it moves on again every tenth of a second. **MenuChoose** (Enter, Space or A) chooses the button with the focus, just as a click would. A menu's first button has the focus to start with, so Down moves to the second. The focus is on the topmost screen with buttons, and moving the mouse over a button moves the focus there (the ring hides until the keys are used again).

While a screen with buttons is shown, the game's own actions wait: only MenuMove, MenuChoose, **MenuBack** (Backspace, B) and **Pause** (P, Start) reach game code, so Space choosing a button doesn't also jump. Use MenuBack and Pause in game code to close a menu or open the pause menu: `is Input:pressed(Action.Pause)? then { showScreen(Screen.PauseMenu); }`. They're ordinary [input actions](input.md), rebindable in **Edit > Input actions…**.

**A pause menu.** `World:setGameSpeed(0)` stops the game while game code keeps running: game time, physics, projectiles and sprites stand still and every `dt` is 0, but components' `update`s, events and the screens go on, so a menu can be used. `World:setGameSpeed(1)` carries on (anything between 0 and 4 runs slower or faster). The Game panel's corner says *Game speed 0* meanwhile, and the speed is kept in save games. The turret sample has one: press P (or Start) while it plays.

```
component Pauser {
    method update(float dt) {
        is Input:pressed(Action.Pause)? then {
            showScreen(Screen.PauseMenu);
            World:setGameSpeed(0);
        }
    }
    on Resumed() {
        hideScreen(Screen.PauseMenu);
        World:setGameSpeed(1);
    }
}
```

In Lua: `solis.World:setGameSpeed(0)`. In Rust: `cx.host().set_game_speed(0.0)`.

**What a screen can use:** `column`, `row`, `stack`, `space`, `text`, `button`, `progress` bars, and `canvas` with `box`, `circle` and `line` placed by `x` and `y` (anything reaching past the canvas's edges is cut off there, so a needle or a bar can't spill out of its dial). Styles: `pad` (and `padX`, `padTop`…), `gap`, `width` and `height` (a number, or `fill`), `grow`, `size`, `bold`, the alignment words, colour words (`white`, `red5`, `gray9`, `#ff8800`), `background`, `round`, `border` and `opacity`. Other pieces are laid out but not drawn yet.

## Settings

The **Screen** component:

| Setting | What it does |
|---|---|
| Screen | Which screen: a file in `assets/ui`, by its name (`ui/hud.lst` is **Hud**). |
| Shown | Whether it's on screen. Game code shows and hides screens with `showScreen` and `hideScreen`. |
| Order | Which screen is on top when several are shown: higher is on top. |

## Tips and common problems

- **Problem:** Problems says "each file in ui/ passes the assembly named after it". **Why:** Solis finds a screen by its file's name. **Fix:** the file `ui/hud.lst` holds `assembly Hud { … }` and ends with `pass Hud;`.
- **Problem:** the Console says "The screen Hud stopped: … can't be changed here". **Why:** a screen only reads the game; it tried to change one of the game's objects. **Fix:** change the game in game code, and let the screen show the result. A screen may change its own fields.
- **Problem:** the screen stopped, and the Game panel still shows the last picture. **Why:** something in its view failed (the Console names the file and line), so its last good picture stays. **Fix:** fix the line; the screen runs again when the code is saved.
- **Problem:** the Console says "this entity has no Health, so health() has nothing to give". **Why:** the entity whose Screen shows the screen has no such component. **Fix:** put the Screen component on the entity that has it, or add the component.
- **Problem:** the Console says "Screens can't read the component Position: entities already have a position()". **Why:** a reading method is named after its component, and this name is an engine call on entities already. **Fix:** give the component another name.
- **Problem:** game code says there's no file `solis/screen.lst`. **Why:** only screens can fetch it: game code reads other entities through its own calls. **Fix:** fetch it only in files in the `ui` folder.
- **Problem:** clicking a button does nothing in the game. **Why:** the event goes to the entity the screen is shown for (`Ui:entity()`), and only its components' `on` blocks hear it. **Fix:** put the component that handles it on the entity with the Screen, and check the event's name and values match its `on` line.
- **Problem:** `Screen.PauseMenu` is "not an option". **Why:** a screen's name comes from its file, first letter made a capital: `ui/pauseMenu.lst` is `PauseMenu`. Names in all capitals (`HUD`) can't be Lust names. **Fix:** rename the file, or use the name the Problems panel suggests.
- **Problem:** the Console says a screen "sent events the game couldn't take". **Why:** at most 64 events wait for each tick, and an event from a screen carries plain values (no assets, texts of at most 4,096 letters). **Fix:** send fewer, smaller events.
- **Problem:** text is dark and hard to read over the game. **Why:** like Lust's windows, text is dark unless told otherwise. **Fix:** give it a colour word: `white`.

## Related

- [Game code](game-code.md): game-wide objects, which screens read.
- [Play mode](play-mode.md): the Game panel, where screens are drawn, and recordings.
- [Concepts](concepts.md): [game UI](concepts.md#playing), [screen](concepts.md#playing).
