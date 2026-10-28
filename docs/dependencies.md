# Dependencies: the register and the safeguards

Solis is built on other people's libraries, which saves years of work. Each library is also something Solis relies on: it can stop being maintained, change its licence, break in an update, or disappear. This page is how Solis stays safe from that "dependency hell" ([decision 0052](decisions/0052-dependency-safeguards.md)).

## The safeguards

1. **The register** (below). Every library Solis's crates use is listed, with:
   - what it does for Solis;
   - its licence;
   - which crates may use it;
   - how deep it reaches;
   - an **exit plan**: what Solis does if the library goes bad.

   A test (`tools/layer-check`, run by CI) fails if a crate uses a library that isn't listed, uses one in a crate the register doesn't allow, or if the register lists a library or a use that no longer exists.
2. **Walls.** A library is used in as few crates as possible, ideally one, behind Solis's own types, so replacing it means rewriting one crate, not the engine. The "allowed in" column is the wall, and the test holds it.
3. **Licences are checked on every build.** `cargo deny check` (CI, `deny.toml`) fails if any library, including the libraries libraries use, has a licence that isn't on the allowed list.
   - **A library can't slip into another licence unnoticed:** if an update changes its licence to one not allowed, the update fails.
   - **The version Solis uses keeps its licence:** a library that changes its licence can usually only do so for new versions, and the version already published stays under the licence it was published with. Solis stays on the last good version and replaces the library in its own time. This is general understanding, not legal advice.
4. **Exact versions, from known places.**
   - `Cargo.lock` pins every version, and nothing updates on its own. Dependabot proposes updates as pull requests, checked like any change.
   - Libraries come only from the official registry, or from the project lead's own Lust repository pinned to one commit (`deny.toml`, `[sources]`).
   - Removed ("yanked") versions fail the check.
5. **A copy of every library's source.** Whenever `Cargo.lock` changes on main, the **Source archive** workflow saves the source of every library in it (`cargo vendor`, about 60 MB compressed) as a pre-release on GitHub, named after the lock file's hash. A library deleted from the internet can't stop Solis from building: unpack the archive and build offline (see "Building from the archive").
6. **Known problems are watched.** The weekly security check reads the RustSec advisories, and every exception in `deny.toml` carries its reason.
7. **A reason for every new library.** Adding one needs a line in the register (the test insists), and the pull request says why Solis shouldn't write it itself. Solis writes its core systems itself ([decision 0051](decisions/0051-understand-then-build.md)), so the list should shrink in the core over time.
8. **A look at every phase's end.** The phase review (decision 0027) reads this register:
   - Is each library still maintained?
   - Has its reach grown?
   - Is an exit plan now cheap enough to just do?

## The register

**Reach** says how deep a library goes:
- **walled:** one crate, behind Solis's own types;
- **shared:** small and used in several crates (maths, data formats), each use simple;
- **foundation:** Solis is built on it and replacing it would be a large job, an accepted risk (each is large, maintained and permissively licensed);
- **tests** or **build:** never in the program itself.

**Allowed in** lists the crates that may use it. `(tests)` means in tests only, `(build)` in the build script only.

| Library | What it does for Solis | Licence | Allowed in | Reach | Exit plan |
| --- | --- | --- | --- | --- | --- |
| `base64` | Decodes models' embedded data (glTF `data:` addresses) | MIT OR Apache-2.0 | assets | walled | Write it (about 100 lines, tested against the standard's examples) |
| `bytemuck` | Turns Solis's structs into bytes for the graphics card, safely | Zlib OR Apache-2.0 OR MIT | app, assets, editor, render | shared | Stay on the last good version (it's small, stable and needs nothing else); or write the few safe casts Solis uses |
| `chrono` | Times on Console messages, and the names of backup folders | MIT OR Apache-2.0 | asset_db, editor | walled | Use the standard library's clock and format the time ourselves |
| `eframe` | Opens the editor's window and runs egui in it | MIT OR Apache-2.0 | app | foundation | Stay on the last good version; then a window of our own (`winit`, already underneath) with egui drawn by Solis's renderer |
| `egui` | The editor's whole interface | MIT OR Apache-2.0 | editor | foundation | Stay on the last good version, or maintain a fork (MIT); a different interface library would mean rewriting the editor's panels |
| `egui_dock` | Dockable panels and tabs | MIT | editor | walled | Fork it (MIT, small) and maintain it ourselves, or write docking into the editor |
| `egui_kittest` | Drives the editor in tests, as a user would | MIT OR Apache-2.0 | editor (tests) | tests | Keep the last version for tests, or test the editor's state without driving its interface |
| `gilrs` | Reads gamepads | Apache-2.0/MIT | app | walled | Read gamepads through Windows directly (XInput or GameInput) |
| `glam` | Vectors, rotations and matrices: Solis's maths | MIT OR Apache-2.0 | app, area, asset_db, assets, editor, input, physics, reflect, render, script, world | foundation | Fork it (MIT or Apache-2.0); it's self-contained, so keeping a fork is cheap |
| `gltf` | Reads glTF models | MIT OR Apache-2.0 | assets | walled | Fork it, or read glTF ourselves (the format is a public specification: JSON plus binary) |
| `hecs` | The game's fast storage for entities | MIT OR Apache-2.0 | world | walled | Solis's own storage: `tools/storage-bench` already has one, measured against `hecs` for decision 0024 |
| `hound` | Reads WAV sound files | Apache-2.0 | sound | walled | Read WAV ourselves (a simple, public format: a header and the samples) |
| `flate2` | Unpacks the zlib-packed pictures inside Aseprite files | MIT OR Apache-2.0 | assets | walled | Use `miniz_oxide` directly, or write an inflater |
| `image` | Reads PNG and JPEG textures | MIT OR Apache-2.0 | assets, app (tests) | walled | Use smaller PNG and JPEG readers, or fork |
| `kira` | Plays and mixes sound: tracks, effects, sound placed in 3D (decision 0056) | MIT OR Apache-2.0 | sound | walled | Fork it (MIT or Apache-2.0), or mix ourselves on top of `cpal` underneath; only the sound crate changes |
| `lewton` | Reads Ogg Vorbis sound files | MIT OR Apache-2.0 | sound | walled | Fork it (no release since 2021, so Solis's own fuzz tests vet it), or another permissive Vorbis reader |
| `log` | Passes messages to the Console | MIT OR Apache-2.0 | app, render, sound, updater | shared | A few lines of Solis's own logging |
| `lust` | Lust, the project lead's language (Solis's game code) | MIT | script | walled | Ours: the project lead's own repository, pinned to one commit |
| `meshopt` | Makes levels of detail (simpler meshes for far away) | MIT OR Apache-2.0 | assets | walled | Write a simplifier (quadric error, a known method), or fork; meshoptimizer underneath is MIT |
| `mint` | Hands places and turns to Kira (the shared maths types it takes) | MIT | sound | walled | Kira's own; would go with it |
| `mlua` | Runs Lua and Luau game code, sandboxed | MIT | script | walled | Fork it (MIT), or bind to Luau ourselves (Luau is MIT); only the script crate changes |
| `notify` | Notices when files change on disk | CC0-1.0 | asset_db | walled | Ask Windows directly (`ReadDirectoryChangesW`), or check files on a timer |
| `notify-debouncer-mini` | Groups bursts of file changes into one | MIT OR Apache-2.0 | asset_db | walled | Group them ourselves (a short timer) |
| `ogg` | Reads the Ogg pages a Vorbis file is packed in, so Solis can check its comment header before `lewton` reads it | BSD-3-Clause | sound | walled | Read Ogg pages ourselves (a simple, public format) |
| `opener` | Opens web pages and folders | MIT OR Apache-2.0 | app | walled | Ask Windows directly (`ShellExecute`) |
| `percent-encoding` | Decodes `%20`-style names in model files | MIT OR Apache-2.0 | assets | walled | Write it (a few lines) |
| `pollster` | Waits for the graphics card in tests | Apache-2.0/MIT | app (tests), render (tests) | tests | Write it (about 20 lines) |
| `proptest` | Tests with thousands of generated cases | MIT OR Apache-2.0 | area (tests), editor (tests), physics (tests), script (tests), sound (tests), updater (tests) | tests | Keep the last version (tests only), or another property-testing library |
| `rapier3d` | Physics, the same on every PC (decision 0036) | Apache-2.0 | physics | walled | Fork it (Apache-2.0) and maintain it; a physics engine of our own is a large job, and only the physics crate would change |
| `rayon` | The job system that uses every core (decision 0031) | MIT OR Apache-2.0 | app, editor, physics, render, world | shared | Fork it, or a job system of Solis's own (the uses are simple parallel loops) |
| `rfd` | Open and Save file windows | MIT | app | walled | Ask Windows directly (its common file dialogs) |
| `ron` | Reads a test file format in tests | MIT OR Apache-2.0 | editor (tests) | tests | Drop it from the tests |
| `serde` | Turns data into files and back | MIT OR Apache-2.0 | asset_db, editor, input, reflect, updater | shared | Stay on the last good version (it's the standard, with a vast following); or Solis's own readers and writers through `reflect` |
| `serde_json` | Reads GitHub's answers in the updater | MIT OR Apache-2.0 | updater | walled | Read the few fields the updater needs ourselves |
| `sha2` | Checks downloaded updates (SHA-256) | MIT OR Apache-2.0 | updater | walled | Write SHA-256 (about 150 lines, tested against the standard's examples) |
| `toml` | Reads and writes settings and input files | MIT OR Apache-2.0 | asset_db, input, reflect (tests) | shared | Fork it, or read the small part of TOML Solis uses ourselves |
| `trash` | Moves deleted files to the Recycle Bin | MIT | asset_db | walled | Ask Windows directly (`IFileOperation`) |
| `ureq` | Talks to GitHub for updates | MIT OR Apache-2.0 | updater | walled | Use Windows' own web client (WinHTTP), or another client |
| `wgpu` | Talks to the graphics card (DirectX 12, Vulkan) | MIT OR Apache-2.0 | render | foundation | Stay on the last good version, or maintain a fork (MIT or Apache-2.0); it's walled into `render`, so the rest of Solis never sees it |
| `winresource` | Puts the logo into `solis.exe` when building | MIT | app (build) | build | Write the small resource file ourselves and run Windows' resource compiler |

Outside the register: the tools in `tools/` and the fuzzing workspace in `fuzz/`, which never ship. Their libraries are still checked by `cargo deny` where they share the workspace.

## Patches: libraries Solis stands in for

When a library Solis needs depends on something whose licence isn't allowed, and the dependency can't be switched off, Solis writes a small permissive stand-in with the same interface, and the root `Cargo.toml`'s `[patch.crates-io]` points the library at it. Each stand-in lives in `patches/`, is written for Solis from the interface alone (no code taken from the original), and has its own tests.

| Stands in for | Needed by | Original licence | What it does | Stand-in uses |
| --- | --- | --- | --- | --- |
| `triple_buffer` | `kira` | MPL-2.0 | Hands the newest command to the audio thread without waiting | `crossbeam-utils` (MIT OR Apache-2.0), already in Solis through other libraries |

If the library drops the dependency, or changes what it uses, the patch is removed or updated; `cargo deny check` fails if the original ever comes back.

## Building from the archive

If a library can no longer be downloaded:

1. Download `sources-<hash>.tar.xz` from the GitHub release whose tag matches the `Cargo.lock` you're building (the release notes give the lock file's hash).
2. Unpack it into the repository: `tar -xJf sources-<hash>.tar.xz`. This makes a `vendor/` folder.
3. Tell Cargo to use it: add to `.cargo/config.toml`:

   ```toml
   [source.crates-io]
   replace-with = "vendored-sources"

   [source."git+https://github.com/Rhyslos/Lust?rev=<rev>"]
   git = "https://github.com/Rhyslos/Lust"
   rev = "<rev>"
   replace-with = "vendored-sources"

   [source.vendored-sources]
   directory = "vendor"
   ```

   The archive's `README-vendor.txt` holds the exact text, as written by `cargo vendor` for that lock file.
4. Build as usual. Nothing is downloaded.
