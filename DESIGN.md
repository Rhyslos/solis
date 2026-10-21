# Solis: design

> A free, open-source 3D game engine.
>
> **Status:** design only. No code yet. The engine is Rust and games are Lust ([decision 0002](docs/decisions/0002-engine-in-rust-games-in-lust.md)); the build order is in [ROADMAP.md](ROADMAP.md).

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

**Solis** (Latin for "of the sun"). No 3D engine by that name was found; the nearest are "Solas", a small .NET engine, and an old 2D game called Solis on SourceForge. The VFX system is **Solis Eclipse** (§9): the sun-and-eclipse pun works, and the "Solis" prefix keeps it apart from the Eclipse IDE in searches.

---

## 2. Scale target

**Empire at War-style battles:** hundreds of units normally, a few thousand at peak (cheap unit spam), and tens of thousands of projectiles and effects on screen, running smoothly.

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
- **The engine doesn't wait for Lust.** Lust is needed from [ROADMAP.md](ROADMAP.md) Phase 7.

### What Solis needs from Lust v2

These should go into Lust's design while its Rust output is being built:

1. **Using Rust crates from Lust.** Essential: the engine is Rust, and `fetch solis;` is a Rust crate seen from Lust. Lust already has this on its to-design list, as its standard library; Solis is its biggest test.
2. **Plain data in packed arrays.** Components written in Lust have to be stored as tightly as the engine's own; cArrays are close.
3. **States that own memory** fit zones (§8) and game modes: when a zone or a battle unloads, its memory is freed in one go. Lust plans arenas with handles for this in Rust, which is also how ECS storage works (§5), so the two may share one design.
4. **Rust that compiles quickly.** See §3.1.

### 3.1 Iteration speed (to decide)

Rust compiles slowly. Godot users are used to pressing Play and seeing a change in a second or two; a Rust build after changing one component can take 10 to 60 seconds. For a beginner-focused engine, this is the biggest open question.

Options, which can be combined:

1. **The engine comes prebuilt.** Only the game's own crate is compiled, and Rust only rebuilds what changed. Probably a few seconds for small changes.
2. **A faster compiler for development:** Cranelift instead of LLVM for debug builds, and a fast linker (mold on Linux, lld on Windows).
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
│ Game API: fetch solis; (Rust, with Lust and  │
│ Lua faces)                                   │
├──────────────────────────────────────────────┤
│ Editor scripting (LustScript / Lua, Luau)    │
├──────────────────────────────────────────────┤
│ Engine (Rust): components, scenes and zones, │
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

A sketch. `like Component` and `entity:send` are proposals, not settled Lust:

```
fetch solis;

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

**Storage underneath (to decide, ROADMAP.md Phase 2):** `bevy_ecs` on its own, or Solis's own.

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

**0.1:**
- Materials, including a stylized and toon option
- Lights and shadows
- **GPU instancing** (essential for §2)
- Frustum culling, LODs
- Post-processing: tonemapping, bloom, colour grading (much of a stylized look lives here)
- Sky and fog

**Later:**
- Occlusion culling
- Day and night
- A node-based shader editor (stylized games live on custom shaders)
- Impostors for distant objects

**Lighting approach:** a choice per project ([decision 0008](docs/decisions/0008-lighting-is-a-project-choice.md)). Real-time lights with sky light come first; baked lightmaps, light probes and real-time global illumination are added later as other sources of indirect light.

---

## 8. Worlds, zones and streaming

- **Scenes** are text files, made to merge in Git: one entity per block, stable IDs, fields in a fixed order (a selling point: Unity and Godot scenes are notorious for conflicts; [decision 0006](docs/decisions/0006-git-built-in.md)).
- **Prefabs,** nested, with per-instance overrides.
- **Game state above scenes:** state that outlives a scene (a campaign, a save), with loading a scene and returning as a normal operation.
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
- **Game UI** is its own system: custom UI components written with Lust's `view { }`, and a visual UI editor in the spirit of Unreal's. Lust's plan has its own `view { }` renderer as its biggest item; Solis's game UI renderer could be that renderer, but Solis doesn't wait for it. A Rust UI layer comes first, which `view { }` compiles to later. **(to decide)**

---

## 13. The editor

Settled in [decision 0004](docs/decisions/0004-editor-layout.md): Halberd's panels with Unreal's tabs, and every panel dockable.

- **Top bar, one row:** menus and document tabs on the left (Unreal's tab system), and Play, Build and Build & Play on the right.
- **Default arrangement:** Scene and Layers above Properties on the right, full height; the viewport in the centre, with the tools as icons at its bottom centre (Select, Move, Rotate, Scale, Transform) and its other overlays in the corners, switchable from its ≡ menu; the library (with Code as a second tab) beside the console (with Problems) underneath. The camera is Fly only.
- **Shaped like Aventail:** flat panels with underlined tabs and a small menu row under each title, bordered buttons with small corners, square fields.
- **Dockable:** drag any panel to an edge, onto another panel to tab them, or out to float it. Saved per computer, with presets in Window → Layouts (Default, Wide screen with the library on the left, Small screen with the library and console as tabs). Built on `egui_dock`.
- **Theme:** Aventail's dark neutral greys with one solid accent colour from the settings (petrol blue by default; red, green and plum as in Aventail).
- **Basics:** undo and redo, gizmos, several viewports, and a play mode that resets the scene when you stop.
- **The scripting window** (§3).
- **Debugging:** a frame profiler, debug drawing for physics and navigation, a command console, and stepping through Lust code.

---

## 14. Everything else a full engine needs

**0.1:**
- **Asset pipeline:** importing glTF, FBX, textures and audio; an asset database with stable IDs, so renaming a file doesn't break references; hot reload; texture compression. Importing Aventail's `.avtl` directly.
- **Input:** named actions instead of raw keys, gamepads, rebinding.
- **Physics:** Rapier.
- **Save games.**
- **Build and export:** Windows and Linux, macOS later.
- **Git, for teams** ([decision 0006](docs/decisions/0006-git-built-in.md)): project files made to merge, Git LFS for big files, a Git panel with commits, history and locking, and a merge tool that merges scenes entity by entity.

**Later:**
- **Navigation and AI:** navmeshes, flowfields (moving many units), behaviour trees or state machines.
- **Steam:** achievements, cloud saves, Workshop, Steam Deck.
- **Localization and accessibility:** subtitles, text scaling, colourblind options.
- **Modding** for games made with Solis (separate from editor scripting).
- **Cutscene timeline.**

**Decisions:**
- **License:** MIT ([decision 0007](docs/decisions/0007-mit-licence-and-separate-from-aventail.md)).
- **Platforms:** PC only. Consoles need NDAs that clash with open source, as with Godot.
- **Documentation, templates and tutorials** are a pillar of their own for a beginner-focused engine.

---

## 15. Roadmap

**Solis 0.1 in about a year,** in twelve phases (0 to 11), each ending with a gate the project lead tries by hand. The full plan is in [ROADMAP.md](ROADMAP.md). In short: groundwork and the editor shell, rendering, the world, the editor, scale, simulation, sound and animation, then **Lust and Lua join** (Phase 7), netcode, effects and game UI, scripting and export, and the 0.1 release.

**The flagship sample:** a small space battle. Two capital ships, a carrier launching fighter squadrons, a few hundred lasers in the air, over the network. It tests instancing, components, VFX and lockstep at once.

**Not in 0.1:** open-world streaming, the full Solis Eclipse toolkit, the UI editor, rollback, advanced animation.

**Risks:**
1. **Lust is needed from Phase 7.** Phases 0–6 don't need it, and if it's late, Phases 8 and 9 go first.
2. **Iteration speed** (§3.1). If changes take too long to see, the engine feels slow however fast games run.
3. **Time.** The pace is set by decisions, testing and review, not by writing code. Too many projects at full speed turns a year into three.

---

## 16. Working together

Tandem work, with issues tagged by difficulty so there's always something the right size.

- **The project lead:** the decisions before each phase, every gate, the theme, and choosing controls. From Phase 7: the ready-made gameplay components in Lust, the flagship sample's game logic, and LustScript editor tools. Before then, small Rust tasks tagged `good first issue`, if wanted.
- **Claude:** the engine in Rust, and the Lust side of Phase 7 in the Lust repo.

Both sides review each other's pull requests.

**Builds:** the repo is private, so builds don't run on every update. See [decision 0001](docs/decisions/0001-builds-on-demand.md).

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
- 2026-10-21: [0006](docs/decisions/0006-git-built-in.md): Git built into the editor, with project files made to merge, Git LFS, locking, and a scene merge tool.
- 2026-10-21: [0007](docs/decisions/0007-mit-licence-and-separate-from-aventail.md): MIT licence; Solis stays separate from Aventail, copying and adapting Aventail's code, which its owner relicenses.
- 2026-10-21: [0008](docs/decisions/0008-lighting-is-a-project-choice.md): lighting is a choice per project; real-time lights with sky light come first, baked and global illumination later.
- 2026-10-21: [0009](docs/decisions/0009-fly-camera-controls.md): the viewport camera flies with Unreal's controls (hold right mouse; W A S D, Q and E).
- 2026-10-21: [0010](docs/decisions/0010-windows-exe-on-merge.md): a Windows `solis.exe` is built on GitHub when code is merged into main, and by hand. Loosens 0001 for this one build.
