# Save games

> Save the whole game and load it back exactly as it was: every entity, every component's values, the lasers in flight, physics, the clock and the random numbers.

## When to use it
Whenever a player should be able to stop and carry on later: a quick-save key, a save point, a checkpoint before a boss. Solis saves **everything** for you, so you don't write any code to list what to keep, and nothing gets forgotten. A loaded game carries on *exactly* as the saved one would have: the same crate falls the same way, the same random numbers come up.

It's also a debugging tool: save just before something goes wrong, and you can load and watch it again and again.

## How to use it

**From game code** (in Lust, `get "solis/world":[saveGame, loadGame];`; in Lua, `solis.saveGame("quick")`):

```
get "solis/world":[saveGame, loadGame];
get "solis/input":[pressed];

component QuickSave {
    method update(float dt) {
        is pressed("QuickSave")? then {
            saveGame("quick");
        }
        is pressed("QuickLoad")? then {
            loadGame("quick");
        }
    }
}

pass QuickSave;
end
```

Add the QuickSave and QuickLoad actions in **Edit > Input actions…** and bind them to keys of your choice ([Input](input.md)).

- A **slot** is a save's name: letters, digits, `_` and `-` (`quick`, `slot_1`, `before-boss`). Saving to a slot that has a save replaces it.
- Both happen at the **end of the tick**, after every component's `update`, like moves ([what game code sees](game-code.md#components)). A load replaces the whole game, so nothing else runs in that frame after it.

**From the editor:** **Play > Load a save…** picks a save from the project's `saves` folder and plays from it, whether or not the game is running. **Stop** still puts the area back as it was before you pressed Play: a save is a moment in a game, not a change to your area.

**Where saves live:** in the project's `saves` folder, one `.solsave` file per slot, beside `project.solis`. New projects keep this folder out of Git, since saves are your test plays, not part of the game. Exported games (Phase 10) will keep players' saves in their own saved-games folder.

## Settings
Save games have no settings: everything is saved.

## Shortcuts
None of its own. A game chooses its own save and load keys through [input actions](input.md).

## Tips and common problems
- **Problem:** the Console says the game wasn't saved because save games are kept in a project. **Why:** saves are files in a project's folder. **Fix:** open or make a project (File > New project…).
- **Problem:** "Couldn't load quick: it was saved by the project …". **Why:** a save belongs to the project that made it. **Fix:** load it in that project.
- **Problem:** "Couldn't load …: it's damaged". **Why:** the file was cut short or changed (a save carries a checksum, so a damaged one is refused rather than half loaded). **Fix:** save again.
- **Problem:** a value in my component isn't the same after loading. **Why:** a save keeps every field in a component's `has` block, but not other variables the code keeps for itself. **Fix:** put anything that must survive a load in `has`.
- **Problem:** loading ended my recording. **Why:** a recording plays back from Play, so it can't carry on across a load. **Fix:** record again from Play.

## Related
- [Game code](game-code.md)
- [Input](input.md)
- [Play mode](play-mode.md)
- Concepts: [save game, determinism, checksum](concepts.md#simulation)
