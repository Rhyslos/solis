# tools/scale

The scale check ([decision 0011](../../docs/decisions/0011-scale-check-at-phase-gates.md)): times the editor, game code and the viewport against their targets, and prints a table for the pull request.

| File | What it does |
| --- | --- |
| `scale-check.sh` | Builds in release mode and runs every ignored test whose name contains `scale_check`, in the `editor` crate, the `app` crate and `crates/render/tests/gpu.rs`: editor frames with big areas, several views, play mode and game code; the scale levels (decision 0030: target, stress and ceiling); a Lust edit reaching a running game; and viewport frames. Anything over its target is marked SLOW and the script fails. Viewport times depend on the graphics card, so they are reported, not judged. |

Run `tools/scale/scale-check.sh` from the repository's top folder at the end of each phase and whenever the project lead asks; not on every pull request.

**Adding a case:** add a test marked `#[ignore = "scale check: run by tools/scale/scale-check.sh"]` whose name starts with `scale_check`, which prints a line starting "Scale check" (and "SLOW" when over target) and fails when over target. A test in a crate the script doesn't run yet needs a `run` line added for that crate.
