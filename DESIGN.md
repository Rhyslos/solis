# Solis: design

> A free, open-source 3D game engine.
>
> **Status:** Phase 1 (seeing things) is built; the progress tracker is in [README.md](README.md). The engine is Rust and games are Lust ([decision 0002](docs/decisions/0002-engine-in-rust-games-in-lust.md)); the build order is in [ROADMAP.md](ROADMAP.md).

Sections marked **(to decide)** are open. Everything else is the current plan, and changes go in the decision log (§17) with a file in `docs/decisions/`.

---

## 1. What Solis is

A lightweight, free alternative to Godot, Unity and Unreal for people getting into game development, aimed at:

- **Stylized games,** with tools for that look out of the box.
- **Strategy games** in the Empire at War mould (§2).
- **Medium-poly online co-op and PvP games,** with good netcode built in.
- **Big open worlds,** built from zones and LODs.

**3D only.** The only 2D is UI. Godot, Unity and many others already do 2D well, and focus is a feature.

**Who it's for:** people who want to make games without learning a huge engine, and without an engine that takes a cut of their money.

### Name

**Solis** (Latin for "of the sun"). No 3D engine by that name was found; the nearest are "Solas", a small .NET engine, and an old 2D game called Solis on SourceForge. The VFX system is **Solis Eclipse** (§9): the sun-and-eclipse pun works, and the "Solis" prefix keeps it apart from the Eclipse IDE in searches. **Under review** (2026-10-26): the Eclipse Foundation's ECLIPSE trademark covers software development tools, and an "Eclipse Engine" is being built for *Star Wars: Eclipse*; "Solstice" is recommended instead ([lighting research](docs/reports/lighting-research.md), "Names for the effects suite").

---

## 2. Scale target

**Empire at War-style battles:** hundreds of units normally, a few thousand at peak (cheap unit spam), and tens of thousands of projectiles and effects on screen, running smoothly.

**Tested at three levels** ([decision 0030](docs/decisions/0030-scale-is-tested-at-three-levels.md)): the **target**, 2,000 moving units and 20,000 projectiles at 60 fps; **stress**, 10,000 units and 100,000 projectiles holding 30 fps without stutter; and a **ceiling** run that keeps adding until 60 fps ends, and reports where. Games will go past the target (two fleets of fighter squads, a horde of 6,000 zombies), so we test past it.

That's the number the renderer, the component system and VFX are designed against. It is not Beyond All Reason's tens of thousands of units.

Game-specific systems (squads, hardpoints, hangars, a campaign map, fleet caps) belong in each game's own Lust code, not the engine.

---

## 3. Languages

| Language | Job | Runs as |
| --- | --- | --- |
| **Rust** | The engine itself; also game code, for people who prefer it | Machine code; the engine comes prebuilt |
| **Lust** (`.lust`) | Game code: the language the docs teach first, and the engine's ready-made gameplay components | Rust, which Lust writes and Rust's compiler turns into machine code |
| **Lua** and **LustScript** (`.lsc`) | Game code, for people who know Lua; and editor scripting: custom tools, batch jobs, importers, generators, in a scripting window like Blender's or 3ds Max's | Luau, through `mlua`, the same as Aventail |
| **LustQL** (`.lql`) | Stored data in SQLite | **(to design in Lust first)** |

- **Games can be written in Lust, Rust or Lua** ([decision 0003](docs/decisions/0003-games-in-rust-lust-or-lua.md)). Nobody has to learn Lust to make a game. All three see the same game API (components, inspector fields, typed events), and a game can mix them.
- **Lua is quick to change and slower to run:** no compile step, but slower than compiled code, with a garbage collector. Fine for most gameplay; heavy per-unit work at the scale in §2 is better in Lust or Rust. For deterministic netcode (§6), Lua code has to follow a few rules, such as not relying on the order `pairs` walks a table in.
- **The engine is Rust; games are Lust** ([decision 0002](docs/decisions/0002-engine-in-rust-games-in-lust.md)), the split Godot (C++ and GDScript) and Unity (C++ and C#) use. An engine needs the control Lust deliberately hides; game makers need what Lust is good at.
- **Lust compiles to Rust** (Lust's own plan, LUST.md), so a Lust component becomes the same kind of Rust struct as the engine's own components, and the engine can't tell them apart. No C interface in between.
- **Lust joins in Phase 4,** right after the editor ([ROADMAP.md](ROADMAP.md), [decision 0016](docs/decisions/0016-how-lust-joins-solis.md)). Solis describes the engine to Lust in plain Lust files with `native` methods (`solis/entity.lst` and so on, generated from the engine's Rust), and a game's Lust is built as a Rust library the engine includes.

### What Solis needs from Lust v2

These should go into Lust's design while its Rust output is being built:

1. **Using Rust crates from Lust.** Essential: the engine is Rust. Its own API reaches Lust through description files with `native` methods (done: LUST.md §21, decision 0016); other crates through Lust's standard library. Lust already has this on its to-design list, as its standard library; Solis is its biggest test.
2. **Plain data in packed arrays.** Components written in Lust have to be stored as tightly as the engine's own; cArrays are close.
3. **States that own memory** fit zones (§8) and game modes: when a zone or a battle unloads, its memory is freed in one go. Lust plans arenas with handles for this in Rust, which is also how ECS storage works (§5), so the two may share one design.
4. **Rust that compiles quickly.** See §3.1.

### 3.1 Iteration speed (decided: [decision 0028](docs/decisions/0028-lust-runs-as-luau-while-editing.md))

**Decided:** while editing, Lust runs as Luau (option 4 below), swapped in while the game runs and keeping values; built games run Lust compiled to Rust. Tests compare the two. The options as first weighed:

Rust compiles slowly. Godot users are used to pressing Play and seeing a change in a second or two; a Rust build after changing one component can take 10 to 60 seconds. For a beginner-focused engine, this is the biggest open question.

Options, which can be combined:

1. **The engine comes prebuilt.** Only the game's own crate is compiled, and Rust only rebuilds what changed. Probably a few seconds for small changes.
2. **A faster compiler for development:** Cranelift instead of LLVM for debug builds, and a fast linker (lld on Windows).
3. **Hot reload:** game code built as a library the running editor swaps in, so play mode keeps going (as `hot-lib-reloader` does for Rust today). Fiddly, but the best feel.
4. **Running Lust through Lua while developing,** compiled to Rust only for builds. Fastest to start, but the two can behave differently (numbers, memory), so bugs could show up only in the shipped game. Probably ruled out.

The target to decide against: **a change to one component is running in play mode within 2 seconds.**

---

## 4. Architecture

```
┌──────────────────────────────────────────────┐
│ Games (Lust, Rust or Lua), and the           │
│ ready-made gameplay components (Lust)        │
├──────────────────────────────────────────────┤
│ Game API: Rust, described to Lust in solis/  │
│ files, with a Lua face                       │
├──────────────────────────────────────────────┤
│ Editor scripting (LustScript / Lua, Luau)    │
├──────────────────────────────────────────────┤
│ Engine (Rust): components, areas and zones, │
│ simulation and netcode, rendering,           │
│ animation, VFX, sound, game UI, the editor   │
├──────────────────────────────────────────────┤
│ Libraries: wgpu, winit, egui, Rapier, Kira,  │
│ Steam Audio, mlua (Luau)                     │
└──────────────────────────────────────────────┘
```

- **The game API is the line between the two languages:** small, stable, and designed for Lust to read well. Until Lust is ready, it's used from Rust.
- **Separate from Aventail** ([decision 0007](docs/decisions/0007-mit-licence-and-separate-from-aventail.md)): no shared crates. Aventail's wgpu setup, picking, camera, egui shell, docking and theme are copied in and adapted, which its owner can relicense as MIT.
- **What doesn't carry over from Aventail:** its renderer is built for modelling. Solis's renderer grows its own game features (§7) on the same foundation.
- **Layers:** like Aventail, a crate or module may only depend on its own layer or below, checked by tests.

---

## 5. Entities and components

Solis has **entities with components**, stored ECS-style underneath (packed arrays, for the scale in §2). No node tree, no empty nodes that exist only to hold a script, and no string-named signals.

- **A script is a component,** attached straight to the entity.
- **Components ask each other directly:** `entity:get(Health)`.
- **Events are declared types,** so Lust's checker catches a misspelled event before the game runs.
- **A `has` block puts fields in the inspector,** with their starting values, as Aventail does with command settings.
- **Built-in components:** transform, mesh, camera, light, collider, rigid body, audio source, animator, and more. Everything else is custom, written the same way.

A sketch. `like Component` and `entity:send` are proposals, not settled Lust. The engine's side comes from the generated `solis/` files (decision 0016):

```
fetch "solis/entity":Component, Died;

object Health like Component {
    has {
        int[maxHp 100];
        int[hp 100];
    }

    method damage(int amount) {
        hp = hp - amount;
        is hp <= 0? then {
            entity:send(Died);
        }
    }
}

end
```

**Storage underneath:** an area document of component values for the editor ([decision 0019](docs/decisions/0019-scene-document-now-game-storage-later.md)), and `hecs` for the running game, built from it when Play is pressed ([decision 0024](docs/decisions/0024-game-storage-is-hecs.md)).

**How Lust and Rust components meet:** every component is a plain Rust struct that can describe its fields to the inspector. The engine's built-in components are written that way. A Lust `object … like Component` compiles to exactly the same kind of struct, so built-in and custom components work the same way everywhere.

---

## 6. Simulation and netcode

No single netcode fits every game, so Solis provides it in layers ([decision 0005](docs/decisions/0005-netcode-in-layers.md)).

**Built in, for every game:**
- **Transport:** connections, reliable and unreliable messages, encryption.
- **Sessions:** lobbies, invites, relays to get through home routers, and Steam networking.
- **Foundations:** a fixed simulation timestep, separate from the frame rate; seeded randomness; compact message encoding.
- **Tools:** a lag and packet-loss simulator, network stats, replays, and desync detection.

**One model, optional: lockstep** for strategy games, where only inputs are sent. Strict cross-platform determinism (careful float maths; Rapier's determinism mode for physics) is switched on for lockstep games, and Lust's checker can help keep to its rules.

**Other models plug in through existing Rust crates:** `renet` or `lightyear` for server-authoritative games, `ggrs` for rollback, `matchbox` for peer-to-peer. Solis makes sure they fit and documents how.

**The pitch:** the shared networking layers built in properly, with a strategy-ready model included.

---

## 7. Rendering

wgpu, lightweight, built for medium-poly, stylized games.

**Light is Solis's selling point** (the project lead, 2026-10-26; [decision 0046](docs/decisions/0046-light-before-netcode.md), proposed): skies, sunsets and light that feel right, never flat or sterile. Physics underneath (a physical sky with ozone and haze over distance, clouds lit by the sun), art direction on top (white balance, grading, colour by time of day, a painted sky kept as a style). The research is in [docs/reports/lighting-research.md](docs/reports/lighting-research.md) and the plan in [docs/plans/phase-8-light.md](docs/plans/phase-8-light.md).

**0.1:**
- Materials, including a stylized and toon option
- **GPU instancing** (essential for §2), frustum culling, LODs
- Tonemapping, automatic exposure, bloom, white balance and colour grading (much of a stylized look lives here)
- **A physical sky** (Hillaire's method), time of day and night, flat and volumetric clouds, haze over distance, volumetric fog and light shafts
- **Light with shape:** sky light from the real sky, ambient occlusion, cascaded and contact shadows, hundreds of lamps with a few shadowed, reflection probes, and bounced light every way that can be made fast: baked light probes, bounce from the screen, real-time probes traced against a simplified world, and a time-boxed experiment for a better method
- Quality levels (Low, Default, High), so the scale target holds on mid-range cards

**Later:**
- Baked lightmaps, screen-space reflections, TAA and upscaling, hardware ray tracing once wgpu's is stable
- Occlusion culling
- A node-based shader editor (stylized games live on custom shaders)
- Impostors for distant objects

**Lighting approach:** a choice per project ([decision 0008](docs/decisions/0008-lighting-is-a-project-choice.md)). Real-time lights with sky light came first; baked and real-time bounced light arrive in Phase 8; lightmaps are a further source after 0.1.

---

## 8. Worlds, zones and streaming

- **Areas** are text files, made to merge in Git: one entity per block, stable IDs, fields in a fixed order (a selling point: Unity and Godot scenes are notorious for conflicts; [decision 0006](docs/decisions/0006-git-built-in.md)).
- **File extensions:** `project.solis` for the project, `.solarea` for areas, `.solassembly` for assemblies and `.solmat` for materials ([decision 0015](docs/decisions/0015-file-extensions.md)).
- **Assemblies,** nested, with per-instance overrides.
- **Game state above areas:** state that outlives an area (a campaign, a save), with loading an area and returning as a normal operation.
- **Zones** stream in and out by distance, each owning its memory (§3).
- **Large worlds** need a floating origin, so precision holds far from the centre.
- **Terrain:** heightmaps, texture painting, and scattering of grass, trees and rocks. Needed for open worlds.

---

## 9. VFX: Solis Eclipse

Gets a lot of attention. Stylized VFX tools out of the box: flipbooks, trails, ribbons, mesh particles, stylized shading, and lots of particles through instancing.

**Open question:** the wishlist mentions reusing Aventail's VFX system (originally named Gauntlet). Aventail's plans don't contain one so far; "Gauntlet" came up as a rejected name for Aventail itself. So Solis Eclipse is probably new design work, built for games first, and Aventail could adopt it later. **(to confirm)**

---

## 10. Animation

- Skeletal animation imported from glTF and FBX
- State machines deciding which animation plays
- Blending between animations, and blend spaces
- Animation events (a footstep sound on a frame)
- **Later:** IK, root motion, retargeting, a timeline for cutscenes

---

## 11. Sound

The area to spend extra time on. In order of payoff:

1. **Sound events, not files.** "Footstep" picks one of several clips with slight pitch and volume variation, so nothing sounds repeated.
2. **A mixer with buses** (music, effects, voice, UI) and ducking.
3. **3D sound:** distance falloff, occlusion behind walls, and reverb zones (tying into §8).
4. **Adaptive music:** layers and transitions following the game's state.

**Libraries:** Kira (Rust) for playback and mixing; Steam Audio (open source) for 3D sound and occlusion. FMOD and Wwise are proprietary, so they're ruled out.

---

## 12. UI

- **The editor** uses egui, like Aventail and Halberd.
- **Game UI** is drawn in Solis, with **Sungraph UI** ([decision 0039](docs/decisions/0039-game-ui-is-drawn-in-solis.md)): a visual UI editor where UI is drawn canvas style (lines, shapes, curves, text) rather than built from imported images, so it's sharp at any size; any value can be tied live to game code; every element is reachable from code, because a UI file is ordinary Lust (`view { }`); and styles such as glow, pixelation and stylized outlines apply to the whole UI. Lust's plan has its own `view { }` renderer as its biggest item; Solis's game UI renderer could be that renderer, but Solis doesn't wait for it. A Rust UI layer comes first, which `view { }` compiles to later. It's built, whole, in Phase 10 (decision 0046, proposed; it was Phase 9).
- **Other visual editors** follow the same rule, a graph saved as readable text: state machines (Phase 7), behaviour trees, and shader and material graphs.

---

## 13. The editor

Settled in [decision 0004](docs/decisions/0004-editor-layout.md): Halberd's panels with Unreal's tabs, and every panel dockable.

- **Top bar, one row:** menus and document tabs on the left (Unreal's tab system), and Play, Build and Build & Play on the right.
- **Default arrangement:** Area and Layers above Properties on the right, full height; the viewport in the centre, with the tools as icons at its bottom centre (Select, Move, Rotate, Scale, Transform) and its other overlays in the corners, switchable from its ≡ menu; the library (with Code as a second tab) beside the console (with Problems) underneath. The camera is Fly only.
- **Shaped like Aventail:** flat panels with underlined tabs and a small menu row under each title, bordered buttons with small corners, square fields.
- **Dockable:** drag any panel to an edge, onto another panel to tab them, or out to float it. Saved per computer, with presets in Window → Layouts (Default, Wide screen with the library on the left, Small screen with the library and console as tabs). Built on `egui_dock`.
- **Theme:** Aventail's dark neutral greys with one solid accent colour from the settings (petrol blue by default; red, green and plum as in Aventail).
- **Basics:** undo and redo, gizmos, several viewports, and a play mode that resets the area when you stop.
- **The scripting window** (§3).
- **Debugging:** a frame profiler, debug drawing for physics and navigation, a command console, and stepping through Lust code.

---

## 14. Everything else a full engine needs

**0.1:**
- **Asset pipeline:** importing glTF, FBX, textures and audio; an asset database with stable IDs, so renaming a file doesn't break references; hot reload; texture compression. Importing Aventail's `.avtl` directly.
- **Input:** named actions instead of raw keys, gamepads, rebinding.
- **Physics:** Rapier.
- **Save games.**
- **Build and export:** Windows ([decision 0012](docs/decisions/0012-windows-only.md)). Linux and macOS only if that changes.
- **Git, for teams** ([decision 0006](docs/decisions/0006-git-built-in.md)): project files made to merge, Git LFS for big files, a Git panel with commits, history and locking, and a merge tool that merges areas entity by entity.

**Later:**
- **Navigation and AI:** navmeshes, flowfields (moving many units), behaviour trees or state machines.
- **Steam:** achievements, cloud saves, Workshop, Steam Deck.
- **Localization and accessibility:** subtitles, text scaling, colourblind options.
- **Modding** for games made with Solis (separate from editor scripting).
- **Cutscene timeline.**

**Decisions:**
- **License:** MIT ([decision 0007](docs/decisions/0007-mit-licence-and-separate-from-aventail.md)).
- **Platforms:** Windows PCs ([decision 0012](docs/decisions/0012-windows-only.md)): the editor and the games it builds. Consoles need NDAs that clash with open source, as with Godot.
- **Documentation, templates and tutorials** are a pillar of their own for a beginner-focused engine.

---

## 15. Roadmap

**Solis 0.1 in about sixteen months,** in fourteen phases (0 to 13), each ending with a gate the project lead tries by hand. The full plan is in [ROADMAP.md](ROADMAP.md). In short: groundwork and the editor shell, rendering, the world, the editor, **Lust and Lua join** (Phase 4), scale, simulation, sound and animation, **light, sky and atmosphere**, effects, game UI, netcode, scripting and export, and the 0.1 release.

**The flagship sample:** a small space battle. Two capital ships, a carrier launching fighter squadrons, a few hundred lasers in the air, over the network. It tests instancing, components, VFX and lockstep at once.

**Not in 0.1:** open-world streaming, the full Solis Eclipse toolkit, rollback, advanced animation.

**Risks:**
1. **Lust is needed from Phase 4** (decision 0016). Its connection to the engine is built and tested already; if the rest is late, Phase 5 (Scale) goes first.
2. **Iteration speed** (§3.1). If changes take too long to see, the engine feels slow however fast games run.
3. **Time.** The pace is set by decisions, testing and review, not by writing code. Too many projects at full speed turns a year into three.

---

## 16. Working together

Tandem work, with issues tagged by difficulty so there's always something the right size.

- **The project lead:** the decisions before each phase, every gate, the theme, and choosing controls. From Phase 4: the ready-made gameplay components in Lust, the flagship sample's game logic, and LustScript editor tools. Before then, small Rust tasks tagged `good first issue`, if wanted.
- **Claude:** the engine in Rust, and the Lust side of Phase 4 in the Lust repo.

Both sides review each other's pull requests.

**Builds:** the repo is public, so checks run on every pull request update, on Linux and Windows. See [decision 0018](docs/decisions/0018-checks-for-a-public-repo.md).

---

## 17. Decision log

- 2026-10-20: Solis is 3D only, aimed at stylized, strategy, online co-op/PvP and open-world games.
- 2026-10-20: LustScript/Lua only for editor scripting, never game code.
- 2026-10-21: Lust compiles to Rust (Lust's decision), so Lust uses the foundation's crates directly, with no C interface.
- 2026-10-20: Entities and components with typed events; no node tree or signals.
- 2026-10-20: Scale target: Empire at War-style battles (§2). Game-specific systems stay out of the engine.
- 2026-10-20: [0001](docs/decisions/0001-builds-on-demand.md): builds and checks don't run on every push.
- 2026-10-21: [0002](docs/decisions/0002-engine-in-rust-games-in-lust.md): the engine is Rust; games are Lust. Replaces "the engine in Lust on a thin Rust foundation". The build order is in [ROADMAP.md](ROADMAP.md).
- 2026-10-21: [0003](docs/decisions/0003-games-in-rust-lust-or-lua.md): games can be written in Lust, Rust or Lua. Replaces "LustScript/Lua only for editor scripting, never game code".
- 2026-10-21: [0004](docs/decisions/0004-editor-layout.md): the editor's layout and theme. Halberd's panels with Unreal's tabs, every panel dockable, dark greys with one accent. Replaces "mid-greys with accent gradients".
- 2026-10-21: [0005](docs/decisions/0005-netcode-in-layers.md): netcode in layers: transport, sessions, foundations and tools built in; lockstep as an optional model; other models through existing crates. Replaces "deterministic by default" as the netcode pitch.
- 2026-10-21: [0006](docs/decisions/0006-git-built-in.md): Git built into the editor, with project files made to merge, Git LFS, locking, and an area merge tool.
- 2026-10-21: [0007](docs/decisions/0007-mit-licence-and-separate-from-aventail.md): MIT licence; Solis stays separate from Aventail, copying and adapting Aventail's code, which its owner relicenses.
- 2026-10-21: [0008](docs/decisions/0008-lighting-is-a-project-choice.md): lighting is a choice per project; real-time lights with sky light come first, baked and global illumination later.
- 2026-10-21: [0009](docs/decisions/0009-fly-camera-controls.md): the viewport camera flies with Unreal's controls (hold right mouse; W A S D, Q and E).
- 2026-10-21: [0010](docs/decisions/0010-windows-exe-on-merge.md): a Windows `solis.exe` is built on GitHub when code is merged into main, and by hand. Loosens 0001 for this one build.
- 2026-10-21: [0011](docs/decisions/0011-scale-check-at-phase-gates.md): a scale check (`tools/scale/scale-check.sh`) at every phase gate and on request, growing into the §2 target in Phase 5 (Scale, numbered 4 before decision 0016).
- 2026-10-21: [0012](docs/decisions/0012-windows-only.md): Solis is made for Windows, the editor and exported games; Linux and macOS builds are dropped. CI's checks still run on a Linux machine, the cheapest kind.
- 2026-10-21: [0013](docs/decisions/0013-looks-for-the-whole-view-first.md): the standard, stylized (cel-shaded, with outlines) and unlit looks, chosen for the whole view until materials become assets in Phase 2.
- 2026-10-22: [0014](docs/decisions/0014-lights-placed-two-ways.md): lights come two ways until areas arrive: with a model's glTF file, or added from the Entity menu; strengths in Blender's units (W, W/m²); shadows from the sun only, up to 16 lamps at once.
- 2026-10-22: [0015](docs/decisions/0015-file-extensions.md): file extensions `project.solis`, `.solarea`, `.solassembly` and `.solmat`; `.sol` is Solidity's.
- 2026-10-22: [0016](docs/decisions/0016-how-lust-joins-solis.md): Lust joins in Phase 4, right after the editor; the engine is described to Lust in generated `solis/*.lst` files with `native` methods, and a game's Lust is built as a library the engine includes. Scale, simulation, and sound and animation become Phases 5–7.
- 2026-10-22: [0017](docs/decisions/0017-hdr-tone-mapping-sky-and-fog.md): the world drawn in HDR and tone mapped (AgX by default, ACES or none), with exposure; the editor backdrop or a daylight sky that follows the sun; distance fog. The editor's own backdrop and lines are never tone mapped.
- 2026-10-22: [0018](docs/decisions/0018-checks-for-a-public-repo.md): the repository is public, so checks run on every pull request update and merge, with Windows and Direct3D 12 GPU tests, a weekly security check and Dependabot. Replaces 0001.
- 2026-10-23: [0019](docs/decisions/0019-scene-document-now-game-storage-later.md): entities and components live in an area document of values with stable IDs, described by schemas looked up by name; the game's fast storage is chosen in Phase 3 with a benchmark.
- 2026-10-23: [0020](docs/decisions/0020-scene-files-in-lust-style.md): areas and assemblies are Lust-style data blocks, one per entity, sorted by random IDs, one line per field; assembly instances are a reference plus overrides.
- 2026-10-23: [0021](docs/decisions/0021-asset-database-and-models.md): a project folder with `.solmeta` sidecars holding asset and sub-asset IDs, a rebuildable cache, file watching; models become node trees drawn with per-instance transforms.
- 2026-10-23: [0022](docs/decisions/0022-undo-as-reversible-edits.md): undo and redo as reversible edits to the area document; selection restored by undo but never a step.
- 2026-10-23: [0023](docs/decisions/0023-areas-and-assemblies.md): "area" instead of "scene" and "assembly" instead of "prefab"; files `.solarea` and `.solassembly`.
- 2026-10-23: [0024](docs/decisions/0024-game-storage-is-hecs.md): the game's fast storage is `hecs`, in a `world` crate built from the area when Play is pressed; chosen with a benchmark at the scale target (`tools/storage-bench`), where both candidates were far inside the budget.
- 2026-10-23: [0025](docs/decisions/0025-picking-and-gizmos.md): picking by an ID image drawn on request and read back without waiting; a copy of an assembly is selected whole first; Aventail's gizmos and controls (Ctrl flips snapping, Shift fine, Esc or right click cancels, X/Y/Z axis locks, L for object axes); a drag is one merged undo step.
- 2026-10-24: [0026](docs/decisions/0026-draw-only-when-something-changes.md): the editor draws only when something changes (frames back to back while things move, a frame a second otherwise); the corner says "Idle".
- 2026-10-24: [0027](docs/decisions/0027-regular-code-reviews.md): at the end of every phase and when asked, three separate reviewers read the new code for stability, efficiency and correctness where parts meet; fixes go in pull requests by kind.
- 2026-10-24: [0028](docs/decisions/0028-lust-runs-as-luau-while-editing.md): while editing, Lust runs as Luau, swapped into the running game keeping values; built games run Lust compiled to Rust; tests compare the two. Settles §3.1.
- 2026-10-24: [0029](docs/decisions/0029-the-sample-s-rust-component-is-built-in.md): the turret sample's Rust component is compiled into Solis, offered only in the sample, until Phase 10 builds a game's own Rust.
- 2026-10-24: [0030](docs/decisions/0030-scale-is-tested-at-three-levels.md): scale is tested at three levels: target (2,000 units, 20,000 projectiles, 60 fps), stress (10,000 and 100,000, 30 fps without stutter) and a ceiling run that reports where 60 fps ends. Extends §2.
- 2026-10-24: [0031](docs/decisions/0031-the-job-system-is-rayon.md): the job system is `rayon`; jobs write only their own slice, so a frame is the same on any number of cores.
- 2026-10-24: [0032](docs/decisions/0032-lods-made-on-import-or-the-model-s-own.md): LODs are made on import with meshoptimizer, or come from the model (`Name_LOD1` …), chosen by size on screen.
- 2026-10-24: [0033](docs/decisions/0033-game-code-sees-the-start-of-the-tick.md): during a tick, game code sees the world as it was when the tick started (and its own changes); changes land together at the end in a fixed order, so game code can run on every core with the same result everywhere.
- 2026-10-25: [0034](docs/decisions/0034-a-raw-interrupt-for-the-time-limits.md): the time limits' interrupt is set on Luau directly, in one small module that may use `unsafe` (mlua's own cost a fifth of game code's time).
- 2026-10-25: [0035](docs/decisions/0035-game-code-spawns-into-the-area-while-playing.md): `spawn` and `despawn` change the area while playing (not as undo steps; Stop puts it back), landing after the tick like moves, with IDs made from the tick's order so every core count makes the same ones.
- 2026-10-25: [0036](docs/decisions/0036-physics-is-rapier-strict-on-every-pc.md): physics is Rapier in its strict mode (`enhanced-determinism`), so the same inputs give the same result on every PC; one choice for every game.
- 2026-10-25: [0037](docs/decisions/0037-input-as-named-actions.md): game code reads named actions (Move, Look, Jump, Sprint, Interact by default: W A S D, mouse, Space, Shift, F; gamepad too), rebindable; clicking the Game panel gives the game the input, Esc gives it back.
- 2026-10-25: [0038](docs/decisions/0038-save-games-hold-everything.md): a save game holds everything (entities, components' fields, projectiles, physics, the clock, the randomness); game code calls `saveGame` and `loadGame`.
- 2026-10-25: [0039](docs/decisions/0039-game-ui-is-drawn-in-solis.md): game UI is drawn in Solis, canvas style, with Sungraph UI, in a visual UI editor: no outside images needed, values tied live to code, every element reachable from code (a UI file is Lust), styles such as glow and pixelation; built whole in Phase 9. Settles §12.
- 2026-10-25: [0040](docs/decisions/0040-a-project-s-preferred-language.md): setting up a project asks for its preferred language (Lust, Rust, Lua, in that order); it decides only what a new code file starts as, and all three can always be used.
- 2026-10-25: [0041](docs/decisions/0041-camera-controls.md): the ready-made cameras' controls, chosen by the project lead: arrows pan, middle-drag drags the map, Alt + arrows turn, the wheel and Page Up/Down zoom (RTS); the third-person camera keeps its distance unless Wheel zooms is ticked; the playground saves on F6 and loads on F7. Input gains key combinations and the mouse wheel.
- 2026-10-25: [0042](docs/decisions/0042-a-hardening-milestone-before-phase-7.md): a hardening milestone comes before Phase 7: merge the open pull requests, whole-game invariant tests, fuzzing every file reader, coverage and mutation testing, and checks that stay. Chosen by the project lead after Phase 6's reviews kept finding problems, some caused by earlier fixes.
- 2026-10-25: [0043](docs/decisions/0043-installed-and-updated-from-github.md): Solis is installed with an Inno Setup installer (per user, no administrator), and each merge into main is published as a GitHub release `build-<number>`. Installed copies check GitHub when they open and every six hours, download a newer build while you work, check its SHA-256, and install it when you close (or now, with Restart to update). Asked for by the project lead.
- 2026-10-26: [0044](docs/decisions/0044-third-party-notices-ship-with-the-program.md): every build writes `THIRD-PARTY-NOTICES.txt` (the libraries built into the program, with their copyright notices and licences) with cargo-about, and it ships in the download and the installer, as those licences ask.
- 2026-10-26: [0045](docs/decisions/0045-a-splash-screen-that-asks-before-updating.md): Solis starts with a splash screen (Aventail's, with Solis's logo) for at least 0.6 s while it asks GitHub for a newer build; a newer one brings up a question (Update now, Open without updating, Skip this version), and Help > Check for updates when starting turns checking off. Updates no longer download quietly, and the six-hourly checks are gone. Asked for by the project lead.
- 2026-10-26: [0046](docs/decisions/0046-light-before-netcode.md) (proposed): light, sky and atmosphere get Phase 8, effects and game UI a phase each (9 and 10), and netcode moves to Phase 11; bloom and grading move into Phase 8, and bounced light, baked and real-time, arrives there (the order, clouds and bounced light chosen by the project lead; the rest of the Phase 8 plan's questions are open).
