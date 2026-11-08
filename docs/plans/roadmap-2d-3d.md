# Where 2D and 3D stand

A snapshot from 8 November 2026 (after pull request 217), so the overall picture isn't lost. ROADMAP.md stays the build order; this page only shows where each side of the engine is. "Built" means merged; none of these phases has had its hand-tried gate yet, apart from Phase 0. Also published as a page: [Solis 2D and 3D Roadmaps](https://claude.ai/artifact/M7DGnKCpzi3BwMaPESqpPZ).

**Kinds:** *Built*. *Planned*, with a phase. *Later*: written down in DESIGN.md or ROADMAP.md, with no phase. *Not talked about yet*: common in other engines, nothing decided.

## 2D

**Built**
- Phase 9, seeing and drawing: the pixel-perfect view, 2D settings and cameras; sprites from Aseprite; frame animation (clips, the Clips and Frames tabs, events); animators in a graph editor; lamps and day and night on sprites.
- Phase 9, the ground: tile maps that change as you play, with cell values; tilesets and autotiling; the tile painter.
- Usable first: solid terrains, walls, Collider2D and top-down walking; a 2D follow camera; game screens and the UI editor (Phase 12, brought forward); export for playtesting (Phase 11, brought forward); parallax layers that slide smoothly (from Phase 10, brought forward, decision 0094).

**You are here:** start the farming game in Solis (the "usable first" gate), then Phase 10.

**Planned**
- Phase 10, 2D part 2: levels and the world (typed fields, rooms on a world map, sort points); advanced tiling (decoration rules, scatter brushes, stamps); pathfinding on tile maps; soft shadows, normal and glow layers; more movement (a platformer controller with coyote time, jump buffering and wall-jump, grid-step movement, one-way platforms, slopes, moving platforms); polish (palette swaps and cycling, pixel-art rotation, hit-stop and squash, the 2D sample, the manual).
- Phase 15: a pixel mode for effects (pixel-shaped particles: rain, wind streaks, fireflies, bubbles).

**Later:** translations, subtitles and colour-blind options; Steam (achievements, cloud saves, Workshop, Steam Deck).

**Not talked about yet:** bone animation for 2D characters (like Spine); isometric and hex maps; importing levels from LDtk or Tiled; sprite effects (dissolve, wobble, ripples, glowing edges); camera rooms and limits, screen shake; ropes, chains and swinging (2D joints); water (swimming, floating, waves); big streamed 2D worlds; diggable or breakable ground.

## 3D

**Built**
- Phase 1: glTF models and textures; standard, stylized and unlit looks; sun shadows, point lights and spotlights, sky light; HDR, tone mapping, sky and fog; day and night.
- Phases 2–3: entities, components, parenting, undo, assemblies; areas as text Git merges; gizmos, inspector, Library, console, Play, several views.
- Phases 4–8: Lust, Lua and Rust with edits in about 0.3 s; scale (every core, culling, levels of detail; 2,000 units and 20,000 lasers at 60 fps); 3D physics, a character controller and three cameras; controls as actions, save games, replays that always match; 3D sound, the Mixer, reverb zones, listeners; compatibility across updates.

**You are here:** built through Phase 8, waiting for its gates; 3D is paused while 2D is finished.

**Planned**
- Phase 13, light, sky and atmosphere: a physical sky, moons, stars, aurora, clouds, weather; real light with automatic exposure; bloom and grading; ambient occlusion, far shadows, hundreds of lamps; volumetric fog and light shafts; reflections, wet surfaces, bounced light; quality levels.
- Phase 14, materials and shaders: materials as files and node graphs; values driven by light, time, distance, wetness or game code; stylized building blocks.
- Phase 15, effects (Solstice): particles, flipbooks, trails, stylized effects.
- *A pause to rethink after Phase 15.*
- Phase 16, 3D animation: skeletons from glTF, blending and blend spaces, cues, crowds.
- Phases 17–19: netcode; the full export, a scripting window, the Git panel and merge tool; Solis 0.1 with the flagship sample and docs.

**Later:** navigation and AI (navmeshes, flow fields, behaviour trees); terrain (heightmaps, painting, scattering); big open worlds (streamed zones, floating origin); IK, root motion, retargeting, a cutscene timeline; occlusion culling, impostors, lightmaps, upscaling, ray tracing; Steam, translations and accessibility, mods.

**Not talked about yet:** water and oceans; decals; ragdolls and joints; vehicles; cloth and hair; breaking things apart; screens on 3D surfaces; quick level blocking in the editor (like ProBuilder); painting foliage by hand, plants that bend as you walk through.

## For both

The tools in [ideas.md](ideas.md): timers, tween, states that know when they start and stop, tags and questions about the world, field hints, more kinds of settings, screen shake and fades, smooth noise, data tables, weighted random, save slots, debug drawing and a cheat box, game tests, bug reports with recordings, text effects, music that follows the game. Parallax, built, came from the same kind of thinking.
