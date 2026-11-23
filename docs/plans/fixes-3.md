# Fixes 3: clips with directions, and looks

**The project lead answered the questions on 2026-11-23 (see [their answers](#the-project-leads-answers-2026-11-23)); the second round of mock-ups waits for their approval.** Fixes 3 of [the fixes from making Orc Invasion](game-stages.md#fixes-from-making-orc-invasion) ([decision 0133](../decisions/0133-fixes-from-making-orc-invasion.md)), planned on 2026-11-23 under [the scope reset](scope-reset.md): Lust only, small steps, the light path except where a file format or a save changes.

## Why

Making Orc Invasion ([the report](../reports/orc-invasion-hurdles.md)), the knight had 10 actions × 8 directions × 2 looks = 160 clips:
- **M1** (High): game code couldn't ask for "the Walk clip, facing north-east", so choosing a clip took a 300-line generated lookup table.
- **M2** (Medium): there was no way to switch the knight to its mana-charged look, so both looks shared one file, which doubled the table.
- **D2** (Low): how an Aseprite tag becomes a clip's name wasn't written down.

## What the project lead asked for (2026-11-23)

Offered "read Aseprite tag names like walk/NE", they said:

> "Neither. I want an animation interface where I make a tileset in engine, just like Godot. I name the animation and call on it with code. Probably a check block in Lust, or a movement object with a Has block."

- **Facing:** game code names directions (named directions, not a vector). Lust refuses names of one or two capitals (N, NE; hurdle L3), so they're spelt out: `Facing.NorthEast`.
- **Another look:** "Animator variants": a choice picks among clip sets, so the mana-charged knight is the same clips in another set.

Read "like Godot" as a sheet of frames made in the editor: any picture is cut into a grid, cells are picked from the grid into named clips, and code plays a clip by its name.

## The mock-ups

One Design canvas, [Fixes 3: clips with directions and looks](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L), drawn in the G layout's language on the rethink's frame, 1440 × 900 like the design's screens. An orange dot marks every part that doesn't exist today. This is the second round, redrawn after the project lead's answers: calmer, with the clips editor as the main page and cutting in a pop-up. The first round's frames are replaced.

| Frame | Mock-up | What it shows |
| --- | --- | --- |
| (a) The clips editor | [The clips editor](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-Main) | The main page: Animate with `knight.clips.lst` open, in the rethink's slots. Left, the **Clips** (each with how many directions) and the **Looks**. Centre, the preview with its facing pad and the **Cells** cut from the picture, with **Cut picture…**; below, the Frames tab with walk as a table, a row per direction and a column per frame, the frame's time and event, and the hint "Drag cells onto a direction, or pick a frame and click a cell". Right, the picked clip's settings: Plays, Speed, Direction, Plays first, then **Directions** (One · Four · Eight), **Same frames in every direction** and **Mirror the west side**, and its name in game code. |
| (b) Cutting: Grid | [Cut, grid](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-CutGrid) | The pop-up **Cut knight.png into cells** in **Grid** mode: the picture with its live grid and numbered cells; Cell, Between and Edge, **Find grid**, "45 cells: 9 across, 5 down"; **Cancel** and **Use these cells**. |
| (c) Cutting: Free crop | [Cut, free crop](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-CutFree) | The same pop-up in **Free crop** mode, Atlas Crop's way: cell 23 picked with its handles, moved a pixel left of the grid's box (dashed) because the frame was drawn a pixel off; its X, Y, Width and Height; **Back to the grid's box**; arrows move it a pixel; dragging on the picture draws a new cell, which gets the next number. |
| (d) Looks | [Looks](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-Looks) | The same page with the ribbon's Look switch on **Mana**: the preview, the cells and the rows show `knight-mana.png`; the right slot is the **Look**: Name, Picture, the size check, the name in game code, Make main, Remove look. |
| (e) Where it lives | [Where it lives](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-Where) | Home with the knight picked. The Sprite card's Sheet is `knight.clips.lst`, Clip says "walk · 8 directions", and two new rows: **Facing** (a pad of eight arrows) and **Look**. The Library offers **Make clips…** on a picture. An inset shows an animator's state that plays one clip with directions and **Faces by** its *facing* parameter. |
| (f) Lust | [Lust example](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-Code) | The knight in the decided shape (calls, and the knight's own `check`), the `Facing` state, and each call checked against the guidelines. |

## What it is

**The clips file grows; there's no new tool.** Today a clips file (`.solframes`) sits beside an Aseprite file, lists its tags and keeps clips made in Solis, edited in the Frames and Clip tabs ([Animating sprites](../manual/sprite-animation.md)). Fixes 3 gives it three things, and lays the page out in the rethink's slots: the clips and looks on the left, the clip's settings on the right (they leave the Clip tab), the frames below.

1. **A picture cut into cells, in a pop-up.** Any PNG (or JPEG) gets a clips file: **Make clips…** on it in the Library opens the file and the pop-up; later, **Cut picture…** on the ribbon or above the cells opens it again. The pop-up has two modes, and both make the same thing, **cells**, numbered from 1, which clips use alike:
   - **Grid:** the **cell** size, the gap **between** cells and the **edge** around them cut the whole picture, numbered across, then down, as Aseprite numbers frames. The grid moves as the numbers change, and **Find grid** offers the one Solis finds (`tiles::autotile::detect_tile`, which tilesets already use, taught the gap).
   - **Free crop**, Atlas Crop's way: drag on the picture to draw a cell (it gets the next number), or pick any cell and move or size it, with its handles, its X, Y, Width and Height, or the arrow keys a pixel at a time. **Back to the grid's box** undoes a moved grid cell. This is for frames a grid can't catch: one drawn a pixel off centre, or a swing wider than the rest.

   A cell is drawn with its bottom middle at the sprite's place, as a plain picture is today, so a cell moved a pixel puts the art back in line. **Use these cells** keeps the change as one step Ctrl + Z undoes; **Cancel** drops it. An Aseprite file needs no cutting: its frames are its cells, and its tags stay clips, as today.
2. **A clip with directions.** A clip has one, four or eight directions, a **row** each, saying which cells it shows. Cells go onto a row by dragging a run of them from the cells, or by picking a frame and clicking a cell; a tag can be dragged onto a row too. Two settings of each clip, on the right:
   - **Same frames in every direction** (on for a new clip): the clip is a table, a **column** per frame with its time and event, shared by every row. Turning mid-walk keeps the walk's step, and a footstep is set once. Off, each row has its own frames, times and events; turning keeps how far into the clip it is, wrapped to the new row's length.
   - **Mirror the west side** (on for a new clip): NorthWest, West and SouthWest are NorthEast, East and SouthEast flipped, so five drawn directions give eight (with four directions, West is East flipped). Off, every direction is drawn, for a character that isn't the same both ways.
3. **Looks.** A look is another picture with the same cells (`knight-mana.png` beside `knight.png`). Every clip plays the same cells from it, so the mana knight needs no clips of its own: one choice switches all of them. The first look is the main one, the one a sprite starts in; a picture of another size is a Problem.

**What it builds on:** the Frames tab, the Clip tab and Animate's ribbon (`crates/editor/src/frames_editor.rs`, `frames_view.rs`); Atlas Crop's drawing of boxes on a picture, for Free crop; `animation::ClipSet` and `clip::Clip`; the animator's `Facing` and `Facing::nearest` (`animation::animator_file`), which already know eight directions; the tileset's grid guess; the Sprite component's Sheet and Clip; the way Solis writes the project's names into `solis/project.lst`.

**What it leaves alone:** an Aseprite tag's own frames (still changed in Aseprite), clips with one direction (they work as today), the animator's graph, `setFlip` (still handy for a sprite with no directions).

### The file: `knight.clips.lst`

The clips file becomes a Lust data file, as decision 0081 planned for it (step L2 of [what Solis makes is Lust](lust-data.md)): `sprites/knight.clips.lst`, beside its picture. It replaces `.solframes`: there are no older projects to move (the project lead, 2026-11-23). The shape follows the atlas file's; the builder fits it to `area::lust_data`:

```lust
-- Clips of knight.png. Written by Solis; safe to edit by hand.
fetch "solis/data/clips":[Sheet, Cell, Look, Clip, Way];

lock int version = 2;
lock Sheet sheet = new Sheet { cell = [32, 32], between = [2, 2], edge = [1, 1], first = "idle" };

-- Cells moved or drawn in Free crop; every other cell is the grid's.
lock Cell array cells = [
    new Cell { number = 23, box = [136, 69, 32, 32] },
    new Cell { number = 46, box = [274, 136, 40, 32] },
];

-- The first look is the main one. Every look is cut by the same grid.
lock Look array looks = [
    new Look { name = "Normal", picture = "1BWHNR3RV8PYY" },
    new Look { name = "Mana", picture = "2KQ7X0M1C4RTA" },
];

-- With the same frames in every direction, the times and events are the clip's;
-- a row per direction gives its cells. The west side is mirrored, so it has no rows.
lock Clip array clips = [
    new Clip { name = "walk", speed = 1.0, direction = "forward", plays = 0,
               directions = 8, sameFrames = true, mirror = true,
               times = [120, 120, 120, 120], events = ["footstep", "", "footstep", ""] },
];
lock Way array ways = [
    new Way { clip = "walk", facing = "south", cells = [3, 4, 5, 6] },
    new Way { clip = "walk", facing = "southEast", cells = [12, 13, 14, 15] },
    new Way { clip = "walk", facing = "east", cells = [21, 22, 23, 24] },
    new Way { clip = "walk", facing = "northEast", cells = [30, 31, 32, 33] },
    new Way { clip = "walk", facing = "north", cells = [39, 40, 41, 42] },
];
end
```

- A clip with one direction has one `Way` with `facing = "any"`. With `sameFrames = false`, each `Way` has its own `times` and `events`. An Aseprite file's tag changes (speed, events) keep a list of their own, as `.solframes` has today.
- Pictures are named by ID, as files point at each other today.

### Names in game code

- **Clips keep their names:** `Clip.Sprites.Knight.Walk` (the clips file's path without `.clips.lst`, then the clip). One clip, whatever its directions.
- **Looks are a new project name**, written into `solis/project.lst` beside the clips: `state Look`, `Look.Sprites.Knight.Mana`.
- **Directions are a state in `solis/animation`:** `state Facing = [North, NorthEast, East, SouthEast, South, SouthWest, West, NorthWest];`. The animator's direction words (`up`, `up-right`) stay in its files; the editor shows the compass names everywhere.

### Playing it

- **A sprite remembers its facing and its look** (two new Sprite settings, saved in levels and in save games). `animate` plays a clip; with directions, the row for the facing.
- **Fewer directions than asked:** a clip with four directions shows the one already showing if it's one of the two nearest, and otherwise East or West, so walking diagonally doesn't flicker; a clip with one shows it whatever the facing.
- **Turning and switching look keep the clip's place**: the same frame, from another row or another picture (a clip without the same frames in every direction keeps its time, wrapped to the new row's length).
- **With an animator**, a state that plays a clip with directions **faces by** a direction parameter: the row nearest it plays. "A clip by facing" stays, for older animators and for clips from different sheets.

## Game code

Calls, and the character's own `check` (decided by the Lust feel, below). Inside a component the calls are bare; on another entity they're methods (`orc:setLook(...)`). Lust only (the scope reset); `describe.rs` names each once, so the catch-up phase can add Lua and Rust.

```lust
fetch "solis/project":[Action, Clip, Look];
fetch "solis/animation":Facing;
fetch "solis/input":Input;

state Doing = [Standing, Walking, Attacking];
event ManaChanged(bool charged);

component Knight {
    has {
        state = Doing.Standing;
    }

    method update(float dt) {
        -- Face where the stick points; standing still keeps the last facing.
        vec3 stick = Input:axis(Action.Move);
        setFacingAlong(stick);

        -- A clip for each thing the knight does: Lust says if one is missing.
        check state {
            when Standing {
                animate(Clip.Sprites.Knight.Idle);
            }
            when Walking {
                animate(Clip.Sprites.Knight.Walk);
            }
            when Attacking {
                animate(Clip.Sprites.Knight.Attack);
            }
        }
    }

    -- Charged: the same clips, in the other look.
    on ManaChanged(bool charged) {
        is charged? then {
            setLook(Look.Sprites.Knight.Mana);
        } or {
            setLook(Look.Sprites.Knight.Normal);
        }
    }
}

pass Knight, ManaChanged;
end
```

| Call | New? | What it does | Guidelines |
| --- | --- | --- | --- |
| `setFacing(Facing.NorthEast)` | New | Which direction a clip with directions plays. It stays until changed. | 5 (`set` + noun), 3 (a state) |
| `setFacingAlong(direction)` | New | The facing nearest a `vec3` (`Input:axis`, `velocity()`), its height left out as the animator's `setFacing` does. A zero direction changes nothing. | 7 (the difference at the end) |
| `facing()` | New | The facing. | 5 (a noun reads) |
| `setLook(Look.Sprites.Knight.Mana)` | New | Shows the clips from another look; the clip keeps its place. A look from another clips file stops with a message naming the line. | 3, 5, 11 |
| `look()` | New | The look. | 5 |
| `animate`, `animateFromStart`, `stopAnimation`, `setFlip` | As today | With directions, `animate` plays the row for the facing. Still safe every tick. | 8 |
| `anim:setFacing(v)` | As today | With an animator, its direction parameter picks the row. `setFacing` on an entity with an animator stops with a message saying to use the animator. | 4 |

Checklist (CONTRIBUTING.md, rule 25): every call acts on one entity, so it's a method on `entity`; every fixed choice is a state (`Facing`, `Look`, `Clip`); the clips are made in an editor and reach code through `solis/project.lst`, as clips already do; names follow the shapes; nothing answers several things at once; every call is safe each tick; no new events (`ClipFinished` and `CueReached` work as today); a wrong look stops with the line; the manual shows a before and after (the lookup table, then `check`).

## Steps

One step per pull request, each well under an hour. Steps F3.1 and F3.8 change a file or a save, so they take the full path's tests (fuzz, the full suite); the rest take the light path. Every step updates the manual for what it adds and the CHANGELOG; UI steps add a picture test.

| Step | What | Tests |
| --- | --- | --- |
| **F3.1** The clips file as Lust | `animation::clips_file`: the shape above (`solis/data/clips`): the grid, free cells, looks, clips with `directions`, `sameFrames` and `mirror`, their rows; read within the limits `.solframes` has, written, `version = 2`; a fixture in `tests/fixtures/clips/v2/`; a fuzz target. What `.solframes` holds today (a tag's changes, clips made here) has a place in it. | Round trip; every limit; a hostile file; a cell number used twice is refused with its line; fuzz |
| **F3.2** The new file everywhere | `.clips.lst` replaces `.solframes` (no older projects to move, the project lead, 2026-11-23): **Edit clips…** makes `.clips.lst`; the Library kind, `own_files`, `name_follow` and `project_names` (strip `.clips.lst`) follow. | Clip names from a `.clips.lst` are as they were from a `.solframes` |
| **F3.3** Cells | `assets::sprite_sheet` cuts a picture into numbered cells by the grid, then applies the free cells (a moved box, or a new number after the grid's); each cell drawn with its bottom middle at the sprite's place; `ClipSet` builds a clip's frames from cells; `detect_tile` learns the gap. | 306 × 170 at 32, 2, 1 gives 45 cells; a free cell replaces its grid cell and a new one is 46; a grid bigger than the picture gives none and a Problem; a zero cell is refused; a box past the picture's edge is a Problem; the guess finds 32 with 2 between |
| **F3.4** The cutting pop-up: Grid | **Cut picture…** (ribbon and the cells' header) and **Make clips…** on a picture open the pop-up; **Grid** mode: Cell, Between, Edge, the live grid with numbers, **Find grid**, the count; **Use these cells** as one undo step, **Cancel**; the page's **Cells** show what was cut. | Editor tests: Use these cells is one undo step, Cancel changes nothing; a picture test of frame (b) |
| **F3.5** The cutting pop-up: Free crop | **Free crop** mode, with Atlas Crop's box drawing: drag to draw a new cell; pick a cell and move or size it by its handles or its X, Y, Width and Height; the arrow keys move it a pixel; **Back to the grid's box**; the grid's boxes shown faint. | Editor tests: a drawn cell gets the next number; arrows move the picked cell one pixel; Back to the grid's box removes the free cell; a picture test of frame (c) |
| **F3.6** The clips editor's page and directions | The page in the rethink's slots: Clips and Looks left, the clip's settings right (leaving the Clip tab), the Frames tab below; **Directions** One, Four, Eight; a row per direction with a column per frame, its time and event; a run of cells dropped on a row fills it; a picked frame plus a clicked cell swaps the cell; a tag dropped on a row; Ctrl + Z undoes each. | Editor tests: from Eight to Four keeps the four straight rows; a drop of five cells onto four columns adds a column; undo; a picture test of frame (a) |
| **F3.7** The two settings and the preview's facing | **Same frames in every direction** (off: each row gets its own frames, times and events, copied from the shared ones when turned off) and **Mirror the west side** (the west rows drawn faded as the east side flipped; off: they become rows of their own, copied flipped); the preview's facing pad. | Turning either setting off and on again gives the same file back; the preview plays the flipped cells; a picture test |
| **F3.8** Playing by facing | The Sprite's **Facing** and **Look** settings (levels and save games: full path); `ClipSet` gives a clip's frames for a facing; four directions round as decided (the one showing if it's one of the two nearest, otherwise East or West); turning keeps the place (or the time, wrapped, without the same frames); mirrored rows flip; Properties shows the pad and the list. | A table of every facing against one, four and eight directions, from each one showing; a mirrored row flips; turning at frame 3 stays at frame 3; a save game comes back facing the same way in the same look |
| **F3.9** Animators face by a parameter | A state plays one clip and **Faces by** a direction parameter; the State tab's new row; older "a clip by facing" files read as before. | The animator picks the row nearest its parameter; old animator files give the same clips |
| **F3.10** Looks | The **Look** switch and **+ Look** on the ribbon, the **Looks** list, the **Look** settings on the right (Name, Picture, the size check, Make main, Remove look); the preview, the cells, the rows and sprites draw the look's picture; `state Look` in `solis/project.lst`. | A picture of another size is a Problem; switching look keeps the place; the names are written and follow a rename; a picture test of frame (d) |
| **F3.11** Game code | `Facing` in `solis/animation`; `setFacing`, `setFacingAlong`, `facing`, `setLook`, `look` in `describe.rs` (Lust); the two mistakes (a look from another file, `setFacing` with an animator). | Native tests for each call and each mistake; the manual's examples compile (`manual_examples_tests`) |
| **F3.12** The manual's last pass | [Animating sprites](../manual/sprite-animation.md) rewritten around the clips file (cutting a picture in both modes, directions and their two settings, looks, the before-and-after with the lookup table); [Animators](../manual/animators.md) (Faces by); [concepts](../manual/concepts.md) (cell, direction, look); D2, the rule that turns a tag into a clip's name; "directional animation" marked *works well* on the "Can I make this?" page if it exists by then. | The user guide's checks |

## The project lead's answers (2026-11-23)

### On the mock-ups

> "A and B looks best, but they're both a bit too messy. I think B is probably the one I'm leaning the most towards, but I think the cutout part should be a pop-up menu, since that will make the page better. Also it's important we get the atlas crop mode in the menu too, in case an animation is like one pixel off center, or something like that so a traditional grid won't work for the entire thing. That way we can free cutout a frame and still make it work."

"A" and "B" are the first round's frames (a), cutting a sheet, and (b), a clip with directions. So:
- **Calmer pages:** fewer badges and labels, more space, one clear focus each. The second round marks new parts with a small dot.
- **Cutting is a pop-up** opened from the clips editor, with two modes, **Grid** and **Free crop** (Atlas Crop's way: draw a cell, nudge it a pixel at a time). Both make cells that clips use the same way.
- **The clip with a row per direction is the main page**, tidied: the second round's frame (a).

### What it does

- **Same frames in every direction?** "A settings option." A setting of each clip, on for a new clip.
- **Mirrored directions?** "A setting, since some characters are asymmetrical, in which case we need to draw all 8." A setting of each clip, **Mirror the west side**; off, all eight are drawn.
- **Four directions, facing NorthEast:** the one already showing if it's one of the two nearest; otherwise East or West.
- **A look changes the picture only:** another picture with the same cells.
- **Where a picture is cut:** in the pop-up above (it replaces the choice between the Animate tab and a Library tools tab).
- **Putting cells on a direction (new mouse actions):** both ways: drag a run of cells onto a row, or pick a frame and click a cell.

### The code's shape (decided by the Lust feel)

The project lead asked for engine calls to feel like Lust and left the code's shape to whoever designs it ([the guidelines' "The Lust feel"](../game-api-guidelines.md#the-lust-feel)), so these were settled from Lust's rulebook and the guidelines:

- **How code plays clips: calls, plus the character's own `check`.** It reads out loud (`setFacing(Facing.NorthEast); animate(Clip.Sprites.Knight.Walk);`) and adds nothing new to learn. The clips reach code through `solis/project.lst`, as they already do (rule 4). And the project lead's own idea, a `check` on the character's state, is what picks the clip: `check` must cover every option, so a new state points at the missing clip. B would have game code set fields Solis plays later, which is driving through a side door. C adds a second set of names (`KnightClip.Walk`) beside `Clip.Sprites.Knight.Walk` for the same clips.
- **The facing is its own call, `setFacing`.** Lust has no optional arguments, and a facing the sprite remembers means idle keeps facing the way the knight walked (rules 5, 7 and 8).
- **A movement becomes a facing with `setFacingAlong(direction)`.** The same shape as the animator's `setFacing`, and safe every tick.
- **They're clips, as today.** One word for one thing: every call that takes a clip takes these.
- **The clips file grows and becomes Lust, `knight.clips.lst`.** One file per character. There are no older projects yet (the project lead, 2026-11-23), so `.solframes` is replaced, not moved: step F3.2 shrinks to the renames.

### Still to ask

- **Keys in Free crop** (CONTRIBUTING.md, rule 17): the arrow keys move the picked cell a pixel (recommended, as most drawing programs do); alternatives: Shift + arrow also moves it a whole cell, or no keys, only the X and Y fields. Asked before F3.5 is built.
- **The second round of mock-ups** waits for the project lead's approval before F3.4 is built.

## Out of this milestone

- Reading direction names from Aseprite tags (the project lead said no).
- Blending between clips, and clips for 3D models.
- Directions other than one, four and eight (such as two, left and right: use one direction and `setFlip`).
