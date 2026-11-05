# Game screens: a HUD and menus in Lust

> A screen (the HUD, a pause menu, a shop) is a Lust file that says what's on it, drawn over the game while it plays and reading the game's values as it goes.

## When to use it

Use a screen for anything the player reads on top of the game: the day and time, money, an energy or health bar, an ammo count. A screen only shows the game; it can't change it, so it can never make the game play differently on another computer or in a replay.

For now screens are written by hand in Lust. The UI editor, where you draw them, comes later and edits the same files. Buttons and menus you move through with the keys come next.

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

**What a screen can read:** the game's [game-wide objects](game-code.md) (`Clock.hour`, `Money.amount`), and its own fields (`coins` above). Reading the entity it's shown for (its health, say) comes next.

**What a screen can use:** `column`, `row`, `stack`, `space`, `text`, `progress` bars, and `canvas` with `box`, `circle` and `line` placed by `x` and `y`. Styles: `pad` (and `padX`, `padTop`…), `gap`, `width` and `height` (a number, or `fill`), `grow`, `size`, `bold`, the alignment words, colour words (`white`, `red5`, `gray9`, `#ff8800`), `background`, `round`, `border` and `opacity`. Other pieces are laid out but not drawn yet.

## Settings

The **Screen** component:

| Setting | What it does |
|---|---|
| Screen | Which screen: a file in `assets/ui`, by its name (`ui/hud.lst` is **Hud**). |
| Shown | Whether it's on screen. Game code will show and hide screens by changing this. |
| Order | Which screen is on top when several are shown: higher is on top. |

## Tips and common problems

- **Problem:** Problems says "each file in ui/ passes the assembly named after it". **Why:** Solis finds a screen by its file's name. **Fix:** the file `ui/hud.lst` holds `assembly Hud { … }` and ends with `pass Hud;`.
- **Problem:** the Console says "The screen Hud stopped: … can't be changed here". **Why:** a screen only reads the game; it tried to change one of the game's objects. **Fix:** change the game in game code, and let the screen show the result. A screen may change its own fields.
- **Problem:** the screen stopped, and the Game panel still shows the last picture. **Why:** something in its view failed (the Console names the file and line), so its last good picture stays. **Fix:** fix the line; the screen runs again when the code is saved.
- **Problem:** text is dark and hard to read over the game. **Why:** like Lust's windows, text is dark unless told otherwise. **Fix:** give it a colour word: `white`.

## Related

- [Game code](game-code.md): game-wide objects, which screens read.
- [Play mode](play-mode.md): the Game panel, where screens are drawn.
- [Concepts](concepts.md): [game UI](concepts.md#playing), [screen](concepts.md#playing).
