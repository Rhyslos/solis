# Phase 4 plan: Lust and Lua join

The working plan for Phase 4 (ROADMAP.md, "Phase 4: Lust and Lua join"). It records the choices made before any code and a plan for every milestone, so a later session can carry on where this one stopped. Update it as milestones are built: tick them off here and in the README tracker, and change a milestone's plan if its design changes.

**Gate (unchanged):** change a Lust component while the game runs and see the change within **2 seconds**; a Lua component changes instantly. A mistake in Lust shows a friendly message, never a Rust error. One entity can carry a Rust, a Lust and a Lua component that talk to each other through events.

## Choices made before any code

- **How changes reach play mode** (the roadmap's "decide before starting"): while editing, Lust runs as Luau and is swapped into the running game, keeping values; built games run Lust compiled to Rust ([decision 0028](../decisions/0028-lust-runs-as-luau-while-editing.md), chosen by the project lead 2026-10-24).
- **Lust is ready** (Lust's steps 1–5, merged as Lust pull requests #62–#66, version 2.0.0-alpha.32). Solis uses its compiler as a library, pinned to the merge commit until the tag exists. What Solis relies on is in LUST.md §21:
  - `lust::check` and `lust::to_lua` take the files' paths and text (Lust opens no files) and give back every mistake with file, line and column;
  - the Lua chunk gives `components`, `events`, `new`, `swap` and `changes`;
  - `chunk.locate` points a failure at the Lust;
  - native methods' bodies, and `send_event`, come from the table `LUST_NATIVES`.
- **Lust's own engine test** (`engine-test/` in the Lust repo) does all of this in Luau through mlua, the way Solis does. Solis's runtime follows its pattern.

## Where game code lives

- **Anywhere in the project's `assets` folder,** as the Library's Code tab already lists it. Lust looks for `main.lst` as the program's first file, so a game's Lust starts at `assets/main.lst`. Files fetch each other by their path inside `assets` (`fetch "weapons/gun":Gun;`).
- **The engine described to Lust** (decision 0016): Solis writes `assets/solis/world.lst` and `assets/solis/entity.lst` when a project opens, generated from the list of native methods in the `script` crate, so they always match the Solis that wrote them. Each starts with a line saying Solis made it and changes to it are undone. They're committed with the game, so `lust check` works outside Solis too.
- **Lua components** (M5) are `.lua` files in `assets`, each describing components the same way the Lust chunk does.

## Safety (CONTRIBUTING rules 8 to 10)

- **Reading the code:** every `.lst` file under `assets`, skipping hidden folders. At most 1 MB a file, 16 MB and 10,000 files in all. Read on a worker, never on the frame.
- **Compiling** (`lust::check`, `lust::to_lua`) runs on a worker too. A panic in the compiler is caught and reported as a problem, never a crash.
- **Running:** Luau has no file, process or network access. The runtime sets a memory limit (64 MB). An interrupt stops code that runs longer than 100 ms in one go, with a message ("Gun's update took too long and was stopped"), so a loop that never ends can't freeze the editor.
- **A failing component** (a runtime error) is reported once in the Console with its file and line, and stops running until the code changes; the rest of the game keeps going.

## The shape of the code

| Crate | Layer | What it gains | Milestone |
|---|---|---|---|
| `script` (new) | 1 | Gathering a project's code, the description files, checking and compiling on a worker's behalf, the Luau runtime: components, ticks, natives, events, swaps | M1–M5 |
| `editor` | 2 | Code problems in the Problems panel; script components in the registry, the inspector and Add component; running them in play mode; taking in new code while playing | M1–M5 |
| `app` | 3 | Reading and compiling the code on a worker when a project opens and when a code file changes | M1, M4 |
| `world` | 1 | Native methods' reach into the game: positions, rotations, names, finding entities | M2 |

## Milestones

### M1. Lust builds into Solis ☑

- The `script` crate, with `lust` and `mlua` (Luau).
- The description files: `solis/world.lst` (`say`, `time`, `find`) and `solis/entity.lst` (`position`, `moveTo`, `rotation`, `turnTo`, `name`), written when a project opens, unless already the same.
- Gathering the project's `.lst` files within limits, and checking them on a worker when a project opens and whenever a code file changes.
- **Problems:** each Lust mistake is a line in the Problems panel ("main.lst, line 12: ammo is an int, but this is text"). Clicking it opens the file in the code editor. The Console says how many there are after each check.
- No `main.lst` but other `.lst` files: one problem saying the game's code starts at `main.lst`.
- Tests: the description files check cleanly with Lust and fetch into a sample; mistakes come back with file, line and column; limits; a fuzz test of gathering damaged and huge files.

### M2. Lust components ☑

- Each Lust `component` becomes a component in the registry: its fields and starting values as a `reflect` schema (a state becomes a Choice). So it shows in Properties, in **Add component**, and is saved in area files like the built-in ones.
- **Play mode runs them:** each entity's Lust components are made (`new`) with the area's values, then run every tick (`update`), entities in ID order and components in name order. The native methods reach the game's world: positions, rotations, names, finding entities, and `say` to the Console.
- Changing a field in Properties while playing reaches the running component and keeps the rest (ammo left, cooldowns).
- Runtime failures are pointed at the Lust (`chunk.locate`).
- Tests: a component's fields round-trip between the area and Lua; ticks in order; natives move entities in the world; a failing component stops alone; the time limit stops an endless loop.

### M3. Events between components ☐

- `send` queues an event; Solis delivers the queue each tick, in order, to each component on the entity in order of their names. Events sent while delivering join the end of the same tick's queue (as agreed with the Lust chat).
- Rust components can send and handle events through a small trait, so a game's Rust components join in (the gate's three languages). A sample Rust component in the tests stands in for a game's.
- A runaway chain of events (one sending another forever) is cut off after 10,000 deliveries in a tick, with a message.
- Tests: delivery order; events sent while delivering; a Rust component and a Lust component talking both ways; the cut-off.

### M4. Fast changes ☐

- When a code file changes while playing, the code is checked and turned into Lua on a worker. Then each running component is swapped in place (`swap`), keeping its values by name. Notes on fields that started over go to the Console. Components whose fields changed are shown again in Properties (`changes`).
- A mistake in the new code leaves the running game on the last code that worked, with the mistakes in Problems.
- **Measured:** a Lust edit is running in play mode within 2 seconds (a scale-check entry times saving to running).
- Tests: a swap keeps values; a removed component stops; a mistake keeps the old code running.

### M5. Lua components ☐

- `.lua` files describe components with the same face as the Lust chunk: fields with starting values, `update`, event handlers. The same inspector fields, ticks, events and natives, through the same runtime.
- A Lua edit takes effect at once while playing, swapped the same way.
- Tests: a Lua component's fields in the registry; a Lua and a Lust component exchanging events.

### M6. A first game in Lust ☐

- A small sample project: a turret with ammo, cooldown and damage, shooting at targets with health. It's written in Lust, with one Lua and one Rust component talking to it through events. The later phases keep growing it.
- The first page of the Lust guide for Solis in the manual: writing a component, its fields in Properties, events, and changing code while the game runs. Every example is shown in Lust, Rust and Lua.
- Comparison tests (decision 0028): the sample's Lust is run both ways, in Luau and as Rust (`lust::to_rust_library`, built in a test), and the results compared tick by tick.
- The phase's code review (decision 0027) and scale check, before the gate.
