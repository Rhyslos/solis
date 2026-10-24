# Game code

> Your game's own rules (how a turret aims, when a door opens) are written as code, as components you put on entities. Solis checks the code as you save, shows its fields in Properties, and runs it when you press Play.

## When to use it
Whenever your game needs to *do* something: react to the player, count score, move things on its own. Areas and assemblies say what's in the world; [game code](concepts.md#game-code) says what happens in it.

Solis games can be written in three languages:
- **[Lust](concepts.md#game-code)**, made for Solis's kind of games and the one this manual teaches first;
- **Lua**, if you know it from Roblox or modding;
- **Rust**, the language Solis itself is written in.

**Right now:** Lust components work: they appear in Properties and run in play mode. Components that talk to each other through events, changes that reach a running game, and Lua and Rust components come with the next updates of Phase 4.

## How to use it
1. Open a [project](projects-and-the-library.md). Solis puts a `solis` folder in its `assets`, holding `world.lst` and `entity.lst`. They describe what your code can ask of the engine, such as `say` (write in the Console) or `moveTo` (put an entity somewhere).
2. Make a file called `main.lst` in the `assets` folder, in any text editor. Your game's code starts there: Lust reads `main.lst` first, and every other code file is fetched from it, or from a file it fetches.
3. Use what the engine offers with Lust's `get`:

   ```
   get "solis/world":[say, time];
   get "solis/entity":[positionOf, moveTo];

   component Spinner {
       has {
           float speed = 2;
       }
   }

   pass Spinner;
   end
   ```

4. Save. Solis checks the code within a moment. Mistakes appear in the [Problems](problems.md) panel, such as "main.lst, line 6:15: speed is a float, but this is text". The Console says how many there are.
5. Click a mistake to open its file in your code editor, fix it and save. It disappears from Problems by itself.

## Components

A [component](concepts.md#the-area) written in Lust works like Solis's own: put it on an entity, set its fields in Properties, and it does its job when the game runs.

```
get "solis/entity":[positionOf, moveTo];

component Mover {
    has {
        float speed = 2;
    }

    method update(float dt) {
        vec3 ahead = [1, 0, 0];
        moveTo(me, positionOf(me) + ahead * speed * dt);
    }
}

pass Mover;
end
```

1. Save the file. When the code has no mistakes, **Mover** appears in Properties' **Add component** menu.
2. Select an entity and add **Mover**. Its `has` fields show in Properties with their starting values (`speed` as **Speed**), and are saved in the area like any other component's.
3. Press **Play**. Every [tick](concepts.md#game-code) (60 times a second), Solis runs each component's `update`, with `dt`, how long a tick is in seconds. `me` is the entity the component is on. The Game panel shows the entity moving.
4. Change **Speed** in Properties while it plays: the running Mover takes the new speed at once, and keeps everything else it has (here, where it is).
5. Press **Stop**. Everything is back as it was before Play.

What a field can hold: `bool`, `int`, `float`, `string`, a `state` (shown as a choice of its options), `vec3`, `rotation` and `entity` (another entity, or `nothing`).

**Order:** each tick runs entities in the order of their IDs, and an entity's components in the order of their names, so a game runs the same way every time.

**When something goes wrong while it runs,** such as dividing by zero, that component stops and the Console says which one, on which entity, and where in the Lust: "Divider on Splitter stopped: main.lst, line 8: …". The rest of the game keeps going. A component that runs for more than a tenth of a second without stopping (a loop that never ends) is stopped the same way, so it can't freeze Solis.

Other code files can sit in any folder in `assets`. Fetch them by their path from `assets`, such as `fetch "weapons/gun":Gun;` for `assets/weapons/gun.lst`. The Library's **Code** tab lists them all.

## What the engine offers

| In | Method | What it does |
|---|---|---|
| `solis/world` | `say(string words)` | Writes words in the Console. |
| `solis/world` | `float time()` | How long the game has been running, in seconds. |
| `solis/world` | `entity find(string name)` | The first entity with this name, in the Area panel's order, or `nothing`. |
| `solis/entity` | `vec3 positionOf(entity which)` | Where an entity is in the world, in metres. |
| `solis/entity` | `moveTo(entity which, vec3 place)` | Puts an entity at a place in the world. |
| `solis/entity` | `rotation rotationOf(entity which)` | Which way an entity faces. |
| `solis/entity` | `turnTo(entity which, rotation facing)` | Turns an entity to face this way. |
| `solis/entity` | `string nameOf(entity which)` | An entity's name. |

## Tips and common problems
- **Problem:** Problems says the game's code starts at `main.lst` and there isn't one. **Why:** there are `.lst` files, but none is called `main.lst` in the `assets` folder itself. **Fix:** make `assets/main.lst`, and fetch your other files from it.
- **Problem:** changes I made to `solis/world.lst` disappeared. **Why:** Solis writes the `solis` files again when the project opens, so they always match the engine. **Fix:** put your own code in your own files.
- **Problem:** my component isn't in **Add component**. **Why:** the code has a mistake (see Problems), or the file it's in isn't fetched from `main.lst` (directly, or through a file `main.lst` fetches). **Fix:** fix the mistake, or fetch the file.
- **Problem:** a component is called the same as one of Solis's (`Light`). **Why:** two components can't share a name. **Fix:** rename yours; Problems says which.
- **Problem:** the Console says a code file was left out. **Why:** each code file can be at most 1 MB, and all of them together 16 MB. Files that aren't text are left out too. **Fix:** split a huge file into smaller ones.
- Commit the `solis` folder with your game, so `lust check` works on other computers too.

## Related
- [Problems](problems.md)
- [Projects and the Library](projects-and-the-library.md)
- [Entities and components](entities.md)
- [Play mode](play-mode.md)
- Concepts: [game code, Lust, native method, tick](concepts.md#game-code)
- The Lust language itself: [Lust's guide](https://github.com/Rhyslos/Lust/blob/main/LUST.md)
