# app/src

The program's code: the command line, the window and its frame loop, the GPU hookup, and the slow work (files, imports, updates, sound) done on workers. It holds no feature logic; it connects the editor (`editor` crate) to the engine crates.

**Start reading at** `main.rs`, then `window.rs`.

## The program and its window

| File | What it does | Main items |
| --- | --- | --- |
| `main.rs` | The entry point: plays the exported game beside the program if there is one (decision 0093); otherwise reads the command line, then opens the editor, or prints the version or help. | `main` |
| `lib.rs` | The library part shared by `main.rs` and the tests: lists the modules, builds the version banner and the editor's `AppInfo`. | `version_banner`, `app_info` |
| `branding.rs` | The program's name and identity, in one place. Never write the name anywhere else (see `RENAMING.md`). | `APP_NAME`, `APP_ID` |
| `cli.rs` | Reads the command line: `--version`, `--help`, or open the editor (with `--import <file>`). | `Command`, `parse`, `usage` |
| `startup.rs` | Starting up behind the splash screen, which asks for a newer build first (decision 0045). Tests in `startup_tests.rs`. | `Startup`, `Next` |
| `window.rs` | The editor window: opens it with eframe, restores saved settings, hands each frame to the workbench, carries out its actions, and starts the job pool. | `run`, `start_jobs` |
| `external.rs` | The one place a file or folder is handed to the computer's own programs: a file opens only if its kind is safe to open (the editor's `LibraryKind::opens_in_its_program`, asked about the kind its own name gives) and is shown in its folder if not; a path the editor asks for must be inside the open project's assets. Tests in `external_tests.rs`, with sweeps of names and paths. | `External`, `Handed`, `open_externally` |
| `window_playtest.rs` | The window as an exported game (decision 0093): opens the project beside the program, plays its area once its code is ready, draws only the game, makes the window follow the player's settings, keeps saves, settings and a log in the player's folder, and writes a crash log on a crash; and File > Export for playtesting… carried out on a thread. A child module of `window.rs`. | `run_playtest` |
| `playtest.rs` | The playtest export (decision 0093): copying a project and the program into a folder, finding an exported game beside the program, the player's Saved Games folder, their settings file and the crash log. Tests in `playtest_tests.rs`, including fuzzed settings files. | `export`, `beside`, `saved_games`, `load_settings` |
| `gpu.rs` | Connects the `render` crate to the window's GPU device and to egui: one image for the viewport, one for the Game panel; sprite sheets and tile map layers' pictures uploaded; game UI laid out with the fonts it keeps (Inter regular and semibold) and drawn after the frame. | `GpuViewport` |
| `logger.rs` | Passes what libraries write through the `log` facade to the Console: every library's warnings and errors, information only from the program's own crates. | `Messages`, `Level`, `install` |
| `gamepad.rs` | Reads the first gamepad each frame on Windows and hands it to the editor as `input::Pad` (decision 0037). | `Gamepads` |

## The project and its files

| File | What it does | Main items |
| --- | --- | --- |
| `project.rs` | The open project: its asset database, the watcher on its assets folder, and the projects Solis knows (decision 0021); notes this build in each project it opens (decision 0086). | `Projects`, `Opened` |
| `known_projects.rs` | Every project the launcher knows (decision 0086): folder, pin, when last opened and the name last read, up to 200, saved with the settings and read once from the recent list kept before; each one's project file read for the launcher's rows; copying a project and scanning a folder on a worker (`Tidying`). | `KnownProjects`, `Known`, `Tidying` |
| `area_file.rs` | Reading and saving area files, the Open and Save windows, and noticing when the open file changes on disk (decision 0020). | `AreaFiles` |
| `import.rs` | Imports models on a small pool of workers (through the import cache when a project is open), so the window never waits; also the system's Open windows. | `Importer`, `Finished`, `FolderFor` |
| `thumbnails.rs` | The Library's thumbnails: pictures shrunk on a worker, models drawn once on the GPU. | `Thumbnails` |
| `game_code.rs` | The project's game code on a worker: writes the files describing the engine to Lust, and reads, checks and turns the project's Lust into Lua whenever its assets change. | `GameCode`, `Checked` |
| `play_files.rs` | Files for playing: the input actions (`input.solinput`), the buses (`mixer.solmix`; a damaged one kept as a backup when saved again, an older one copied to the backups), recordings of play (`recordings/*.solrec`) and save games, each read within limits. | `PlayFile`, `RecordingDialog`, `save_game` |
| `check_project.rs` | `--check-project <folder>` (Phase 8, C4): opens a project without the window, gives files just made their sidecars (`settle_new_files`, waiting as the editor would), reads every file, checks the game's code, and plays back `recordings/checks/` in their areas headless, with the sprite sheets, tilesets and tile maps read once (`read_sheets`, `Read`); a plain report, and a failure code if anything changed. Tests in `check_project_tests.rs` (the samples, a changed area, damaged files). | `check`, `Report`, `workbench_for` |
| `upgrades.rs` | Older files as the editor meets them (Phase 8, C1): a Console note when one is opened (it's upgraded in memory), and saving through `asset_db::backup`, which copies the older file first. Every format's example files, in `tests/fixtures/`, are opened by `fixtures_tests.rs`. | `older_note`, `save`, `kept_note` |
| `to_lust.rs` | Moving a project's older files to Lust when it opens (decision 0081): each is written as a Lust data file beside it, checked, its sidecar moved with it and the older file kept in the backups. | `move_older_files` |
| `sound.rs` | Opens the sound card on a worker, using a silent device until it is open or when there is none. | `Sound`, `Opener` |
| `playground_sounds.rs` | The playground's sounds, made in code (CC0): footsteps, music, a radio's chatter and a drip, as WAV files, the same every time. | `footstep`, `music`, `radio`, `drip` |
| `game_audio.rs` | The game's sounds each frame: picks and plays what game code asked for through its bus, around the camera, following entities, held while paused and faded on Stop; ducking, mute, solo and `setBusVolume` as the buses' levels, and the meters back to the Mixer; `AudioSource` loops, and choosing the most important sounds to hear (S5); the reverb zones the camera is in (S6); only what the listener should hear (S8). | `GameAudio` |
| `sounds.rs` | The project's sound files and sound events in the editor: sounds measured and events read on a worker for the Library and Properties, and listened to through the sound device (an event plays a new pick each press). | `Sounds` |
| `text_files.rs` | The text file shown in Properties, read when it's picked and again when it changes on disk, at most 512 KB, refused if it holds a zero byte. | `TextFiles`, `read_text` |
| `sprites.rs` | The project's sprite sheets (Phase 9, D2): Aseprite files read and packed on a worker, again whenever one is saved, a failed read keeping the last good one; clips files, animators, tilesets, tile maps, atlas files (Atlas Crop) and the pictures tilesets and atlases use read beside them. | `Sprites`, `read_sheet`, `read_tileset`, `read_tile_map`, `read_atlas`, `read_art` |
| `updates.rs` | Keeping the program up to date while it runs: Help > Check for updates, downloading while you work, installing on close (decisions 0043 and 0045). A copy that wasn't installed is only told a newer build is out. | `Updates`, `Pending` |

## Ready-made projects (Help menu)

| File | What it does |
| --- | --- |
| `sample_game.rs` | The turret sample (Help > Make the sample game): a turret in Lust shooting a drone with armour in Rust and sparks in Lua. Its files come from `samples/turret/assets`. Tests in `sample_game_tests.rs`. |
| `ui_sample.rs` | The UI sample (Help > Make the UI sample): a HUD, a health bar and a pause menu with the player's settings, every screen drawn on the canvas. Its files come from `samples/ui/assets`. Tests in `ui_sample_tests.rs`, including its recording checked with the menu used. |
| `benchmark.rs` | The benchmark (Help > Make the benchmark, decision 0030): two fleets of ships and lasers at three sizes. Its game code comes from `samples/benchmark/assets`. |
| `playground.rs` | The playground (Help > Make the playground): a level for trying the character, cameras, physics, saves and sound (footsteps, music, a radio ducking it, a cave that echoes). Its game code comes from `samples/playground/assets`. Tests in `playground_tests.rs`. |

## How the pieces fit

`main.rs` calls `cli::parse`, then `window::run`, which shows the splash (`startup.rs`) and then the editor. Each frame, `window.rs` asks the editor's `Workbench` to draw itself, passing it a `GpuViewport` (`gpu.rs`) to draw the 3D views, and acts on the actions it returns; anything slow goes to a worker (`import.rs`, `game_code.rs`, `thumbnails.rs`, `sounds.rs`, `sprites.rs`, `updates.rs`) and is picked up between frames.

**Read next:** the crate's [README](../README.md).
