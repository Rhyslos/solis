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
| `0063-sprites-read-from-aseprite-cut-out-front-from-the-foot.md` | Sprites read from `.aseprite` files by our own reader, one atlas per file, cut out with front and behind from the foot, colours as painted | Accepted (builds on 0054 and 0062) |
| `0064-frame-animation-clips-files-and-tabs.md` | Clips from Aseprite tags and `.solframes` clips files, the Clips and Frames tabs, clip time as game state, events as sounds and `AnimationCue`, Preview animation | Accepted (builds on 0057 and 0063) |
| `0065-engine-api-guidelines.md` | Game code's calls follow the project lead's API guidelines for Lust: methods on things, the project's names as states, editor-made things reached through files the editor writes | Accepted (changes 0057's choice 9 and 0064's event names) |
| `0066-animator-state-machine.md` | Animators (`.solanim`): parameters, states playing a clip or a clip by facing, transitions with conditions, stepped each tick; the Animator tab on the shared graph editor; saves version 4 | Accepted (builds on 0057, 0064 and 0065) |
| `0067-tile-maps.md` | Tile maps: terrains in a tileset file, a map file per map placed by a Tile Map, the map's own layers and cell values, in the checksum and saves (version 5); the Map tab | Accepted (builds on 0059, 0063 and 0065) |
| `0068-tile-art-and-autotiling.md` | Tile art and autotiling: a picture per tileset, terrain kinds (16-tile, 47-tile, edges, rules), pieces and variations, missing tiles pink, tileset format 2, the Tileset tab | Accepted (builds on 0067) |
| `0069-the-tile-painter.md` | The tile painter: tools on the viewport's tool bar (B, U, N, G, I), the Map tab's swatches as palette, copies flipped and turned, exact tiles in tile map format 2, a stroke one undo step | Accepted (builds on 0067 and 0068) |
| `0070-usable-first.md` | Usable first: the audit chose a revamp, a new build order around the farming game, blue as the standard accent, nothing shown that isn't built, New in the Library, a start page | Accepted (changes the build order and 0060's standard colour) |
| `0071-2d-physics-and-top-down-movement.md` | 2D physics and top-down movement: a separate 2D world (Rapier2D), solid terrains per tileset, walls merged per chunk, a Collider2D in pixels, the Walls switch, the kit's TopDownController, the arrow keys walk too | Accepted (builds on 0036, 0054, 0062, 0067 and 0070) |
| `0072-lust-alpha-59-and-the-engine-folder.md` | Lust alpha 59: the engine's folder found from the first line of its description files, the compiler's notes in the Console, `this` instead of `me` | Accepted (builds on 0002, 0065 and the Lust proposal of 2 November) |
| `0073-world-and-entity-calls-as-methods.md` | World and entity calls as methods (`position()`, `target:moveTo(p)`, `World:say`, `Saves:saveTo`), the project's assemblies as `state Assembly` in `solis/project.lst`, old calls as mistakes that say the new way | Accepted (carries out 0065 for the world and entity calls; builds on 0072) |
| `0074-input-calls-on-input-with-actions.md` | Input calls on `Input` (`Input:pressed(Action.Jump)`), the project's input actions as `state Action` in `solis/project.lst` | Accepted (carries out 0065 for input; builds on 0037 and 0073) |
| `0075-physics-calls-on-the-entity-and-one-ray-call.md` | Physics calls on the entity (`push`, `velocity`, `walk`, `grounded`), one call for a ray (`Physics:castRay` gives a `RayHit`) | Accepted (carries out 0065 for physics; builds on 0036, 0071 and 0073) |
| `0076-sound-calls-on-audio-with-sounds-and-buses.md` | Sound calls on `Audio` and the entity, the project's sound events and buses as `state Sound` and `state Bus`, all-capital names folded (`Bus.Ui`) | Accepted (carries out 0065 for sound; builds on 0056, 0073 and 0074) |
| `0077-clips-and-cues-as-names.md` | Clips and cues as the project's names (`Clip.Sprites.Farmer.Walk`, `Cue.Footstep`), `animate` on the entity, `CueReached` and `ClipFinished` carrying them | Accepted (carries out 0065 for clips; builds on 0064 and 0073 to 0076) |
| `0078-animators-reached-through-their-own-files.md` | Each animator's own Lust file (`FarmerAnimator`, `FarmerState`), held by a component as a field the engine fills in | Accepted (carries out 0065's rule 4 for animators; builds on 0057, 0077 and Lust alpha 52's filled fields) |
| `0079-tile-maps-reached-through-their-own-files.md` | Each tile map's own Lust file (`FarmMap`, `FarmLayer`, `FarmTerrain`), held by a component as a field Solis fills in | Accepted (carries out 0065's rule 4 for tile maps; builds on 0067 and 0078) |
| `0080-objects-change-only-in-their-own-update.md` | Game-wide objects tick first on one core, are copied to the others, change only in their own update, and are kept in saves | Accepted (builds on 0033 and 0038) |
| `0081-what-solis-makes-is-lust.md` | Everything Solis makes but the project file is a Lust data file, `<name>.<kind>.lst`; older files move to Lust when a project opens | Accepted (carries out 0070's second step) |
| `0082-the-2d-follow-camera-and-cells-at-places.md` | The kit's FollowCamera2D, and turning places into a map's cells and back | Accepted |
| `0083-game-ui-on-any-screen.md` | Game UI's anchors, scale rules and screen previews; pictures cut from sheets | Accepted |
| `0084-game-ui-text.md` | Game UI text: `ab_glyph` and single-channel distance images in one atlas, instead of the plan's `parley` and `fdsm` | Accepted (carries out 0083's text) |
| `0085-the-editor-overhaul.md` | The editor overhaul from the 4 November mock-up, in seven milestones before the game UI editor | Accepted (carries out 0070's UI overhaul; builds on 0004) |
| `0086-the-launcher.md` | The launcher inside Solis: projects (pin, search, sort, rename, copy, scan, the build each was saved with), Learn and Updates | Accepted (grows 0070's start page; builds on 0043 and 0045) |
| `0087-screens-through-the-engine.md` | Screens are Lust's own views read through the engine (objects, `Ui:entity()`, a reading method per component); no new Lust words | Accepted (replaces the game UI plan's choices 3 and 4) |
| `0088-menus-send-events.md` | Menus: buttons send events into the next tick's input; `showScreen` and `hideScreen` on the entity, screens as project states | Accepted (builds on 0087) |
| `0090-always-through-the-launcher.md` | Solis always starts on the launcher in a small window; closing the editor returns to it; a newer build is a bar in the launcher, not a question at start | Accepted (changes 0086 and 0045) |
| `0091-player-settings-and-the-window.md` | The game's own menu offers text size, UI size, fullscreen and resolution through `Screens:` and `Window:` calls; Alt+Enter switches fullscreen; the settings are the player's, not the game's | Accepted |
| `0092-day-and-night-and-lamps-on-sprites.md` | Day and night on the Environment (length, start, colour stops) with `World:timeOfDay` and `setTimeOfDay`, kept in saves; Light sprites lights sprites by the day and the lamps; Only at night on every Light | Accepted |
| `0093-the-playtest-export.md` | File > Export for playtesting: Solis renamed after the game with `playtest.txt` and the project in `game/`, opening straight into the game; `Window:quit()`; the player's settings, saves, log and crash log in Saved Games/<Game> | Accepted |
| `0094-parallax-layers.md` | Parallax layers: a Parallax component on a sprite (scroll across and up, repeat, in front), drawn where placed when the camera looks at it; behind ones before everything, slowest first; each layer its own small picture, slid smoothly, through a 2D camera | Accepted |
| `0095-rules-from-the-video-editor.md` | Rules from the project lead's video editor project: no name-dropping, folder READMEs and CLAUDE.md always current, readable code, competing approaches compared for big systems | Accepted |
| `0096-graphite-theme-and-rounded-panels.md` | A second theme, Graphite, and rounded panels as a separate switch, in Settings > Appearance; colours that mean something unchanged | Accepted |
| `0089-atlas-crop-and-the-ui-editor.md` | Atlas Crop (pictures cut from sheets, `.atlas.lst`), the UI editor's approved layout, and screens' looks, pictures and text size | Accepted (renames 0083's Cut out pictures; builds on 0087 and 0088) |
