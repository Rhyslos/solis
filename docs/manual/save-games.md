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

- A **slot** is a save's name: letters, digits, `_` and `-` (`quick`, `slot_1`, `before-boss`), but not a name Windows keeps for a device (`CON`, `NUL`, `PRN`, `AUX`, `COM1`, `LPT1` and so on). Saving to a slot that has a save replaces it. A project keeps at most 100 saves; past that, game code can replace one but not add more.
- Both happen at the **end of the tick**, after every component's `update`, like moves ([what game code sees](game-code.md#components)). A load replaces the whole game, so nothing else runs in that frame after it.
- Asking for the same slot again in one tick saves once (a save is the whole game, so a `saveGame` in every unit's `update` would otherwise make thousands at a time). At most four slots are saved or loaded in one tick.
- Loading doesn't run the game's starts again (the code outside components, and its fields' starting values): what they did already happened in the saved game.
- `saveGame`, `loadGame` and `rebind` work from `update` and events, not while a component starts (the Console says so if you try).
- A `loadGame` from game code leaves the game the mouse and keys, so the player carries on at once; **Play > Load a save…** gives them back to the editor.
- **Physics** is kept as each body's place and motion. Saving starts physics afresh from those, which a loaded game does too, so the loaded game carries on exactly as the saved one does (a stack of crates may settle by a hair as you save).

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
- **Problem:** "Couldn't load …: it was made by an earlier Solis". **Why:** saves made before the Phase 6 review kept physics in a form this Solis no longer reads. **Fix:** save again.
- **Problem:** "The game wasn't saved: it would be … MB". **Why:** a save can be at most 256 MB to load back, and this game is bigger. **Fix:** fewer entities or physics bodies.
- **Problem:** loading ended my recording. **Why:** a recording plays back from Play, so it can't carry on across a load. What was recorded up to the load is saved in the `recordings` folder. **Fix:** record again from Play to record past it.
- **Problem:** replaying a recording didn't change my saves. **Why:** a replay makes the same saves the recorded play did (so it plays out the same), but doesn't write them over your saves.

## Related
- [Game code](game-code.md)
- [Input](input.md)
- [Play mode](play-mode.md)
- Concepts: [save game, determinism, checksum](concepts.md#simulation)
