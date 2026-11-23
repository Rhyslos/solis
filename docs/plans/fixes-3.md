# Fixes 3: clips with directions, and looks

**Draft: the mock-ups and the questions at the end wait for the project lead.** Fixes 3 of [the fixes from making Orc Invasion](game-stages.md#fixes-from-making-orc-invasion) ([decision 0133](../decisions/0133-fixes-from-making-orc-invasion.md)), planned on 2026-11-23 under [the scope reset](scope-reset.md): Lust only, small steps, the light path except where a file format or a save changes.

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

One Design canvas, [Fixes 3: clips with directions and looks](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L), drawn in the G layout's language on the rethink's frame, 1440 × 900 like the design's screens. Orange **New** marks every part that doesn't exist today; the rest is the Animate tab with a clips file open, as built ([round 3's `04-sprite-clips-a`](../design/g-layout/additions/round-3/README.md)).

| Frame | Mock-up | What it shows |
| --- | --- | --- |
| (a) Cutting a sheet | [Cut the sheet](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-Main) | Animate with `knight.clips.lst` open. The sheet group (today's list of the sheet's frames) shows `knight.png` itself with a live grid over it: the **Cut into a grid** strip (Cell 32 × 32, Between 2 px, Edge 1 px, "Found 32 × 32 with 2 px between"), cells numbered in reading order, the column and row rulers, a hovered cell saying where it's used. The ribbon gains **Grid**, **Directions** and **Look** groups. Below, the Frames tab with idle's eight directions. |
| (b) A clip with directions | [Directions](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-Directions) | The Frames tab grown: the walk clip as a table, a row per direction (↓ South to ↙ SouthWest) and a column per frame, sharing the frame's time and its footstep event. Four cells picked in the sheet are dragged onto the NorthEast row. NorthWest, West and SouthWest are mirrors of NorthEast, East and SouthEast (faded, "flipped"); West's menu is open (Its own frames, East mirrored, Clear the row). Directions One · Four · Eight on the ribbon; a facing pad in the preview; timing and looping as today (the frame's **time**, **Plays: over and over**, **Speed**, **Direction**). |
| (c) Looks | [Looks](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-Looks) | The ribbon's Look switch on **Mana**: the sheet group shows `knight-mana.png` on the same grid, the preview plays the mana knight beside a faded normal one. The new **Look · Mana** tab: the looks (Normal is the main one), Name, Picture, the size check ("the same as Normal: every cell lines up"), the name in game code, Make main, Remove look, and walk and attack side by side in each look. |
| (d) Where it lives | [Where it lives](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-Where) | Home with the knight picked. The Sprite card's Sheet is `knight.clips.lst`, Clip says "walk · 8 directions", and two new rows: **Facing** (a pad of eight arrows) and **Look**. The Library shows the pictures, the clips file (kind Clips, with its compass chip) and the animator; right-clicking `orc.png` offers the new **Make clips…** beside Atlas Crop…. An inset shows an animator's state that plays one clip with directions, **Faces by** its *facing* parameter, instead of eight clips picked by hand. |
| (e) Lust | [Lust example](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-Code) | The same knight in three ways (question 1): A, calls and the knight's own `check` (recommended); B, a part in the `has` block; C, the clips file's own Lust file. Then the new calls, checked against the guidelines. |

## What it is

**The clips file grows; there's no new tool.** Today a clips file (`.solframes`) sits beside an Aseprite file, lists its tags and keeps clips made in Solis, edited in the Frames and Clip tabs ([Animating sprites](../manual/sprite-animation.md)). Fixes 3 gives it three things:

1. **A picture cut into a grid.** Any PNG (or JPEG) gets a clips file: **Make clips…** on it in the Library. Its **cell** size, the gap **between** cells and the **edge** around them cut it into cells, numbered from 1 in reading order, as Aseprite numbers frames. Solis offers the grid it finds (`tiles::autotile::detect_tile`, which tilesets already use, taught the gap). An Aseprite file needs no grid: its frames are its cells, and its tags stay clips, as today.
2. **A clip with directions.** A clip has one, four or eight directions. It's a table: a **column** per frame, with the frame's time and event, and a **row** per direction, saying which cell it shows. A row can be another row **mirrored** (West is East flipped), so an artist draws five directions for eight. Because every direction shares the columns, turning mid-walk keeps the walk's place, and a footstep is set once. Cells go onto a row by dragging a run of them from the sheet, or by picking a frame and clicking a cell. A tag can be dragged onto a row too, filling it with the tag's frames.
3. **Looks.** A look is another picture drawn on the same grid (`knight-mana.png` beside `knight.png`). Every clip plays the same cells from it, so the mana knight needs no clips of its own: one choice switches all of them. The first look is the main one, the one a sprite starts in; a picture of another size is a Problem.

**What it builds on:** the Frames tab, the Clip tab and Animate's ribbon (`crates/editor/src/frames_editor.rs`, `frames_view.rs`); `animation::ClipSet` and `clip::Clip`; the animator's `Facing` and `Facing::nearest` (`animation::animator_file`), which already know eight directions; the tileset's grid guess; the Sprite component's Sheet and Clip; the way Solis writes the project's names into `solis/project.lst`.

**What it leaves alone:** an Aseprite tag's own frames (still changed in Aseprite), clips with one direction (they work as today), the animator's graph, `setFlip` (still handy for a sprite with no directions).

### The file: `knight.clips.lst`

The clips file becomes a Lust data file, as decision 0081 planned for it (step L2 of [what Solis makes is Lust](lust-data.md)): `sprites/knight.clips.lst`, beside its picture. An older `.solframes` is moved when a project opens, as other files were (`app::to_lust`), and keeps its ID. The shape follows the atlas file's; the builder fits it to `area::lust_data`:

```lust
-- Clips of knight.png. Written by Solis; safe to edit by hand.
fetch "solis/data/clips":[Sheet, Look, Clip, Way];

lock int version = 2;
lock Sheet sheet = new Sheet { cell = [32, 32], between = [2, 2], edge = [1, 1], first = "idle" };

-- The first look is the main one. Every look is cut by the same grid.
lock Look array looks = [
    new Look { name = "Normal", picture = "1BWHNR3RV8PYY" },
    new Look { name = "Mana", picture = "2KQ7X0M1C4RTA" },
];

-- A column per frame (its time and event), a row per direction (its cells).
lock Clip array clips = [
    new Clip { name = "walk", speed = 1.0, direction = "forward", plays = 0,
               times = [120, 120, 120, 120], events = ["footstep", "", "footstep", ""] },
];
lock Way array ways = [
    new Way { clip = "walk", facing = "south", cells = [3, 4, 5, 6] },
    new Way { clip = "walk", facing = "east", cells = [21, 22, 23, 24] },
    new Way { clip = "walk", facing = "west", mirrors = "east" },
];
end
```

- A clip with one direction has one `Way` with `facing = "any"`. An Aseprite file's tag changes (speed, events) keep a list of their own, as `.solframes` has today.
- Pictures are named by ID, as files point at each other today.

### Names in game code

- **Clips keep their names:** `Clip.Sprites.Knight.Walk` (the clips file's path without `.clips.lst`, then the clip). One clip, whatever its directions.
- **Looks are a new project name**, written into `solis/project.lst` beside the clips: `state Look`, `Look.Sprites.Knight.Mana`.
- **Directions are a state in `solis/animation`:** `state Facing = [North, NorthEast, East, SouthEast, South, SouthWest, West, NorthWest];`. The animator's direction words (`up`, `up-right`) stay in its files; the editor shows the compass names everywhere.

### Playing it

- **A sprite remembers its facing and its look** (two new Sprite settings, saved in levels and in save games). `animate` plays a clip; with directions, the row for the facing.
- **Fewer directions than asked:** a clip with four directions shows the nearest of its four (question 7 says which wins a tie); a clip with one shows it whatever the facing.
- **Turning and switching look keep the clip's place**: the same frame, from another row or another picture.
- **With an animator**, a state that plays a clip with directions **faces by** a direction parameter: the row nearest it plays. "A clip by facing" stays, for older animators and for clips from different sheets.

## Game code

Shown as option A (question 1). Inside a component the calls are bare; on another entity they're methods (`orc:setLook(...)`). Lust only (the scope reset); `describe.rs` names each once, so the catch-up phase can add Lua and Rust.

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

One step per pull request, each well under an hour. Steps 1 and 5 change a file or a save, so they take the full path's tests (fuzz, the full suite); the rest take the light path. Every step updates the manual for what it adds and the CHANGELOG; UI steps add a picture test.

| Step | What | Tests |
| --- | --- | --- |
| **F3.1** The clips file as Lust | `animation::clips_file`: the shape above (`solis/data/clips`), read within the limits `.solframes` has, written, `version = 2`; a fixture in `tests/fixtures/clips/v2/`; a fuzz target. Today's `.solframes` content maps onto it (a tag's changes, clips made here with one direction). | Round trip; every limit; a hostile file; an old `.solframes` read into the new shape gives the same clips; fuzz |
| **F3.2** Moving the old files | `.solframes` moved to `.clips.lst` when a project opens (`app::to_lust`, `reflect::data_files::KINDS`), sidecar and ID kept; **Edit clips…** makes `.clips.lst`; the Library kind, `own_files`, `name_follow`, `project_names` (strip `.clips.lst`) follow. | A project with an old file opens, moves it, and every clip name stays the same; the old file goes into the backups |
| **F3.3** Cells from a grid | `assets::sprite_sheet` cuts a picture by cell, between and edge into numbered cells; `ClipSet` builds a clip's frames from cells; `detect_tile` learns the gap. | 306 × 170 at 32, 2, 1 gives 45 cells; a grid bigger than the picture gives none and a Problem; a zero cell is refused; the guess finds 32 with 2 between |
| **F3.4** Cutting in the editor | **Make clips…** on a picture; the sheet group shows the picture with the live grid, numbers, rulers and the hovered cell; the **Cut into a grid** strip and the ribbon's **Grid** group with **Find grid**; dragging across cells picks a run. | `frames_editor` tests for the grid's undo; a picture test of frame (a) |
| **F3.5** Directions in the Frames tab | **Directions** One, Four, Eight; a row per direction, a column per frame with its time and event; a run dropped on a row fills it; a picked frame plus a clicked cell swaps the cell; a tag dropped on a row; Ctrl + Z undoes each. | Editor tests: going from Eight to Four keeps the four straight rows; a drop of five cells onto four columns adds a column; undo; a picture test of frame (b) |
| **F3.6** Mirrors and the preview's facing | A row's menu (Its own frames, *another row* mirrored, Clear the row); mirrored rows drawn faded; the preview's facing pad. | A mirror of a mirror is refused; the preview plays the flipped cells; a picture test |
| **F3.7** Playing by facing | The Sprite's **Facing** and **Look** settings (levels and save games: full path); `ClipSet` gives a clip's frames for a facing; rows with fewer directions round as question 7 says; turning keeps the place; Properties shows the pad and the list. | A table of every facing against one, four and eight directions; a mirrored row flips; turning at frame 3 stays at frame 3; a save game comes back facing the same way in the same look |
| **F3.8** Animators face by a parameter | A state plays one clip and **Faces by** a direction parameter; the State tab's new row; older "a clip by facing" files read as before. | The animator picks the row nearest its parameter; old animator files give the same clips |
| **F3.9** Looks | The **Look** ribbon group and **+ Look**; the **Look** tab (Name, Picture, the size check, Make main, Remove look); the preview and sprites draw the look's picture; `state Look` in `solis/project.lst`. | A picture of another size is a Problem; switching look keeps the place; the names are written and follow a rename |
| **F3.10** Game code | `Facing` in `solis/animation`; `setFacing`, `setFacingAlong`, `facing`, `setLook`, `look` in `describe.rs` (Lust); the two mistakes (a look from another file, `setFacing` with an animator). | Native tests for each call and each mistake; the manual's examples compile (`manual_examples_tests`) |
| **F3.11** The manual's last pass | [Animating sprites](../manual/sprite-animation.md) rewritten around the clips file (cutting a picture, directions, looks, the before-and-after with the lookup table); [Animators](../manual/animators.md) (Faces by); [concepts](../manual/concepts.md) (cell, direction, look); D2, the rule that turns a tag into a clip's name; "directional animation" marked *works well* on the "Can I make this?" page if it exists by then. | The user guide's checks |

## Questions for the project lead

Each is asked as a question to click, one at a time, with the mock-up named.

1. **How should game code play a character's clips?** (frame e, [Lust example](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-Code))
   - **A. Calls, and your own check (recommended).** `setFacing(Facing.NorthEast)`, `setFacingAlong(stick)`, `setLook(Look.Sprites.Knight.Mana)` beside today's `animate(Clip.Sprites.Knight.Walk)`. The character's own state picks the clip with a `check`, so a new state points at the missing clip:
     ```lust
     check state {
         when Standing {
             animate(Clip.Sprites.Knight.Idle);
         }
         when Walking {
             animate(Clip.Sprites.Knight.Walk);
         }
     }
     ```
   - **B. A has block.** A part in `has`, filled in by Solis, whose fields Solis plays at the end of the tick:
     ```lust
     has {
         SpritePlayer sprite;     -- filled in by Solis
     }
     ...
     sprite.facing = Facing.NorthEast;
     sprite.clip = Clip.Sprites.Knight.Walk;
     sprite.look = Look.Sprites.Knight.Mana;
     ```
   - **C. The clips file's own Lust file**, as animators have: Solis writes `solis/clips/knight.lst` with `KnightClip`, `KnightLook` and a class, and the component's own state can be its clip:
     ```lust
     has {
         state = KnightClip.Idle;
         KnightClips clips;       -- filled in by Solis
     }
     ...
     clips:setFacing(Facing.NorthEast);
     clips:play(state);
     clips:setLook(KnightLook.Mana);
     ```
2. **Should the facing be its own call, or part of `animate`?** (frame e)
   - **Its own call (recommended):** `setFacing(Facing.NorthEast)`, then `animate(Clip.Sprites.Knight.Walk)`. The sprite remembers its facing, so idle keeps facing the way the knight walked, and `animate` stays as it is.
   - **Part of the call:** `animateFacing(Clip.Sprites.Knight.Walk, Facing.NorthEast)` (Lust has no optional arguments, so it's a second name).
   - **Both.**
3. **How does code turn a movement into a facing?** (frame e)
   - **`setFacingAlong(direction)` (recommended):** the nearest facing to a `vec3` such as the stick or `velocity()`; standing still keeps the last.
   - **A Sprite setting, "Faces where it moves":** no code, but it can't face the stick while standing to attack.
   - **Nothing new:** game code works it out with its own questions.
4. **What are they called?** (frames b and d)
   - **Clips, as today (recommended):** `Clip.Sprites.Knight.Walk`, a clip that has directions. One word, and every call that takes a clip takes these.
   - **Animations:** `Anim.Sprites.Knight.Walk` for clips with directions, beside clips. Two words for nearly the same thing.
5. **Do a clip's directions share their frames' times and events?** (frame b, [Directions](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-Directions))
   - **Yes, a column per frame (recommended):** every direction has the same number of frames and the same times, so turning mid-walk keeps its step and a footstep is set once.
   - **No:** each direction has its own frames and times; turning starts the clip again in the new direction.
6. **Can a direction be another one mirrored?** (frame b)
   - **Yes, in this milestone (recommended):** West is East flipped, so five drawn directions give eight.
   - **Later:** draw all eight, or use `setFlip` in code.
7. **A clip has four directions and the knight faces NorthEast: which shows?** (frame b)
   - **The one already showing, if it's one of the two nearest; otherwise East or West (recommended):** walking diagonally doesn't flicker between two rows.
   - **Always East or West.**
   - **Always North or South.**
8. **What does a look change?** (frame c, [Looks](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-Looks))
   - **The picture only (recommended):** a look is another picture on the same grid, and every clip plays the same cells from it. Nothing to fill in per clip.
   - **Its own cells:** each look picks cells for each clip (any picture), and a clip it leaves empty plays the main look's.
9. **Where is the clips file kept?** (frame d, [Where it lives](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-Where))
   - **The clips file grows and becomes Lust, `knight.clips.lst` (recommended):** one file per character; an old `.solframes` is moved when a project opens.
   - **A new file beside it, `knight.anims.lst`:** clips files stay as they are, and a project has two kinds of file for clips.
10. **Where is a picture cut into a grid?** (frame a, [Cut the sheet](https://claude.ai/artifact/Lo5SxALJQ8tox5tKxmZc7L#artboard-Main))
    - **In the Animate tab, where its clips are made (recommended):** the sheet group shows the picture and its grid while you pick cells.
    - **In its own Library tools tab first**, like Atlas Crop and tilesets, then in Animate.
11. **How do cells go onto a direction (new mouse actions)?** (frame b)
    - **Both ways (recommended):** drag across cells to pick a run, then drag the run onto a row; or pick a frame in a row and click a cell to swap it.
    - **Clicking only:** pick a row, then click cells one by one, as **Add frames** does today.
    - **Dragging only.**

## Out of this milestone

- Reading direction names from Aseprite tags (the project lead said no).
- Blending between clips, and clips for 3D models.
- Directions other than one, four and eight (such as two, left and right: use one direction and `setFlip`).
