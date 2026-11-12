# Play mode

> Run the area as a game inside the editor, try things, and press Stop: keep what you changed, or get everything back exactly as it was.

## When to use it
Whenever you want to see the area the way a player will: through the game's camera, with no grid, gizmos or markers. The game's own [code](game-code.md) runs too: every component written in Lust runs its `update` 60 times a second. You can move things, delete things and experiment freely while playing: when you press **Stop**, Solis lists what you changed and asks whether to **Keep** it or **Undo all**, so nothing is lost by accident and nothing sticks that you didn't mean.

## How to use it

**Add a camera first** (once per area)
1. Move the viewport to look the way the game should look.
2. On the **Home** tab, click **Camera** in the ribbon's **Create** group. A camera appears exactly where you're looking from, facing the same way, and is selected. In the viewport it's drawn as a small wire pyramid pointing the way it looks; click it to select it.
3. Move and turn it like anything else (**W**, **R**). Its **Field of view** in Properties sets how wide it sees.

The game looks through the **first camera** in the Area panel's list. With no camera, the Game panel shows the editor's own view and says so.

**Play, pause and stop** (the **Play** group, at the right end of the [ribbon](interface-tour.md) on every tab)
1. Press **Play** (▶), the one light button in the editor. A **Game** tab appears beside the area's tab on Home and comes to the front, showing the game (on the Debug tab the Game is always there, beside the Profiler, and comes to the front too). Play itself becomes **Stop** (■), still the one light button, and **Pause** and **Step** appear beside it. Nothing else changes colour: the editor is the same editor while the game runs, and the button that would start the game is the button that ends it. (Earlier builds tinted the whole window blue while playing and amber while paused; the design the editor now follows doesn't.)
2. Work as usual: the Area panel, Properties, the viewport and its gizmos all still work. Changes show in the game straight away. For game code's components, Properties shows their values as they are right now (a counter counting, a camera's `yaw` turning), and a value you set there reaches the game: the running component takes exactly what you set, whatever the game had made of it. That holds when you set the value it started with, too: a turret that has fired down to 7 has 12 again when you type 12, or click the **↺ 12** beside the field. Its other values stay as the game has them. When you press Stop, every value the game changed goes back as it was before Play; a value you set yourself is one of your changes, which Stop asks about (step 6). Click the area's tab beside **Game** to move things with the gizmos in the viewport; click **Game** to go back. On Stop the Game tab goes from Home, and the area's tab is in front again.
3. Press **Pause** (❚❚) to stop the game's clock; the button stays lit while the game is paused (game code can pause itself too, for a pause menu: the corner then says *Game speed 0*, and the game still runs its code, see [Game screens](screens.md)); press Pause again to carry on. The Game panel's corner shows *Playing* or *Paused* and how long the game has run.
4. While paused, press **Step** (▶❙, beside Pause; faint until the game is paused) to run exactly one [tick](concepts.md#game-code): game code runs once, everything moves on by a sixtieth of a second, and the game stays paused. It's the way to watch something that happens too fast to see, one step at a time.
5. Press **Stop** (■). The game is thrown away and the area comes back exactly as it was when you pressed Play: every entity and setting, the undo history (Ctrl + Z carries on from where you were) and the selection.
6. If you changed the area while playing, a question lists what you did (*Add Point light 1*, *Move Crate*…):
   - **Keep** makes your changes again on the area as it was before Play, and each can still be undone with Ctrl + Z.
   - **Undo all** leaves the area as it was before Play.

   Why ask? Other engines quietly throw away everything changed while playing, and people lose work that way. Solis asks instead. The question waits for an answer; until then, Play and the editor's keys wait too. So does anything that would leave the area: quitting, closing the window, a new or another area, and another project go ahead only once you've answered, so your changes can't slip away unasked. If you close the editor's window to go back to the [launcher](launcher.md) while the game plays, the question is asked there.

**Only your own edits are kept, never what the game did.** After **Keep**, the area is exactly the area as it was when you pressed Play, plus the changes in the list. Entities the game spawned are gone, entities it removed are back, and every value and place it changed is as it was. That holds after loading a [save game](save-games.md) while playing too: your edits are made on your area, not on the save's. Solis can tell the two apart because each edit of yours is an undo step and nothing the game does is one. What that means when the two meet:
- **A value you set that the game also changes** (a counter, where a cart stands) is kept as *you* set it, not as the game left it.
- **A value you set back to what it was before Play** (the turret's 12 again) is in the list too, since you did set it. Keep and Undo all then end the same way for it: the area holds that value either way.
- **A change to something the game made** (renaming a bullet it spawned) can't be kept, because that entity goes with the game. It's left out of the list, and the Console says which changes those were. When one change touched your entities and the game's together (a crate and a bullet deleted in one go), your entities' share of it is kept.
- **A change to an entity the game removed afterwards** is kept: Stop brings the entity back, and Keep makes your change to it.
- **An entity you deleted** stays deleted, whatever the game would have done with it.
- **Undoing, while playing, a change made before Play** is a change too: the list says *Undid a change made before Play*, and Keep leaves it undone (Ctrl + Y makes it again).
- A change you made while playing and undid again with Ctrl + Z is simply gone: after Stop, Ctrl + Y can't bring it back.

**An edit while the game runs changes only what you edit.** The field you set takes your value at once, even if the game had changed it. Everything else on that entity stays as the game has it: the [screen](screens.md) it shows, a sound game code switched off, the clip its sprite plays and which way it's flipped, its animator, its tile map, where it stands and how it moves. So you can tune a lamp on the entity that shows the health bar without the health bar vanishing, and adding or deleting another entity doesn't put anything back either.

You can also type `play`, `pause`, `step` and `stop` in the [Console](console.md).

**Playing with the keys, mouse and gamepad:** click the Game panel to give the game the mouse and keys (the pointer disappears, and the editor's own keys are off); press **Esc** to give them back. A gamepad works whenever the game plays. The **Debug** tab's **Session** group also records a play's input and plays it back exactly (**Play and record inputs** and **Replay a recording…**, see [Input](input.md)), and its **Load save** plays from a [save game](save-games.md).

**Smooth on any screen:** the game itself always runs 60 [ticks](concepts.md#game-code) a second, so it plays the same on every computer. A screen can show more frames than that (120 or 144 a second): between two ticks, the Game panel draws everything part way from where it was at the last tick to where it is now, so movement looks smooth instead of stepping. What you see is up to a sixtieth of a second behind the game, which no one can notice. While paused, it shows exactly where things are.

**What waits until you stop:** saving, opening another area or project, new areas, and making or changing assemblies. If you try, the Console says *Stop playing first*: saving now would save the game's changes, which Stop is about to undo. Quitting or closing Solis while playing stops the game first. If you changed the area while it played, **Keep** or **Undo all** is asked before anything closes; then Solis asks about unsaved changes as usual (after **Keep**, the changes you kept are unsaved, so it asks about them).

**Changes from outside while playing:** if an assembly's file changes (a Git pull, or Notepad), the game shows it at once, and its copies are updated again after Stop. That isn't one of your changes, so Stop doesn't ask about it, and the area you **Keep** has the updated copies too. If the area's own file changes on disk, Solis waits until Stop, and until you've answered **Keep** or **Undo all**, to offer it.

**Renaming, moving or deleting the area's file while playing** (in the [Library](projects-and-the-library.md)) holds after Stop: the area keeps its new name and place, whether you **Keep** your changes or not, and Ctrl + S saves it there. If you deleted the file, the area counts as unsaved after Stop, and saving asks where to put it. That's the one thing Stop doesn't put back, because the file really did move.

## Settings
| Setting | What it does |
|---|---|
| Camera > View | **Perspective**, as the eye sees, or **2D**: flat, straight down, drawn pixel-perfect at the project's base resolution ([2D](2d-pixel-perfect.md)). |
| Camera > Field of view | How wide the camera sees, top to bottom, in degrees. 50° looks like a normal photo; 90° sees much more but stretches the edges. A 2D camera doesn't use it. |
| Camera > Near | The nearest it sees. Raise it if far surfaces flicker. |
| Camera > Far | The farthest it sees; anything beyond isn't drawn. |

## Shortcuts
Play, Pause, Step and Stop are buttons only, so a stray key never starts or stops the game. They are in the search too (press **Space** and type the name), and in the Console: `play`, `pause`, `step`, `stop`.

## Tips and common problems
- **Problem:** I changed something while playing and it went back when I pressed Stop. **Why:** you chose **Undo all** when Stop asked. **Fix:** Undo all can't be taken back; next time choose **Keep**. A value the *game* changed (not you) always goes back: copy the component's values with ⋯ > Copy values before Stop, and paste them after.
- **Problem:** I chose **Keep**, but one change I made while playing isn't there. **Why:** it was a change to something the game made (an entity it spawned, or one a loaded save held), and those go with the game; the Console named it when you pressed Stop. **Fix:** make the change where the entity comes from: the [assembly](assemblies.md) game code spawns, or an entity of the area itself.
- **Problem:** the Game panel shows the editor's view. **Why:** the area has no camera. **Fix:** on the Home tab, Create > Camera.
- **Problem:** new entities appear in the Area panel while playing. **Why:** game code made them (`World:spawn`, see [Game code](game-code.md#making-and-removing-entities)). **Fix:** nothing: they go when you press Stop.
- **Problem:** pressing keys while playing changes the area (W picks the move tool). **Why:** the editor has the keys. **Fix:** click the Game panel to give them to the game; Esc gives them back.
- **Problem:** I can't tell whether the game is running. **Why:** the window no longer changes colour while it plays. **Fix:** look at the ribbon's right end: **Stop** is there instead of **Play** while the game runs, and **Pause** is lit while it is paused. The status bar says **Playing** or **Paused** too, and the Game tab is in front.
- **Problem:** Step is greyed out. **Why:** it runs one tick of a paused game. **Fix:** press Pause first.
- **Problem:** the game shows the wrong camera. **Why:** it uses the first one in the Area panel's list. **Fix:** delete the cameras you don't need, or take their Camera component off (**Remove Camera**, in the **⋯** on its card in Properties).
- **Problem:** the picture in the Game panel looks different from the viewport. **Why:** the game always uses the standard [look](looks.md); the viewport's look is only for editing. Hidden entities (the eye in the Area panel) show in the game, because hiding is only for the editor.

## Related
- [Interface tour](interface-tour.md)
- [The Console](console.md)
- [Entities and components](entities.md)
- Concepts: [play mode, game, camera](concepts.md#playing)
