# Game code

> Your game's own rules (how a turret aims, when a door opens) are written as code, as components you put on entities. Solis checks the code as you save, shows its fields in Properties, and runs it when you press Play.

## When to use it
Whenever your game needs to *do* something: react to the player, count score, move things on its own. Areas and assemblies say what's in the world; [game code](concepts.md#game-code) says what happens in it.

Solis games can be written in three languages:
- **[Lust](concepts.md#game-code)**, made for Solis's kind of games and the one this manual teaches first;
- **Lua**, if you know it from Roblox or modding;
- **Rust**, the language Solis itself is written in.

**Right now:** Lust and Lua components work: they appear in Properties, run in play mode, talk to each other through events, and take new code while the game runs. Rust components can join them, but for now only in a Solis built from source.

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

The same component in **Lua**, in any `.lua` file in `assets` (here `mover.lua`):

```lua
local Mover = component("Mover", {
    { "speed", "float", 2 },
})

function Mover:update(dt)
    solis.moveTo(self.me, solis.positionOf(self.me) + vec3(1, 0, 0) * self.speed * dt)
end
```

`component` takes the name and the fields in order, each as `{ name, kind, starting value }` (a `state` also lists its options: `{ "mode", "state", "Single", { "Single", "Burst" } }`). The fields are `self.speed` and so on, the entity is `self.me`, the engine's methods are under `solis.`, and `vec3(x, y, z)` makes a vector (a Luau vector, the same kind of value a Lust `vec3` is) you can add, take away and scale, with `v.x` reading a part. A Lua component works the same way as a Lust one from here on, and both can be in the same game.

1. Save the file. When the code has no mistakes, **Mover** appears in Properties' **Add component** menu.
2. Select an entity and add **Mover**. Its `has` fields show in Properties with their starting values (`speed` as **Speed**), and are saved in the area like any other component's.
3. Press **Play**. Every [tick](concepts.md#game-code) (60 times a second), Solis runs each component's `update`, with `dt`, how long a tick is in seconds. `me` is the entity the component is on. The Game panel shows the entity moving.
4. Change **Speed** in Properties while it plays: the running Mover takes the new speed at once, and keeps everything else it has (here, where it is). Properties shows the values the entity started with, not the ones the running game has changed since: a turret that has fired still shows its full ammo there.
5. Press **Stop**. Everything is back as it was before Play.

What a field can hold: `bool`, `int`, `float`, `string`, a `state` (shown as a choice of its options), `vec3`, `rotation` and `entity` (another entity, or `nothing`).

**Order:** each tick runs entities in the order of their IDs, and an entity's components in the order of their names, so a game runs the same way every time.

**What game code sees during a tick:** the world as it was when the tick started, plus its own changes. Solis runs game code on every core of your processor at once, so components can't see each other's moves while they happen. Instead:

- `positionOf`, `rotationOf`, `nameOf` and `find` read the world as it was at the start of the tick. A component that moves its own entity sees the new place at once; another component sees it next tick.
- `moveTo`, `turnTo` and `say` are kept, and happen when every `update` is done, in the order above. If two components move the same entity, the last one in that order wins.
- Events are delivered after that, a round at a time; each round sees the world as the round before left it.

So a game runs the same way on a computer with 2 cores or 32. Most code never notices: a unit chasing another simply aims at where the other was a sixtieth of a second ago. Code that needs to see a move straight away should move things from one component, or tell the other with an event.

**When something goes wrong while it runs,** such as dividing by zero, that component stops and the Console says which one, on which entity, and where in the Lust: "Divider on Splitter stopped: main.lst, line 8: …". The rest of the game keeps going. A component that runs for more than a tenth of a second without stopping (a loop that never ends) is stopped the same way, so it can't freeze Solis.

**When the whole tick is too slow:** if the game's code takes more than a quarter of a second in one tick on any one core (many slow components, say), the rest of that tick waits and the game **pauses**, with a message in the Console saying why. None of the components is to blame on its own, so none stops; make the slow ones faster, then press **Pause** or **Play** to carry on. Without this, the game would try to catch up and the editor would hardly respond.

**What game code says** goes to the Console a line at a time. A very long line is cut short, and past 200 lines in one tick the rest are counted ("Game code said 800 more lines just now"), so a component that talks too much can't drown the Console or fill the computer's memory.

## Changing code while the game runs

Keep the game playing, change your Lust, and save. In about a third of a second the running game has the new code, and nothing starts over:

- **Every running component keeps its values,** field by field, by name: a turret keeps the ammo it has left, a mover stays where it got to. Only `update` and the `on` reactions are new.
- **A field whose kind changed** (an `int` that's now a `float`), **or a new field,** takes its starting value. The Console says which: "Gun's ammo is float now, not int, so it starts at 10."
- **A removed field** is dropped; **a removed component** stops.
- **A component new in the code** starts on every entity that has it.
- **A component that stopped** after a failure runs again, since its code changed.
- **A mistake in the new code** leaves the game running the last code that worked, and the mistake shows in Problems. Fix it and save again.

The Console says what changed each time: "The running game took the new code (Gun changed; Radar new); components keep their values."

## Events

Components tell each other things with [events](concepts.md#game-code): a gun tells a target it was hit, the target's health tells its entity it died. An event says what it carries. A component reacts to it with `on`, and anything sends it to an entity with `send`:

```
get "solis/world":[say];

event Hit(entity by, float damage);
event Died();

component Health {
    has {
        float hp = 10;
    }

    on Hit(entity by, float damage) {
        is hp > 0? then {
            hp = hp - damage;
            is hp <= 0? then {
                send Died() to me;
            }
        }
    }

    on Died() {
        say("down!");
    }
}

component Gun {
    has {
        entity target = nothing;
    }

    method update(float dt) {
        send Hit(me, 2.5) to target;
    }
}

pass Health, Gun;
end
```

- **When they arrive:** after every component's `update` in the tick (and after the moves those made), in the order they were sent (by the sending entity's ID, then component name). On the entity, each component with an `on` for it reacts, in the order of the components' names. An event sent while reacting to another (Hit sends Died) arrives later in the same tick.
- **An event to `nothing`** goes nowhere, so a gun with no target is harmless.
- **Lust checks** that every `send` carries what the event says, so a misspelt event or a missing value is a mistake in Problems before anything runs.
- **Components that keep sending to each other** forever are stopped after 10,000 events in one tick, with a message in the Console (once, until the code changes), so the game can't freeze. At most 10,000 events can wait at once, holding at most 16 MB of text: past that, the `send` itself fails, so the Console points at the component sending too much.
- **A component whose reaction keeps failing** (an `on` that divides by zero, every time) is said once in the Console; it keeps running, and its later failures with events aren't said again until the code changes.

**In Lua**, `solis.send(to, "Hit", by, 2.5)` sends an event, and a function called `on_` and the event's name reacts to it:

```lua
function Health:on_Hit(by, damage)
    self.hp = self.hp - damage
end
```

When the Lust declares the event, what Lua sends is checked against what it carries: a wrong value means the event isn't sent, and the Console names the Lua component that sent it. Lua can also send events no Lust declares; their numbers arrive as floats (Lua has one kind of number, so `2` and `2.5` arrive alike).

A Lua component's fields can't be called `me` (the entity it's on), `update`, or anything starting with `on_` or `__`: those are its methods, and Lua's own.

**Each Lua file has globals of its own.** A helper a file makes global (`function helper() … end`) is seen by that file only, and no file can change Lua's own libraries (`string`, `table`, `math`) or `solis`, `component` and `vec3`: trying to is a mistake. That way one file can't break another, or the Lust. To share code between Lua files, put it in a component's events, or in Lust.

**In Rust**, a component implements `script::RustComponent`, and takes and sends the same events by name. Solis can't build a game's own Rust until Phase 10; until then, the [turret sample](first-game.md)'s Rust component is built into Solis to show how it works.

```rust
fn handle(&mut self, cx: &mut Context<'_>, event: &Event) -> bool {
    if event.name != "Hit" {
        return false;
    }
    self.dents += 1;
    let me = cx.me();
    cx.send(Some(me), "Dented", vec![Value::Int(self.dents)]);
    true
}
```

Other code files can sit in any folder in `assets`. Fetch them by their path from `assets`, such as `fetch "weapons/gun":Gun;` for `assets/weapons/gun.lst`. The Library's **Code** tab lists them all.

## Projectiles

Lasers, bolts and tracer rounds come in their thousands, so they aren't entities: they're [projectiles](concepts.md#game-code), which Solis moves itself, every tick, on every core.

```
get "solis/world":[fire];
get "solis/entity":[positionOf];

component Turret {
    has {
        int cooldown = 0;
    }

    method update(float dt) {
        cooldown = cooldown - 1;
        is cooldown <= 0? then {
            fire(positionOf(me), [0, 1, 0], 60, 2, [4, 0.3, 0.2]);
            cooldown = 30;
        }
    }
}

pass Turret;
end
```

`fire` takes where it starts, which way it flies (any length), its speed in metres a second, how long it lives in seconds, and its colour as red, green and blue. Above 1 glows brighter: `[4, 0.3, 0.2]` is a hot red laser. In Lua it's `solis.fire(from, direction, speed, life, colour)`, with `vec3(…)` for each.

- It flies in a straight line and disappears when its time is up. It doesn't hit anything yet: that comes with physics (Phase 6).
- It's drawn in the **Game** panel as a glowing streak, as long as it flies in two frames. Streaks add their light where they cross.
- Up to 1,000,000 can fly at once. Past that, new shots aren't fired until some run out, and the Console says so once.
- Like a move, a shot lands after every `update` in the tick is done ([what game code sees](#components)), so it starts flying next tick.

## Making and removing entities

Game code can make copies of [assemblies](assemblies.md) while it plays, and take entities away:

```
get "solis/world":[spawn, despawn];

component Hangar {
    has {
        int made = 0;
    }

    method update(float dt) {
        is made < 10? then {
            entity ship = spawn("ships/fighter.solassembly", [made * 5, 0, 10]);
            made = made + 1;
        }
    }
}

pass Hangar;
end
```

- **Which assembly:** its file in the project's assets (`ships/fighter.solassembly`), that without the extension (`ships/fighter`), or just its name (`fighter`) when only one assembly has it. A name that isn't an assembly makes nothing, and the Console says so once.
- **When:** like a move, it happens after every `update` in the tick ([what game code sees](#components)). `spawn` gives back the new entity at once, so you can keep it, but it's in the game, with its components running, from the next frame.
- **Where it shows:** copies game code makes appear in the Area panel while playing, and are gone when you press **Stop**, like everything else changed while playing. They aren't undo steps.
- `despawn(me)` is how a component takes its own entity away. It ends at the end of the tick.

## What the engine offers

| In | Method | What it does |
|---|---|---|
| `solis/world` | `say(string words)` | Writes words in the Console. |
| `solis/world` | `float time()` | How long the game has been running, in seconds. |
| `solis/world` | `entity find(string name)` | The first entity with this name, in the Area panel's order, or `nothing`. |
| `solis/world` | `fire(vec3 from, vec3 direction, float speed, float life, vec3 colour)` | Fires a projectile (see Projectiles, below). |
| `solis/world` | `entity spawn(string file, vec3 place)` | Makes a copy of an assembly at a place (see Making and removing entities, below). |
| `solis/world` | `despawn(entity which)` | Takes an entity, and everything under it, out of the game. |
| `solis/world` | `float frameTime()` | How long the last frame took, in seconds of real time (1/60 at 60 frames a second). |
| `solis/entity` | `vec3 positionOf(entity which)` | Where an entity is in the world, in metres. |
| `solis/entity` | `moveTo(entity which, vec3 place)` | Puts an entity at a place in the world. |
| `solis/entity` | `rotation rotationOf(entity which)` | Which way an entity faces. |
| `solis/entity` | `turnTo(entity which, rotation facing)` | Turns an entity to face this way. |
| `solis/entity` | `string nameOf(entity which)` | An entity's name. |

## Tips and common problems
- **Problem:** Problems says the game's code starts at `main.lst` and there isn't one. **Why:** there are `.lst` files, but none is called `main.lst` in the `assets` folder itself. **Fix:** make `assets/main.lst`, and fetch your other files from it.
- **Problem:** changes I made to `solis/world.lst` disappeared. **Why:** Solis writes the `solis` files again when the project opens, so they always match the engine. **Fix:** put your own code in your own files.
- **Problem:** my component isn't in **Add component**. **Why:** the code has a mistake (see Problems), or its Lust file isn't fetched from `main.lst` (directly, or through a file `main.lst` fetches). Lua files are all read, wherever they are in `assets`. **Fix:** fix the mistake, or fetch the file.
- **Problem:** "solis.moveTo isn't something Solis offers" in Lua. **Why:** the name is misspelt, or the method was used outside `update` or an `on_` function, when the game isn't running. **Fix:** check the name against the table above.
- **Problem:** a component is called the same as one of Solis's (`Light`) or a Rust one, or a Lust and a Lua component share a name. **Why:** two components can't share a name. The one in your code doesn't run (Solis's own, or the Rust one, does). **Fix:** rename yours; Problems says where it is.
- **Problem:** I saved new code while playing, and the Console says the running game couldn't take it. **Why:** the code's start failed with the game's world (it ran fine with no world while checking). **Fix:** fix the line it points at; the game keeps the last code meanwhile.
- **Problem:** Problems says something is nested too deeply. **Why:** more than 150 levels of brackets, blocks or operators inside each other (`a + b + c …` counts each `+`) is more than Lust takes. **Fix:** split it up with variables or methods.
- **Problem:** Problems says the code stopped while starting. **Why:** Solis runs the code's own start (the lines outside components, and each field's starting value) to read its components, before anything plays and with no world: `find` finds nothing then. A start that fails or takes more than a second is a mistake. **Fix:** fix the line Problems points at.
- **Problem:** the Console says a code file was left out. **Why:** each code file can be at most 1 MB, and all of them together 16 MB. Files that aren't text are left out too. **Fix:** split a huge file into smaller ones.
- **Problem:** a component reads another entity's place and gets where it was before this tick's move. **Why:** during a tick, game code sees the world as it was when the tick started (see Components, above), so every core sees the same world. **Fix:** usually nothing: it catches up next tick. If it must be at once, do both in one component, or send an event (events see the moves made before them).
- Commit the `solis` folder with your game, so `lust check` works on other computers too.

## Related
- [Your first game: the turret sample](first-game.md)
- [Problems](problems.md)
- [Projects and the Library](projects-and-the-library.md)
- [Entities and components](entities.md)
- [Play mode](play-mode.md)
- Concepts: [game code, Lust, native method, tick, event](concepts.md#game-code)
- The Lust language itself: [Lust's guide](https://github.com/Rhyslos/Lust/blob/main/LUST.md)
