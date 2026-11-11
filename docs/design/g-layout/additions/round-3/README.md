# docs/design/g-layout/additions/round-3

Additions to the project lead's design: Solis's 2D editors, Audio, Debug, Build and the unbuilt screens (milestones M10 to M15). They are parts the G layout doesn't draw, mocked up in its own language on copies of its own screens.

**Approved by the project lead on 2026-11-11** ("All of these look fine as templates, can't wait to see them in engine!"), with these choices (each asked as a question):

- **A tile map's paint tools are in the ribbon** of a contextual tab, HOME TOOLS · Tile map (`01-tile-map-painting-a`), not in the toolbox.
- **A clip's settings are their own Clip shelf tab** (`04-sprite-clips-a`, `04-sprite-clip-tab-a`), not a column beside the frames.
- **The game-screen editor's drawing tools are a toolbox** down the left of the canvas (`07-ui-editor-a`), not in the ribbon.
- **The mixer's Master strip is last**, on the right, and the Bus ribbon group stays.
- **Two new actions for the hand** are approved: clicking a layer's row paints on that layer; Esc puts a dragged document tab back.
- **Everything else in the round** is approved as drawn.

"As templates": the project lead will judge the real thing in the engine. The layout and contents should match; small differences are expected.

**These pictures are templates at screen size** (1440 × 900, one pixel to the point, with a reduced palette to keep the repository small), unlike the design's own screens and round 1, which are saved at twice the size. Take layout, contents and words from them; take exact colours and sizes from the tokens (`crates/editor/src/tokens.rs`) and the specification. Only the chosen alternative of each choice is kept. `00-overview.png` shows the whole round small.

## The pictures

| Picture | What it shows | Answers |
| --- | --- | --- |
| `01-tile-map-painting-a` | Home with the tile map's entity selected, mid-stroke with the Brush. The contextual tab **HOME TOOLS · Tile map**: Paint (Brush, Rectangle, Line, Fill, Eyedropper), the terrain in hand with Layer and Exact, Show (Map as: Art or Meaning, Values, Chunks, Walls), Copy (Flip X, Flip Y, Turn, Put away), Play. The **Map** shelf tab: the map with its Tileset and Size, Layers, Terrains as tiles with their cell counts, Cell values, the cell under the pointer. The hint pill has today's keys. | Inventory a.4 (Map), plan M10 |
| `01-tile-map-layers` | A layer's ⋯ menu (Move up, Move down, Delete), with a copy of 3 × 2 cells in hand: the ribbon says "Copy 3 × 2", the Copy group is lit, the pill shows X, Y, Z and Esc. | Inventory a.4 |
| `01-tile-map-exact-and-values` | Exact on (the chosen terrain's tiles under the swatches), Values, Chunks and Walls on over the map, and a terrain's right-click menu (Solid). | Inventory a.4 |
| `02-tileset-editor` | **LIBRARY TOOLS · Tileset**, opened from the Library. Center: the picture with its grid, the four steps and the numbers of the chosen terrain's slots; the Slots group with two **missing** slots (dashed red, "13 of 15 filled") and the Try-it patch. Shelf tab Terrains: the list, the terrain's settings (Name, Code, Colour, Kind, Solid, Remove terrain) with the warning, and the picked slot's tiles (Weight, Animation). | Inventory a.4 (Tileset), plan M10 |
| `02-tileset-add-terrain` | A terrain just added: its name being typed in the list, no art yet, and the editor's guess at its kind ("it looks like an edges set. Is that right?"). | the same |
| `02-tileset-rules` | A terrain of its own rules: the **Rules** shelf tab (patterns tried in order, Shows slot, ↑ ↓ ×, + Rule, what the marks mean) and "Slots the rules show". | the same |
| `03-picture-crops` | **LIBRARY TOOLS · Picture** (today's Atlas Crop): the picture with its grid and nine crops, one picked, one being dragged; the Crops shelf tab (the list; the picked crop's Name and Rename, size, place, and how game code and a screen name it). | Inventory a.4 (Atlas Crop) |
| `04-sprite-clips-a` | Animate with a clips file: the sprite playing, the sheet's frames to add, and the **Frames** shelf tab in the Sequencer's shapes: the Clips list where Tracks are, a time ruler, frames as cells as wide as they last (a ping-pong's way back faded), events as marks on a lane, the accent playhead with its flag; transport on the shelf bar. The clip is one made in Solis. | Inventory a.4 (Clips and Frames), plan M11 |
| `04-sprite-clip-tab-a` | The **Clip · greet** shelf tab of the same answer: Playback (Speed, Direction, Plays, Plays first), Events, Sheet. | the same |
| `05-animator-graph` | Animate with an animator: the state graph in the node graph's language (states as nodes, transitions as wires with their conditions, Any state dashed, Start marked, the preview's state lit), a Preview group, the **Parameters** shelf tab as the design's parameter cards, and the pill with the graph's real keys. | Inventory a.4 (Animator), plan M11 |
| `05-animator-state` | The **State · Walk** shelf tab: Name, Plays, Clip, Speed, Speed by, Start here. | the same |
| `05-animator-transition` | A transition picked: **Transition · Idle → Walk** with its conditions, + Condition, Tried Earlier / Later. | the same |
| `06-animate-sequencer-unbuilt` | Animate with neither open: the design's own ribbon and shelf tabs as unfinished parts and one page card for the Sequencer, which also says how clips and animators get here. | Inventory b.5, plan M11 |
| `06-animate-ribbons` | The Animate ribbon in its three forms, and one with unfinished parts hidden, with which groups are real. | brief item 6 |
| `07-ui-editor-a` | The **UI** tab: the canvas with a picked piece and its three handles, the safe area, a piece made by code with its badge; the drawing tools in a **toolbox**; the ribbon (Screen: size, Text %, Safe area, Snap; Preview; Open as code); the **Screen** shelf tab: Elements, Properties as cards (the Align grid's chosen cell in the accent), Pictures. | Inventory a.4 (Canvas and Elements), D6, plan M12 |
| `07-ui-preview-values` | Preview values open (sliders for what the screen reads from game code, never saved) and Text 150%. | manual, "Preview values" |
| `07-ui-code-error` | The screen's own code stopped: "The screen's view stopped" in place of the screen, with the line, Open as code and Show in Problems. | manual, ui-editor.md |
| `08-audio-mixer` | The design's mixer with Solis's buses (Music, Effects, Footsteps inside Effects, Voice, Ambience, UI, Master). A strip has its fader and scale (-60 to +12 dB), the meter pair, the readout, "ducked -8", M and S (Master has M only); inserts and pan are unfinished parts. The right group is the picked **Bus** with a Ducking / Effects switch: Name, Inside, Volume, the ducking rule, Add a ducking rule, Remove bus. A **Bus** group in the ribbon. Sound events on the shelf with a real event's takes and settings. | Inventory a.7, b.10, plan M13 |
| `08-audio-sound-event` | An event heard everywhere (one stereo take, Loop on): the distances and Fade are off. | Inventory a.7 |
| `08-audio-effects-unbuilt` | The Effects side of the switch: a page card. | `audio.effect` |
| `09-debug-profiler` | The design's profiler with what Solis measures: the kept frames as bars with the 60 and 30 fps lines and a picked frame; the CPU's six parts and the GPU's three as two lanes (Physics, Render and Workers lanes unfinished); Statistics with the Profiler's real counts (Memory unfinished); round 1's Console with the commands that fit what's typed at the right of the command line. | Inventory b.6, plan M14 |
| `09-debug-problems` | The **Problems** shelf tab: the game's code first, then the area's. | Inventory a.3 |
| `10-build-tab` | The **Build** tab: Export for playtesting as a page in the File pages' pattern (Game's name, Starts in, Into the folder, Window, Export, the result with Open folder), a card saying what goes in the folder, the Console in front on the shelf, and the unfinished parts: Targets, Full export, a Build log tab. | Inventory a.6, b.14, plan M15 |
| `10-build-tab-hidden` | The same with "Hide unfinished parts" on. | plan 3.3 |
| `11-unbuilt-shading`, `11-unbuilt-vfx`, `11-unbuilt-model-viewer` | A whole unbuilt screen: the design's ribbon and shelf tabs as unfinished parts, one centred page card. The model viewer has its contextual tab (itself unfinished) and one real button, Place. | D2, Inventory b.4, b.9, b.11 |
| `11-unbuilt-model-viewer-menu` | How the model viewer is reached: a model's right-click menu in the Library. | Inventory b.9 |
| `11-unbuilt-hidden-library` | "Hide unfinished parts" on: Shading and VFX are gone from the tab bar, and the model's menu has no way into the viewer. | plan 3.3 |
| `12-drag-tab-split` | Dragging the Game tab: the two halves of the center as drop targets, the right one lit. | D15 row 23 |
| `12-drag-tab-merge` | Two groups: the tab dragged onto the other group's tab strip, which joins them. | D15 row 23 |

## What the drawing guessed

**All of them**
- **File names.** The brief's sample names are drawn as Solis's real files: a sheet with clips is an Aseprite file (a PNG is a sheet of one frame), so `sailor.aseprite` and its clips file `sailor.solframes`; the animator is `sailor.solanim`; a screen is a Lust file, `ui/hud.lst`; `harbour.tileset` and the map Docks drop only the `.lst`.
- A document tab of a file has its kind's dot and a ×, as the design's model viewer; the kinds' colours are the lead's choice from round 2 (the design's, with today's for sound events, code and areas).
- LIBRARY TOOLS ribbons have no Play group, as the design's Mesh ribbon; HOME TOOLS · Tile map and the ribbon tabs do.
- Six small icons are new (a minus, add state, transition, start here, a wall, take a frame out); the rest are round 1's sheet.

**Tile map**
- The contextual tab is shown in front as soon as the map's entity is selected. It has no Undo and Redo group: Ctrl + Z works as today.
- The plan's ribbon lacked **Exact** and the **layer painted**: both are in a group with the terrain in hand.
- The map file's **Tileset** and **Size** are in the Map tab's first column. Today they're in the file's Properties, reached from the Library. "Open the tileset" is a new link.
- The Map tab sits after Entity on the shelf bar.

**Tileset and crops**
- The plan's shelf tabs were Terrains, Slot, Rules, Try it. Drawn: **Terrains** (list, terrain, slot as three columns) and **Rules**; Try it stays under the slots, as today.
- "Zoom: 300% ▾" in both ribbons is a new control (today: − and + on the tileset's picture, the wheel on a crop's).
- The Crop group's button reads "Every tile" (the hover text says Crop every tile).

**Animate**
- A clips file's ribbon: Clip (New, Copy, Delete), Frames (Add, Take out), Events (Sound), then **Playback** (Plays, Speed, Direction), then the design's Keys, Interpolation and Tracks as unfinished parts.
- An animator's ribbon: State (Add state, Transition, Start here, Delete), Graph (Fit), Preview (Start again), then the design's four groups unfinished.
- The animator's parameters and preview leave the panel's left side: Preview is an editor group, Parameters a shelf tab.
- Transport has the design's three buttons plus Stop, and reads "0.29 / 0.70 s · frame 3" as today.

**UI**
- The manual says "piece"; the column is **Elements** and sentences say piece.
- Properties' cards are today's sections: Size and place, Look, Tied to the game, All its settings (When clicked shows for a button).
- The tool ring (E) stays as today and isn't drawn.

**Audio, Debug, Build**
- **Master is the last strip**, as the design; today's Mixer has it first.
- "Mute the game" and "Heard at once" are in the Monitor group. The Bus ribbon group repeats two buttons of the right group.
- A sound event has no waveform of its own (it has takes), so the design's waveform isn't drawn.
- The frame-time bars are the design's greys, not today's stacked colours; a part shorter than about 0.8 ms shows no name on its block.
- The Build page's button is **Open folder**, today's words.
- The model's menu item reads "Open in the model viewer".

## What could not be drawn as proposed

1. **No eraser and no stamp tool exist.** Right-drag empties with every tool; the Eyedropper's copy is the stamp. The five real tools are drawn.
2. **`01-tile-map-painting-b` doesn't fit one column:** ten tools need 383 px and the view has 370. Drawn in two columns; the design's four unbuilt tools give way while a map is selected.
3. **The plan's "Use › Place a map with it"** for a tileset doesn't exist: left out.
4. **A clip has no start and end** (the plan's Clip tab named them): its settings are Speed, Direction, Plays and Plays first.
5. **The sample sheet faces one way**, so the Walk state plays one clip; "a clip by facing" is described on the State tab, not shown filled in.
6. **Align and distribute** (the brief's UI ribbon) don't exist as commands: Solis has the Align grid in Properties. The Button tool has no key (1 to 7 are Select to Image).
7. **Draw calls are real.** The Inventory lists `debug.stats.draw-calls` as unbuilt, but the Profiler counts them today.
8. **The build log is unbuilt** (Inventory b.14): what an export says goes to the Console, drawn in front, with Build log as an unfinished tab.
9. **Build's unfinished parts have one id** (`build.targets`); the picture draws five (Windows, Platforms, Build, Installer, Your Rust code) and a Build log tab. They need ids, or fewer are drawn.
10. **"The same three with Hide unfinished parts on" is one picture, not three:** Shading and VFX hidden are the same tab bar, and the model viewer hidden is a missing menu item.
11. **Audio is sparse, as the plan warned:** each strip's inserts and pan, Send, Snapshot, the Effects group and side, Meters and Listen from are unfinished.

## New keys or mouse actions these screens need

1. **Click a layer's row to paint on it** (`01-tile-map-painting-a`: the brush mark on the row). Today only the tool bar's layer menu chooses. Recommended: yes; the ribbon's Layer ▾ stays.
2. **Esc puts a dragged document tab back** (`12-drag-tab-split`). Recommended: yes, as Esc gives up every other drag.

Nothing else is new: every key in the hint pills is today's, and the Frames tab's arrow keys, the click on a frame-time bar and the tab's drop places were approved in round 2.

Both were approved by the project lead on 2026-11-11.
