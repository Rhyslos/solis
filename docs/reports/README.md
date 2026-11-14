# docs/reports

Reports written for the project lead: research done before a phase, and what a milestone found. Each says where its numbers came from.

| File | What it holds |
| --- | --- |
| `ground-study.md` | Tools 1b, G0 (rule 22): three ways to keep a ground with caves (fixed chunks, sparse chunks, an octree) and two ways to draw each of its looks, built in [`tools/ground-study`](../../tools/ground-study/README.md) and measured at the scale target; what to build, the three questions for the project lead, and the draft decisions. Its pictures are the `ground-study-*.png` files beside it. Feeds [`../plans/tools-1.md`](../plans/tools-1.md). |
| `hardening-phase-6.md` | What the hardening milestone (decision 0042, [plan](../plans/phase-6-hardening.md)) did, what it found, and how much the tests now catch (coverage and mutation testing). |
| `lighting-research.md` | Research for 3D's light (3D 2 to 3D 5 in ROADMAP.md, once Phase 13): techniques for light, sky and atmosphere, what they cost, and which open-source code may be reused. Feeds [`../plans/light.md`](../plans/light.md). |
| `lua-native-code.md` | Phase 8, C6 (decision 0055): Luau's native code generation tried on the same work in Rust, Lust and Lua; Lust's heavy work got faster, Lua's slower, so it stays off. The numbers, and when to try again. |
