# app/src

The program's code: the command line, the window and its frame loop, the GPU hookup, and the slow work (files, imports, updates, sound) done on workers. It holds no feature logic; it connects the editor (`editor` crate) to the engine crates.

**Start reading at** `main.rs`, then `window.rs`.

## The program and its window

| File | What it does | Main items |
| --- | --- | --- |
| `main.rs` | The entry point: reads the command line, then opens the editor, or prints the version or help. | `main` |
| `lib.rs` | The library part shared by `main.rs` and the tests: lists the modules, builds the version banner and the editor's `AppInfo`. | `version_banner`, `app_info` |
| `branding.rs` | The program's name and identity, in one place. Never write the name anywhere else (see `RENAMING.md`). | `APP_NAME`, `APP_ID` |
| `cli.rs` | Reads the command line: `--version`, `--help`, or open the editor (with `--import <file>`). | `Command`, `parse`, `usage` |
| `startup.rs` | Starting up behind the splash screen, which asks for a newer build first (decision 0045). Tests in `startup_tests.rs`. | `Startup`, `Next` |
| `window.rs` | The editor window: opens it with eframe, restores saved settings, hands each frame to the workbench, carries out its actions, and starts the job pool. | `run`, `start_jobs` |
| `gpu.rs` | Connects the `render` crate to the window's GPU device and to egui: one image for the viewport, one for the Game panel; sprite sheets and tile map layers' pictures uploaded. | `GpuViewport` |
| `logger.rs` | Passes what libraries write through the `log` facade to the Console: every library's warnings and errors, information only from the program's own crates. | `Messages`, `Level`, `install` |
| `gamepad.rs` | Reads the first gamepad each frame on Windows and hands it to the editor as `input::Pad` (decision 0037). | `Gamepads` |

## The project and its files

| File | What it does | Main items |
| --- | --- | --- |
| `project.rs` | The open project: its asset database, the watcher on its assets folder, and the recent projects list (decision 0021). | `Projects`, `Opened` |
| `area_file.rs` | Reading and saving area files, the Open and Save windows, and noticing when the open file changes on disk (decision 0020). | `AreaFiles` |
| `import.rs` | Imports models on a small pool of workers (through the import cache when a project is open), so the window never waits; also the system's Open windows. | `Importer`, `Finished`, `FolderFor` |
| `thumbnails.rs` | The Library's thumbnails: pictures shrunk on a worker, models drawn once on the GPU. | `Thumbnails` |
| `game_code.rs` | The project's game code on a worker: writes the files describing the engine to Lust, and reads, checks and turns the project's Lust into Lua whenever its assets change. | `GameCode`, `Checked` |
| `play_files.rs` | Files for playing: the input actions (`input.solinput`), the buses (`mixer.solmix`; a damaged one kept as a backup when saved again, an older one copied to the backups), recordings of play (`recordings/*.solrec`) and save games, each read within limits. | `PlayFile`, `RecordingDialog`, `save_game` |
| `check_project.rs` | `--check-project <folder>` (Phase 8, C4): opens a project without the window, reads every file, checks the game's code, and plays back `recordings/checks/` in their areas headless; a plain report, and a failure code if anything changed. Tests in `check_project_tests.rs` (the samples, a changed area, damaged files). | `check`, `Report`, `workbench_for` |
| `upgrades.rs` | Older files as the editor meets them (Phase 8, C1): a Console note when one is opened (it's upgraded in memory), and saving through `asset_db::backup`, which copies the older file first. Every format's example files, in `tests/fixtures/`, are opened by `fixtures_tests.rs`. | `older_note`, `save`, `kept_note` |
| `sound.rs` | Opens the sound card on a worker, using a silent device until it is open or when there is none. | `Sound`, `Opener` |
| `playground_sounds.rs` | The playground's sounds, made in code (CC0): footsteps, music, a radio's chatter and a drip, as WAV files, the same every time. | `footstep`, `music`, `radio`, `drip` |
| `game_audio.rs` | The game's sounds each frame: picks and plays what game code asked for through its bus, around the camera, following entities, held while paused and faded on Stop; ducking, mute, solo and `setBusVolume` as the buses' levels, and the meters back to the Mixer; `AudioSource` loops, and choosing the most important sounds to hear (S5); the reverb zones the camera is in (S6); only what the listener should hear (S8). | `GameAudio` |
| `sounds.rs` | The project's sound files and sound events in the editor: sounds measured and events read on a worker for the Library and Properties, and listened to through the sound device (an event plays a new pick each press). | `Sounds` |
| `sprites.rs` | The project's sprite sheets (Phase 9, D2): Aseprite files read and packed on a worker, again whenever one is saved, a failed read keeping the last good one; clips files, animators, tilesets, tile maps and tilesets' pictures read beside them. | `Sprites`, `read_sheet`, `read_tileset`, `read_tile_map`, `read_art` |
| `updates.rs` | Keeping the program up to date while it runs: Help > Check for updates, downloading while you work, installing on close (decisions 0043 and 0045). | `Updates`, `Pending` |

## Ready-made projects (Help menu)

| File | What it does |
| --- | --- |
| `sample_game.rs` | The turret sample (Help > Make the sample game): a turret in Lust shooting a drone with armour in Rust and sparks in Lua. Its files come from `samples/turret/assets`. Tests in `sample_game_tests.rs`. |
| `benchmark.rs` | The benchmark (Help > Make the benchmark, decision 0030): two fleets of ships and lasers at three sizes. Its game code comes from `samples/benchmark/assets`. |
| `playground.rs` | The playground (Help > Make the playground): a level for trying the character, cameras, physics, saves and sound (footsteps, music, a radio ducking it, a cave that echoes). Its game code comes from `samples/playground/assets`. Tests in `playground_tests.rs`. |

## How the pieces fit

`main.rs` calls `cli::parse`, then `window::run`, which shows the splash (`startup.rs`) and then the editor. Each frame, `window.rs` asks the editor's `Workbench` to draw itself, passing it a `GpuViewport` (`gpu.rs`) to draw the 3D views, and acts on the actions it returns; anything slow goes to a worker (`import.rs`, `game_code.rs`, `thumbnails.rs`, `sounds.rs`, `sprites.rs`, `updates.rs`) and is picked up between frames.

**Read next:** the crate's [README](../README.md).
