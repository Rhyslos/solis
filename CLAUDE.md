# Working on Solis

Solis is a 3D game engine. Read DESIGN.md before any change; it is the plan, and its decision log records what's settled.

## Start of every session

1. Read `README.md` (what's built, what's next), `ROADMAP.md` and `ARCHITECTURE.md`.
2. Read the README of every crate you will touch.
3. Check `docs/decisions/` before changing anything a decision covers.

## Rules

- **Engine development has begun** (Phase 0 onward). Design changes still go through DESIGN.md and a decision.
- **Decisions go in two places:** a line in DESIGN.md §17, and a file in `docs/decisions/` numbered in order (`0002-short-name.md`) with Status, Date, Problem, Options where useful, Decision and Consequences.
- **The engine is Rust; games can be Lust, Rust or Lua** (decisions 0002 and 0003). Lust is taught first; game code docs show all three.
- **ROADMAP.md is the build order.** One milestone per pull request; a phase ends with its gate, which the project lead tries by hand.
- **Game-specific systems stay out of the engine.** The engine meets the scale target in DESIGN.md §2; squads, campaigns and the like belong in games.
- **Tandem work:** the project lead writes some of the code. Leave issues tagged by difficulty, explain what you build in plain words, and review their pull requests kindly and concretely.

## Builds (decision 0001)

The repo is private, so CI minutes are limited.

- Never add a workflow that runs on every push, on a schedule, or for docs-only changes.
- Workflows run on ready-for-review pull requests and by hand only. Linux by default; Windows and macOS by hand or for release tags.
- Run the full checks locally before pushing, and list the results in the pull request: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`, the GPU tests with `REQUIRE_GPU=1`, and `tools/smoke/window-smoke.sh` under `xvfb-run` (see ARCHITECTURE.md, Checks).

## Code

- Tests for every batch (normal, edge, failure, regression), and the whole suite run before pushing.
- Update documentation in the same pull request: code comments, crate READMEs, `ARCHITECTURE.md`, `CHANGELOG.md`, and the README status when a milestone completes.
- The pull request's "How to test" list covers what automated tests can't (feel, looks), written for a non-programmer.
- Never hard-code the program's name: use `crates/app/src/branding.rs`.
- Aventail's code may be copied in and adapted (decision 0007).

## Local environment notes

- If rustup cannot download the pinned toolchain, run commands with `RUSTUP_TOOLCHAIN=stable` when the installed stable version matches `rust-toolchain.toml`.
- With no GPU, run the editor and GPU tests on Mesa's software driver: `WGPU_BACKEND=gl` under `xvfb-run`.

## Git

- Work on a branch and open a pull request; `main` stays clean.
- Plain-language commit messages and pull request descriptions.
