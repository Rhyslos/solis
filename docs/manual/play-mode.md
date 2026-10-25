# Play mode

> Run the area as a game inside the editor, try things, and press Stop to get everything back exactly as it was.

## When to use it
Whenever you want to see the area the way a player will: through the game's camera, with no grid, gizmos or markers. The game's own [code](game-code.md) runs too: every component written in Lust runs its `update` 60 times a second. The habit is worth learning now: **anything you change while playing is undone when you press Stop**, so you can move things, delete things and experiment freely.

## How to use it

**Add a camera first** (once per area)
1. Move the viewport to look the way the game should look.
2. Choose **Entity > Add camera**. A camera appears exactly where you're looking from, facing the same way, and is selected. In the viewport it's drawn as a small wire pyramid pointing the way it looks; click it to select it.
3. Move and turn it like anything else (**W**, **R**). Its **Field of view** in Properties sets how wide it sees.

The game looks through the **first camera** in the Area panel's list. With no camera, the Game panel shows the editor's own view and says so.

**Play, pause and stop** (the buttons at the top right)
1. Press **Play** (▶). The **Game** panel comes to the front and shows the game. A coloured border goes round the viewport and the Game panel, Play is outlined, and **Stop** turns red: all reminders that you're playing.
2. Work as usual: the Area panel, Properties, the viewport and its gizmos all still work. Changes show in the game straight away. Switch to the **Viewport** tab beside Game to move things with the gizmos.
3. Press **Pause** (❚❚) to stop the game's clock; press Pause or Play to carry on. The Game panel's corner shows *Playing* or *Paused* and how long the game has run.
4. While paused, press **Step** (▶❙, beside Pause) to run exactly one [tick](concepts.md#game-code): game code runs once, everything moves on by a sixtieth of a second, and the game stays paused. It's the way to watch something that happens too fast to see, one step at a time.
5. Press **Stop** (■). The game is thrown away and the area comes back exactly as it was when you pressed Play: every entity and setting, the undo history (Ctrl + Z carries on from where you were) and the selection.

You can also type `play`, `pause`, `step` and `stop` in the [Console](console.md).

**Smooth on any screen:** the game itself always runs 60 [ticks](concepts.md#game-code) a second, so it plays the same on every computer. A screen can show more frames than that (120 or 144 a second): between two ticks, the Game panel draws everything part way from where it was at the last tick to where it is now, so movement looks smooth instead of stepping. What you see is up to a sixtieth of a second behind the game, which no one can notice. While paused, it shows exactly where things are.

**What waits until you stop:** saving, opening another area or project, new areas, and making or changing assemblies. If you try, the Console says *Stop playing first*: saving now would save the game's changes, which Stop is about to undo. Closing Solis while playing stops first, then asks about unsaved changes as usual.

**Changes from outside while playing:** if an assembly's file changes (a Git pull, or Notepad), the game shows it at once, and its copies are updated again after Stop. If the area's own file changes on disk, Solis waits until Stop to offer it.

## Settings
| Setting | What it does |
|---|---|
| Camera > Field of view | How wide the camera sees, top to bottom, in degrees. 50° looks like a normal photo; 90° sees much more but stretches the edges. |
| Camera > Near | The nearest it sees. Raise it if far surfaces flicker. |
| Camera > Far | The farthest it sees; anything beyond isn't drawn. |

## Shortcuts
Play, Pause, Step and Stop are buttons only, so a stray key never starts or stops the game. In the Console: `play`, `pause`, `step`, `stop`.

## Tips and common problems
- **Problem:** I changed something while playing and it went back when I pressed Stop. **Why:** that's what Stop is for. **Fix:** make lasting changes after Stop. To keep a value you tried while playing, note it down (or copy the component's values with ⋯ > Copy values, and paste them after Stop).
- **Problem:** the Game panel shows the editor's view. **Why:** the area has no camera. **Fix:** Entity > Add camera.
- **Problem:** new entities appear in the Area panel while playing. **Why:** game code made them (`spawn`, see [Game code](game-code.md#making-and-removing-entities)). **Fix:** nothing: they go when you press Stop.
- **Problem:** Step is greyed out. **Why:** it runs one tick of a paused game. **Fix:** press Pause first.
- **Problem:** the game shows the wrong camera. **Why:** it uses the first one in the Area panel's list. **Fix:** delete the cameras you don't need, or take their Camera component off (the × beside it in Properties).
- **Problem:** the picture in the Game panel looks different from the viewport. **Why:** the game always uses the standard [look](looks.md); the viewport's look is only for editing. Hidden entities (the eye in the Area panel) show in the game, because hiding is only for the editor.

## Related
- [Interface tour](interface-tour.md)
- [The Console](console.md)
- [Entities and components](entities.md)
- Concepts: [play mode, game, camera](concepts.md#playing)
