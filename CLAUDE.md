# Working on Solis

Solis is a 3D game engine. Read DESIGN.md before any change; it is the plan, and its decision log records what's settled.

## Rules

- **The project is in its design phase.** Changes are to DESIGN.md and `docs/decisions/` until engine development begins.
- **Decisions go in two places:** a line in DESIGN.md §17, and a file in `docs/decisions/` numbered in order (`0002-short-name.md`) with Status, Date, Problem, Options where useful, Decision and Consequences.
- **The engine is Rust; games can be Lust, Rust or Lua** (decisions 0002 and 0003). Lust is taught first; game code docs show all three.
- **ROADMAP.md is the build order.** One milestone per pull request; a phase ends with its gate, which the project lead tries by hand.
- **Game-specific systems stay out of the engine.** The engine meets the scale target in DESIGN.md §2; squads, campaigns and the like belong in games.
- **Tandem work:** the project lead writes some of the code. Leave issues tagged by difficulty, explain what you build in plain words, and review their pull requests kindly and concretely.

## Builds (decision 0001)

The repo is private, so CI minutes are limited.

- Never add a workflow that runs on every push, on a schedule, or for docs-only changes.
- Workflows run on ready-for-review pull requests and by hand only. Linux by default; Windows and macOS by hand or for release tags.
- Run the full checks locally before pushing, and list the results in the pull request.

## Git

- Work on a branch and open a pull request; `main` stays clean.
- Plain-language commit messages and pull request descriptions.
