# docs/design/g-layout/additions/round-2

Additions to the project lead's design: the everyday screens (milestones M5 to M9). They are parts the G layout doesn't draw, mocked up in its own language on copies of its own screens.

**Approved by the project lead on 2026-11-11**, with these choices (each asked as a question):

- **Hints:** a card under Properties (`01-hints-card-a`), chosen over a strip pinned to the bottom.
- **Placing is entered by the Place shelf tab** being in front (`06-placing-a`); no ribbon button.
- **Colours for the kinds of file:** `07-library-kinds-a` (mapped onto the design's colours), **but with `07-library-kinds-b`'s colour for three kinds: sound events, code files and areas.** Both pictures are kept for that reason. The Library's milestone makes one final sheet of every kind's colour for the project lead to check.
- **Templates:** `09-recent-projects-a`: the blank project and Solis's four examples (real), then the design's templates as unfinished parts, without VR / XR.
- **Solis starts in the full editor window** on the Recent projects page (`09-recent-projects-start`); the small launcher window goes, its two promises stay (always the projects page first; nothing reopened by itself).
- **2D settings get a tab of their own** in Project settings, right after General (`12-project-settings-2d-a`).
- **New keys and mouse actions:** every recommended answer on `14-keys-and-mouse` is approved (the plan lists them one by one under "What the project lead has decided").
- **Everything else in the round** is approved as drawn.

**These pictures are templates at screen size** (1440 × 900, one pixel to the point, with a reduced palette to keep the repository small), unlike the design's own screens and round 1, which are saved at twice the size. Take layout, contents and words from them; take exact colours and sizes from the tokens (`crates/editor/src/tokens.rs`) and the specification. Only the chosen alternative of each choice is kept. `00-overview.png` shows the whole round small.

## The pictures

| Picture | What it shows | Answers |
| --- | --- | --- |
| `01-hints-card-a` | Today's Hints panel as the last, quieter card under the component cards in Properties: one sentence per component, the first brighter. "Turn hints off" on the card switches it off. | Inventory a.3 (Hints), plan M5 |
| `01-hints-card-off` | Hints off: the card is gone, and the status bar says for three seconds where to turn them back on (Preferences › General, picture 11). | the same |
| `02-viewport-pills` | Snapping with its third row (as approved), and the viewport's pills: top left Perspective ▾, Standard ▾, Show ▾; bottom right Lock X Y Z (Z locked, to show the lit state), World ▾ and the camera's speed. | Inventory a.5 |
| `02-viewport-view-menu`, `-look-menu`, `-show-menu` | What the three top-left pills open: the views (Perspective, Top, Front, Right, 2D; then how many views), the looks (Standard, Stylized, Unlit; bands of light and outlines), and what the viewport shows. | Inventory a.5 |
| `02-viewport-pills-space-menu` | World or Object, from its pill, with today's key L. | Inventory a.5 |
| `02-viewport-pills-2d` | The 2D view: the bottom-right pills become Lock X Y, Snap: Pixels ▾, Grid 16 px, Zoom 300% ▾. | Inventory a.5 |
| `03-toolbox` | Select, Move, Rotate, Scale, Transform (in use), then the design's four tools as placeholders; the hover text with today's key. | Inventory a.5, b.1 |
| `04-properties-cards` | Properties as cards for Solis's components: Transform; Model; Light (kind, colour, strength, range, cone, soft edge, shadows, only at night); a game-code component with a number, a text, a tick, a colour, an entity and a file, its file's name and **Open code**; the dashed **+ Component**; ? and ⋯ on each card. | plan M5, D4, D11 |
| `04-properties-add-component` | + Component open: a search field and the list in today's two groups (Built in, Game code), with a hover text. | plan M5 |
| `04-properties-card-menu` | Right-click on a card (the same list as its ⋯): Copy values, Paste values, Reset to defaults, Open code, Show the file in the Library, Remove. | Inventory b.1 (`home.object.card-menu`) |
| `04-properties-playing` | While the game plays: a note across Properties (Stop asks Keep or Undo all; what the game changes goes back by itself; "1 change so far"), and the value changed while playing with the accent border and "↺ 0.8", the value from before Play. | spec "Play mode", `cross.play-changes` |
| `05-area-column` | The Area column: search, fold and ⋯ in the header; a hovered row with lock and eye; a locked row; a hidden row; an assembly's copy with its parts; a tile map entity. | Inventory a.3 (Area) |
| `05-area-column-search` | The search field open, with a word typed and the matches. | the same |
| `05-area-column-menu` | Right-click on a row: Rename, Duplicate, Delete; Add entity, point light, spotlight, camera; Create assembly from selection, Show in the Library; Select all, Select nothing. | the same, a.2 |
| `05-area-column-more-menu` | The header's ⋯: Unfold everything, Fold everything, Show everything hidden, Unlock everything. | the same |
| `06-placing-a` | Placing, entered by bringing the **Place** shelf tab to the front (recommended): the ribbon in its build form, no toolbox, the ghost, the hint strip, the Placement column with only Grid and Free real, the Assets column's chips. | D16, D12, plan M6 |
| `06-placing-a-entering` | The same answer seen from the level editor: Place is always the first shelf tab. | D16 |
| `07-library-kinds-a` | The Library with one tile of every kind of file Solis has, in the plan's colours: the design's seven and the three it adds. Chips follow the colours. | D17 (recommended) |
| `07-library-kinds-b` | The same in today's colours (`theme::kind`). | D17 (alternative) |
| `08-library-details-sound` | Details for a sound: the waveform with the playhead, Stop (Listen), the time, and the facts Solis knows. | Inventory b.3 (`lib.details.*`) |
| `08-library-details-sprite-sheet` | Details for a sprite sheet: the sheet's frames with one picked, its facts, the clips it can play, **Edit clips…**. | the same |
| `08-library-details-code` | Details for a code file: its first lines, **Open code**, the components it defines; Used by is real for code. | the same, D11 |
| `08-library-details-text` | The same for a text file: **Open in its program**. | the same |
| `08-library-ribbon` | The Library ribbon: Add, View, Filter (Type), Use (Place) real; Tags, Missing refs, Source control and Show in area as placeholders. Also Details for a model, with what Solis knows of one. | Inventory b.3 |
| `09-recent-projects-a` | Recent projects with the launcher's things: pin, the build mark (this one, older, newer), a project whose folder is missing (Locate…, Remove), Sort by, Open… ▾, the newer-build notice (Updates has a **New** mark; the status bar says "Build 58 is out"). Templates as recommended: Blank project and the four examples real, then three of the design's as placeholders. | D8 (recommended), Inventory a.8 |
| `09-recent-card-menu` | Right-click on a card: Open, Pin, Show in Explorer, Rename…, Make a copy…, Remove from list. | Inventory a.8 |
| `09-recent-open-menu` | Open… ▾: Open a project folder…, Scan a folder for projects…. | Inventory a.8 |
| `09-recent-projects-start` | How Solis starts, recommended: the page in the editor's own window, the other tabs dimmed, "No project open", "Carry on without a project" in place of Back to editor. | D7 (recommended) |
| `10-file-learn` | File › Learn: the four examples, the first hour's five steps, the manual, the sample area. | Inventory a.8, a.2 |
| `10-file-updates` | File › Updates: the notice card, this build, the newest build with Check now, "Look for updates when Solis starts", the releases page, the changes list, what's new. | Inventory a.8 |
| `10-file-about` | About as a File page: the name, version and build, the two sentences of today's About, the licences. | Inventory a.2 |
| `11-prefs-general` | Preferences › General: Look for updates when Solis starts, Show hints, **Hide unfinished parts**, with a small card showing what an unfinished part looks like. | plan 3.3, Inventory a.9 |
| `11-prefs-viewport`, `11-prefs-viewport-2` | Preferences › Viewport, top and scrolled to its end: what the viewport shows, the snapping steps, the camera's speed, the views, the look. | Inventory a.9 |
| `11-prefs-shortcuts` | Preferences › Shortcuts: today's keys as a table in the style of the design's Input screen, with a filter and a search; **Change a key** is a placeholder. | Inventory b.8 |
| `12-project-settings-2d-a` | A **2D** sub-tab right after General, with Solis's four 2D settings and a card saying how the base resolution fits three screens. | D9 (recommended) |
| `12-project-settings-build` | Project settings › Build: what Export for playtesting remembers in the project (the game's name, the area it starts in, the window). | Inventory b.12 |
| `12-project-settings-general` | Project settings › General with **Scripting** as plain words, the real rows (Project name, Game starts in, Engine version) and the unbuilt ones as placeholders; Default game mode and Units and axes left out. | D11, D12 |
| `13-project-settings-input` | Input with Solis's thirteen ready-made actions and one of the user's own; a binding picked, with the hint that Backspace clears it; what is real in the right column. | D15 row 32, Inventory b.13 |
| `14-keys-and-mouse` | Every new key and mouse action of D15 (17 rows), by milestone, each with the recommended answer and the alternative. | D15, rule 17 |

## What the drawing guessed

**Shell and style**

- Round 1 drew Inter 500 and 700 with the nearest weight, because Solis didn't bundle them. Since M1 it does, so round 2 uses the design's own weights: active tabs and setting names are a little heavier than in round 1's pictures, as in the lead's own.
- Every icon has square line ends, round 1's 69 included (the lead's answer 15). No new icons were needed.
- With no project open the status bar shows only "Ready", the update notice and the version.

**Home**

- **Model has one setting today, the model.** The brief asked for "its look and shadows"; Solis has neither on a model (the look belongs to the viewport, and only the sun casts shadows, on the Light card). The card is drawn with the design's own **Material** and **Shadows** rows as placeholders. Neither is in the registry (Inventory b.1 has no row for them). Round 1 drew Shadows as a working switch; that was wrong.
- A card's title row has **?** (its manual page) and **⋯**. A game-code card has no ?; its second line is the file's name and **Open code**: four cards across are 207 px each, too narrow for the name, the file and the button on one line.
- The changed-from-its-start mark (today's dot and ↺) is the design's settings dot, small, before the label, with ↺ at the field's end (Light › Strength).
- **Changed while playing:** the spec says "highlighted". Drawn as the design's just-edited accent border, kept until Stop, with "↺ 0.8" to put back the value from before Play. The note is an accent-mixed strip under the Properties header.
- Kind icons stay on the Area column's rows (today's panel has them), in grey, since the kinds' colours are D17's question; an assembly's copy is the design's blue. The few words today's panel adds ("Spotlight", "tile map") follow the name in grey. Today's × on a model's row is not drawn: Delete and the right-click menu do it.
- The 2D view's drawing (flat boxes, the "Game screen" frame, the pointer's readout in pixels) is a stand-in.
- The pills at the bottom right hold World / Object and the axis locks, as the brief says; the Inventory had proposed a click on the status bar's "Snap 0.25 · World". The status bar still says it.
- The camera's speed is the bare number today's corner shows.

**Placing**

- **The Edit group (Undo, Redo) stays in the build form.** The plan lists Place, Snapping, Paint, Transform, Play.
- **Transform holds all five tools** (Select, Move, Rotate, Scale, and "All three" for the Transform tool): with no toolbox, Select and Transform would otherwise have no button.
- Snapping keeps its approved three rows; "Snap to surface" is a placeholder row in a second column, so with unfinished parts hidden the group is the level editor's.
- The design's Screen 2 has no document tab strip. It is kept here (the viewport is 400 px, not 430): the area's tab carries the unsaved dot, the + and the Game tab.
- The ghost's label reads "x … y …": Solis's ground is X and Y, Z is up.
- The Assets column's chips: ★ Favorites and Recent (placeholders), the folder, Lights, then Models, Assemblies, Sprites and Tile maps with the dots of D17's recommended colours. A sound can't be placed, so the design's sound tile became a sprite sheet and a tile map.
- The shelf bar's hint reads "Drag into the area, or double-click to place where the camera looks": click-then-click is unbuilt (D15 row 17).
- Alternative b's button is the first group of the ribbon, so it doesn't move when the ribbon changes form.

**Library**

- Chips are one per colour, so they differ between a (nine) and b (nine, other groups). Today's Library has five chips (Models, Sprites, Maps, Sounds, Code).
- The plan's mapping names no colour for **atlas crops** (drawn in the picture's) or for **any other file** (grey #8C8C8C).
- "Lit · Wire · UV" on a model's preview are placeholders; the sound's and the sheet's previews have their own controls in the same place.
- "Edit clips…", "Open code" and "Open in its program" sit at the top right of Details. The sprite sheet's file is "gull.ase"; no program is named.
- The status bar says "Drag models anywhere to import", as the Inventory says it must until any kind can be imported.

**File pages**

- A project's picture is the placeholder the Inventory describes: its first letter (a ? when the folder is gone) with the mark. The pin is at the right of the name.
- The build mark's words are the launcher's ("this one", "older", "newer"), not the design's "upgrade".
- The template tiles are 126 px wide (the design's are 150) so eight fit one row at 1440 px.
- Learn's "Add the sample area" and the manual are two setting rows. Updates' "What's new" text is a sample. About's "Open LICENSE" and "Open the notices" are new: today's About has only the name, the version and two sentences.
- Preferences › General has no placeholder rows: none is registered for it. The card "How an unfinished part looks" is an addition.
- **Shortcuts is read-only.** Checked: the editor's own keys are fixed in the code; only a game's input actions can be rebound. The picture shows the first fourteen keys; the rest follow in the same table.
- "Scale step" shows "10%" as plain words: today's scale snapping is always 10%.
- The 2D page's "How 480 × 270 fits a screen" card is today's line under the page, as a card.
- Build: where an export goes is remembered on the computer, never in the project, so it isn't a row; "Go to the Build tab" leads to the export itself.
- Input: "Add action ▾" stands for today's Add button and Add axis; "Restore the ready-made actions" is a link under the cards; the details card names the action as game code does ("Action.Jump"). Mouse speed and Stick and key speed belong to an axis's details and aren't in this picture (a button is picked).
- On `14-keys-and-mouse` the plan gives an alternative for rows 7, 18 and 30 only. The other alternatives are mine: "as today" or "as the design says".

## What could not be drawn as proposed

1. **Model's look and shadows** don't exist (above). Either the two rows are registered as placeholders, or the Model card has one row.
2. **The Hints card ends up below the fold.** The shelf's columns are 312 px tall; the Light card alone is about 270. With any tall card the Hints card is out of sight until Properties is scrolled (`04-properties-cards` shows it honestly: only its title is in view). `01-hints-card-b` is a way round it.
3. **A taller shelf cuts the toolbox.** Nine 34 px tools need 370 px, exactly the design's viewport. Drag the shelf up 30 px and the last tool is cut: the toolbox has to scroll or wrap. Every picture here keeps the design's shelf height for that reason.
4. **D17's recommended colours have two near-twins:** code and text (#8FB8F0) beside assembly (#7FB2FF), and screen (#C38CFF) beside clips and animator (#B48CFF). `07-library-kinds-a` shows both pairs side by side.
5. **Today's colours aren't sixteen for files.** Six kinds share one gold (sprite sheet, clips, animator, atlas crops, tileset, tile map), close to the folder's and the assembly's yellows; three of the sixteen are for entities (sun, environment, camera). `07-library-kinds-b` shows it.
6. **A screen is not a kind of file today:** it is a Lust file, shown as Code. Giving it its own colour (recommended mapping) means the Library has to tell screens from other code.
7. **Used by is unbuilt for every kind but code.** For a sound, a sheet, a text file and a model, the third column of Details is a placeholder.
8. **Input's conflict mark needs a rule.** The ready-made actions share inputs on purpose (the arrow keys are Move, Pan and MenuMove; Space is Jump and MenuChoose). "The same input on two actions" would mark them all. The picture shows one real clash, with an action of the user's own.
9. **Preferences › Viewport is longer than the window** (thirteen rows): two pictures.
10. **"Y up" in the plan (D12) is wrong for Solis: Z is up.** It only matters to the row D12 leaves out.
11. **Project settings › Build** has an unbuilt half with no id in the Inventory (targets and platforms): nothing is drawn for it.
