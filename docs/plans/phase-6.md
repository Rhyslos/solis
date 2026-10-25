# Phase 6 plan: Simulation and play

The working plan for Phase 6 (ROADMAP.md, "Phase 6: Simulation and play"). It records the choices made before any code and a plan for every milestone, so a later session can carry on where this one stopped. Update it as milestones are built: tick them off here and in the README tracker, and change a milestone's plan if its design changes.

**Gate:** walk a character around a level with physics, save, quit Solis, open it again and load. The same recorded inputs always give exactly the same result.

## Choices made before any code

- **Physics is Rapier, strict on every PC** ([decision 0036](../decisions/0036-physics-is-rapier-strict-on-every-pc.md), chosen by the project lead 2026-10-25). Rapier (`rapier3d`, Apache-2.0) in its `enhanced-determinism` mode: the same inputs give exactly the same result on every PC, so recordings replay anywhere and lockstep play works in Phase 8. It costs physics some speed, which doesn't matter at Solis's scale (characters, vehicles, a few hundred bodies). It is built into Solis, so it is one choice for every game. Writing physics from scratch was weighed and isn't worth it: years of hard, solved problems.
- **The game's controls** ([decision 0037](../decisions/0037-input-as-named-actions.md), chosen by the project lead 2026-10-25). Game code reads named actions, never raw keys. Ready-made actions and their default bindings:

  | Action | Keyboard and mouse | Gamepad |
  |---|---|---|
  | Move | W A S D | left stick |
  | Look | mouse | right stick |
  | Jump | Space | A |
  | Sprint | Shift | left stick click |
  | Interact | F | X |

  Clicking the Game panel gives the game the mouse and keys (the pointer is hidden and held); Esc gives them back. While the game has them, the editor's own keys (W, R, S, Delete, Ctrl+Z, …) are off. Players can rebind.
- **A save holds everything** ([decision 0038](../decisions/0038-save-games-hold-everything.md), chosen by the project lead 2026-10-25): every entity, every component's fields, the projectiles in flight, the physics, the clock and the randomness. Loading puts the game back exactly as it was; game code only calls `saveGame("slot")` and `loadGame("slot")`.
- **Three cameras now, templates later** (chosen by the project lead 2026-10-25): the ready-made camera controllers are RTS (top-down, pan and zoom), first person and third person. New-project templates (RTS, first person, third person), each starting with its camera, stay in Phase 11. The RTS camera's controls are asked about before it's built.

## Where things stand

- **A fixed step exists** (Phase 2): the world moves on in ticks of 1/60 s, at most 8 a frame, whatever the frame rate. But drawing shows the last tick as it is, so on a 144 Hz screen things move in steps; and a paused game can't be stepped on by one tick.
- **Game code runs the same on any number of cores** (decision 0033), which determinism builds on. But Luau's `math.random` is seeded by the time, so game code can't have randomness that repeats.
- **No input, no physics, no saving** reach game code yet. Areas are saved as text files (decision 0020), which a save game can reuse.

## Determinism rules

What the same inputs giving the same result needs, everywhere:

- **One order:** everything that adds results up does it in a fixed order (decision 0031, decision 0033); nothing depends on a hash map's order, the time, or the number of cores.
- **Randomness from a seed, counted, not drawn in turn:** each random number is made from the game's seed, the tick, and the place in the tick's order it was asked for (like spawned IDs, decision 0035), so it doesn't matter which core asks first. Luau's own `math.random` is replaced by this.
- **Physics strict** (decision 0036), stepped once per tick, after game code's changes land.
- **Input sampled once per tick:** every core sees the same actions for a tick; a recording is those samples.
- **A checksum of the game state** each tick (places, component values, physics, projectiles, randomness), so a difference is found the tick it starts, not a minute later.
- Floats in game code are allowed: on Windows only (decision 0012) with one runtime, Luau's maths gives the same results. Lua code that reaches for the time (`os.clock`) can't: it isn't in the sandbox.

## Safety (CONTRIBUTING rules 8 to 10)

- **Limits:** the most physics bodies (100,000) and colliders, with a message past them; a collider made from a model is limited in triangles; the physics step has a time limit like game code's tick, past which the game pauses and says why.
- **Files read get fuzz tests:** save games, recordings and input bindings are read within limits and never crash on damage; a save from another project or a newer Solis is refused with a message.
- **No panics reach the frame:** Rapier is given only finite numbers (a NaN from game code is refused at the native, as `moveTo` does).

## The shape of the code

| Crate | Layer | What it gains | Milestone |
|---|---|---|---|
| `world` | 1 | The clock's progress through a tick, for drawing between ticks; stepping one tick; a checksum; the physics world | M1, M2 |
| `physics` (new) | 1 | Rapier wrapped: bodies and colliders from components, stepping, rays, the character mover, body states for saves | M2, M5 |
| `script` | 1 | Natives: randomness, physics, input, save and load; the replaced `math.random` | M2–M5 |
| `area` | 0 | Built-in components: `RigidBody`, `Collider`, `CharacterBody` | M2, M5 |
| `editor` | 2 | Drawing between ticks; Step; input capture in the Game panel; recording and replaying; the Input actions window; saves | M1–M4 |
| `app` | 3 | Gamepads (`gilrs`); the pointer held while the game has it; Help > Make the playground… | M3, M5 |

## Milestones

### M1. A fixed-timestep simulation ☑

- **Drawing between ticks:** the world remembers where each drawn entity was at the last tick as well as now, and the Game panel draws them part way between, by how far the clock is into the next tick. So a 144 Hz screen shows smooth movement while the simulation stays at 60 ticks a second. Projectiles too. The editor's viewport (not playing) is unaffected.
- **Step one tick:** while paused, a Step button beside Pause runs exactly one tick (no new keys).
- **The tick's order, written down** (ARCHITECTURE.md): land what's waiting, move projectiles, game code's updates and events, land its changes, physics (M2), land spawns.
- Game time and real time stay apart: `time()` is game time, `frameTime()` real time.
- Tests: drawing between ticks puts an entity part way; stepping runs one tick and no more; a stall still skips ahead; a game run at 30, 60 or 144 frames a second gives the same world at the same tick.
- **Built.** The world keeps each entity's place as the tick started (`World::remember`, before every tick); `placed_between` and `camera_between` draw part way by `progress`, moving each number of the matrix in a straight line (no decomposing: cheap at 10,000 units, and a sixtieth of a second's turn doesn't visibly bend). Projectiles are drawn back along their velocity by the same share. Paused, things are drawn exactly where they are. The Game panel's picture is made again every frame while playing (the share moves), and only on changes while paused. Step is a button beside Pause and `step` in the Console; no keys were added.

### M2. Determinism: seeded randomness, checksums and physics ☑

- **Randomness:** `random()` (0 to 1) and `randomRange(low, high)` natives, made as described above, from a seed set per area (a field on the area's settings, or chosen at Play when 0) and `setSeed(n)`; Lua's `math.random` gives the same numbers.
- **Checksums:** `World::checksum()` each tick, shown in the Profiler, kept in recordings; tests compare runs by it.
- **Physics with Rapier** (decision 0036) in a new `physics` crate: built-in components `RigidBody` (dynamic, kinematic or fixed; mass; damping; gravity on or off) and `Collider` (box, sphere, capsule, or the model's own shape: convex or exact triangles for fixed things; friction; bounciness). Bodies are made when Play is pressed and when entities spawn, and stepped once per tick. Natives: `push(entity, force)`, `velocityOf`, `setVelocity`, `castRay(from, direction, reach)` (what it hits and where).
- Physics can be seen while editing: colliders drawn as outlines on the selected entity.
- Tests: the same game twice gives the same checksums tick by tick; on 1 core and 4; a pile of boxes settles the same every run; physics' limits and a NaN from game code. Scale case: 1,000 bodies falling into a pile.
- **Built.** Randomness: `random`, `randomRange`, `randomInt`, `setSeed`, and Lua's `math.random`; the seed is new each Play unless fixed with `seed` in the Console (no seed field on the area: a fixed one belongs to testing, and game code can `setSeed`). Checksums: `World::checksum` and `Game::checksum` (`reflect::Checksum`, pinned by a test so saved checksums stay comparable), and `checksum` in the Console (the Profiler doesn't show it: a number that changes every tick says nothing at a glance). Physics: the `physics` crate (Rapier 0.36, `enhanced-determinism`), `RigidBody` and `Collider` components, `push`, `velocityOf`, `setVelocity`, `castRay`, `castRayHit`, `castRayNormal` (three natives because Lust gives back one value), selected colliders outlined green, a 250 ms physics limit per tick. A ray passes out of colliders it starts in. 1,000 crates falling into a pile take 1.7 ms a tick on average, 3.75 ms at worst (release, 2-core machine). **Left for later:** colliders from a model's own shape (convex or exact triangles) need the model's vertices in the game's world; they come with the character controller (M5) if the playground needs them, otherwise as an issue. Lust's checker flagging code that could break determinism waits for the Lust repo; in Solis, the sandbox already has no clock, and Lua's random follows the seed.

### M3. Input as named actions, with gamepads and rebinding ☑

- **Actions** are kept in the project (`input.solinput`, a Lust-style text file like areas, decision 0020), starting with the five above. Each has its keyboard, mouse and gamepad bindings; Move and Look are two-way axes.
- **Project > Input actions…** edits them: add an action, choose its bindings by pressing the key or button.
- **Natives:** `held(action)`, `pressed(action)` (this tick), `released(action)`, `axis(action)` (a vec3, x and y used). Actions are sampled once per tick, before game code.
- **The Game panel takes the input** when clicked (pointer hidden and held), Esc gives it back; editor keys are off meanwhile.
- **Gamepads** with `gilrs` (Apache-2.0 or MIT), the first one connected.
- **Players rebind** through natives (`rebind(action, …)`), kept in their own settings; a rebinding menu needs game UI (Phase 9).
- **Recording and replaying:** Play > Record inputs keeps each tick's actions (`.solrec`, with each tick's checksum); Play > Replay plays them back and says the tick where a checksum first differs, if any.
- Tests: bindings read and written round trip; damaged files are refused (fuzz); a replay matches its recording's checksums; editor keys don't fire while the game has the input.
- **Built.** A new `input` crate (layer 1): `Bindings` (TOML, `input.solinput`, read within limits), `Devices` sampled into one `Frame` per tick (a tap between ticks is still a press), and `Recording` (text, numbers by their exact bits). Natives `held`, `pressed`, `released`, `axis`, `bindingOf`, `rebind` in `solis/input`. Look's axis is how far to turn this tick (mouse pixels × mouse speed, sticks and keys × their speed × the tick), so mouse and stick feel alike to game code; Move's is which way, each from -1 to 1. Edit > Input actions… binds by pressing; the Play menu (new) holds the play buttons, Play and record inputs (recordings in `recordings/`, needing a project) and Replay a recording…. Gamepads come from gilrs on Windows only. Building the replay found that the world's checksum included the clock, which during a frame of several ticks has moved on by all of them: the clock left the checksum, and the replay test plays back in different frame sizes from the recording. Rebinding lasts for the play; players' own saved bindings wait for exported games (Phase 10).

### M4. Save games ☑

- `save(slot)` and `load(slot)` natives (decision 0038), landing at the end of the tick. A save holds the area as it stands (in the area file's text form), every running component's fields, projectiles, the physics state, the clock, the seed and the input's state.
- In the editor, saves go in the project's `saves/` folder (outside `assets`, ignored by Git); exported games will use the player's saved-games folder (Phase 10). Play > Load save… loads one by hand.
- Loading checks the save is from this project and a Solis that can read it, and says why if not.
- Tests: save, change things, load: the same checksum as at the save; save, quit (a new workbench), open, load: the same; damaged saves refused (fuzz); a save during a tick with spawns waiting.
- **Built.** `saveGame(slot)` and `loadGame(slot)` (`save` and `load` are Lust's own words). A save is a text file in sections (`editor::save_game::Save`) with a checksum: the area as it stands with each entity where the game has it, every running component's values apart (the world keeps the values components started from, so the checksum carries on the same), the input actions with game code's rebinding, the projectiles by their exact bits, and physics as bytes (`Physics::snapshot`, postcard, base64). Loading rebuilds the world, restores physics exactly, starts game code and syncs the saved values in; the editor shows the saved area without undo steps and Stop still goes back to before Play. Play > Load a save… loads by hand; new projects keep `saves/` out of Git. Tests: a loaded game's checksums match the saved game's tick by tick, in the same play and after quitting; game code's save and load; another project's save and damaged saves refused, with nothing changed. Building it found that a frame stopping part way (a pause, a load) left the clock counting ticks that never ran; the clock now moves on as each tick begins (`World::due`, `World::begin_tick`). A physics step that panics on a tampered save empties physics and pauses the game rather than crashing. The open-world follow-up (only what changed per region) is noted in decision 0038.

### M5. Ready-made components in Lust: character and camera controllers ☑

- **Character controller** (`solis/kit/character.lst`): walks with Move, looks with Look, jumps, sprints, climbs steps and slopes, slides along walls, stands on moving platforms; built on a `CharacterBody` component and a `moveCharacter(entity, wanted)` native using Rapier's character mover.
- **Camera controllers:** third person (orbits behind, pulls in when a wall is in the way), first person (from the eyes), and RTS (top-down).
- They're written in Lust, as examples for game makers to read and copy; the manual walks through them.
- **Help > Make the playground…** makes a project for the gate: a level with ramps, steps, crates to push and a moving platform, a character with the third-person camera, and save and load actions.
- **Controls, chosen by the project lead (2026-10-25, [decision 0041](../decisions/0041-camera-controls.md)):** RTS: "arrowkeys to move or holding down middle mouse to pan, rotation can be alt + arrow keys, and zoom is scroll wheel and pg up and pg down keys". Third person: "Fixed distance by default, but a toggle to wheel zoom should be in the attributes panel" (Properties). Playground: "F6 to save, F7 to load". The gamepad gets left stick to pan, right stick to turn, triggers to zoom.
- Tests: the character climbs a step, stops at a wall, lands after a jump; the cameras keep out of walls; the playground plays the same from a recording.
- **Built.** `CharacterBody` (area) becomes a kinematic capsule (`physics::Physics::add_character`), moved by `moveCharacter` at the physics step with Rapier's `KinematicCharacterController` (autostep, slopes, sliding, pushing dynamic bodies, carried by kinematic ones); `onGround` reads whether it stood on something, which is saved and checksummed. Building it found that Rapier's snapping to the ground sinks a resting character a little each tick, and that it only gets out of things it starts inside when standing still: Solis snaps it itself and gets it out first. `facingAlong(direction)` gives an upright rotation (Lust's `Maths` has no `atan2`). The kit (`script::describe::KIT`) is written into every project's `solis/kit/` with the descriptions: `CharacterController`, `ThirdPersonCamera` (fixed distance, `wheelZooms`), `FirstPersonCamera` and `RtsCamera`. Building the controller found that a camera's yaw can't be read with `:degrees()` when it looks level (its X tilt is 90°, where the angles fold together), so it works out the way the camera looks instead. Input gained combinations (`Alt+ArrowLeft`, shadowing the key alone) and the wheel (`Mouse Wheel`, `Mouse WheelUp`, `Mouse WheelDown`); Pan, Turn, Zoom and Drag joined the ready-made actions, and the Input actions window adds them to older projects. Help > Make the playground… (`app::playground`) makes the level in code with its models (`assets::sample::playground_glb`). Tests: the character walks the camera's way, sprints, jumps 1.2 m and lands, climbs a step, stops at a wall and pushes a crate; the cameras keep out of walls, zoom only when allowed, and the RTS camera answers its controls; a recorded walk replays exactly; the playground is made and plays.

### End of the phase

- The review (decision 0027): three reviewers, the fixes in pull requests by kind, the scale check with all three levels.
- The gate, tried by the project lead on their PC.
- **The review, done (2026-10-25).** Three reviewers (stability, efficiency, where parts meet) read Phase 6's code; every finding was checked, then fixed in three pull requests:
  - **Stability:** a damaged or hand-made save could give Rapier nonsense that froze or closed Solis, so saves now keep each body's place and motion as text and build physics afresh from them (`World::rebuild_physics`; saving does the same to the running game, so both carry on alike); repeated saves in a tick collapse to one per slot (at most four); saves and recordings are capped at what can be read back; a damaged `input.solinput` is backed up before being overwritten; `rebind` keeps within its limits; restored projectiles keep `fire`'s limits.
  - **Where parts meet:** colliders, kinematic bodies and characters under a moving entity follow it; loading runs game code's starts on a thrown-away world; edits keep bodies' spin, sleep and characters' ground state; input actions changed while playing reach the game; input during a pause or replay is forgotten; `setSeed` holds from the next tick; saves on a pausing tick are made.
  - **Efficiency:** characters' moves are worked out on every core at once and made in order (`Physics::move_characters`); the world's checksum works out what only changes with the revision once; a component's own moves, turns and velocities are found by entity, not by searching everything it changed; an edit rebuilds a body only when its place or physics changed. New scale case: 500 characters walking (and 2,000, reported).
  - **Second round (2026-10-25)**, asked for by the project lead, with two security audits (one of Phase 6, one of the whole app and its libraries): security (#81: game code's text always loaded as text, never as bytecode; no `require`; the Library opens only pictures and Lust; writes never through a planted link; save slot limits), stability (#82), where parts meet (#83: a mistake of the first round, resting bodies kept asleep after an edit, fixed; dynamic children follow edits; saves hold this tick's spawns; recordings keep Play's seed), efficiency (#84: characters don't press into the floor while standing; physics built once on rebuilds and loads; one save text per tick; `moved()` from Rapier's awake list). Left for the project lead: updating mlua 0.10 / Luau 0.663 to current (0.12 / 0.744), its own pull request.
  - **Left for later (issues):** `frameTime()` isn't in recordings, so games that use it don't replay exactly (the mismatch message names it); colliders under a dynamic body follow it but bump into it (a rule to join them into one body); recordings keep each tick's actions by name (about 140 MB at the hour's limit; by number would be far smaller); game code's checksum while recording builds every component's values (unmeasured); writing big saves and recordings happens on the frame (a worker later).
