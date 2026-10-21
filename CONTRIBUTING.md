# Contributing to Solis

Solis is a free, open-source 3D game engine with high standards for stability and feel. These rules keep it that way.

## Who can contribute

**Solis isn't accepting outside contributions yet: it is too early in development.** The design still changes quickly. It is built by the project lead and Claude together: the project lead writes some of the code (issues tagged by difficulty are left for that), and Claude writes much of the rest. Pull requests from others won't be merged yet; please don't spend time on them.

**Bug reports and ideas are welcome** through Issues.

## Setting up

1. Install Rust with [rustup](https://rustup.rs). The Rust version is pinned in `rust-toolchain.toml` and installs automatically.
2. Clone the repository and run:

```
cargo run -p app                # build and start the editor
cargo test --workspace          # run all tests
cargo clippy --workspace --all-targets -- -D warnings   # lint
cargo fmt --all                 # format
cargo doc --workspace --no-deps --open                  # read the API docs
```

Optional, matching CI: `cargo install cargo-deny` and run `cargo deny check`.

**GPU tests** (`cargo test -p render --test gpu`) draw real frames and check their pixels. Without a GPU they skip with a note; set `REQUIRE_GPU=1` to make a missing GPU a failure instead, as CI does. Without a graphics card, Mesa's software driver works: add `WGPU_BACKEND=gl`.

**The window test** runs on Linux only, because CI's cheapest machines are Linux ones with a virtual screen (Solis itself is made for Windows, [decision 0012](docs/decisions/0012-windows-only.md)). It opens the real editor on a virtual screen, as CI does. It checks the viewport's colours, picks a tool with its key, flies the camera, drags a panel to a new place, quits with File > Quit, and checks that the next launch puts the panel and camera back. Pass the program's name (`APP_NAME` in `crates/app/src/branding.rs`) as the window title:

```
sudo apt-get install xvfb xdotool imagemagick mesa-vulkan-drivers libxkbcommon-x11-0
cargo build -p app
xvfb-run -s "-screen 0 1440x900x24" tools/smoke/window-smoke.sh target/debug/solis "Solis" smoke-output
```

**The scale check** (`tools/scale/scale-check.sh`) times the editor and the viewport against their targets. It runs at the end of every phase and whenever the project lead asks, not on every pull request ([decision 0011](docs/decisions/0011-scale-check-at-phase-gates.md)).

## Third-party material

Anything embedded in the program itself (fonts, icons, images) needs its licence text in `licenses/`, listed in `licenses/README.md`; that folder ships with every build. Code libraries are covered by `cargo deny`, and must use a licence compatible with MIT ([decision 0007](docs/decisions/0007-mit-licence-and-separate-from-aventail.md)).

Never add material copied from Unreal Engine, Unity or any other proprietary software. Recreating layout and behaviour is fine; copying icons, images, names or code is not. Aventail's code may be copied in and adapted, since its owner relicenses it for Solis (decision 0007).

## Ground rules

A pull request that breaks one of these is sent back, however good the feature.

**Structure**

1. **One crate, one job.** New code goes in the crate whose job it is (see each crate's README). If none fits, propose a new crate first.
2. **Dependencies point down only.** The engine knows nothing of the editor; games ship without it. `tools/layer-check` enforces the layers in `ARCHITECTURE.md`.
3. **Small units.** As a guide: files under about 500 lines, functions under about 60.
4. **Narrow public surface.** Expose as little as possible from each crate.
5. **Reuse before writing.** Check whether a crate already does it.
6. **Game-specific systems stay out of the engine.** The engine meets the scale target in DESIGN.md §2; squads, campaigns and the like belong in games.

**Safety**

7. **No `unsafe` code** without a decision record. The workspace denies it.
8. **No crashes on bad input.** Library code never uses `unwrap`, `expect` or `panic!` outside tests (enforced by clippy). Return errors that explain the problem in plain words.
9. **Untrusted files are hostile.** Every reader checks sizes and counts before allocating, and gets a fuzz test: thousands of randomly damaged inputs, none of which may crash it. Saved settings count as files.
10. **The frame loop never waits** on the GPU, files or the network; slow work runs on other threads and reports back with messages.
11. **Never lose work.** Saving writes to a temporary file and swaps it in; edits are undoable.

**Working with existing code**

12. **Read before changing:** the crate's README and tests first.
13. **Change the smallest thing that works.** Refactors get their own pull request.
14. **Tests move with the code.** Bug fixes add a test that reproduces the bug; features add tests for their behaviour.
15. **Documentation changes with the code.** See below.
16. **The program's name lives in `branding.rs`.** Never hard-code it; see `RENAMING.md`.

**Feel**

17. **Ask before adding controls.** A feature that adds keys or mouse actions starts with a question to the project lead: a recommended choice (common in game engines, no clashes) and alternatives. If there's no answer, use the recommended choice and say so at the top of the pull request.
18. **Speed is a feature.** Code on a performance path gets a benchmark or a scale-check entry, and the editor stays under its frame target with everything full (a full console, a big scene).

## Documentation rule

A change is not done until its documentation is.

| Document | Update when |
| --- | --- |
| Code comments (`///` and `//!`) | The item changes. Missing docs fail the build. |
| Crate `README.md` | The crate's purpose or limits change |
| `ARCHITECTURE.md` | A crate is added, removed or moved, or a data flow changes |
| `CONTRIBUTING.md` | Tooling or rules change |
| `DESIGN.md`, `ROADMAP.md` | Scope or the plan changes |
| `docs/decisions/` | A major choice is made or reversed |
| User manual (`docs/manual/`) | Anything a user would notice. Follow `docs/manual/_template.md`. |
| `CHANGELOG.md` | Any change a user would notice (CI checks this) |
| README progress tracker | A milestone or gate is completed |
| `RENAMING.md` | The program's name appears somewhere new |

## Testing: two sign-offs per pull request

1. **Code tests** by the author: normal cases, edge cases, failure cases (bad input gives an error, never a crash), a regression test for every bug fixed, round trips for anything that reads and writes a format, fuzz tests for anything that reads files, and benchmarks or scale-check entries for code on a performance path. Run the full suite and report the results in the pull request.
2. **Functional test** by the project lead: the "How to test" list in the pull request, tried in the editor.

Both must pass before merging.

## Pull requests

- One milestone or one fix per pull request, on its own branch (for example `phase1/models`).
- Fill in the pull request template: what changed, test results, docs updated, how to test.
- All CI checks must be green. CI runs only when the pull request is ready for review ([decision 0001](docs/decisions/0001-builds-on-demand.md)), so run the checks locally first.
- If users wouldn't notice the change, label the pull request `internal` and the changelog check lets it through without an entry.
- Commit messages: a short summary line in plain words ("Add the fly camera"), then details if needed.

## AI assistance

Much of the code is written with AI assistance (Claude by Anthropic), and commits say so with a `Co-Authored-By` line. AI-written changes follow exactly the same rules and reviews as anyone else's.

## Licensing

Solis is licensed under the MIT licence (see `LICENSE`). Write your own code, or adapt Aventail's: nothing copied from proprietary engines. File formats are implemented from public specifications.
