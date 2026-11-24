# The uniformity pass

Asked for by the project lead on 2026-11-23, to go over with them before it's built; their answers are [decision 0137](../decisions/0137-the-uniformity-pass.md). It comes after Fixes 3 and before dialogue. It's a big rework, and better done now, while there are no projects to move.

## What they asked

1. **One look, built once.** The editor shouldn't style the same kind of element in several ways, and a part made for one place should be usable everywhere else it fits. Their example: the atlas's cutting of a picture into boxes. They expected sprites to use it already.
2. **How the engine's calls read in Lust.** This means Solis's calls, not Lust itself. Long dotted names (`Something.Something.Something`, as in `Clip.Knight.Walk`, `Module.Ships.Fighter` or the planned `Look.Sprites.Knight.Mana`) are not how they want Solis code to look. The calls are to be rethought across the whole engine.

## The project lead's rule for code (2026-11-23)

Code should be **blocky and easy to read**:

- Things list what they hold in a `has` block, and `method` is the one word for what they do.
- A value is made by saying what it is (`something name = "something new"`).
- **`:` and `.` are avoided as far as they can be.** Lust keeps them rare, and Solis's calls are where it goes wrong (`World:spawn(…)`, `Items:row(Item.IronSword).price`, `Clip.Knight.Walk`).

Lust itself was partly shaped by Claude, so the rule isn't always enforced there either. The aim is a middle ground that still works: Solis's calls as close to that blocky style as Lust allows, and where Lust can't reach, a note of what Lust would need, for its own rulebook.

## Settled already (2026-11-23, decision 0136)

Three directions were settled before the audit, so new work follows them now:
- **actions are made or set**: `new Module.Fighter { position = gate };`, `animation = Clip.KnightWalk;`, with a plain call only where neither fits;
- **names have one dot**: `Clip.KnightWalk`;
- **a read is named first**: `Item sword = Item.IronSword;` then `sword.price`.

The audit applies them to every existing call and brings the cases that don't fit to the project lead.

## Lessons from what developers complain about (2026-11-24)

Research into recurring complaints about other engines (kept outside the repo, which never names them) gave lessons the audit checks Solis against:
- **One way to do each thing.** Several systems for one job (two UI kits, two input systems) confuse people and break more. An old system is retired only once its replacement does everything it did.
- **Names, not strings.** A name typed wrong must be a mistake on its line when the project loads, never a silent failure at play time.
- **One place for one tool.** A tool's parts (painting, rules, collision for tilemaps) belong together, not spread over three panels.
- **Errors for beginners.** Every mistake says what went wrong, where, and one fix, with a link to the manual page.
- **Play feels instant.** Pressing Play on a small project stays under a second.
- **The 2D basics complete.** Sprite sheets into named clips, tilemaps in one panel, and a pixel-perfect camera that doesn't jitter.
- **Upgrades never break projects.** Every file has a version and an upgrade step, tested on examples of every older version (already the rule).
- **Honest shipping.** The export screen shows the game's size and what fills it, and the "Can I make this?" page says which platforms work today.
- **Trust.** Solis stays free, games owe nothing, and no account or online check is needed to open the editor or export.

## How it's done

1. **An audit, written down first.** This plan gains two lists:
   - **The interface:** every kind of element (buttons, fields, lists, pop-ups, tabs, headers, pickers, drawing a box on a picture, grids) and each place it's drawn, with the ones drawn differently and the parts built twice (the atlas's box drawing, the tileset's grid guess, the sprite sheet's cells).
   - **Game code:** every engine call and project name in [the interface file](../game-code-interface.txt) and the manual's examples, grouped by shape (dotted names, methods, events, data files' classes), checked against the [guidelines](../game-api-guidelines.md) and Lust's own rulebook.
2. **Proposals as questions.** A recommended shape for each with a before and after, side by side, asked one choice at a time (CLAUDE.md, "Choices come as questions"). The look's proposals are mock-ups first.
3. **A decision**, then the guidelines and `docs/design/g-layout/` are updated, and the rework is built in milestones of the usual size: shared widgets first, then each panel moved onto them; the calls renamed with the manual, the sample games and the interface file in the same pull requests.

## Decided (2026-11-24)

The project lead answered every question the same day: [decision 0137](../decisions/0137-the-uniformity-pass.md) has the answers. Two differ from the proposals: moving and turning stay calls (physics moves things too), and checkboxes and toggles are chosen by how heavy a setting feels, with a field hint for game code's own fields. Reading another entity's component, `despawn` and the zone events stay as they are.

### The milestones

Built in this order, one pull request each, the manual, the sample games and the interface file moved in the same pull request as what they show:
- **U1 Buttons and fields** (built in two parts: U1a the buttons, U1b the fields and renaming): the shared button (plain and filled), text box, number box (with a slider when a field has a range) and in-place rename; every panel moved onto them.
- **U2 Switches and pickers:** checkboxes and toggles by weight, the `toggle` and `checkbox` field hints; the name picker for longer lists (a Sprite's Look first); the facing pad in Properties and for an animator's direction parameter.
- **U3 One picture board:** the Tileset's picture on the shared board.
- **U4 One dot:** every kind of the project's names with one dot, folders only on a clash, groups as values; the manual and samples moved; `World:find` out of the manual.
- **U5 Setting with `=`:** the setters on a component's own entity as fields the engine reads (the sprite's, the sound's, the screen's, tags, visible, solid, velocity), then the game-wide objects' (`Screens.textSize`, `Window.fullscreen`, `Audio`, `World`).
- **U6 Making with `new`:** the classes Solis writes for modules, sounds, timers and shots, and `new` replacing the spawning, playing, firing and timer calls.

Each is planned in detail when it starts, as the game stages are.

*As built (U1a):* every hand-drawn button in a panel or pop-up is `widgets::button::plain`, `plain_if` (greyed out when it can't be used) or `primary`; the Frames tab's Play is `Button::light`. Left as they are: menu items, the frameless glyph buttons (a layer's ◉, a sound's ×, the Library tiles' +), which are icon buttons, and the tag chips. Panel tests that draw these load the editor's fonts first (`test_support::fonts_ready`).

*As built (U1b):* every number box outside Properties goes through `widgets::field::number` (`number_in` for one that shows words, `number_if` greyed out), so all are drawn as Properties' are; the text boxes already shared the field's fill and corners through the theme. Renaming a row is `widgets::rows::rename_in` (`rename_row` across the room left), used by the Level panel, the Library and the Clips list. The Level panel's and the Library's F2 stay: the project lead asked for no new key, not fewer.

## The audit (2026-11-24)

Written while Fixes 3's last pull requests waited on their checks. Nothing here is decided: each group ends in the question it puts to the project lead, and the proposals come as questions, one at a time, with a before and after.

### Game code

Every engine call in [the interface file](../game-code-interface.txt) (112 calls and 8 events) and every kind of project name, grouped by the shape decision 0136 gives it.

**1. Making something in the world: `new`**: `World:spawn`, `World:spawnWith`, `World:fire`, `Audio:play`, `Audio:playAt`, `entity:playSound`, `entity:startTimer`, `startTimerEvery`, `startTimerAtHour`.
- Shape: `new Module.Fighter { position = gate };`, `new Sound.Horn { at = place };`, `new Timer.DoorOpens { after = 2.0 };`, `new Shot { from = muzzle, speed = 30 };`. `spawn` and `spawnWith` become one: the braces hold the place and the root's values.
- **Lust can't yet:** `new` makes a class, and `Module.Fighter` is a state option. Either Solis writes a class per module, sound and timer into the files it already writes (`solis/project.lst` gains `class Fighter { has { vec3 position; … } }`), or Lust lets a state option be made (asked of Lust). The first works today but doubles the names; the second is the clean one.
- Question: one `new` for everything the world makes, through classes Solis writes, or ask Lust first?

**2. Setting what something is: `=`**: The setters: `animate` and `animateFromStart` (as `animation = Clip.KnightWalk;`, with `restart = true;` or a second field), `stopAnimation` (`playing = false;`), `setVisible`, `setFlip`, `setSolid`, `setVelocity`, `moveTo` (`position = …`), `turnTo` and `face` (`rotation = …`), `setSoundPlaying`, `showScreen` and `hideScreen` (`screen = Screen.Hud;`), `addTag` and `removeTag` (a `tags` field), `Audio:setVolume`, `World:setGameSpeed`, `setTimeOfDay`, `setSeed`, every `Screens:set…` and `Window:set…`, and the maps' and animators' setters. Fixes 3's `facing` and `look` already work this way (`script::engine_fields`).
- Shape: a field in the component's `has` block that the engine reads as each tick ends, the way `facing` is read. One component can own each field; two that both hold `position` would fight, so that's a mistake before the game runs.
- **Lust can't yet:** setting another entity's field (`orc.position = …`): Fixes 2's readings are read-only copies. Until Lust has a way to write to another thing's component, those stay methods on the other entity (`orc:moveTo(place)`), or become an event the other entity handles.
- The game-wide objects' settings (`World`, `Screens`, `Window`, `Audio`) become fields of those objects (`Screens.textSize = 1.2;`), which Lust allows for objects today.
- Question: move every setter to a field, including `position` and `rotation`, which game code sets most?

**3. Reading: named first**: `position`, `rotation`, `velocity`, `name`, `grounded`, `exists`, `hasTag`, `inside`, `timerRunning`, `timeLeft`, `screenShown`, `World:time`, `frameTime`, `viewSize`, `gameSpeed`, `timeOfDay`, `random…`, `Input:held`, `pressed`, `released`, `axis`, `binding`, every `Screens` and `Window` getter, the maps' and animators' readers, `Physics:castRay`, `World:find`, `within`, `nearest`, `the`, `text`, `nearestFacing`, `facingAlong`.
- With group 2, most readers become the same field read back (`position`, `facing`). The ones that ask a question stay methods, read once into a name: `RayHit hit = Physics:castRay(…);` then `hit.distance`.
- Chains to break: `Items:row(Item.IronSword).price` (a table's row), `quest:quest().hero` (another entity's component), `Input:axis(Action.Move).x`.
- Question: reading another entity's component as `Knight seen = orc;` rather than `orc:knight()`, if Lust allows it?

**4. Plain calls that are neither**: `despawn`, `stopSounds`, `push`, `walk`, `stopTimer`, `keepStarted`, `finishTyping`, `Input:rebind`, `Saves:saveTo`, `Saves:loadFrom`, `Window:useNextResolution`, `Window:quit`.
- Each is tried as an event or a value first: `walk` as a `move` field the engine reads each tick (the character controller's wanted move), `stopTimer` as setting the timer's field to nothing, `despawn` as `send Despawn to orc;`. What's left (`quit`, `saveTo`, `loadFrom`) stays a call on its object.
- Question: is `send Despawn to orc;` (Lust's own `send`) better than `orc:despawn();`?

**5. Names with more than one dot.** `Clip.Sprites.Knight.Walk`, `Module.Ships.Fighter`, `Sound.Sfx.Footstep`, `Picture.Sprites.Tools.Axe`, `Level.Forest.Cave`, `Font.Fonts.Lora`, `Text.Shop.Welcome`. Only `Look` has one dot so far (`Look.KnightMana`, Fixes 3).
- Shape: the file's name and the part's, joined (`Clip.KnightWalk`, `Module.Fighter`, `Sound.Footstep`, `Picture.ToolsAxe`, `Text.ShopWelcome`), with folders in front only where two would clash, as looks do.
- **What's lost:** folders as options inside options. Today `is clip == Clip.Sprites.Knight?` is true for any of the knight's clips, and `check` can handle a whole folder at once. With one dot, a clip's sheet would need to be a value of its own (`clip.sheet`), or the folder kept as a second state.
- Question: one dot everywhere, giving up folders as groups in `check`?

**6. Names still written as text**: `World:find("Gate")`, `Saves:saveTo("slot1")`, `Input:rebind(…, "Space")`, the maps' `tileMap` argument (`"maps/farm"`), the animators' `name` and `named` arguments, `Screens:languageName`'s answer. The animators' and maps' are reached through their own generated files (`anim:setSpeed`, `farm:setGrowth`), so their text arguments are what those files call. `World:find` is the last one game writers type.
- Question: retire `World:find` from the manual and keep it only for the console?

**7. Events**: `CueReached`, `ClipFinished`, `Entered`, `Exited`, `EnteredZone`, `LeftZone`, `TimerFinished`, `TypingFinished`. These already fit Lust (`on ClipFinished(Clip clip) { … }`); only the pairs `Entered`/`EnteredZone` and `Exited`/`LeftZone` read differently for the same thing seen from either side.
- Question: `Entered` and `Left` on both sides, the zone's and the walker's?

### The game code proposals, before and after

The recommended shape for each group above, to show beside its question. Code after the arrow is what the proposal would make possible; where it needs something of Lust, that's said.

**1. Making:** one `new` for what the world makes, through a class Solis writes for each module, sound and timer (works today; asking Lust to let a state option be made would remove the extra classes later).
```text
-- Before
entity ship = World:spawnWith(Module.Ships.Fighter, gate, [new FighterValues { speed = 9 }]);
Audio:playAt(Sound.Sfx.Horn, gate);
startTimer(Timer.DoorOpens, 2.0);
-- After
entity ship = new Fighter { position = gate, speed = 9 };
new Horn { at = gate };
new DoorOpens { after = 2.0 };
```

**2. Setting:** a field in `has` that the engine reads, as `facing` and `look` are now (Fixes 3).
```text
-- Before
animate(Clip.Sprites.Knight.Walk);
setFlip(true, false);
moveTo(place);
showScreen(Screen.Hud);
Screens:setTextSize(1.2);
-- After
animation = Clip.KnightWalk;
flip = true;
position = place;
screen = Screen.Hud;
Screens.textSize = 1.2;
```

**3. Reading:** the same fields read back; a question asked once into a name.
```text
-- Before
is Items:row(Item.IronSword).price > coins? then { … }
float hp = orc:health().hp;
-- After
Item sword = Item.IronSword;
is sword.price > coins? then { … }
Health theirs = orc;
float hp = theirs.hp;
```
(`Health theirs = orc;` needs Lust to read a component off an entity by its kind; until then `orc:health()` stays, read once into a name.)

**4. Neither:** an event where one fits, a call on its object where not.
```text
-- Before
orc:despawn();
Window:quit();
-- After
send Despawn() to orc;
Window:quit();
```

**5. One dot:** the file's name and the part's, joined; folders only on a clash.
```text
-- Before
animate(Clip.Sprites.Knight.Walk);
is clip == Clip.Sprites.Knight? then { … }    -- any of the knight's clips
-- After
animation = Clip.KnightWalk;
is clip.sheet == Sheet.Knight? then { … }    -- the group becomes a value of the clip
```

**6. Text names:** `World:find("Gate")` leaves the manual; a one-off entity is reached by `World:the(Component.Gate)` or a field set in Properties.

**7. Events:** the zone and the walker hear the same two words.
```text
-- Before
on Entered(entity who) { … }      -- the zone
on EnteredZone(entity zone) { … } -- the one walking in
-- After
on Entered(entity who) { … }
on Entered(entity zone) { … }    -- needs Lust to tell two events of one name apart by what they carry, or keep EnteredZone
```

### The interface

The shared parts are in `crates/editor/src/widgets/` (buttons, cards, checkboxes, chips, columns, dialogs, fields, keycaps, menus, pills, rows, segmented switches, sliders, sub-tabs, swatches, tiles, toggles, and Fixes 3's picture board). What's drawn beside them instead, found by searching the editor's code outside that folder:

- **Buttons drawn by hand** (`ui.button`, `egui::Button::new`) in 17 files: the animator, atlas, tileset, map, clips, screen, sound event, canvas and tile properties views, the Properties header, the field rows, the tile library, the tag chips, the ribbon's groups and the start-up splash. Some are meant (the ribbon's), most are the same small text button styled a few ways.
- **Text boxes drawn by hand** (`egui::TextEdit::singleline`) in 15 files, against `widgets::field`: the inspector, the Level panel's rename, the Library's rename, search, the Console, the table grid, the tag chips and most views above. Renaming in a list is written three times (Level panel, Library, Clips list).
- **Number boxes** (`egui::DragValue`) in 9 files and **sliders** (`egui::Slider`) in 2, beside `widgets::field::whole_number_field_on` and `widgets::slider`.
- **Checkboxes** (`ui.checkbox`) in 4 files, beside `widgets::checkbox` and `widgets::toggle`, which also mean the same thing in places (Fixes 3's two settings are toggles; the animator's are checkboxes).
- **Pickers:** a field of one of the project's names gets the name picker (search, ▶, thumbnails, drops), a plain state gets a dropdown, and Fixes 3's Look and Facing on a Sprite are dropdowns too.
- **A facing** is chosen three ways: the Sprite's Facing dropdown, the facing pad under the clips preview, and the animator's direction parameter (a vector).
- **A picture cut into pieces** is drawn twice: Atlas Crop and Cut picture… share the picture board (Fixes 3), but the tileset's picture with its tile grid, its zoom and its picking (`tileset_view.rs`) is its own. The grid's guess (`tiles::autotile::detect_grid`) is already shared by both cutters.
- **Lists with rows:** `widgets::rows` draws the rows of the buses, the Clips and Looks lists, the atlas's crops, the preferences and about pages and a few more; the Level panel, the Library's list view and the input page draw their own.
- **Pop-ups:** `widgets::dialog` serves Cut picture…, making a module, the module page, Keep or Undo all after playing, leaving with unsaved changes and the languages and tags pages; the start-up splash has a window of its own.

Questions, each with a mock-up of the before and after on one Design canvas, [Uniformity pass: one look for each element](https://claude.ai/artifact/TdXzWhTMhtqRuRsR5wgpTZ) (five artboards, the today side drawn after the editor's code):
- One button style for small actions in a view, and the ribbon's for the ribbon?
- One text box, one number box (with a slider when a range is known), and one way to rename a row in any list?
- Toggles for settings that take effect at once, checkboxes only in forms that wait for OK?
- The name picker for every choice from a list longer than a handful, the facing pad wherever a facing is chosen?
- The tileset's picture on the picture board, so cutting, zooming and picking work the same in all three?

## Until then

- New UI reuses an existing part where one fits. Fixes 3's cutting pop-up is built on the atlas's box drawing, as its plan says, not beside it.
- New calls follow the guidelines as they stand. Anything Fixes 3 adds (`Facing.NorthEast`, looks' names) is in the audit's list like everything else.
