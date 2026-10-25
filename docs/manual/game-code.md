# Game code

> Your game's own rules (how a turret aims, when a door opens) are written as code. Solis checks it as you save and shows any mistake with its file and line.

## When to use it
Whenever your game needs to *do* something: react to the player, count score, move things on its own. Areas and assemblies say what's in the world; [game code](concepts.md#game-code) says what happens in it.

Solis games can be written in three languages:
- **[Lust](concepts.md#game-code)**, made for Solis's kind of games and the one this manual teaches first;
- **Lua**, if you know it from Roblox or modding;
- **Rust**, the language Solis itself is written in.

**Right now:** Solis reads and checks your Lust, and points at its mistakes. Running Lust components in play mode, and Lua components, come with the next updates of Phase 4.

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
- **Problem:** the Console says a code file was left out. **Why:** each code file can be at most 1 MB, and all of them together 16 MB. Files that aren't text are left out too. **Fix:** split a huge file into smaller ones.
- Commit the `solis` folder with your game, so `lust check` works on other computers too.

## Related
- [Problems](problems.md)
- [Projects and the Library](projects-and-the-library.md)
- [Entities and components](entities.md)
- Concepts: [game code, Lust, native method](concepts.md#game-code)
- The Lust language itself: [Lust's guide](https://github.com/Rhyslos/Lust/blob/main/LUST.md)
