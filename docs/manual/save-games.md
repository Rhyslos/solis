# Save games

> Save the whole game and load it back exactly as it was: every entity, every component's values, the lasers in flight, physics, sprites' clips, animators' states, tile maps as the game changed them, the [timers](timers.md) waiting, the clock, the time of day and the random numbers.

## When to use it
Whenever a player should be able to stop and carry on later: a quick-save key, a save point, a checkpoint before a boss. Solis saves **everything** for you, so you don't write any code to list what to keep, and nothing gets forgotten. A loaded game carries on *exactly* as the saved one would have: the same crate falls the same way, the same random numbers come up.

It's also a debugging tool: save just before something goes wrong, and you can load and watch it again and again.

## How to use it

**From game code** (in Lust, `fetch "solis/world":Saves;` then `Saves:saveTo("quick")`; in Lua, `solis.Saves:saveTo("quick")`; in Rust, `cx.host().save_game("quick", false)`, with `true` to load):

```lust
fetch "solis/world":Saves;
fetch "solis/input":Input;
fetch "solis/project":Action;

component QuickSave {
    method update(float dt) {
        is Input:pressed(Action.QuickSave)? then {
            Saves:saveTo("quick");
        }
        is Input:pressed(Action.QuickLoad)? then {
            Saves:loadFrom("quick");
        }
    }
}

pass QuickSave;
end
```

```rust
use script::{Context, RustComponent};

pub struct QuickSave;

impl RustComponent for QuickSave {
    fn update(&mut self, cx: &mut Context<'_>, _dt: f64) {
        // The slot's name, then whether to load it (true) or save it (false).
        if cx.host().action("QuickSave").pressed {
            cx.host().save_game("quick", false);
        }
        if cx.host().action("QuickLoad").pressed {
            cx.host().save_game("quick", true);
        }
    }

    // values() and set(): as Mover's in Game code, under Components.
}
```

```lua
local QuickSave = component("QuickSave", {})

function QuickSave:update(dt)
    if solis.Input:pressed("QuickSave") then
        solis.Saves:saveTo("quick")
    end
    if solis.Input:pressed("QuickLoad") then
        solis.Saves:loadFrom("quick")
    end
end
```

Add the QuickSave and QuickLoad actions in **File › Project settings › Input** and bind them to keys of your choice ([Input](input.md)).

- A **slot** is a save's name: letters, digits, `_` and `-` (`quick`, `slot_1`, `before-boss`), but not a name Windows keeps for a device (`CON`, `NUL`, `PRN`, `AUX`, `COM1`, `LPT1` and so on). Saving to a slot that has a save replaces it. A project keeps at most 100 saves, and 2 GB of them together; past that, game code can replace one but not add more (the Console says so).
- Both happen at the **end of the tick**, after every component's `update`, like moves ([what game code sees](game-code.md#components)). A load replaces the whole game, so nothing else runs in that frame after it.
- Asking for the same slot again in one tick saves once (a save is the whole game, so a `Saves:saveTo` in every unit's `update` would otherwise make thousands at a time). At most four slots are saved or loaded in one tick.
- Loading doesn't run the game's starts again (the code outside components, and its fields' starting values): what they did already happened in the saved game.
- `Saves:saveTo`, `Saves:loadFrom` and `Input:rebind` work from `update` and events, not while a component starts (the Console says so if you try).
- A `Saves:loadFrom` from game code leaves the game the mouse and keys, so the player carries on at once; **Debug › Load save** gives them back to the editor.
- **Game-wide objects** (a clock, the money: [Game code](game-code.md#game-wide-objects)) are kept with every field a component's could have, and loading gives them back. An object or field the code no longer has is left out, and the Console says which.
- **Physics** is kept as each body's place and motion. Saving starts physics afresh from those, which a loaded game does too, so the loaded game carries on exactly as the saved one does (a stack of crates may settle by a hair as you save).

**From the editor:** **Debug › Load save** picks a save from the project's `saves` folder and plays from it, whether or not the game is running. **Stop** still puts the level back as it was before you pressed Play: a save is a moment in a game, not a change to your level. If you edit the level after loading, Stop offers to keep those edits as usual, and **Keep** makes them on your level from before Play, never on the save's (a change to an entity only the save had can't be kept; see [Play mode](play-mode.md)).

**Saves survive updates** ([Backups and upgrades](backups-and-upgrades.md)): a save made by an earlier Solis loads in a later one, upgraded as it's read. A save is never refused just for being older, so a game's update never wipes its players' progress. The very first saves (format 1, from before the Phase 6 review) kept physics in a form Solis no longer reads, so things that were moving start at rest where they were, and the Console says so. A save is still refused when it was made by a *newer* Solis.

**Your game's own versions:** when your game changes in a way old saves don't fit (a new map, say), that's for your game to decide, not Solis. Keep a version number in a component's `has` block (`int saveVersion = 3;`), and check it in code after a load: carry on, put things right, or tell the player.

**Where saves live:** in the project's `saves` folder, one `.solsave` file per slot, beside `project.solis`. New projects keep this folder out of Git, since saves are your test plays, not part of the game. An exported game keeps players' saves in their own saved-games folder instead ([Exporting for playtesting](playtest-export.md)).

### Why the game must come out the same everywhere

A save, a recording and (later) a game played online all rest on one promise: **the same game, given the same input, comes out exactly the same**, tick by tick, on any PC and with any number of cores ([determinism](concepts.md#simulation)). A loaded save only carries on as the saved game would have if every sum works out to the very same bits; a recording only replays if it does on the PC replaying it; two players online only stay in step if their PCs agree, since they send each other only their input.

The **game state** is what has to agree: where everything is, physics, every component's values, tile maps, animators, zones, timers, the random numbers. What's only shown (sounds, light, the sky, screens, shakes) doesn't, and needn't.

Solis keeps its own side of the promise:

- game code reads the world as the tick started and its changes land in a fixed order, so the number of cores never changes the result ([what game code sees](game-code.md#components));
- physics is built to give the same answer on every PC ([Physics](physics.md));
- game code's maths (`Maths:sin`, `Maths:exp`, `Maths:power` and the rest in Lust; `math.sin`, `math.exp`, `math.pow` and the rest in Lua) is worked out by Solis's own maths, the same to the last bit everywhere, never by the computer's system library, which rounds the last digit differently on Windows and on Linux;
- every tick, the game's [checksum](concepts.md#simulation) sums up its whole state, so a replay that stops matching says at which tick;
- Solis checks itself: every change to Solis plays a recorded game, with physics, 2D and 3D, spawns, tile maps, animators, zones, timers, speed changes and game code in all three languages, on Linux and on Windows, on one core and on four, and compares every tick. A sum that works out differently on another PC is caught before it reaches your game.

Your side, in game code:

- **Roll random numbers with the game's own:** `World:random()`, `World:randomRange` and `World:randomInt`; in Rust `cx.host().random()`, and in Lua `math.random` follows it too ([Random numbers](game-code.md#random-numbers)). They follow the game's seed, so a save or a replay rolls the same numbers. Lust's own `Random` (`Random[1, 6]`, `Random:chance`) doesn't follow it yet, so avoid it in anything the game keeps.
- **Never keep real time in game state:** `World:frameTime()` is how long the frame took on this PC, which differs every time. Use the `dt` your `update` is given.
- **In Lua, raise to a power with `math.pow(a, b)`**, not `a ^ b`: the `^` sign is worked out by the computer's own library, which can round differently on another PC (a whole power of two, such as `2 ^ 3`, is exact anywhere). In Lust, `Maths:power` is always exact.
- **A tween's done block** (`tween … { … }`) can run in a different order, against another tween's ending in the same tick, with another number of cores. Until Solis puts them in a fixed order, keep what a done block does to the component's own fields, or glide the value yourself in `update`.

## Settings
Save games have no settings: everything is saved.

## Shortcuts
None of its own. A game chooses its own save and load keys through [input actions](input.md).

## Tips and common problems
- **Problem:** the Console says the game wasn't saved because save games are kept in a project. **Why:** saves are files in a project's folder. **Fix:** open or make a project on the [launcher](launcher.md).
- **Problem:** "the saves folder is a link to somewhere else". **Why:** Solis only writes saves into a real folder in the project, so a project you downloaded can't send them elsewhere on your computer. **Fix:** delete the `saves` link (or move the real folder into the project).
- **Problem:** "Couldn't load quick: it was saved by the project …". **Why:** a save belongs to the project that made it. **Fix:** load it in that project.
- **Problem:** "Couldn't load …: it's damaged". **Why:** the file was cut short or changed (a save carries a checksum, so a damaged one is refused rather than half loaded). **Fix:** save again.
- **Problem:** a value in my component isn't the same after loading. **Why:** a save keeps every field in a component's `has` block, but not other variables the code keeps for itself. **Fix:** put anything that must survive a load in `has`.
- **Problem:** the Console says a component's saved values were left out. **Why:** the game's code has no component by that name any more (it was renamed or removed since the save). The rest of the save loads. **Fix:** nothing, if it was removed on purpose; if it was renamed, its values start fresh.
- **Problem:** "Couldn't load …: it was made by a newer Solis". **Why:** someone saved it with a newer build. **Fix:** update Solis.
- **Problem:** "The game wasn't saved: the tile map on … has grown too big to be loaded again". **Why:** a save keeps each [tile map](tile-maps.md) as the game has changed it, and a map is read back within the same limits as a map file: 64 MB and 12 million numbers, texts and symbols, which a number in a million and a half cells is past. A save that couldn't be loaded would be the player's progress lost, so it isn't written; the save in that slot before stays. **Fix:** keep values only on the cells that need them, or split the world into several maps.
- **Problem:** "The game wasn't saved: it would be … MB". **Why:** a save can be at most 256 MB to load back, and this game is bigger. **Fix:** fewer entities or physics bodies.
- **Problem:** loading ended my recording. **Why:** a recording plays back from Play, so it can't carry on across a load. What was recorded up to the load is saved in the `recordings` folder. **Fix:** record again from Play to record past it.
- **Problem:** replaying a recording didn't change my saves. **Why:** a replay makes the same saves the recorded play did (so it plays out the same), but doesn't write them over your saves.

## Related
- [Game code](game-code.md)
- [Input](input.md)
- [Play mode](play-mode.md)
- [Checking a project](checking-a-project.md): recordings a project keeps, played back before an update
- Concepts: [save game, determinism, checksum](concepts.md#simulation)
