# tests/two-pcs

The two-PC check's game (stage 1, "the same game on two PCs"; decisions 0120 and 0122): a recorded game and the state it must come out as at every tick. The check (`crates/app/src/two_pcs.rs`, its tests in `two_pcs_tests.rs`) plays the recording back and compares each tick's checksums with the list. It runs in `cargo test --workspace` on Linux, where the list was made, and on GitHub's Windows machine in every pull request, on one core and on four: maths that rounds differently on another processor, operating system or number of cores is caught at the first tick it shows.

| File | What it is |
| --- | --- |
| `game/` | The game's project, as Solis opens it (its files at the top, each referenced asset with its sidecar). The check opens a copy, never this folder: Solis writes its own files into a project it opens. |
| `game.solrec` | Ten seconds (600 ticks) of recorded play: a gamepad walking the character up through a zone and back, jumping, and pressing the hive's button. |
| `game.states` | What the game holds at the end of every tick: the world's and game code's checksums, and its state part by part (each entity's place and components, physics, tile maps, zones, game code's values and objects), written as what changed since the tick before (each part numbered the first time it comes, and a value longer than 32 characters kept as a checksum of it, so the list stays small). Made on Linux. |

## What the game does

Everything in it is game state, so a difference anywhere shows:

- **3D:** a character with the ready-made controller, three crates and a moving platform (physics), a hive spawning drones at random places that are pushed about and despawn, a turret firing projectiles and sending hits to a lantern, a box zone the character walks through and a round zone that fires once.
- **2D:** a walker (the ready-made top-down controller) with a sprite and an animator, in a tile map whose walls game code raises and lowers at random, a solid post, and a freeform zone.
- **Game code** in all three languages: Lust (`main.lst`: the game-wide `Director` object slowing the game to half speed and back up, a tween with no done block, sines, powers and arctangents), Lua (`wobble.lua`: the lantern's sway) and Rust (`armour.rs`, Solis's own Armour, as in the turret sample). Every random roll is `World`'s, kept to the game's seed.

It is kept clear of the two gaps decision 0106 records: no tween has a done block (their order across cores isn't fixed yet), and nothing rolls Lust's own `Random` (which ignores the seed). Shared 3 closes them.

## When it fails

The test names the tick and the parts that differ, with what each should hold and what it holds here side by side (for a part the list keeps as a checksum, what it held a tick earlier, when both PCs still agreed), and writes both states to `target/two-pcs/` (CI keeps that folder as the `two-pcs-states` artifact). A part the list keeps as a checksum shows only this PC's value; to see what it holds on Linux, run the check there with `SOLIS_TWO_PCS_STATE_AT=<tick> cargo test -p app two_pcs`, which writes that tick's whole state beside it.

## Changing it

- **A deliberate change to the game's rules** (physics, what game code's calls do, what the checksum covers): write the list again on Linux with `SOLIS_UPDATE_TWO_PCS=1 cargo test -p app two_pcs`, and commit it.
- **A change to the level or its code:** record the game again, which writes the list too: `cargo test -p app record_the_game -- --ignored`.

The folder is kept byte for byte (`.gitattributes`): a Windows checkout must play exactly the files Linux recorded.
