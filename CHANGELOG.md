# Changelog

What changed in each version, for people using Solis. Newest first.

## Unreleased

### Fixed

- **Game code runs on mlua 0.12** (the Lua engine's Rust side): it now gives every Luau a `require` that reads files from anywhere and a `loadstring`, and Solis takes both out, so game code is as sandboxed as before. Its Luau source is held at 0.740, as mlua 0.12.2 can't start with 0.744.
- **Ships game code made weren't always drawn:** the benchmark's stress area showed only its lasers, because nothing loaded the models of copies only game code places. Every model the project's assemblies use is now loaded with the area.
- **Updates never finished downloading:** every complete download was refused at its very end ("the response body is larger than request limit"), so installed copies couldn't update. Copies installed before this fix can't update themselves past it: run the newest `solis-setup.exe` from the Releases page once, and updates work from then on.

### Changed

- **Who hears what** is settled ([decision 0058](docs/decisions/0058-listeners-and-who-hears-what.md)): a Listener component gives a game several sets of ears (the camera hears when there's none), and each sound event will say who hears it: everyone, only who caused it, or named groups. It's a new Phase 7 milestone, S8.
- Every folder in the source now has a README saying what's in it and how its parts fit, for finding your way around.
- **Phase 7's plan is settled** ([decisions 0056](docs/decisions/0056-sound-with-kira.md) and [0057](docs/decisions/0057-animation-near-live-far-baked.md)): sound with Kira, and animation with live poses near and baked clips far, one state machine for 2D and 3D, in thirteen milestones.
- **A new phase order** ([decisions 0046](docs/decisions/0046-light-before-netcode.md), [0048](docs/decisions/0048-materials-and-shaders-before-effects.md), [0053](docs/decisions/0053-compatibility-before-2d.md) and [0054](docs/decisions/0054-first-class-2d.md), chosen by the project lead): after sound and animation come compatibility (Phase 8), first-class 2D in two parts (Phases 9 and 10), a playtest export (11), game UI (12), light, sky and atmosphere (13), materials and shaders (14), effects, now called Solstice (15, [decision 0047](docs/decisions/0047-the-effects-suite-is-solstice.md)), netcode (16), scripting, the full export and Git (17), and the 0.1 release (18). The editor's notes about what arrives when say the new numbers. The plans are in `docs/plans/`.
- **Smooth play on fast screens** (Phase 6): the game still runs 60 ticks a second, but the Game panel draws everything part way between ticks, so a 120 or 144 Hz screen shows smooth movement instead of steps.
- Game code uses Lust 2.0.0-alpha.35. A `vec3` is now a Luau vector in Lust and Lua alike (Lua's `vec3(x, y, z)` makes one), which is quicker than a table.
- **The benchmark** (Phase 5): **Help > Make the benchmark…** makes a project of two fleets of ships flying loops and firing lasers, in three areas: 2,000 ships and 20,000 lasers, 10,000 and 100,000, and a ceiling run that adds ships until 60 frames a second ends and says in the Console where. The manual's new page, The benchmark, explains it.
- **Game code makes and removes entities** (Phase 5): `spawn("ships/fighter", place)` makes a copy of an assembly while the game runs, and `despawn(entity)` takes one away; `frameTime()` says how long the last frame took. What game code makes shows in the Area panel while playing, and goes on Stop.
- **Faster still** (the Phase 6 review's third round): a crowd of characters standing still no longer makes every one follow its body each tick; checking the game's state while recording reads the components' values once per change instead of every tick; the Game panel skips sorting what it draws when it's already in order; and `rebind` changes the bindings in place instead of copying them. Recordings made before this stop matching at once (the check is worked out differently): record them again.
- **Faster again** (the Phase 6 review's second round): the ready-made character no longer presses into the floor while standing on it (the CharacterBody keeps it to the ground), which makes a walking crowd about a fifth quicker; an edit, an undo or a loaded save builds physics once instead of two or three times; all the saves made in one tick share one copy of the game; and physics follows only the bodies that are awake, not every body in the level.
- **Faster crowds of characters** (the Phase 6 review): each character's move is worked out on every core at once, then made in order, so the result is the same on any computer. Checking the game's state each tick while recording does less work (what only changes when entities come and go is worked out once), a component that moves or steers many entities no longer slows down with each one it has already moved, and an edit during Play builds a body again only when its place or physics changed.
- **Faster where it counts** (the Phase 5 review): game code's work is shared out evenly between cores (copies made by `spawn` used to pile onto one), the first core works instead of waiting for the others, a component that turns and moves its entity changes it in one go, gathering 100,000 copies to draw went from about 12 ms to about 10.5 ms, the Game panel's picture isn't worked out again on frames where nothing moved (or while paused), and lasers are handed to the graphics card without a copy.
- **Game code is quicker:** the safety timer that stops code running too long costs far less (game code's work in the benchmark's ships fell by about a seventh).
- **Projectiles** (Phase 5): game code fires lasers with `fire(from, direction, speed, life, colour)` (`solis.fire` in Lua). Up to a million fly at once, moved on every core and drawn in the Game panel as glowing streaks in one go. They don't hit anything until physics arrives. The manual's Game code page has a turret firing them.
- **Far away, models are drawn simpler** (Phase 5): each imported model gets up to three simpler versions of its meshes (levels of detail), made once when it's imported, and copies far from the camera draw the simplest one that looks no different. Name an object `Ship_LOD1` in your modelling program to use your own instead. Properties shows a model's levels, and the Profiler how many copies are drawn at each. The manual's new page, Levels of detail, explains them.
- **Only what the camera sees is drawn** (Phase 5): copies of models out of view are left out, so a huge area costs only what's in view, and the sun's shadows now cover what you're looking at (up to 200 m away) instead of the whole area, so they stay sharp in big areas. Things just out of view still cast their shadows into it. Each copy sent to the graphics card is half the size, and copies that didn't move aren't sent again. The Profiler shows how many copies cast shadows and how much was sent.
- **Game code runs on every core** (Phase 5): during a tick, game code sees the world as it was when the tick started, plus its own changes; moves and Console lines land when every `update` is done, in a fixed order, so a game plays the same on any computer (the manual's Game code page explains it). With 10,000 units playing, game code went from about 26 ms to about 13 ms a frame on a 2-core computer, and further with more cores. The quarter-second tick limit now counts per core.
- **Solis uses every core of your processor** (Phase 5): the game's view is worked out, and what the graphics card draws is gathered, in jobs on every core; the editor's own view of the area is worked out again only when the area changes. With 10,000 units playing, preparing to draw went from about 6 ms to under 1 ms a frame.
- **Game code runs faster** (the Phase 4 review): a tick of 2,000 Lust components went from about 14 ms to about 8.5 ms; `find` no longer gets slower with every entity in the area (2,000 components finding by name each tick went from about 33 ms to under 2 ms); pressing Play and saving code while playing do less on the frame; and saving a model or area no longer checks the game's code again when the code didn't change.
- The editor draws only when something changes, instead of as fast as it can all the time, so it uses next to no power while you aren't doing anything. The viewport's corner says **Idle** then (decision 0026).
- What other engines call a *scene* is an **area** in Solis: the Area panel, "Untitled area", and `.solarea` files when saving arrives. Reusable things you place many times (prefabs elsewhere) will be **assemblies**.
- Everything in an area is now an entity made of components: the sun, the environment, lights and imported models. Properties shows each entity's name and components (Transform first), drawn the same way for every kind of component. The sun's direction is now its **Rotation** under Transform (X tilts it from straight down, Z turns it round) instead of Heading and Height.
- The sun and the sky, fog and exposure belong to the area now, so they're no longer remembered between sessions on their own; they'll be saved with the area once saving arrives.
- Lights brought in with a model are listed under it in the Area panel, and deleting the model deletes them.

### Added

- **Sound, the first step** (Phase 7, S1, [decision 0056](docs/decisions/0056-sound-with-kira.md)): Solis opens the computer's sound card as it starts, without holding anything up, and reads WAV and Ogg sound files, checking each one as carefully as any file it's handed. Sounds in the Library show their waveform; point at one and click ▶ to listen, or click it to see its length, channels and sample rate in Properties, with Play, Stop and a line that follows the sound. With no sound card, the Console says so once and everything else works; Solis tries again every 30 seconds. The manual's new page, Sounds, explains it.
- **A splash screen** as Solis starts (decision 0045): the logo, name and version for at least 0.6 s while Solis asks GitHub for a newer build. A newer one brings up a question: **Update now** downloads it, installs it and opens it again; **Open without updating** carries on, and **Skip this version** stops asking about that build until a newer one is out. **Help > Check for updates when starting** turns the checking off, and **Help > Check for updates** asks now. Updates no longer download quietly in the background.
- **Solis has a logo**: a sun-like swirl round a dark centre. It's the program's icon on the desktop, in the Start menu, on the taskbar and in the window's corner, the setup program's icon, and at the top of the README.
- **The libraries' notices ship with Solis** (decision 0044): `THIRD-PARTY-NOTICES.txt`, beside the program and in the installed folder, lists the open-source libraries built into it with their copyright notices and licence texts, as their licences ask.
- **Ten accent colours** (Window > Accent colour), each shown with a swatch: Aventail's Blue and Olive join Petrol blue, Red, Marine green (called Green before) and Plum, with four sun colours of Solis's own: Copper, Orange, Amber and Gold. Each is deep enough for white text to read on it. A colour chosen before stays chosen.
- **Solis installs and updates itself** (decision 0043): download `solis-setup.exe` from the repository's Releases page and run it once (no administrator needed). Every merge into main is published there as a new build, and an installed Solis checks for one when it opens and every six hours, downloads it while you work, checks it arrived whole, and installs it when you close; **Restart to update** in the top bar does it now. **Help > Check for updates** checks at once, and **Help > Update automatically** turns the checking off. The window title and Help > About show the build number. The manual's new page, Installing and updating, explains it.
- **Characters and cameras** (Phase 6): a **CharacterBody** makes an entity a character that game code walks around with `moveCharacter` (`onGround` says whether it stands on something): it climbs steps and slopes, slides along walls, pushes crates and rides moving platforms. Every project now has ready-made components in `solis/kit/`, written in Lust to use or copy: a **CharacterController** (walk, sprint, jump), and three cameras: **ThirdPersonCamera** (follows behind, keeps out of walls, the wheel zooms when Wheel zooms is ticked), **FirstPersonCamera** and **RtsCamera** (arrow keys pan, middle-drag drags the map, Alt + arrows turn, the wheel and Page Up / Page Down zoom). `facingAlong(direction)` gives the rotation facing a way. **Help > Make the playground…** makes a level to walk around in, with steps, ramps, crates, a moving platform and quick saves on F6 and F7. The manual's new page, Characters and cameras, explains it all.
- **Key combinations and the mouse wheel in input** (Phase 6): an action can be bound to a key with Ctrl, Alt or Shift held (`Alt+Left`; hold the modifier while binding), and to the mouse wheel. The new ready-made actions Pan, Turn, Zoom and Drag are the RTS camera's; the Input actions window adds them to older projects.
- **Save games** (Phase 6): game code calls `saveGame("quick")` and `loadGame("quick")` (`solis.saveGame` in Lua). A save holds everything: every entity where the game has it, every running component's values, the lasers in flight, physics, the clock and the random numbers, so a loaded game carries on exactly as the saved one would have. **Play > Load a save…** plays from one. Saves go in the project's `saves` folder, which new projects keep out of Git. The manual's new page, Save games, explains it.
- **Input** (Phase 6): game code reads the player's controls as named actions (`axis("Move")`, `pressed("Jump")`, `held`, `released`, `bindingOf`, `rebind`), never keys. The ready-made actions are Move (W A S D), Look (mouse), Jump (Space), Sprint (Shift) and Interact (F), with a gamepad too (Windows). Click the Game panel to give the game the mouse and keys, Esc to take them back; the editor's own keys are off meanwhile. **Edit > Input actions…** changes the bindings by pressing the key, saved in the project's `input.solinput`. The new **Play** menu records a play's input and plays it back, saying whether the game came out exactly the same. The manual's new page, Input, explains it all.
- **Physics** (Phase 6): a **RigidBody** makes an entity fall, collide and get pushed, and a **Collider** gives it a shape (box, sphere or capsule); a collider on its own makes a floor or wall. Physics is strict: the same game comes out exactly the same on every computer. Game code pushes bodies (`push`), reads and sets their speed (`velocityOf`, `setVelocity`) and looks along lines (`castRay`, `castRayHit`, `castRayNormal`). A selected collider shows as a green outline. The manual's new page, Physics, explains it all.
- **Random numbers** (Phase 6): `random()`, `randomRange` and `randomInt` in Lust (and Lua's `math.random`) come from the game's seed, so the same seed plays the same on any computer. `seed` in the Console shows it or fixes it for every Play; `checksum` shows a number for the game's whole state, to check two plays agree.
- **Step** (Phase 6): while the game is paused, the new Step button beside Pause (or `step` in the Console) runs exactly one tick.
- **The Profiler (Phase 5):** a new panel beside the Console shows where each frame's time goes: game code, the game's world, preparing and handing over what to draw, the rest of the editor, and the graphics card's own passes (on cards that can time them), with a graph of the last few seconds and how much was drawn.
- **A first game (Phase 4):** **Help > Make the sample game…** makes the turret sample as a new project: a Lust turret shoots at a drone whose health (Lust), armour (Rust) and sparks (Lua) react to each other through events. The manual's new page, Your first game, walks through it.
- **Lua components (Phase 4):** a `.lua` file in the assets makes components with `component("Name", { { "speed", "float", 2 } })`, the way Lua code usually makes classes. They show in Properties, run in play mode with the same engine methods (`solis.moveTo`), send and handle the same events as Lust's (`solis.send`, `on_Hit`), and take new code while the game runs. Mistakes show in Problems with their file and line.
- **Changing code while the game runs (Phase 4):** saving Lust while playing swaps the new code into the running game in about a third of a second. Every component keeps its values by name; fields whose kind changed, and new fields, start over, and the Console says which. New components start, removed ones stop, and a mistake leaves the last working code running.
- **Events between components (Phase 4):** Lust components send events (`send Hit(me, 2.5) to target;`) and react to them (`on Hit(…)`), delivered after each tick's updates in a fixed order. Components written in Rust take part through `script::RustComponent`, for now in a Solis built from source. Components that keep sending to each other are stopped after 10,000 events in a tick, with a message.
- **Lust components (Phase 4):** a component written in Lust appears in Properties' Add component menu, with its fields in Properties, saved in the area like Solis's own. Play runs each one's `update` every tick, and its native methods move, turn, find and name entities and write in the Console. Changing a field while playing reaches the running component and keeps the rest. A component that fails, or runs too long, stops alone, and the Console says where in the Lust.
- **Game code, first step (Phase 4):** Solis reads a project's Lust (starting from `assets/main.lst`) and checks it whenever a file is saved. Each mistake shows in the Problems panel with its file, line and column; clicking one opens the file. Each project gets an `assets/solis` folder describing what game code can ask of the engine (`say`, `time`, `find`, `positionOf`, `moveTo`, `rotationOf`, `turnTo`, `nameOf`). While editing, Lust will run as Luau and be swapped into the running game; built games run it as Rust (decision 0028).
- **Several views:** the viewport's **≡** menu splits it into one view, two side by side, two one above the other, or four. Each view has its own camera and look; click in one to use it (F, adding things and the tool bar follow it), and **Maximise** shows it alone. Remembered on this computer.
- **Flat views:** each view can be **Perspective**, **Top**, **Front** or **Right**. Flat views show true sizes with the grid in their own plane; drag with the right or middle button to slide them and turn the wheel to zoom towards the pointer. Selecting, gizmos and dropping from the Library work in them.
- **Play mode:** **Play**, **Pause** and **Stop** at the top run the area as a game in the **Game** panel. Everything still works while playing, and changes show in the game at once; **Stop** puts the area, the undo history and the selection back exactly as they were. Saving and opening wait until Stop. A coloured border, a lit Play and a red Stop show that the game is running. Also `play`, `pause` and `stop` in the Console.
- **Cameras:** **Entity > Add camera** adds one where you're looking from. The game looks through the first camera in the area; cameras show as small wire pyramids in the viewport, and can be clicked.
- **Console commands:** press **'** (beside Æ on a Norwegian keyboard) to jump to the Console's new command line. `select`, `find`, `new`, `delete`, `duplicate`, `get` and `set` (any component's setting, on every selected entity at once), `frame`, `undo`, `redo`, `save`, `look`, `tool`, `stats`, `clear` and `help`. Every change is an undo step. **Tab** finishes names, **Up** and **Down** bring back earlier commands (remembered between sessions), and mistakes get a plain answer in red.
- Warnings and errors from the libraries Solis uses (the graphics layer, for example) now show in the Console, named after the library.
- **The Problems panel** lists mistakes in the open area: models or assemblies missing from the project, a Model with no model chosen, components no one knows, and entities whose parent is gone. The tab shows how many; click one to select its entity.
- **The Library** shows the project's folders (a tree you can put away) and every file in the folder chosen: models and pictures as **thumbnails**, areas, assemblies and other files with a coloured stripe per kind, or as a list; a search box finds files in every folder. **Drag a model or assembly into the viewport** to place it on the surface under the mouse, or **onto an entity** in the Area panel to place it under that one. Double-click opens areas, and other files in their own program. Right-click to rename, show in Explorer, or delete to the Recycle Bin; drag a file onto a folder to move it. IDs always follow.
- **The Code tab** lists the project's code files (Lust, Lua and Rust); double-click opens one in your code editor.
- **The Area panel** has a search box, folds (▾/▸ before an entity with children, and View > Fold or Unfold everything), and on each row an **eye** to hide an entity in the editor and a **padlock** so clicks in the viewport pass through it. Its **Edit** and **Entity** menus work (the same as the top bar's, where Edit also gains Duplicate, Delete, Rename, Select all and Select nothing). Selecting in the viewport unfolds the way to it.
- **Properties with several entities selected** shows the components they all have; settings that differ show **—**, and a change applies to every one as one undo step (changing one number of a position or rotation changes only that number on each). Add component and × work on all of them.
- **Copy and paste components' values:** the ⋯ beside a component has Copy values, Paste values (adding the component where it's missing) and Reset to defaults; Properties' Edit menu copies and pastes all of them.
- **Selecting in the viewport:** click a model or a lamp's marker to select it, **Ctrl + click** to add or take away, drag on empty space to draw a box around what to select (**Ctrl** adds), **Esc** to select nothing and **Ctrl + A** to select everything. Selected things are outlined. A copy of an assembly is selected whole first; click again to go inside it.
- **Gizmos:** with Move (**W**), Rotate (**R**), Scale (**S**) or Transform (**T**), drag the arrows, rings, boxes and squares on the selection to move, turn and resize it. A note by the mouse says how far. A whole drag is one undo step; **Esc** or a right click cancels it. **Shift** drags ten times finer.
- **Snapping and axis locks** in the viewport's bottom-right corner: moves onto the grid (1 m to 10 cm), turns in steps (5° to 90°), sizes in 10% steps; hold **Ctrl** while dragging to flip snapping for that drag. **X**, **Y** and **Z** lock an axis so drags leave it alone; **L** switches the gizmo between the world's axes and the entity's own. Remembered on this computer.
- Assemblies: **Entity > Create assembly from selection** saves an entity and everything under it as an assembly in the project; place copies from the Library. Each copy can have its own changes, shown in bold in Properties with ↺ (put back) and ↑ (apply to the assembly, for every copy). Editing the assembly (or a Git pull) updates every copy and keeps their changes. Assemblies can hold other assemblies. Areas store copies as one block plus only what differs.
- **Unpack** in Properties: turns a copy of an assembly into plain entities, or splits an imported model into one entity per part of the file, each drawing only its own part.
- Saving areas: **Ctrl + S**, **Ctrl + Shift + S**, **Ctrl + O** and **Ctrl + N** (also in the File menu). Areas are `.solarea` text files: one block per entity, one line per setting, made so Git merges two people's work cleanly, and safe to edit by hand. The tab shows a • while there are unsaved changes, and Solis asks before losing them. A file changed on disk (a Git pull) is loaded as one step Ctrl + Z can undo, or, with unsaved changes, Solis asks which to keep.
- Projects: **File > New project…**, **Open project…** and **Recent projects**; the last project opens again on start. A project is a folder with an `assets` folder (watched for changes), a `.solis` cache and Git files set up for games. Imported models are copied into it.
- The **Library** lists the project's models; double-click one (or click its +) to place it.
- Renaming or moving a model breaks nothing: each asset's ID lives in a `.solmeta` file beside it, which follows a file renamed in Explorer. Copies get IDs of their own.
- Models saved again in Blender show the new version straight away; if the new file is broken, the old version stays and the Console says why.
- Models read before open much faster: an import cache remembers them, even after a rename. Models are read by a small pool of worker threads.
- **Add component** at the bottom of Properties gives an entity any component (a Light on a model makes it glow like a lamp), and the × beside a component's name takes it off. Game code will register its own components the same way.
- The Area panel stays fast with thousands of entities: only the rows in view are drawn.
- Building the area's tree: drag an entity onto another in the Area panel to put it under it (it stays where it is in the world), or onto empty space to make it stand alone. **Ctrl + click** and **Shift + click** select several; **Delete** deletes them, **Ctrl + D** duplicates them, **F2** or a double-click renames, and **F** points the camera at them. All of it can be undone.
- New entities go to the end of the Area panel's list instead of a random place.
- Models keep their own parts' placements: a mesh used many times in a file is stored once and drawn as copies, so big kits of repeated pieces load faster and use less memory. Mirrored parts (a negative scale) are drawn the right way out.
- Undo and redo for every change to the area: **Ctrl + Z**, and **Ctrl + Y** or **Ctrl + Shift + Z**, or **Edit > Undo** and **Redo**, which name the change. A whole drag is one step, and undo selects what it changed.
- **Entity > Add entity** adds an empty entity at the point the camera looks at.
- A component Solis doesn't know (for example from game code that isn't there) is kept untouched and shown as **Missing component**, with **Remove**.

### Fixed

- **Found by the new whole-game tests** (Phase 6 hardening):
  - A game saved after game code spawned anything (or with a copy of an assembly in the area) couldn't be loaded, and would have lost what was on each copy's root. Saves keep every entity whole now.
  - A loaded game slowly went its own way from the saved one: rotations came back a hair different (they were made unit length again on reading), and a turned entity was placed with a -0 where the running game had 0. Both are exact now, so a loaded game carries on exactly as the saved one would have.
  - After a load, moving something in the editor while playing kept where the game had turned it, where the game that was saved would have put it as the editor had it. Saves now keep the editor's area and the game's places apart, as the running game does. Saves made before this (version 2) can't be loaded: save again.
- **Found by fuzzing** (Phase 6 hardening): two components (or classes, or objects) with the same name, an easy slip when copying one, made checking the code never finish, so the editor waited for ever and a core stayed busy. It's now a mistake in Problems, saying where the other one is. And should checking ever take more than 30 seconds, it's given up on with a message, instead of waiting for ever.

- **Easier to use** (from a user test of Phase 6):
  - A setting that points at another entity showed a long ID and couldn't be changed in Properties; it shows the entity's name, and a list (with a search box, and None) picks another.
  - While playing, Properties showed the values game code's components started with; it shows them as they are now, and says so.
  - A long Console message was cut off; click it to read it in full and copy it.
  - Add component lists Solis's own components and the game's in two groups.
  - The file window's and window library's warnings no longer fill the Console with things you can't act on.
  - Opening a Lust file says once which program opens it, and how to choose another.

- Closing Solis while playing stopped the game before asking about unsaved changes, so Cancel left it stopped; it asks first now, and the game plays on if you cancel.
- A project whose name had a line break in it could never load its saves.
- A save whose first line had a Windows line ending added said it was "made by a newer Solis"; it says it's damaged. A load whose input actions can't be read says so plainly.
- A hand-changed save could give entities components they don't have, or list millions of lasers before being refused; it's checked first.

- A character standing on a moving platform was left behind by it (a mistake in the last round of fixes); it rides it again, standing or walking.
- Replaying a recording in which the game saved stopped matching right after the save; it matches now (and still doesn't write over your saves). A replay that matched says so as news, not as a warning, and one that didn't says from which tick.
- A quick load from game code (F7 in the playground) gave the mouse and keys back to the editor, so the next key went nowhere; the game keeps them. Loading while recording saves what was recorded so far instead of losing it.
- The arrow keys didn't work in the RTS camera's Pan and Turn: they were bound by a name Solis doesn't use. They're now `Up`, `Down`, `Left` and `Right`, and older input files are read the new way.
- An entity field set before the code was checked (opening an area, say) was lost when Play started; it's kept.
- Esc giving the game's keys back also cleared the editor's selection; it only gives the keys back now.
- The Console's `seed` says the seed Play chose and, if game code or a load changed it, that too.

- A crate at rest stayed hanging in the air after an edit during Play lifted it or deleted its floor (a mistake in the last round of fixes); it falls again.
- A crate in a group that game code or an edit moved was drawn in its new place while its body stayed behind; it goes along now.
- A save made on a tick whose spawned entities spawned more left the second lot out; they join before the save.
- A recording kept the seed a game's start had set instead of the one Play chose, so its replay could fail to match; and a replay's `saveGame` wrote over your saves.
- `setSeed` from `update` already reached the starts of what that tick spawned; it now holds from the next tick, as described. Changing code while recording now says the replay won't match from there.
- The Input actions window could make more than 256 actions, which Solis then refused to read (with its saves and recordings): it stops at 256.
- A second mistake in `input.solinput` was overwritten without a backup: each gets its own (`.bak`, `.bak2` … up to `.bak9`).
- A save's first lines (its clock and seed) weren't covered by its checksum, so a change to them loaded silently; they are now, and a clock out of range is refused.
- Physics took places and turns too big for it (a rotation that overflowed, a collider thousands of kilometres big), which gave bodies no place or emptied physics with a message blaming a save: such numbers are refused or held in range, and the message says what happened.
- Saving put fixed and kinematic bodies back where they were a moment before; they stay where their entities are.
- **Security** (the Phase 6 security audits):
  - Clicking a problem in a hostile project could open a network share in Explorer (which hands it your Windows sign-in): game code writes a problem's text, and the file was read from it. Only files in the project open now, from the Problems panel or the Library.
  - One `table.move` in Lua game code could hold Solis for a minute, past the time limits; it moves at most a million items at once. Game code can no longer stop Lua's memory collector (`collectgarbage` only says how much is used).
  - The backup of a damaged `input.solinput` was copied through a link a project could plant under its name, writing anywhere on the computer; it's now written as a new file. Saves, recordings, the import cache and the `solis` folder are never written into a folder that's a link.
  - Spawning thousands of copies of an assembly too big to fit froze the editor (each was made, then refused); its size is checked first. Assemblies placed inside each other many levels deep could take gigabytes of memory on opening; they're refused past a limit.
  - A model naming one child a million times took gigabytes before its node limit was checked; it's refused first.
  - Files are read only as far as their limit, and only if they're ordinary files (a link to a device never ended). The import cache refuses numbers the importer never makes.
  - A model's file address could reach a hidden stream (`%3A`) or a device (`NUL`); names Solis makes or you rename to can't be a device's name, hide a stream, or turn text around. Folders open in Explorer directly.
  - The saves folder holds at most 2 GB of saves, and game code's speeds and pushes are kept within what a save can hold.
  - A long run of `!` in Lust took the compiler hours; more than 64 in a row is a mistake now.
  - A `.lua` file starting with certain invisible characters was read as compiled Lua, whose reader isn't built for hostile files: just opening a project holding one could crash Solis, and might have let it run code. Game code is now always read as text.
  - Lua's `require` was left in game code's sandbox, so game code could read and run Lua files anywhere on the computer. It's gone.
  - Double-clicking a file the Library doesn't know opened it with Windows, so a program named `brick.png.exe` (shown as "brick.png") would run. Such files now open their folder instead, and show their whole name; only pictures and Lust files open in their own program.
  - Saves and input actions were written through a fixed file name a project could ship as a link pointing elsewhere; every write now goes through a new, unique file.
  - Save slots can't be Windows device names (`CON`, `NUL` …), and a project keeps at most 100 saves, so game code can't fill the disk.
- **Save games can't freeze or crash Solis** (the Phase 6 review): a save kept physics as the physics library's own bytes, which a damaged or hand-changed save could make freeze Solis or close it. A save now keeps each body's place, speed, spin and rest, as text, and physics is built afresh from them; saves are far smaller and survive physics updates. Saves made before this can't be loaded (save again).
- A `saveGame` in every unit's `update` made a whole save for each one in a tick, freezing Solis: the same slot is now saved once a tick, and at most four slots.
- A save or load asked for on a tick that paused the game (a slow tick, the end of a replay) was dropped.
- Saves and recordings bigger than Solis reads back were still written: a save past 256 MB is refused with why, and a recording pauses at 64 MB as well as at an hour.
- With a mistake in `input.solinput`, one change in Edit > Input actions… wrote the ready-made actions over your own: the file is kept as `input.solinput.bak` first.
- `rebind` could give an action more inputs than its file allows, which made a save holding them unloadable.
- Lasers in a hand-changed save could fly forever or faster than `fire` allows.
- `saveGame`, `loadGame` and `rebind` asked for while a component starts were dropped without a word; the Console now says so. What a spawned component said while starting reaches the Console too.
- Replay a recording… and Load a save… could stop opening their window if one ended oddly.
- A collider under another entity (a lift's floor) stayed behind when the entity moved, so characters walked on where it used to be. Colliders, kinematic bodies and characters under something that moves now go with it.
- Loading a save ran the game's starts again on top of it, so anything they moved or the seed they set changed the loaded game.
- Editing during Play stopped crates spinning, woke resting stacks and made characters forget they were on the ground for a tick; Undo did too.
- Changes in Edit > Input actions… while playing didn't reach the game until Stop and Play.
- Presses and mouse movement while paused, or during a replay, arrived all at once when the game went on (the camera jumped).
- `setSeed` took effect part way through a tick; it now holds from the next tick, as described.
- Editing the area while recording now says the recording's replay won't match from there, and a replay that stops matching names `frameTime()` as a usual cause.

- New projects' Git settings now keep Lust files (`.lst`) with the same line endings everywhere, so they merge cleanly; they named `.lust` by mistake. Input actions and recordings are covered too. Projects made before keep their own settings: add `*.lst text eol=lf` to `.gitattributes` by hand.
- When a frame stopped part way through its ticks (the game paused for being too slow), the game's clock had already counted the ticks that never ran, so game time jumped ahead. The clock now moves on as each tick begins.
- What game code spawns joins the game at the end of the tick that spawned it, so its components run from the next tick even when a slow frame runs several; spawns and despawns land in the order asked for, new copies go at the end of the Area panel's tree, and spawning no longer rebuilds the whole game world (which also stopped lasers in flight from vanishing). Copies spawned while components start, or while new code is swapped in, are no longer lost (Phase 5 review).
- Two spawns in different rounds of events in one tick no longer make the same entity.
- If a game code start fails on some of the computer's cores but not others, or new code fails to swap in on some, every core goes back to the same code, so no entity runs different code from the rest; failures are said in the same order however many cores there are.
- A model that keeps its own levels of detail (`Ship_LOD1`) beside the full one in the file no longer gets a box that includes them, and a part of a model placed on its own picks its level of detail from where it stands. Models are imported again once to pick this up.
- Game code that catches the time limit's error in a loop (`pcall` in Lua) is stopped too, instead of freezing Solis (Phase 5 review).
- Game code calling `moveTo`, `fire` or `spawn` in a runaway loop can no longer fill the computer's memory before the time limit stops it: past 200,000 changes in one go the rest are left out, and the Console says so.
- Spawning is limited to 10,000 copies a tick and the area's most entities; past that, the Console says so instead of the area growing until memory runs out.
- A model naming one huge mesh as the level of detail of many parts can't multiply it past the import limits, and models with thousands of parts import their own levels of detail quickly.
- A project made so that many entity IDs share their low digits no longer slows Play to a crawl.
- An assembly that can't be read isn't read again every time the Library updates.
- **Game code can no longer freeze or crash Solis** (the Phase 4 review): very deeply nested Lust no longer crashes Solis (it's a mistake in Problems: Lust takes at most 150 levels of brackets, blocks or operators inside each other); every place Solis meets game code has a time limit, including reading components and swapping in new code; a tick whose code takes more than a quarter of a second pauses the game and says why, rather than catching up ever more slowly; and what game code says or sends is kept within limits, so it can't fill the computer's memory or the Console. A component whose event reaction keeps failing is said once.
- Each Lua file now has globals of its own, and Lua's own libraries can't be changed, so one file can't break another or the Lust.
- The code's start (reading its components) now runs on a worker, never while drawing the editor, and a check that never ends is given up on after a minute with a message.
- **Help > Make the sample game…** never replaces a file already in the folder; it asks for an empty one.
- Code changed while playing kept its components after **Stop**: Properties and Add component no longer lose them.
- A Lust or Lua component named like one of Solis's own (`Light`) or a Rust one is still a problem in Problems, now pointing at where it's written, and it no longer runs anyway.
- Adding or deleting an entity, or undoing, while playing no longer puts things game code moved back where they started.
- New code that fails to start against the running game no longer replaces the components in Properties; the game keeps the last code, and Problems says why.
- `send` in the code's own start, and starting values that call `time()` and the like, no longer stop the code from loading.
- An event sent from Lua or Rust with the wrong values is now blamed on the sender, not on every component that receives it. Numbers in events no Lust declares arrive as floats.
- If the code's start fails at **Play**, the Rust components still run.
- The Console no longer says a component "stopped" when it kept running.
- Changing `armour.rs` in the turret sample now says at once that its changes don't run yet, not only when the project opens.
- A Lua field called `me`, `update` or `on_…` is a mistake now, instead of hiding the entity or the method.
- The manual's "Things to try" for the turret sample changed a starting value, which running components rightly keep; it now changes the code instead.
- On Windows, the turret sample said its `armour.rs` was changed when it wasn't (the file's line endings differed).

- **Work could be lost when saving before quitting or starting a new area.** On an untitled area, choosing **Save** in the "unsaved changes" question went straight on to quitting (or the new area) while the Save window was still open. Now everything waits for the Save window.
- After a save was cancelled or failed while quitting, closing the window later closed it without asking about unsaved changes. Now it asks again.
- A file chosen in an Open or Save window opened before **Play** is now opened or saved only after **Stop**, so it never replaces or saves the game's copy. The question about a file changed on disk also waits until Stop.
- A Save name typed without `.solarea`, matching a file that already exists, replaced it without asking. Now it isn't saved, and the Console says how to replace it.
- An assembly that a Git pull changed to hold a new, not yet loaded assembly could detach its copies, and saving made that permanent. Assemblies are now all read before any is used, and the copies keep the version they were made from until the new one can be read.
- A damaged saved panel layout could crash Solis every time it started. A layout that can't be drawn is now replaced by the default one.
- A problem reported by the graphics card no longer closes Solis; it's shown in the Console. Scenes with more model copies than the card can draw at once are skipped with a message instead of crashing.
- Saving now writes the file to disk before it replaces the old one, so a power cut can't leave an empty file. Two saves of the same file at once no longer mix.
- A model file with hundreds of thousands of empty nodes, or a very deep node tree, could freeze or crash the editor. Such files are refused, and very deep trees keep short part names.
- A sidecar using the biggest part number crashed the editor or reused numbers. Part numbers now never repeat, and a sidecar with two parts sharing one is reported as damaged.
- An area file naming one entity twice with different spellings (lower case, or O for 0) silently lost one of them. Now it's reported.
- After many changes, undoing everything could make the area look saved when it wasn't.
- Choosing the open project again from **Recent projects** emptied the viewport. Now nothing happens.
- Opening another project kept the old project's assemblies, so a copy of a project could save its areas against the wrong assemblies.
- Components registered by game code were forgotten when a new area was started or another opened.
- Renaming, moving or deleting the open area in the Library left Solis saving to the old name, making a second file. The open area now follows its file.
- Renaming a file only by changing capitals (`crate.glb` to `Crate.glb`) was refused on Windows.
- The last project now opens again when Solis starts (it was saved but never read back).
- A damaged area whose entities are inside each other could freeze the editor when one of them was selected or moved. Now the Problems panel reports them.
- The red and green axes on the ground could show as a large filled wedge on some graphics drivers when the camera was close to them.
- Clicking could select the wrong thing after the area changed or a view was resized, because the answer belonged to an older click. Clicking the very last pixel of a view no longer misses.
- In a Top, Front or Right view, **Add > Camera**, **Add > Spotlight** and **Move here** put the new thing far away, and fog covered the area. They now go where the view looks from, and flat views have no fog.
- The game view and the editor could light an area with two suns differently. Both now use the first one in the Hierarchy.
- **F** framed an unpacked model by its whole size, even when only its parts were selected.
- A thumbnail that was replaced or changed on disk showed the old picture, and its memory wasn't freed. Thumbnails from the last project no longer appear in the next.
- The Problems panel didn't update after a model was reloaded or another project opened. It now also reports an entity that is (through others) inside itself.
- A model added to the folder while the area used it, or a changed one, wasn't reloaded in the viewport.
- **Help > Add the sample area** with a project open added a model the project didn't have, so a saved area that used it said it was missing when opened again. It's now added to the project's assets like an import.
- Placing a model by double-click after an earlier drop of it failed sent it to the old drop point.
- A Git checkout or pull while Solis was open could give a model a new ID if Solis looked between Git writing the model and its sidecar. A new file without a sidecar now waits two seconds for one.
- A model copied (with its sidecar) while Solis was closed could take the original's ID, so areas using the original showed the copy. The original, whose sidecar is older, now keeps it.

- A layout saved before a panel existed keeps its arrangement and gets the new panel beside its usual neighbours, instead of being reset to the default.
- A full Console no longer slows each new message down.
- Damaged or hostile model files can no longer use up all memory: a buffer only reads the bytes it says it holds, a model's files and textures each have an overall budget, a tiny file asking for millions of corners is refused before reading, and one mesh placed too many times is refused. Files are only read from the model's folder.
- A very large mesh no longer crashes Solis: it's split into pieces the graphics card can hold.
- Mirrored parts of a model without normals were lit as if inside out.
- A model needing a glTF extension Solis doesn't read (such as Draco compression) now says which, instead of "refers to parts that don't exist".
- The Import model window no longer freezes the editor while it's open, and an import that fails unexpectedly reports an error instead of "Importing…" forever.
- Distant surfaces no longer flicker against each other (depth is now reversed, keeping precision far away).
- Showing the sun or a lamp in Properties no longer changes values that came from a file (a sun below the horizon, a lamp's long range).
- A model with impossibly large bounds no longer breaks the camera.
- Several viewports no longer share one set of frame data, ready for the game view and split views.

### Changed

- The editor is faster with big areas. With 2,000 entities, a frame takes about half as long (2.8 ms to 1.4 ms), four views a third as long (3.4 ms to 1.3 ms), and dragging everything at once 4.1 ms instead of 6.5 ms. The area's tree and every entity's place are worked out once and kept until they change, and a long drag of many entities no longer slows down as it goes.
- The Problems panel no longer looks again on every frame of a drag: moving things can't cause a problem.
- Play no longer copies the whole undo history; the copy shares it.
- The undo history's memory limit now counts the selection kept with each step.
- The import cache in `.solis/` keeps within 4 GB: the models used least lately are deleted when a project opens, and read from their files again if needed.
- Looking over the assets folder no longer reads every `.gltf` again while it's unchanged.

- Writing game code in Lust now arrives in Phase 4, right after the editor, instead of Phase 7; the editor's panels say so. Playing the area arrives with the editor, in Phase 3.
- The Console stays fast when full: it lays out only the messages in view (a full Console took about 10 ms a frame, now 0.3 ms). Long messages are cut to one line and show in full when pointed at.
- A camera saved with broken numbers (from a damaged settings file) is repaired when Solis starts, instead of showing a broken view.
- `solis --help` takes the program's name from one place, like the rest of the editor.

- The editor looks more like the final mockup: the area tab sits on a lighter strip with an icon and ×; every panel tab has an icon and a deeper accent underline; the menus are roomier; Play, Play with others and Stop are icon buttons; panel menu rows end in a search icon; the console has compact filter buttons (icon and count), times on every message and JetBrains Mono text; and the viewport fades from dark above to a lighter ground, with a cool grey-blue grid and no hard horizon.

### Added

- The finished picture: light is worked out brighter than white (HDR) and tone mapped as in Blender (AgX by default, or ACES, or Standard), so bright highlights roll off softly instead of flattening to white. Select **Environment** in the Area panel for the sky (the editor backdrop, or a daylight sky that follows the sun from blue noon to orange sunset and dark night), fog, exposure and tone mapping; they're remembered between sessions. The editor's backdrop and grid look as before.

- Lights and shadows: the sun casts shadows on models and the ground, and its direction, colour and strength can be changed (select **Sun** in the Area panel); it's remembered between sessions. **Entity > Add point light** and **Add spotlight** place lamps, listed in the Area panel and edited in Properties (colour, strength in watts as in Blender, range, cone, **Move here**, **Delete**). Lamps and a sun in an imported glTF file (as Blender exports them) come in with the model. Lamps show as small crosses in the viewport, and light both the standard and stylized looks. Selecting a model in the Area panel shows its size in Properties.

- Materials and looks: models now show glTF's full material (metal, roughness, normal, occlusion and glow textures). The viewport's look menu (bottom left) chooses **Standard**, **Stylized** (cel-shaded in 2 to 4 bands of light, with crisp highlights, a rim of sky light and dark outlines) or **Unlit**, remembered between sessions. **Help > Add the sample area** brings in spheres from glossy to rough, plastic and metal, a grooved wall and a glowing stripe for trying them.
- The viewport's bottom-left menus now open upwards and stay open while you change their settings.

- Importing models: **File > Import model…**, or drop a `.glb` or `.gltf` file onto the window. Meshes, colour textures, see-through and double-sided surfaces come in, upright (glTF's Y-up becomes Solis's Z-up), lit by a fixed sun and the sky. The camera frames the new model; the Area panel lists imported models, each with a × to remove it. Models are read in the background, and damaged files are explained, never a crash. Also `solis --import <file>`.

- The user manual (`docs/manual`): getting started, concepts, a tour of the interface, moving around the viewport, the Console and every shortcut.

- A ready-made `solis.exe` for Windows, built on GitHub whenever code is merged (Actions tab, **Windows EXE**). Release builds on Windows open without a console window.

- The Code panel (beside Library) and the Problems panel (beside Console), as in the mockup. They fill in during Phases 7 and 2. Saved layouts from before reset to the default once, to make room for them.

- Flying around the viewport: hold the right mouse button to look; while it's held, W A S D move, Q and E go down and up, the wheel changes speed (shown in the Camera box) and Shift goes faster. The wheel alone moves forward and back; middle drag slides the view. The camera and its speed are remembered.

- The editor window: a one-row top bar with menus, the area tab, and Play, Stop, Build and Build & Play (which say which phase brings them).
- Dockable panels: Viewport, Game, Area, Layers, Properties, Library and Console. Drag a tab to move a panel; the layout is remembered next time. Window > Layouts has Default, Wide screen and Small screen.
- The viewport draws a sky, a ground grid that fades with distance, and the red X and green Y axes. Its corners hold the view menus (≡, Perspective, Lit, Show), the tools (Select, Move, Rotate, Scale, Transform; keys Q, W, R, S, T), grid, angle and camera speed, and the frame rate.
- The console lists messages, with filters by level and a search box.
- Window > Accent colour: petrol blue, red, green or plum.
