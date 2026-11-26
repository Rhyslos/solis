# Animating sprites

> Make sprites walk, swing and wave: cut a picture into cells, put them into named clips that face four or eight ways, dress them in other looks, and play them from game code by name.

## When to use it
For anything in a 2D game that moves frame by frame: a knight walking, a hoe swinging, crops swaying, a door opening. A [clip](concepts.md#2d) is a run of [cells](concepts.md#cell), the pieces a picture is cut into, each shown for its own time, played forward, backward or there and back (ping-pong), over and over or a number of times.

A character's clips live in one **clips file** (`knight.clips.lst`) beside its picture: every clip by name (*walk*, *idle*, *swing*), each facing one, four or eight ways, and the picture's other **looks** (the knight in mana armour), which every clip plays in. Game code then asks for a clip by its name, says which way the sprite faces and which look it wears, and Solis picks the frames. A character with ten actions, eight directions and two looks is ten clips, not a hundred and sixty.

Drawn your frames in Aseprite? Its tags are clips already, with Aseprite's frame times, and a clips file beside it adds what Aseprite doesn't have.

To choose clips by what's happening (walking when moving, idle when not), use an [animator](animators.md), or a `check` in game code.

## How to use it
1. **Make the clips file.** Right-click the picture (a PNG or JPEG) in the Library and choose **Make clips…**. Solis makes `knight.clips.lst` beside `knight.png`, opens it on the **Animate** tab, and opens the pop-up that [cuts the picture into cells](#cutting-a-picture-into-cells). For an Aseprite file, press **Edit clips…** in its Properties instead: its frames are its cells, and its tags its first clips ([Clips from Aseprite](#clips-from-aseprite)).
2. **Find your way round.** With a clips file open, the Animate tab shows:
   - on the left, the **Clips** (each with how many directions it has, and whether it's *made here* or from *Aseprite*) and under them its **Looks**;
   - in the middle, the **Cells** page: the chosen clip playing on the left, with the **facing pad** under it, and every cell of the picture, numbered, on the right, with **Cut picture…**;
   - below, the **Frames** tab: the chosen clip's frames, a row for each direction, each frame as wide as it lasts along a time ruler, its events in a lane below. A ping-pong's way back is shown faded after them; the line in the accent colour is where it's playing;
   - on the right, the **Clip** slot: the chosen clip's settings (Speed, Direction, Plays, Plays first, Directions), or a look's when one is picked.

   Animate's [ribbon](concepts.md#the-window) is the clips file's: **Clip** (New, Copy, Delete), **Frames** (Take out), **Look** (the look shown, and **+ Look**), and how the clip plays (Plays, Speed, and a **Direction** list for a clip of your own). The design's cutscene controls after them (Keys, Step, Linear, Smooth, Tracks) are [unfinished parts](unfinished-parts.md).
3. **Make a clip:** the ribbon's **New** makes an empty one; double-click it to name it (*walk*). On the Cells page, click a cell, Shift+click to reach a run of them (Ctrl+click adds one), and drag them onto the clip's row in the Frames tab. **Copy** copies the chosen clip, so a run can start from a walk.
4. **Time it:** click a frame (Ctrl+click or Shift+click for several), then set its **time**, drag it to a new place, or press **Delete** to take it out. With a frame picked, click a cell on the Cells page to show that cell in the frame instead. **＋ Add frames ▾** adds the picture's cells after the picked frame.
5. **Give a frame an event:** click the frame, then type a name in **event**, or pick a sound event from **◆ ▾**. When the clip reaches that frame:
   - a sound event of the same name (`footstep`, or its path, `sfx/footstep`) plays on the sprite by itself, with no code;
   - game code on the entity hears `CueReached`, with the event as one of the project's names (`Cue.Footstep`).
6. **Give it directions,** for a character that turns: choose **Four** or **Eight** under **Directions** in the Clip slot, and fill each direction's row ([Directions](#directions)).
7. **Watch it:** press **▶** (or **Space** over the Frames tab). **⏮** goes back to the start, **■** stops, **⏭** goes to the last frame. **−** and **+** zoom the ruler. The facing pad's arrows play each direction.
8. **Play it on a sprite:** drop the clips file on the viewport, or select a sprite and pick the clips file as its **Sheet**. Pick its **Clip**, and its **Facing** and **Look** if it has them. Press **Play**: it plays at its frame times. To see it move while you edit, open the viewport's **≡** menu and tick **Preview animation**.
9. **Close the file:** the **×** after the Frames tab closes it.

### Cutting a picture into cells
A PNG or JPEG with every frame laid out on it, a *sprite sheet*, is cut into cells before its clips can show them. **Make clips…** opens **Cut knight.png into cells** over the clips file's tabs. That pop-up cuts the picture into [cells](concepts.md#cell), the pieces its clips show, numbered from 0 across and then down, as Aseprite numbers frames:
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

### Looks
A character can wear other clothes, armour or colours without making its clips again. Draw the other picture the same size and laid out the same way, `knight-mana.png` beside `knight.png`, and add it as a **look** of the clips file: every clip then plays in it, from the same cells.

- **+ Look** on the ribbon (or **+** beside **Looks** under the clips) adds a look; pick its picture in the **Look** settings on the right. A look's picture must be cut the same way as the main one, so a picture of another size is said under it ("306 × 170, as the main look" when it fits).
- The **Look** switch on the ribbon, or a click in the **Looks** list, shows the clips in that look: the cells, the rows and the preview all draw its picture, and the place in the clip is kept, so you can check one frame in every look.
- In the look's settings, **Name** renames it, **Make main** makes it the look sprites wear when their Look is empty, and **Remove look** takes it out. **← Back to the clip** goes back to the clip's settings.

A sprite wears a look by its **Look** setting in Properties (see [Settings](#settings)).

### Clips from Aseprite
An Aseprite file needs no cutting: its frames are its cells, numbered from 0 as Aseprite numbers them. **Tag** runs of frames in Aseprite and save, and every tag is a clip in Solis with Aseprite's frame times, direction and repeat count; click the file in the Library and Properties lists them. A sprite can play the Aseprite file's clips straight away, with no clips file.

**Edit clips…** in the file's Properties makes `walker.clips.lst` beside `walker.aseprite` (or opens the one there), for what Aseprite doesn't have: a clip's speed, events on frames, directions and clips of your own. A tag's frames stay Aseprite's, so saving there keeps both in step: to change them in Solis, **Copy** the clip and change the copy. Events typed in Aseprite (a cel's user data) work like events added here.

**How a tag's name becomes a clip's name.** The clip keeps the tag's name as it is (`walk`, `Walk NE`). Game code names a clip by its file's path and the tag, each part with its first letter made a capital: `sprites/walker.aseprite`'s `walk` is `Clip.WalkerWalk`. A name of all capitals keeps only its first (`IDLE` is `Idle`). A tag whose name holds a space or a dash, or starts with a digit, plays and edits as usual but can't be named in game code, and the Console says so when the names are written: rename the tag in Aseprite (`WalkNE`), or give the clip a direction instead ([Directions](#directions)), which is what directions are for.

### Saving and undo
Every change is saved at once and shows at once on every sprite playing the file. **Ctrl+Z** and **Ctrl+Y** over the clips undo and redo changes to the file. If the file changes on disk (edited by hand, or by Git), the tabs show it as it is now, as a step **Ctrl+Z** undoes; if it's deleted or moved, they close. If a save can't be made, the Console says why and the change is taken back, kept as an undo step: see [Undo and redo](undo.md).

### In game code
Clips and their events are names Lust checks. Solis writes them into `solis/project.lst` whenever the sheets change: each clip as `state Clip`, under its sheet's path (`sprites/farmer.aseprite`'s walk is `Clip.FarmerWalk`), with `Clip.None` for no clip, and each frame event as `state Cue` (`Cue.Footstep`). A misspelt clip is a mistake before the game runs. Which clip plays is a field of the component, `Clip animation`, set with `=`: a field says what the sprite *is* doing, so the component can read it back, and there's no call to make every tick. Bring in the names and the events at the top of a file:

```lust
fetch "solis/project":[Clip, Cue];
fetch "solis/animation":[CueReached, ClipFinished];

component Farmer {
    has {
        Clip animation = Clip.FarmerIdle;
        int steps = 0;
    }

    on {
        -- Count the footsteps as they land.
        CueReached(Cue cue) {
            is cue == Cue.Footstep? then {
                steps = steps + 1;
            }
        }

        -- Back to standing once the hoe swing is done.
        ClipFinished(Clip clip) {
            is clip == Clip.FarmerHoe? then {
                animation = Clip.FarmerIdle;
            }
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
            "ClipFinished" if named == "FarmerHoe" => {
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
    if clip == "FarmerHoe" then
        solis.entity.animate(self.me, "FarmerIdle")
    end
end
```

- **`Clip animation`** in the component's `has` is the clip its entity's sprite plays: `animation = Clip.FarmerWalk;`. As each tick ends, the engine plays it if it has changed (or the component has just started): set to the clip already playing, or one it has finished, it carries on as it is; another clip starts from its beginning. Its starting value is the sprite's clip from the game's first tick.
- **`restartAnimation()`** plays the clip the sprite is playing again from its beginning, whatever it was doing: a second swing after the first.
- **`animation = Clip.None;`** stops the sprite where it is; setting a clip again carries on from there.
- **Another entity's sprite:** `orc:animate(Clip.OrcWalk)`, `orc:animateFromStart(…)` and `orc:stopAnimation()`.
- **`Flip flip`** mirrors the sprite (its **Flip across** and **Flip up** settings): `flip = Flip.Across;` faces it the other way, `Flip.Down` turns it upside down, `Flip.Both` does both and `Flip.None` neither (`Flip` is in `solis/animation`; on another entity, `orc:setFlip(true, false)`). So you draw a run facing right only once, and one `Run` clip does both ways, instead of a `RunLeft` as well:

  ```lust
  fetch "solis/input":Input;
  fetch "solis/project":[Action, Clip];
  fetch "solis/animation":Flip;

  component Runner {
      has {
          Clip animation = Clip.HeroRun;
          Flip flip = Flip.None;
      }

      method update(float dt) {
          -- Face the way the player pushes.
          is Input:axis(Action.Move).x < 0? then {
              flip = Flip.Across;
          } or {
              flip = Flip.None;
          }
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
      solis.entity.animate(self.me, "HeroRun")
  end
  ```

  The fields land at the end of the tick, like a move. Give each to one component of an entity: two would each set it their own way.
- In Lua: `solis.entity.animate(e, "FarmerWalk")`, `solis.entity.animateFromStart(e, …)`, `solis.entity.stopAnimation(e)` and `solis.entity.setFlip(e, true, false)`; a function called `on_CueReached(cue)` or `on_ClipFinished(clip)` reacts to the events, with the names as text. In Rust: `cx.host().play_clip(entity, "walk", false)` (`true` to start it again), `stop_clip` and `set_flip(entity, true, false)`, and the events arrive in `handle` named `CueReached` and `ClipFinished`.

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
        Clip animation = Clip.KnightIdle;
        bool charged = false;
    }

    method update(float dt) {
        vec3 push = Input:axis(Action.Move);
        is push:length() > 0? then {
            -- Turn the way the player pushes: the walk plays that row.
            facing = nearestFacing([push.x, push.y]);
            animation = Clip.KnightWalk;
        } or {
            animation = Clip.KnightIdle;
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

**Why it's built this way.** Before clips had directions and looks, a knight with ten actions facing eight ways in two looks was 160 clips, each with its own name, and game code needed a lookup table to find the right one: one line for every action, direction and look.

```text
-- Before: a clip for every action, direction and look.
check facing {
    when North { animation = Clip.KnightWalkNorthMain; }
    when NorthEast { animation = Clip.KnightWalkNorthEastMain; }
    -- … six more directions, then all eight again for the mana look,
    -- then the same for idle, swing, hurt and the other actions.
}
```

Now the clip is named once and the facing and the look are values beside it, so the table is gone:

```text
-- After: one clip; the facing and the look choose its frames.
facing = nearestFacing([push.x, push.y]);
animation = Clip.KnightWalk;
```

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
| Look | Which of its clips file's looks it shows: the main one (empty), or another picture cut the same way, such as the knight in mana armour. Every clip plays in it. A look's picture should be the main one's size, or its cells won't line up (the Clip slot says so). A component's `look` field sets it while the game runs ([Facing and looks](#facing-and-looks)). |
| Facing | Which way it faces: North, NorthEast, East, SouthEast, South, SouthWest, West or NorthWest. A clip with directions plays the row nearest it: with four directions, a diagonal keeps showing north or south if that's what it showed, and otherwise turns east or west; a mirrored west side plays its east row flipped. Turning mid-clip keeps its place in the clip. A component's `facing` field, or an animator that faces by a parameter, sets it while the game runs. |
| Playing | Whether the clip moves on while the game runs. A component's `animation` field turns it on (a clip) and off (`Clip.None`). |
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
| Name | The look's name, as a Sprite's **Look** picks it. Game code names it with its file's name in front: `Look.KnightMana`. |
| Picture | The picture it draws, cut as the main look's. Its size is checked against the main one's. |
| Make main | Makes it the look a sprite wears when its Look is empty. |
| Remove look | Takes it out of the clips file (never the main look); sprites wearing it go back to the main look. |

## Shortcuts
| Action | Shortcut |
|---|---|
| Rename a clip (your own clips) | Double-click it in the Clips list (`Enter` keeps the name, `Esc` leaves it) |
| Pick cells on the Cells page | Click, `Shift` + click for a run, `Ctrl` + click to add one; then drag them onto a row |
| Move the picked cell a pixel (Cut picture…, Free crop) | Arrow keys |
| Pick a frame | Click it |
| Pick several frames | `Ctrl` + click, or `Shift` + click for a run |
| Move picked frames (your own clips) | Drag them |
| Take picked frames out (your own clips) | `Delete` over the Frames tab |
| Play or stop the clip | `Space` over the Frames tab |
| Undo or redo a change to the clips file | `Ctrl + Z`, `Ctrl + Y` over the clips |

## Tips and common problems
- **Problem:** a look's frames are cut off or show the wrong part. **Why:** its picture isn't the main one's size or layout. **Fix:** draw it on a copy of the main picture, so the cells line up; the Look settings say when the sizes differ.
- **Problem:** a character jumps a little between frames. **Why:** its cells were cut from different places in the picture, or are different sizes. **Fix:** every cell stands on its bottom middle, so line the feet up there: in **Cut picture…**'s Free crop, nudge each cell with the arrow keys.
- **Problem:** a walk restarts every time the character turns. **Why:** the code plays a different clip per direction. **Fix:** give the clip directions and set the component's `facing` instead; with **Same frames in every direction** on, turning keeps the step.
- **Problem:** game code can't name a clip from Aseprite. **Why:** its tag's name has a space or a dash, or starts with a digit. **Fix:** rename the tag (`WalkNE`), or use one clip with directions ([Clips from Aseprite](#clips-from-aseprite)).
- **Problem:** sprites don't move in the viewport. **Why:** the editor rests unless something changes. **Fix:** tick **Preview animation** in the viewport's **≡** menu, or press **Play**.
- **Problem:** a tag's frames can't be moved, timed or taken out. **Why:** a tag's frames are Aseprite's, so saving there keeps both in step. **Fix:** change them in Aseprite, or **Copy** the clip (on the ribbon) to change the copy here.
- **Problem:** an event doesn't play a sound. **Why:** no sound event has its name. **Fix:** name it after the sound event (`footstep` or `sfx/footstep`), or pick one from **◆ ▾**.
- **Problem:** a whole field of crops sways in step. **Fix:** give each a slightly different **Clip time**.
- **Problem:** two footsteps in a ping-pong walk. **Why:** a ping-pong shows its middle frames twice a pass, there and back. **Fix:** put the event on the first or last frame, or use **Forward** for walks.

## Related
- [Sprites from Aseprite](sprites.md): sheets, placing sprites, pivots.
- [Sound events](sound-events.md): the sounds an event plays.
- [Animators](animators.md): choosing clips by what's happening, and facing by a parameter.
- [Game code](game-code.md): components and events.
- [Concepts](concepts.md#2d): clip, clips file, frame event, [cell](concepts.md#cell), [direction](concepts.md#direction), [look](concepts.md#look).
