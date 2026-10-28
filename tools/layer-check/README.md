# layer-check

**Tool · not part of the program**

Checks that the program's crates follow the architecture rules in [ARCHITECTURE.md](../../ARCHITECTURE.md) and the dependency register in [docs/dependencies.md](../../docs/dependencies.md). It runs as ordinary tests, so `cargo test --workspace` (and CI) fails when a rule is broken, and the message says which crate broke which rule.

## Purpose

Read the root `Cargo.toml`, every crate in `crates/` and the dependency register, then check that:

- each crate declares a layer from 0 (core) to 3 (the app) in its `Cargo.toml` metadata;
- each crate depends only on the program's own crates in its own layer or below, and only on ones that exist;
- each crate uses the workspace lints;
- each crate has a `README.md` with `## Purpose` and `## Must never`;
- every outside library a crate uses is in the register, used only in the crates it allows, and every row of the register is still used (decision 0052).

The program's own crates are the `[workspace.dependencies]` entries whose `path` points into `crates/`.

| Folder | What it holds |
| --- | --- |
| [`src/`](src/README.md) | The checks, as plain functions over parsed data, with tests on made-up crates. |
| [`tests/`](tests/README.md) | The tests that run the checks on the real workspace. |

## Must never

- Depend on any of the program's crates: it reads their manifests as text
- Name the program: crate names are generic on purpose

## Depends on

`toml`. No crates of the program.

## Status

In use since Phase 0; the dependency register check since decision 0052.
