# 06 · Shared 4: data and the editor

ROADMAP.md (ideas 15, 18 to 21 and 12): **data tables** (a file, edited as a table); **save slots** (picture, date, time played, autosave; the player's settings apart); **field hints** (sliders, limits, a sentence); **more kinds of settings** (a colour, a sound, a sprite, a prefab, a list); **components that draw in the editor** (a guard's view cone); **paths drawn in the editor** (a guard's walk, a platform's route, a camera's glide). **Gate:** "a shop reads its prices from a table; a guard's patrol path and view cone are drawn and tuned in the editor; the game saves to three slots."

Six milestones, one per bullet. Order suggestion: field hints → more kinds of settings → data tables → paths → drawing in the editor → save slots.

## What exists

- **Component fields** (Lust, Lua, Rust alike, via `reflect::FieldKind`): bool, int, float, string, a state (shown as a choice), vec2, vec3, colour (decision 0106), rotation, entity. **A state field already works for the project's names**, so `Sound step = Sound.Sfx.Footstep;` is a dropdown in Properties today; what's missing is a good picker. **No lists:** Lust refuses arrays and class things in component fields (LUST.md §21), events and natives' cousins.
- **Properties** (the shared layout, decision 0050), one way to show any schema.
- **Saves:** `Saves:saveTo(slot)` / `Saves:loadFrom(slot)` (text slot names, `valid_slot`), the editor's `save_game` format 8+, exported games save in `Saved Games/<Game>` with the player's settings in `settings.txt` (decision 0091, kept apart from saves already).
- **Lust's data:** `cMap` (rows by key, typed columns), data files (`<name>.<kind>.lst`, decision 0081), and a `save` list (LUST.md §14).
- **Editor overlays:** the viewport already draws brush edges, lamp markers, tile map grids, walls, values; no general drawing API for game code.

## M1. Field hints

**What it gives:** your own settings in Properties get sliders with limits, steps, units and a sentence of help.

**Syntax (Lust):** the ideas list's `float speed = 4 [range 0 10, tip "metres a second"];`, in the `[ ]` style of Lust's screens. Needs a Lust change: attributes on `has` fields, carried into the component's schema. Suggested words: `range a b`, `step s`, `tip "…"`, `unit "m/s"`, `slider` (vs. a number box), `multiline` (text), `hidden`, `readOnly`, `group "Movement"`, `angle` (degrees shown, radians stored?). Lua: the `component(name, fields)` table gains a fifth entry, a hints table; Rust: `reflect::schema` gains a `hints` struct.

**Where:** `reflect::schema::FieldSchema` gains `hints`; Properties reads them (shared widgets: slider, unit suffix, help line under the field as the Hints card already does); values outside a range are clamped in the editor only (never in game code, which may set anything; a note in Problems if a starting value is outside). Hints are not saved in levels (they're the code's).

**Tests:** hints round-trip from Lust/Lua/Rust into the schema; a slider edit is one undo step; a field's hints change when the code is swapped. **Size:** small (plus the Lust change).

## M2. More kinds of settings

**What it gives:** a component can say "play this sound", "show this sprite", "spawn this prefab", "use this colour", "follow these points".

- **Asset references:** the project's names are already states. Add **pickers** for each name kind (Sound, Clip, Prefab, Picture, Screen, Level, Tag, Material later): a dropdown with search and preview (a sound plays, a sprite shows), and **drag from the Library** onto the field. No new value kind; still saved as the option's name (decision 0077 shape) and renames follow.
- **Missing kinds:** models and sprite sheets aren't project states yet; add `state Model` and `state Sheet` to `solis/project.lst` (or fold sheets into `Clip`'s folders). Check with the lead which kinds matter for the gate.
- **Lists:** the big one. Needs (1) a Lust change: arrays of plain kinds (`int`, `float`, `vec3`, a state, `entity`) allowed as component fields and kept fields; (2) `reflect::FieldKind::List(kind)` and `Value::List` (the existing `List` is a vec3's three numbers, so name the new one carefully, e.g. `Value::Items`); (3) the level file format, prefab overrides (override the whole list, or per item? recommend whole list), save games, the checksum, the game-code interface snapshot; (4) a list editor in Properties (add, remove, drag to reorder, one undo step each), with a length limit (say 1,024) for rule 9.
- **Alternatives for lists:** lists as child entities (a patrol's waypoints as marker entities): works today, clumsy; or lists only on Solis's built-in components (a Path component holds its own points, M4). **Recommended:** real lists (a Lust change), since the gate's patrol and many game designs need them; paths use their own component anyway.

**Tests:** each kind round-trips through levels, prefabs, saves and the checksum; a list's undo; fuzz the level reader with lists; old levels without lists read unchanged (rule 23: a format version raise only if needed). **Size:** medium to large (lists are most of it).

## M3. Data tables

**What it gives:** "every crop: days to grow, price, season" in one file, edited as a table, read by game code with spelling checked.

**Ways to build it:**
1. **A data file of rows** (`crops.table.lst`): columns typed (int, float, text, bool, a state, an asset name, colour), a key column whose values become a state (`state Crop = [Wheat, Carrot]`), and a generated Lust file per table (rule 4) with a class per row and a lookup: `CropRow row = Crops:row(Crop.Wheat); int days = row.daysToGrow;`. Spelling checked, `check` works on the keys.
2. **Lust's own `cMap` in a code file:** works today, no editor table, edits mean editing code.
3. **A spreadsheet import** (CSV): convenient for designers with other tools, but loses types; as an import option of 1.

**Recommendation:** option 1, with CSV import/export as buttons. The table is an asset, read-only while playing (constants: not game state, not in the checksum, but its hash goes into recordings and saves so a replay with a changed table warns). Hot-reloads into a running game.

**Editor UI (mock-up first):** a Table document tab: a grid with typed cells (dropdowns for states and asset names, colour swatches, checkboxes), add/remove/reorder rows and columns, column types and hints, sort and filter for viewing (not saved), copy/paste blocks of cells from and to a spreadsheet. **Keys to ask (rule 17):** Enter edits / moves down, Tab moves right, Ctrl + D fills down? Recommended: the common spreadsheet keys (Enter, Tab, arrows, F2 edits, Ctrl + C / V), no new global keys.

**Tests:** fuzz the reader; round trip; renaming a key renames the state and Lust points at uses; a table of 10,000 rows × 20 columns opens and scrolls within the editor's frame target (scale case). **Size:** medium.

## M4. Paths drawn in the editor

**What it gives:** a curve you draw for a guard to walk, a platform to follow, or a camera to glide along.

**Techniques:**
- **Catmull-Rom splines** pass through every point you click: easiest to draw, no handles. Centripetal parameterisation avoids loops and cusps.
- **Cubic Bézier with handles:** more control (sharp corners, smooth curves), the vector-drawing standard, more to learn.
- **Polylines** (straight segments): simplest; enough for grid games.
- **Constant speed:** any curve needs an arc-length table so "move 3 m along" is even; build it when the path changes (a few hundred samples).

**Recommendation:** one `Path` built-in component with a **kind per path** (Straight, Smooth (centripetal Catmull-Rom), Curves with handles (Bézier)), open or closed, points in the entity's space, 2D or 3D. Its points are a list on a built-in component (Rust), so it doesn't wait for Lust lists. Snap to grid or pixels, and to tile centres for 2D.

**Game code:** `float len = route:length();`, `vec3 p = route:pointAt(distance)`, `vec3 d = route:directionAt(distance)`, `float t = route:nearestDistanceTo(place)`; a kit **PathFollower** (speed, loop or ping-pong, wait at points, event `ReachedPoint(int index)`) for platforms (2D 2's moving platforms), guards and cameras. Reads are of the path as the level holds it (paths don't change while playing unless game code moves the entity).

**Editor UI and keys (mock-up first; rule 17):** with a Path picked, its points and handles drawn in every view; **click on empty space adds a point at the end** (recommended, while a Path tool is on), **Ctrl + click on the line inserts**, **Delete removes the picked points**, drag moves (gizmo for many), **Alt + drag a point breaks its handles** (Bézier). Ask before building; alternatives follow the brush editing pattern (Tab to edit a path's points, like a brush's insides, decision 0111: recommended, it reuses learned keys).

**Tests:** arc-length within 0.1%; a follower's speed constant; closed path wraps; undo per point; fuzz the level reader with paths. **Size:** medium.

## M5. Components that draw in the editor

**What it gives:** a guard's view cone, a turret's range, a spawner's area, drawn while you place and tune them, from the component's own code.

**Ways to build it:**
1. **A method the editor calls** (`method drawInEditor() { Draw:cone(position(), facing(), viewAngle, viewRange, Colour.Yellow); }`): runs on the editor's side with the component's field values from the level (no game running), in its own sandboxed Luau with a tight time limit, for selected entities (and all, as a view option).
2. **Declared shapes** (field hints like `float range = 5 [drawCircle]`): no code runs; limited to stock shapes.
3. **Solis-provided gizmos per built-in component only:** what exists now; game code can't add.

**Recommendation:** option 1, with option 2 as a shortcut for the common cases (radius, cone, box from fields). The drawing API (**`Draw`: line, circle, sphere, box, cone, arrow, text, path**) is the **same one Shared 6's "debug drawing" uses while the game runs**, so build it once here: drawing calls append to a per-frame list the viewport renders over the scene (lines with depth test optional), capped (e.g. 100,000 lines) with a note when full. Draw calls never change the game (they're output), so they're safe on any core: each shard's list is concatenated in entity order.

**Safety:** the editor-side Luau runs game code outside play: it must not send events, spawn, or change anything; natives that change state are refused in this context with a clear message. Time limit per call as game code has (`script::luau`).

**Tests:** the cone follows a field changed in Properties the same frame; a draw method that loops forever is cut off without freezing the editor; no drawing method → nothing drawn, no cost. **Scale case:** 1,000 selected entities each drawing a cone within the editor's frame target. **Size:** medium.

## M6. Save slots

**What it gives:** several saves with a picture, the date and time played, and autosave; the player's own settings (volume, controls, language) kept apart from saves.

- **Metadata:** each save gets a small header: the slot's name (player-given text, guideline 3 allows text here), date and time, time played (game time, from the clock), the level, a picture (a thumbnail of the game's view taken when saving), and the game's own fields (a "chapter" or "money" string game code sets: `Saves:setSummary("Day 12, Spring")`).
- **Picture:** read back the game's last frame on the GPU without waiting (rule 10: `render::readback` already reads back without blocking, used by picking), shrink to e.g. 320 × 180, write as PNG beside the save. The save itself is written at once; the picture follows a frame later.
- **Game code:** `SaveSlot array slots = Saves:slots();` (name, date, timePlayed, summary, hasPicture), `Saves:saveTo(name)` (exists), `Saves:delete(name)`, `Saves:autosave()` (a rotating set of N autosave slots, N a project setting), and screens can show a slot's picture (`image` from a save picture, through the screens' picture reading). Players' settings are already apart (decision 0091); add language when Shared 5 lands.
- **Safety:** saves are files a player can damage or swap: fuzz the header reader (rule 9); a missing or broken picture shows a placeholder; writing through a temporary file and swap (rule 11, exists).

**Tests:** three slots listed newest first; autosave rotates; a damaged header lists the slot as damaged without crashing; time played survives save/load. **Size:** small to medium.

## Dependencies and order

- M1 and M2 need **Lust changes** (field attributes; arrays in component fields). Ask early.
- 2D 1's "entities with typed fields" overlaps M1/M2 (see `03-2d-1.md`, question 1).
- M5's `Draw` API is the same as Shared 6's debug drawing: build it here, reuse there.
- M4's `Path` serves 2D 2's moving platforms (built before, with tweens or code; they can adopt paths now) and Shared 10's cutscene camera moves.
- Save slots' language setting waits for Shared 5 (translations).

## Risks and questions for the project lead

1. **Lists in components** need a Lust change and touch many formats. **Recommended:** yes, plain-kind lists with a length limit; alternative: lists only in built-in components (paths), waypoints as entities.
2. **Hint words:** **recommended** `range`, `step`, `tip`, `unit`, `slider`, `group`; alternatives welcome.
3. **Data tables in game state?** **Recommended:** read-only constants (not saved), with a warning when a replay's tables changed; alternative: tables game code can change (then they're game state, saved and checksummed: much bigger).
4. **Paths' kinds:** **recommended** Straight, Smooth and Curves with handles; alternative: only Smooth.
5. **Path editing keys:** **recommended** Tab to edit a path's points (as brushes), click adds, Ctrl + click inserts, Delete removes, Alt + drag breaks a handle.
6. **Editor drawing runs game code in the editor** (sandboxed, no changes allowed): **recommended** for selected entities by default, all with a Show option.
7. **Save pictures:** **recommended** on by default, 320 × 180.

## Crates

None needed (`image` already writes PNG).

## Size

Phase: large (six milestones; lists and data tables are the biggest).

## Sources

- Splines: [Yuksel et al., Parameterization and applications of Catmull-Rom curves (centripetal)](https://www.cemyuksel.com/research/catmullrom_param/catmullrom_cad.pdf); [Freya Holmér, The continuity of splines (talk)](https://www.youtube.com/watch?v=jvPPXbo87ds).
- Editor drawing and inspector hints are long-standing engine patterns (gizmos drawn from component code; range and tooltip attributes on fields); Lust's `[ ]` look list is the syntax precedent (LUST.md §17).
