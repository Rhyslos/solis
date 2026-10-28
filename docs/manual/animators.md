# Animators

> Let Solis choose which clip a sprite plays: idle when standing, walking when moving, a swing when a tool is used, each facing the right way.

## When to use it
A farmer has an idle, a walk and a tool swing, each drawn facing up, right, down and left: twelve clips, and rules for when each plays. You could pick every clip yourself (see [Animating sprites](sprite-animation.md)), but an **animator** keeps those rules in one place, drawn as a picture you can read at a glance:

- **[States](concepts.md#2d)** are what the sprite can be doing: *Idle*, *Walk*, *Use tool*. Each plays a clip, or a clip for each direction.
- **[Transitions](concepts.md#2d)** are the arrows between them: *Idle* goes to *Walk* when *moving* is on.
- **[Parameters](concepts.md#2d)** are the values transitions look at: *moving* (a switch), *speed* (a number), *swing* (a trigger) and *facing* (a direction).

Why a picture rather than code? Because animation rules get tangled fast ("swing, unless already swinging, then go back to walking if still moving…"), and a graph shows every way in and out of each state. The animator also runs inside the game's ticks, so the same inputs always give the same animation, on every computer and in every replay and save.

For one-off sprites (a door, a crop swaying) a plain clip on the Sprite is simpler: no animator needed.

## How to use it
1. **Make one:** in the Library, open the folder you want it in, then **Import > New animator**. Solis makes `animator.solanim` with one state, *Idle*, and opens it. (Rename the file in the Library, as any other.)
2. **The Animator tab** opens first beside the Viewport, and the file shows as a second tab in the top bar (click it, or the area's tab, to switch). On the left are the parameters and a preview; on the right, the graph.
3. **Choose its clips:** click empty space in the graph. Properties shows the animator: pick **Clips from**, the sprite sheet (or clips file) whose clips its states play.
4. **Add states:** right-click empty space > **Add state**. Click a state to change it in Properties:
   - **Name**: what it's called.
   - **Plays**: *one clip*, or *a clip by facing*. With a clip by facing, pick a clip for each direction (**↑ up**, **→ right**, **↓ down**, **← left**); **+ Direction** adds the next one, up to eight with the diagonals. Solis makes a direction parameter, *facing*, the first time.
   - **Speed**, and **Speed by**: a number parameter its speed is multiplied by, so a walk speeds up as the character does.
   - **Start here**: the state the animator starts in, marked **Start ▶** in the graph.
5. **Add parameters:** the **+** beside *Parameters* adds a number, a switch, a trigger or a direction. Click a name to rename it; right-click it to **Delete** it, or **Start at this value**.
6. **Join states:** drag from the edge of one state to another, or right-click a state > **Make transition** and click the other. Click the arrow, then in Properties **+ Condition** and choose what must hold: a switch **is on** or **is off**, a trigger **fired**, a number **>** or **<** a value, or **finished** (the state's clip has played all its times). All of a transition's conditions must hold for it to be taken. An arrow from **Any state** can be taken from every state.
7. **Try it:** the preview always plays. Set the parameters on the left (tick *moving*, turn the *facing* dial, press **Fire** for a trigger) and watch the state glow blue as it changes, the clip playing below.
8. **Use it:** select an entity with a Sprite, then in Properties **Add component > Animator** and pick the file in **Animator**. Press **Play**: the animator chooses the Sprite's clip and speed every tick. Select the entity while it plays to see its state glow, and its parameters' values on the left.

Every change is saved at once and runs at once, in a game that's playing too. **Ctrl + Z** and **Ctrl + Y** over the tab undo and redo changes to the animator. If the file changes on disk (edited by hand, or by Git), the tab shows it as it is now.

### In game code
Game code can't set an animator's parameters yet. Under Solis's [API guidelines](../game-api-guidelines.md), it will reach each animator through a Lust file Solis writes for it, with a state of its state names and a method for each parameter, so a misspelt name is caught before the game runs. Until then, a game picks clips itself with `playClip` (see [Animating sprites](sprite-animation.md)), and the animator is for trying rules out in the preview.

## Settings
The Animator component:

| Setting | What it does |
|---|---|
| Animator | The animator file (`.solanim`) it runs. |
| State | The state it starts in; empty for the animator's own start. While playing, the state it's in. |
| Speed | How fast its states play: 1 as the animator says, 2 twice as fast. It multiplies each state's own speed. |

With an Animator, the animator sets the Sprite's **Clip** and **Speed** while the game runs; in the editor the sprite shows its own Clip.

## Shortcuts
These work while the pointer is over the Animator tab.

| Action | Shortcut |
|---|---|
| Pick a state or an arrow; add to what's picked | Click; Ctrl + click or Shift + click |
| Pick states with a box | Drag on empty space |
| Move picked states | Drag one of them |
| Join two states | Drag from a state's edge to another |
| Add a state, Make transition, Start here, Delete | Right-click empty space, a state or an arrow |
| Take away what's picked | Delete |
| Pan | Middle-drag |
| Zoom round the pointer | Mouse wheel |
| Fit everything in view | F |
| Undo or redo a change to the animator | Ctrl + Z, Ctrl + Y |

## Tips and common problems
- **Problem:** a transition is taken at once. **Why:** it has no conditions (its arrow says *always*). **Fix:** click the arrow and add one.
- **Problem:** the sprite flickers between two states. **Why:** each one's transition to the other holds at the same time (*moving* and *not moving* can't both hold, but *speed > 0* and *speed < 1* can). **Fix:** make the conditions exclude each other.
- **Problem:** a swing never ends. **Why:** its clip plays over and over, so it never *finishes*. **Fix:** make the clip play once (its **Plays** in the Clips tab), or leave it on a different condition.
- **Problem:** a box says *missing* beside a clip. **Why:** the sprite sheet has no clip by that name (renamed in Aseprite?). **Fix:** pick the clip again.
- **Problem:** pressing a trigger twice quickly swings once. **Why:** a trigger lasts one tick: one fired while already swinging is put out (it doesn't wait). That's on purpose, so presses don't pile up.
- Problems lists an Animator with no file chosen, or whose file isn't in the project.

## Related
- [Animating sprites](sprite-animation.md): the clips states play.
- [Sprites from Aseprite](sprites.md).
- [Save games](save-games.md): saves keep each animator's state and parameters.
- [Concepts](concepts.md#2d): animator, state, transition, parameter, trigger.
