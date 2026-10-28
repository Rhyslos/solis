# Working on Solis

Solis is a 3D game engine. Read DESIGN.md before any change; it is the plan, and its decision log records what's settled.

## Start of every session

1. Read `README.md` (what's built, what's next), `ROADMAP.md`, `ARCHITECTURE.md` and `CONTRIBUTING.md` (the ground rules).
2. Read the README of every crate you will touch.
3. Check `docs/decisions/` before changing anything a decision covers.

## Rules

- **Engine development has begun** (Phase 0 onward). Design changes still go through DESIGN.md and a decision.
- **Decisions go in two places:** a line in DESIGN.md §17, and a file in `docs/decisions/` numbered in order (`0002-short-name.md`) with Status, Date, Problem, Options where useful, Decision and Consequences.
- **The engine is Rust; games can be Lust, Rust or Lua** (decisions 0002 and 0003). Lust is taught first; game code docs show all three.
- **ROADMAP.md is the build order.** One milestone per pull request; a phase ends with its gate, which the project lead tries by hand.
- **Game-specific systems stay out of the engine.** The engine meets the scale target in DESIGN.md §2; squads, campaigns and the like belong in games.
- **Tandem work:** the project lead writes some of the code. Leave issues tagged by difficulty, explain what you build in plain words, and review their pull requests kindly and concretely.

## Builds (decision 0018)

The repo is public, so GitHub's machines are free (decision 0018, which replaces 0001).

- Checks run on every update to a non-draft pull request and on merges into main, on Linux and Windows, skipping docs-only changes. A weekly security check and Dependabot keep dependencies safe. The Windows EXE is built when code is merged into main (decision 0010). Solis is made for Windows only (decision 0012); no macOS.
- Keep workflows cheap anyway: cheap checks first, cancel older runs, cache builds, pin actions by commit.
- Run the full checks locally before pushing, and list the results in the pull request: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`, the GPU tests with `REQUIRE_GPU=1`, and `tools/smoke/window-smoke.sh` under `xvfb-run` (see ARCHITECTURE.md, Checks).

## Code

- Follow every ground rule in `CONTRIBUTING.md`; CI enforces many of them, review enforces the rest.
- Tests for every batch (normal, edge, failure, regression, round trip, fuzz, benchmarks as relevant), and the whole suite run before pushing. Anything that reads files or saved settings gets a fuzz test.
- Update documentation in the same pull request: code comments, crate READMEs, `ARCHITECTURE.md`, `CHANGELOG.md` (CI checks it), the **user manual**, and the README progress tracker when a milestone completes. Fill in the pull request template.
- **The user manual is required.** Any change a user would notice updates `docs/manual/` in the same pull request, following `docs/manual/_template.md`. Write for someone new to game engines: explain *why* as well as *how*, and link terms to `docs/manual/concepts.md` (add new terms there). The manual describes only what exists in `main`.
- **Code review (decision 0027):** at the end of each phase (before its gate) and whenever the project lead asks, run three separate reviewers over the code since the last review: stability, efficiency, and correctness where parts meet. Check each finding, then fix them in pull requests grouped by kind, with the scale check and new scale cases for slow spots found.
- **Scale check:** at the end of each phase, and whenever the project lead asks, run `tools/scale/scale-check.sh` and put its table in the pull request (decision 0011). Not on every pull request.
- **Every folder has a `README.md`** saying what's in it and how its parts fit (CONTRIBUTING.md, documentation). Read it before working in a folder, and update it in the same pull request when files are added, removed or renamed. Folders read as data (game assets, fuzz seeds, test data, the manual, fonts) are described in their parent's README instead.
- Any new place the program's name must appear gets added to `RENAMING.md` in the same pull request.
- The pull request's "How to test" list covers what automated tests can't (feel, looks), written for a non-programmer.
- Never hard-code the program's name: use `crates/app/src/branding.rs`.
- Ask the project lead before building a feature that adds keys or mouse actions: offer a recommended choice and alternatives (CONTRIBUTING.md, rule 17).
- **Merge every feature yourself** (the project lead, 2026-10-28). Once a pull request's CI checks are green, merge it without waiting. Its "How to test" steps go into the project lead's testing document, [Solis: things to try by hand](https://claude.ai/artifact/USEHoEYDqbVrNXJUBZ8T8g) (a Claude Doc, edited with the docs tools), as a new section at the top, newest first: a tick box per step, naming the pull request. Read its comments at the start of a session: they are the project lead's findings to fix. The project lead tries everything in one big testing pass once 2D and the effects suite are complete (2026-10-28); until then their only input is answering questions about architecture and signing off UI mock-ups, so don't wait on them for anything else.
- **UI is mocked up first** (the project lead, 2026-10-27). Before building any UI (game UI's canvas and screens, a new editor panel, tool or window), make a mock-up for the project lead to see and approve. The main inspiration is Unreal Engine's interface, unless the project lead brings their own design, which must always be mocked up first. Mock-ups are in-chat pictures or a Design artifact, never built code. A change to the editor is drawn onto a real screenshot of the editor as it is now (run it under `xvfb-run` and capture the window), so it shows exactly where the new part goes and how it sits beside what's there.
- Aventail's code may be copied in and adapted (decision 0007).
- **One line of work** (the project lead, 2026-10-25): Solis is built one milestone at a time on one main thread, never by several agents building parts in parallel. A helper agent may take one specific, bounded job alongside it (the three end-of-phase reviewers, a search, a check) and hands its result back.

## Local environment notes

- If rustup cannot download the pinned toolchain, run commands with `RUSTUP_TOOLCHAIN=stable` when the installed stable version matches `rust-toolchain.toml`.
- With no GPU, run the editor and GPU tests on Mesa's software driver: `WGPU_BACKEND=gl` under `xvfb-run`.

## Git

- Work on a branch and open a pull request; `main` stays clean.
- Plain-language commit messages and pull request descriptions.
