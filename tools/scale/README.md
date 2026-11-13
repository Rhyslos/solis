# tools/scale

The scale check ([decision 0011](../../docs/decisions/0011-scale-check-at-phase-gates.md)): times the editor, game code and the viewport against their targets, and prints a table for the pull request.

| File | What it does |
| --- | --- |
| `scale-check.sh` | Builds in release mode and runs every ignored test whose name contains `scale_check`, in the `editor`, `app`, `world`, `tiles` and `ui` crates and `crates/render/tests/gpu/`: editor frames with big areas (10,000 entities listed, inspected and searched), several views, play mode and game code; the scale levels (decision 0030: target, stress and ceiling); a Lust edit reaching a running game; asking for a big tile map and picture (read on a worker); a big new file in the assets and a change in a project of 10,000 files (looked at and described on workers); tile maps (cell changes, counting a big map's terrains, reading a big map and the biggest with three full layers, a big map in view frame after frame, in two views and the Game panel at once, and selected and seen whole with its walls and values); wrapping a long text (in fixed-width letters and in a real font); a long Markdown note in Properties and reading a long paragraph's marks; viewport frames; eight parallax layers laid over a 4K picture (decision 0094); and handing over a busy 2D frame's last steps (eight layers and three screens of game UI, 20,000 shapes). Anything over its target is marked SLOW and the script fails. Viewport times depend on the graphics card, so they are reported, not judged. |

Run `tools/scale/scale-check.sh` from the repository's top folder at the end of each phase and whenever the project lead asks; not on every pull request.

**Adding a case:** add a test marked `#[ignore = "scale check: run by tools/scale/scale-check.sh"]` whose name starts with `scale_check`, which prints a line starting "Scale check" (and "SLOW" when over target) and fails when over target. A test in a crate the script doesn't run yet needs a `run` line added for that crate.
