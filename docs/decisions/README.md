# docs/decisions

One file per major choice, numbered in order. Each records its Status, Date, the Problem, the Options where useful, the Decision and its Consequences. A decision is never deleted: a later one replaces or changes it and says so in both files' Status. Every decision also has a line in DESIGN.md section 17, "Decision log".

**Adding one:** take the next number, name it `NNNN-short-name.md`, and add a line to DESIGN.md section 17 and to the table below.

Titles below are shortened; the files have the full reasoning. Decisions 0019 to 0022 were written when areas were called scenes and assemblies prefabs (renamed by 0023).

| File | In short | Status |
| --- | --- | --- |
| `0001-builds-on-demand.md` | Builds ran on demand, not on every update | Replaced by 0018 |
| `0002-engine-in-rust-games-in-lust.md` | The engine is Rust; games are Lust | Accepted |
| `0003-games-in-rust-lust-or-lua.md` | Games can be written in Lust, Rust or Lua | Accepted (extends 0002) |
| `0004-editor-layout.md` | The editor's layout: panels and tabs, everything dockable | Accepted |
| `0005-netcode-in-layers.md` | Netcode in layers: the shared parts built in, the model optional | Accepted |
| `0006-git-built-in.md` | Git built into the editor, for teams | Accepted |
| `0007-mit-licence-and-separate-from-aventail.md` | MIT licence, and a project separate from Aventail (whose code may be adapted) | Accepted |
| `0008-lighting-is-a-project-choice.md` | Lighting is a choice per project; real-time with sky light first | Accepted |
| `0009-fly-camera-controls.md` | The viewport camera flies (right mouse with W A S D) | Accepted |
| `0010-windows-exe-on-merge.md` | A Windows EXE is built when code reaches main | Accepted (loosens 0001; 0043 adds an installer) |
| `0011-scale-check-at-phase-gates.md` | A scale check at every phase gate | Accepted |
| `0012-windows-only.md` | Made for Windows | Accepted |
| `0013-looks-for-the-whole-view-first.md` | The standard and stylized looks, chosen for the whole view first | Accepted |
| `0014-lights-placed-two-ways.md` | Lights placed by hand or brought in with models, in the units 3D modelling programs use | Accepted |
| `0015-file-extensions.md` | File extensions for areas, assemblies, materials and the project file | Accepted |
| `0016-how-lust-joins-solis.md` | How Lust joins the engine, moved earlier to Phase 4 | Accepted |
| `0017-hdr-tone-mapping-sky-and-fog.md` | HDR with AgX tone mapping, a daylight sky, and fog | Accepted |
| `0018-checks-for-a-public-repo.md` | Fuller checks, now the repository is public | Accepted (replaces 0001) |
| `0019-scene-document-now-game-storage-later.md` | An area document now; the game's fast storage later | Accepted |
| `0020-scene-files-in-lust-style.md` | Area and assembly files as Lust-style text, made for Git | Accepted |
| `0021-asset-database-and-models.md` | The asset database, and models as node trees | Accepted |
| `0022-undo-as-reversible-edits.md` | Undo and redo as reversible edits to the area document | Accepted |
| `0023-areas-and-assemblies.md` | "Area" instead of "scene", "assembly" instead of "prefab" | Accepted |
| `0024-game-storage-is-hecs.md` | The game's fast storage is the `hecs` library | Accepted |
| `0025-picking-and-gizmos.md` | Picking in the viewport, and the move, rotate and scale gizmos | Accepted |
| `0026-draw-only-when-something-changes.md` | The editor draws only when something changes | Accepted |
| `0027-regular-code-reviews.md` | Regular code reviews for stability and speed | Accepted |
| `0028-lust-runs-as-luau-while-editing.md` | Lust runs as Lua while editing, and as Rust in built games | Accepted |
| `0029-the-sample-s-rust-component-is-built-in.md` | The turret sample's Rust component is built in until Phase 10 | Accepted (an exception, until Phase 10) |
| `0030-scale-is-tested-at-three-levels.md` | Scale is tested at three levels: target, stress and ceiling | Accepted |
| `0031-the-job-system-is-rayon.md` | The job system is the `rayon` library | Accepted |
| `0032-lods-made-on-import-or-the-model-s-own.md` | Levels of detail are made on import, or come from the model | Accepted |
| `0033-game-code-sees-the-start-of-the-tick.md` | During a tick, game code sees the world as it was when the tick started | Accepted |
| `0034-a-raw-interrupt-for-the-time-limits.md` | A raw interrupt for game code's time limits (the one `unsafe` module) | Accepted |
| `0035-game-code-spawns-into-the-area-while-playing.md` | Game code spawns into the area while playing | Accepted |
| `0036-physics-is-rapier-strict-on-every-pc.md` | Physics is the Rapier library, strict on every PC | Accepted |
| `0037-input-as-named-actions.md` | Input as named actions | Accepted |
| `0038-save-games-hold-everything.md` | Save games hold everything | Accepted |
| `0039-game-ui-is-drawn-in-solis.md` | Game UI is drawn by the engine itself | Accepted |
| `0040-a-project-s-preferred-language.md` | A project's preferred language | Accepted |
| `0041-camera-controls.md` | The ready-made cameras' controls | Accepted |
| `0042-a-hardening-milestone-before-phase-7.md` | A hardening milestone before Phase 7 | Accepted |
| `0043-installed-and-updated-from-github.md` | Installed on the computer, and updated from GitHub | Accepted (adds to 0010; changed by 0045) |
| `0044-third-party-notices-ship-with-the-program.md` | The libraries' notices ship with the program | Accepted |
| `0045-a-splash-screen-that-asks-before-updating.md` | A splash screen that asks before updating | Accepted (changes how 0043 updates) |
| `0046-light-before-netcode.md` | Light, sky and atmosphere get a phase of their own; netcode moves later | Accepted |
| `0047-the-effects-suite-is-solstice.md` | The effects suite is called Solstice | Accepted |
| `0048-materials-and-shaders-before-effects.md` | Materials and shaders get a phase, between light and effects | Accepted |
| `0049-amd-first-when-a-choice-must-be-made.md` | When a choice between graphics card makers must be made, which comes first | Accepted |
| `0050-one-way-to-work-everywhere.md` | One way to work, everywhere in the editor | Accepted |
| `0051-understand-then-build.md` | Understand, then build: the engine's own designs for its core systems | Accepted |
| `0052-dependency-safeguards.md` | Safeguards against dependency trouble | Accepted |
| `0053-compatibility-before-2d.md` | Compatibility comes first: updates never lose work | Accepted |
| `0054-first-class-2d.md` | First-class 2D, with game UI and a playtest export right after it | Accepted |
| `0055-lua-speed-measured-and-explained.md` | Lua's speed: measured, sped up where it's safe, and explained honestly | Accepted |
| `0056-sound-with-kira.md` | Sound with the Kira library: WAV and Ogg, sound events, and sound that never changes the game | Accepted |
| `0057-animation-near-live-far-baked.md` | Animation: live poses near, baked clips far, one state machine for 2D and 3D | Accepted |
| `0058-listeners-and-who-hears-what.md` | A Listener component, and "Heard by" on sound events | Accepted (extends 0056) |
| `0059-2d-first.md` | 2D first: Phase 7 is sound only, animation's groundwork moves to 2D, 3D animation becomes Phase 16 | Accepted (reorders 0054 and 0057) |
| `0060-amber-and-colours-that-mean-something.md` | Amber by default; Play, Stop, warnings and kinds keep their own colours | Accepted (changes 0004's standard accent) |
| `0061-settings-window-and-no-layout-presets.md` | Edit → Settings… with pages and a search box; layout presets removed, Reset layout kept | Accepted (changes 0004) |
| `0062-2d-on-the-ground-drawn-small-scaled-whole.md` | 2D on the ground seen from above, 2D cameras, `pixels.solpix`, the pixel stage and the 2D view | Accepted (builds on 0054) |
