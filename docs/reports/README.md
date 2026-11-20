# docs/reports

Reports written for the project lead: research done before a phase, and what a milestone found. Each says where its numbers came from.

| File | What it holds |
| --- | --- |
| `after-the-rethink/` | Research for everything after the editor rethink (17 November 2026): the rest of 2D, the shared tools and the rest of 3D, one report per phase, each milestone with ways to build it compared, how to build it in Solis, tests, questions and size. Feeds [`../plans/after-the-rethink.md`](../plans/after-the-rethink.md). |
| `ground-study.md` | Tools 1b, G0 (rule 22): three ways to keep a ground with caves (fixed chunks, sparse chunks, an octree) and two ways to draw each of its looks, built in [`tools/ground-study`](../../tools/ground-study/README.md) and measured at the scale target; what to build, the three questions for the project lead, and the draft decisions. Its pictures are the `ground-study-*.png` files beside it. Feeds [`../plans/tools-1.md`](../plans/tools-1.md). |
| `open-worlds-study.md` | The open-worlds study (after Tools 1b; rule 22): how the ground reaches 20 × 20 km and beyond: streaming, far ground drawn coarser (three ways of closing seams, rings against an octree, a far-field height map), physics only where needed, the file set and opening it, a starting ground; measured in [`tools/ground-study`](../../tools/ground-study/README.md), with the plan for the Open worlds phase, the questions for the project lead and the draft decisions. Its pictures are the `open-worlds-*.png` files beside it. Feeds [`../../ROADMAP.md`](../../ROADMAP.md). |
| `text-layout-study.md` | Stage 1's text layout (rule 22): `parley` against Solis's own layout (Unicode's line breaks and `harfrust`), timed on the same paragraphs, what each brings into the build and does that the other doesn't, and why Solis's own was chosen (decision 0129). Its program is in `text-layout-study/`. |
| `text-layout-study/` | The study's program, in a workspace of its own, never built with Solis. |
| `hardening-phase-6.md` | What the hardening milestone (decision 0042, [plan](../plans/phase-6-hardening.md)) did, what it found, and how much the tests now catch (coverage and mutation testing). |
| `lighting-research.md` | Research for 3D's light (3D 2 to 3D 5 in ROADMAP.md, once Phase 13): techniques for light, sky and atmosphere, what they cost, and which open-source code may be reused. Feeds [`../plans/light.md`](../plans/light.md). |
| `lua-native-code.md` | Phase 8, C6 (decision 0055): Luau's native code generation tried on the same work in Rust, Lust and Lua; Lust's heavy work got faster, Lua's slower, so it stays off. The numbers, and when to try again. |
| `orc-invasion-hurdles.md` | The 27 hurdles a session hit making Orc Invasion, a small top-down action RPG, with Solis and Lust; fixed or asked for by the plan's four fix milestones (decision 0133) |
