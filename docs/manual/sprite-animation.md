# Animating sprites

> Make sprites walk, swing and wave: play the clips you tagged in Aseprite, make clips of your own in Solis, and give frames events that play footsteps and tell your game code.

## When to use it
For anything in a 2D game that moves frame by frame: a farmer walking, a hoe swinging, crops swaying, a door opening. A [clip](concepts.md#2d) is a run of a [sprite sheet](concepts.md#2d)'s frames, each shown for its own time, played forward, backward or there and back (ping-pong), over and over or a number of times.

The quickest way is Aseprite's own: **tag** runs of frames there (*walk*, *idle*, *hoe*) and every tag is a clip in Solis, with Aseprite's frame times. To change a clip's speed, give frames events (a footstep on the frame a foot lands), or make clips Aseprite doesn't have, Solis keeps a small **clips file** (`.clips.lst`) beside the Aseprite file. A sprite can play either.

To choose clips by what's happening (walking when moving, idle when not, facing the right way), use an [animator](animators.md).

## How to use it
1. **Tag your frames in Aseprite** and save. In Solis, click the file in the Library: Properties lists its tags.
2. **Play a clip on a sprite:** select a sprite and pick a **Clip** in its Sprite component. Press **Play**: it plays at its frame times. To see it move while you edit, open the viewport's **≡** menu and tick **Preview animation**.
3. **Open the clips:** in the Aseprite file's Properties, press **Edit clips…**. Solis makes `walker.clips.lst` beside `walker.aseprite` (or opens the one there) and shows its clips on the **Animate** tab:
   - **Frames**, the timeline: a tab of the bottom panel on the **Animate** tab (the clips file opens there; the × after it closes the file). In the left slot, **Clips**: every clip, with how long it is and where it comes from (*Aseprite* or *made here*). Click one to choose it. Beside them, the chosen clip's frames in order along a time ruler, each as wide as it lasts, so you see its timing at a glance. A ping-pong's way back is shown faded after them. The line in the accent colour is where it's playing; the lane below shows its events.
   - **Clip**, the right slot: the chosen clip's **Speed**, **Direction**, how many times it **Plays**, **Plays first**, and for a clip of your own, its **Directions**.

   While a clips file is open, Animate's [ribbon](concepts.md#the-window) is the clips file's: **Clip** (New, Copy, Delete), **Frames** (Take out), and how the clip plays (Plays, Speed, and a **Direction** list for a clip of your own). The design's cutscene controls after them (Keys, Step, Linear, Smooth, Tracks) are [unfinished parts](unfinished-parts.md).
4. **Watch it:** press **▶** (or **Space** over the Frames tab). **⏮** goes back to the start, **■** stops, **⏭** goes to the last frame. **−** and **+** zoom the ruler.
5. **Give a frame an event:** click the frame, then type a name in **event**, or pick a sound event from **◆ ▾**. When the clip reaches that frame:
   - a sound event of the same name (`footstep`, or its path, `sfx/footstep`) plays on the sprite by itself, with no code;
   - game code on the entity hears `CueReached`, with the event as one of the project's names (`Cue.Footstep`).
   Events typed in Aseprite (a cel's user data) work the same way.
6. **Make a clip of your own:** the ribbon's **New** makes an empty one; **Copy** copies the chosen clip (a tag's copy can then be changed here). In the Frames tab, **＋ Add frames ▾** shows the sheet's frames: click one to add it after the picked frame. Double-click a clip of your own to rename it.
7. **Change frames of your own clips:** click a frame to pick it (Ctrl+click or Shift+click for several), then set its **time**, drag it to a new place, or press **Delete** to take it out.
8. **Close the file:** the **×** after the Frames tab closes it.

### Directions

A character walking in a top-down game faces different ways, and each needs its own frames. Pick a clip of your own and choose its **Directions** in the Clip slot: **One** (it looks the same whatever way it faces), **Four** (south, east, north and west) or **Eight** (the diagonals too). The Frames tab then shows a row for each, south first, round by east to north:
- each frame is a tile as wide as it lasts, with its cell's number;
- click a frame to pick it and its row, then set its **time** and **event**, or add frames after it with **＋ Add frames**;
- click a row's name to pick the row;
- on the **Cells** page, the clips file's page in the middle, click a cell, or Shift+click to reach a run of them (Ctrl+click adds one), then drag them onto a row: the row shows those cells, keeping the times its frames had;
- with a frame picked, click a cell on the Cells page to show that cell in the frame instead.

The Cells page's left side plays the chosen clip's picked row, and its right side shows every cell, numbered. Under the preview, the **facing pad**'s arrows pick the direction it plays.

A clip with directions has two more settings in the Clip slot:
- **Same frames in every direction** (on at first): every direction shares one set of frame times and events, so a character turning mid-walk keeps its step and the footsteps land the same way. Off: each direction has its own times and events, starting as copies of the shared ones.
- **Mirror the west side** (on at first): northwest, west and southwest are northeast, east and southeast flipped, so you draw five directions, not eight. Picking a west direction on the pad plays its east row flipped, and changes made there go to the east row. Off, for a character that isn't the same both ways (a sword in one hand): the west rows become rows of their own, starting as copies of the east cells, for you to swap in the frames you drew.

Turning either setting off and back on gives the clip back as it was.

Going from eight to four keeps the four straight rows. A new row starts as a copy of the first, so you only change what differs. A clip that mirrors its west side shows those rows faded, as the east rows they're flipped from (◈ NorthEast).

### A plain picture's clips

A PNG or JPEG with every frame laid out on it, a *sprite sheet*, gets clips too. Right-click it in the Library and choose **Make clips…**: Solis makes `knight.clips.lst` beside `knight.png`, opens its tabs, and opens **Cut knight.png into cells** over them. That pop-up cuts the picture into [cells](concepts.md#cell), the pieces its clips show, numbered from 0 across and then down, as Aseprite numbers frames:
- **Cell** is a cell's width (W) and height (H) in pixels; **Between** the gap between cells; **Edge** the margin round the picture. The cells and their numbers move over the picture as you type, and the count under it says how many there are ("45 cells: 9 across, 5 down").
- **Find grid** looks for see-through gaps between the cells and round the edge, and fills in all three. With no gaps it guesses the cell size from where the art changes.
- **Free crop**, the pop-up's other mode, is for a picture whose frames aren't on an even grid. The grid's cells show faintly, and you can:
  - drag on the picture to draw a cell, which gets the next number after the grid's;
  - click a cell to pick it, then drag it to move it, drag a corner to size it, or type its **X**, **Y**, **W** and **H**;
  - press the arrow keys to move the picked cell a pixel at a time, for lining frames up exactly;
  - press **Back to the grid's box** to forget a cell's own box: a grid cell goes back to the grid's, and an added one is gone.
- The wheel zooms the picture and the middle mouse button moves it.
- **Use these cells** keeps the grid in the clips file, as one step **Ctrl+Z** undoes; **Cancel** (or **Esc**) leaves it as it was. To change it later, press **Cut picture…** above the cells on the **Cells** page.

Every cell stands on the same spot, its bottom middle where the sprite is, so a character doesn't hop between frames cut from different places. Clips made here then play the cells as they play an Aseprite file's frames. An Aseprite file needs no cutting: its frames are its cells.

Every change is saved at once and shows at once on every sprite playing the file. **Ctrl+Z** and **Ctrl+Y** over the clips undo and redo changes to the file. If the file changes on disk (edited by hand, or by Git), the tabs show it as it is now, as a step **Ctrl+Z** undoes; if it's deleted or moved, they close. If a save can't be made, the Console says why and the change is taken back, kept as an undo step: see [Undo and redo](undo.md).

### Looks
A character can wear other clothes, armour or colours without making its clips again. Draw the other picture the same size and laid out the same way, `knight-mana.png` beside `knight.png`, and add it as a **look** of the clips file: every clip then plays in it, from the same cells.

- **+ Look** on the ribbon (or **+** beside **Looks** under the clips) adds a look; pick its picture in the **Look** settings on the right. A look's picture must be cut the same way as the main one, so a picture of another size is said under it ("306 × 170, as the main look" when it fits).
- The **Look** switch on the ribbon, or a click in the **Looks** list, shows the clips in that look: the cells, the rows and the preview all draw its picture, and the place in the clip is kept, so you can check one frame in every look.
- In the look's settings, **Name** renames it, **Make main** makes it the look sprites wear when their Look is empty, and **Remove look** takes it out. **← Back to the clip** goes back to the clip's settings.

A sprite wears a look by its **Look** setting in Properties (see [Settings](#settings)).

### In game code
Clips and their events are names Lust checks. Solis writes them into `solis/project.lst` whenever the sheets change: each clip as `state Clip`, under its sheet's path (`sprites/farmer.aseprite`'s walk is `Clip.Sprites.Farmer.Walk`), and each frame event as `state Cue` (`Cue.Footstep`). A misspelt clip is a mistake before the game runs. Bring in the names and the events at the top of a file:

```lust
fetch "solis/project":[Clip, Cue];
fetch "solis/animation":[CueReached, ClipFinished];

component Farmer {
    has {
        int steps = 0;
    }

    -- Count the footsteps as they land.
    on CueReached(Cue cue) {
        is cue == Cue.Footstep? then {
            steps = steps + 1;
        }
    }

    -- Back to standing once the hoe swing is done.
    on ClipFinished(Clip clip) {
        is clip == Clip.Sprites.Farmer.Hoe? then {
            animate(Clip.Sprites.Farmer.Idle);
        }
    }
}

pass Farmer;
end
```

```rust
use reflect::Value;
use script::{Context, Event, RustComponent};

pub struct Farmer {
    steps: i64,
}

impl RustComponent for Farmer {
    fn handle(&mut self, cx: &mut Context<'_>, event: &Event) -> bool {
        // The engine's events carry the project's names as text.
        let named = match event.values.first() {
            Some(Value::Text(name)) => name.as_str(),
            _ => "",
        };
        match event.name.as_str() {
            // Count the footsteps as they land.
            "CueReached" if named == "Footstep" => self.steps += 1,
            // Back to standing once the hoe swing is done.
            "ClipFinished" if named == "Sprites.Farmer.Hoe" => {
                let me = cx.me();
                cx.host().play_clip(me, "idle", false);
            }
            _ => return false,
        }
        true
    }

    // values() and set(): as Mover's in Game code, under Components.
}
```

```lua
local Farmer = component("Farmer", {
    { "steps", "int", 0 },
})

-- Count the footsteps as they land.
function Farmer:on_CueReached(cue)
    if cue == "Footstep" then
        self.steps = self.steps + 1
    end
end

-- Back to standing once the hoe swing is done.
function Farmer:on_ClipFinished(clip)
    if clip == "Sprites.Farmer.Hoe" then
        solis.entity.animate(self.me, "Sprites.Farmer.Idle")
    end
end
```

- `animate(Clip.Sprites.Farmer.Walk)` plays a clip on the component's entity (`target:animate(…)` on another). If it's already playing that clip, or has finished it, it carries on as it is, so it's safe to call every tick; another clip starts from its beginning.
- `animateFromStart(Clip.Sprites.Farmer.Hoe)` plays a clip from its beginning, whatever it was doing: a swing each time a button is pressed.
- `stopAnimation()` stops it where it is; `animate` with the same clip carries on from there.
- `setFlip(across, down)` mirrors the sprite (its **Flip across** and **Flip up** settings): `setFlip(true, false)` faces it the other way. So you draw a run facing right only once, and one `Run` clip does both ways, instead of a `RunLeft` as well:

  ```lust
  fetch "solis/input":Input;
  fetch "solis/project":[Action, Clip];

  component Runner {
      method update(float dt) {
          -- Face the way the player pushes.
          bool walkingLeft = Input:axis(Action.Move).x < 0;
          setFlip(walkingLeft, false);
          animate(Clip.Sprites.Hero.Run);
      }
  }

  pass Runner;
  end
  ```

  ```rust
  use script::{Context, RustComponent};

  pub struct Runner;

  impl RustComponent for Runner {
      fn update(&mut self, cx: &mut Context<'_>, _dt: f64) {
          let me = cx.me();
          // Face the way the player pushes.
          let walking_left = cx.host().action("Move").axis.x < 0.0;
          cx.host().set_flip(me, walking_left, false);
          cx.host().play_clip(me, "run", false);
      }

      // values() and set(): as Mover's in Game code, under Components.
  }
  ```

  ```lua
  local Runner = component("Runner", {})

  function Runner:update(dt)
      -- Face the way the player pushes.
      local walkingLeft = solis.Input:axis("Move").x < 0
      solis.entity.setFlip(self.me, walkingLeft, false)
      solis.entity.animate(self.me, "Sprites.Hero.Run")
  end
  ```

  It's safe to call every tick, and like the other calls it lands at the end of the tick.
- In Lua: `solis.entity.animate(e, "Sprites.Farmer.Walk")`, `solis.entity.animateFromStart(e, …)`, `solis.entity.stopAnimation(e)` and `solis.entity.setFlip(e, true, false)`; a function called `on_CueReached(cue)` or `on_ClipFinished(clip)` reacts to the events, with the names as text. In Rust: `cx.host().play_clip(entity, "walk", false)` (`true` to start it again), `stop_clip` and `set_flip(entity, true, false)`, and the events arrive in `handle` named `CueReached` and `ClipFinished`.

#### Facing and looks
Which way a sprite faces and which look it wears aren't calls: they're values the component holds, and Solis follows them. Give the component a field `Facing facing` or `Look look` in its `has` block, set it with `=`, and as each tick ends the entity's sprite turns or changes clothes to match. A field is the plain way to say what something *is*: the component reads its own facing back whenever it likes, it shows in Properties while the game runs, and saved games keep it with the component's other values.

- **`Facing facing`** turns the sprite: `facing = Facing.NorthEast;`. `Facing` is in `solis/animation`, with the eight ways (`North`, `NorthEast`, `East`, `SouthEast`, `South`, `SouthWest`, `West`, `NorthWest`), up the screen being North. A clip with [directions](#directions) plays the row nearest it, keeping its place, so turning mid-walk doesn't restart the step. `nearestFacing(way)` gives the facing nearest a way on the screen, such as which way the player pushes.
- **`Look look`** dresses the sprite in one of its clips file's [looks](#looks): `look = Look.KnightMana;`. Each look is named by its clips file's name and its own, joined (`knight.clips.lst`'s Mana is `Look.KnightMana`; files of the same name in different folders add the folder in front, `Look.HeroesKnightMana`). Solis writes them into `solis/project.lst` with the clips, so a misspelt look is a mistake before the game runs.

```lust
fetch "solis/input":Input;
fetch "solis/project":[Action, Clip, Look];
fetch "solis/animation":Facing;
get "solis/animation":nearestFacing;

component Knight {
    has {
        Facing facing = Facing.South;
        Look look = Look.KnightMain;
        bool charged = false;
    }

    method update(float dt) {
        vec3 push = Input:axis(Action.Move);
        is push:length() > 0? then {
            -- Turn the way the player pushes: the walk plays that row.
            facing = nearestFacing([push.x, push.y]);
            animate(Clip.Sprites.Knight.Walk);
        } or {
            animate(Clip.Sprites.Knight.Idle);
        }
        -- Mana armour while charged.
        is charged? then {
            look = Look.KnightMana;
        } or {
            look = Look.KnightMain;
        }
    }
}

pass Knight;
end
```

The component's field is the sprite's facing and look from the game's first tick: its starting value wins over the Sprite's own **Facing** and **Look** in Properties. Set it only while moving: `nearestFacing` of no way at all is South.

Two mistakes are said in the Console, once a play for each entity, and change nothing:
- **A look from another clips file**, `look = Look.OrcRed;` on the knight: the Console names the file the look belongs to and the looks this sprite can wear.
- **A facing on a sprite whose animator turns it**: a state that [faces by a parameter](animators.md) sets the facing itself each tick, so set the animator's direction parameter instead.

A clip's place is part of the game: the same inputs give the same frames every time, and a saved game comes back on the same frame.

## Settings
The Sprite component's clip settings:

| Setting | What it does |
|---|---|
| Sheet | An Aseprite file (its tags are its clips) or a clips file (`.clips.lst`). |
| Clip | The clip it plays. Empty: the clips file's clip marked **Plays first**, or with none, the still **Frame**. |
| Look | Which of its clips file's looks it shows: the main one (empty), or another picture cut the same way, such as the knight in mana armour. Every clip plays in it. A look's picture should be the main one's size, or its cells won't line up (the Clip slot says so). |
| Facing | Which way it faces: North, NorthEast, East, SouthEast, South, SouthWest, West or NorthWest. A clip with directions plays the row nearest it: with four directions, a diagonal keeps showing north or south if that's what it showed, and otherwise turns east or west; a mirrored west side plays its east row flipped. Turning mid-clip keeps its place in the clip. |
| Playing | Whether the clip moves on while the game runs. Game code's `animate` and `stopAnimation` turn it on and off. |
| Speed | 1 as drawn, 2 twice as fast, 0 stopped. Multiplies the clip's own speed. |
| Clip time | How far into the clip it is, in seconds. Give a crowd of copies different times so they don't all step in step. |
| Frame | The frame shown when no clip plays. |

A clip's settings in the Clip slot:

| Setting | What it does |
|---|---|
| Speed | How fast this clip plays for every sprite, 1 as drawn. A tag's speed is kept in the clips file. |
| Direction | Forward, reverse, ping-pong (there and back) or ping-pong reversed. A tag's is Aseprite's. |
| Plays | Over and over, once, or a number of times; it then stays on its last frame and game code hears `ClipFinished`. A tag's is Aseprite's (its Repeat field). |
| Plays first | The clip a sprite plays when its Clip is empty. |
| Directions | For a clip of your own: **One**, **Four** or **Eight**, the ways it faces. Each has a row of frames in the Frames tab. |

A look's settings, also in the Clip slot when a look is picked:

| Setting | What it does |
|---|---|
| Name | The look's name, as a Sprite's **Look** picks it. |
| Picture | The picture it draws, cut as the main look's. Its size is checked against the main one's. |
| Make main | Makes it the look a sprite wears when its Look is empty. |
| Remove look | Takes it out of the clips file (never the main look); sprites wearing it go back to the main look. |

## Shortcuts
| Action | Shortcut |
|---|---|
| Rename a clip (your own clips) | Double-click it in the Frames tab's list of clips (`Enter` keeps the name, `Esc` leaves it) |
| Pick a frame | Click it |
| Pick several frames | `Ctrl` + click, or `Shift` + click for a run |
| Move picked frames (your own clips) | Drag them |
| Take picked frames out (your own clips) | `Delete` over the Frames tab |
| Play or stop the clip | `Space` over the Frames tab |
| Undo or redo a change to the clips file | `Ctrl + Z`, `Ctrl + Y` over the clips |

## Tips and common problems
- **Problem:** a look's frames are cut off or show the wrong part. **Why:** its picture isn't the main one's size or layout. **Fix:** draw it on a copy of the main picture, so the cells line up; the Look settings say when the sizes differ.
- **Problem:** sprites don't move in the viewport. **Why:** the editor rests unless something changes. **Fix:** tick **Preview animation** in the viewport's **≡** menu, or press **Play**.
- **Problem:** a tag's frames can't be moved, timed or taken out. **Why:** a tag's frames are Aseprite's, so saving there keeps both in step. **Fix:** change them in Aseprite, or **Copy** the clip (on the ribbon) to change the copy here.
- **Problem:** an event doesn't play a sound. **Why:** no sound event has its name. **Fix:** name it after the sound event (`footstep` or `sfx/footstep`), or pick one from **◆ ▾**.
- **Problem:** a whole field of crops sways in step. **Fix:** give each a slightly different **Clip time**.
- **Problem:** two footsteps in a ping-pong walk. **Why:** a ping-pong shows its middle frames twice a pass, there and back. **Fix:** put the event on the first or last frame, or use **Forward** for walks.

## Related
- [Sprites from Aseprite](sprites.md): sheets, placing sprites, pivots.
- [Sound events](sound-events.md): the sounds an event plays.
- [Game code](game-code.md): components and events.
- [Concepts](concepts.md#2d): clip, clips file, frame event.
