# Solis: design

> A free, open-source 3D game engine. Part of the Rhylib family, alongside Aventail 3D and Halberd.
>
> **Status:** design only. No code yet. Lust comes first; Solis ramps up once native Lust builds work.

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
| **Lust** (`.lust`) | The engine itself and all game code: components, gameplay, everything that ships | Machine code (native Lust, through LLVM) |
| **LustScript** (`.lsc`) and **Lua** | **Editor scripting only:** custom tools, batch jobs, importers, generators, in a scripting window like Blender's or 3ds Max's. Never game code. | Luau, through `mlua`, the same as Aventail |
| **LustQL** (`.lql`) | Stored data in SQLite | **(to design in Lust first)** |

- **Lust is written once and compiled natively.** No swapping from Lua to Lust later is needed: editor scripts stay LustScript/Lua permanently, as Blender never replaced Python. `lua { }` blocks and plain `.lua` files are fine in editor scripts.
- **If native Lust is late,** the fallback is building the Rust foundation, renderer and editor shell first (§4), which don't need Lust. See §15.

### What Solis needs from Lust v2

These should go into Lust's design before its native compiler is built:

1. **Calling C and Rust libraries** (a C interface). Essential: the foundation (§4) is Rust.
2. **Threads or a job system.** The engine has to use every CPU core.
3. **Plain data in packed arrays.** cArrays are close; thousands of units need their rows tightly packed in memory.
4. **States that own memory** fit zones (§8) and game modes: when a zone or a battle unloads, its memory is freed in one go.

---

## 4. Architecture

```
┌──────────────────────────────────────────────┐
│ Games (Lust)                                 │
├──────────────────────────────────────────────┤
│ Engine (Lust): components, scenes and zones, │
│ simulation and netcode, animation, VFX,      │
│ game UI, editor logic                        │
├──────────────────────────────────────────────┤
│ Editor scripting (LustScript / Lua, Luau)    │
├──────────────────────────────────────────────┤
│ Foundation (Rust, behind a C interface):     │
│ wgpu, windows and input, physics (Rapier),   │
│ audio (Kira, Steam Audio), sockets, Luau     │
└──────────────────────────────────────────────┘
```

- **The Rust foundation is thin and stable:** only the parts that talk to hardware or wrap existing libraries.
- **Shared with Aventail and Halberd:** the wgpu setup, picking, the viewport camera, the egui editor shell and theme, and the layered crates with generic names. These may move into a shared Rhylib core repo later **(to decide)**.
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

**Storage underneath (to decide):** `bevy_ecs` on its own, or Solis's own. It depends on how Lust's packed data turns out.

---

## 6. Simulation and netcode

**The gameplay simulation is deterministic by default:** the same inputs always give exactly the same result. One core then gives:

- **Lockstep** for strategy games: only inputs are sent.
- **Rollback** for small PvP games (later).
- **Server-authoritative snapshots** with client prediction, for larger co-op.
- **Replays and desync detection,** for free.

No free engine offers this out of the box, and it's Solis's netcode pitch.

**The cost is discipline,** which Lust's checker can help enforce:

- a fixed simulation timestep, separate from the frame rate
- seeded randomness only
- careful float maths (Rapier's cross-platform determinism mode covers physics)

**Also needed:** lobbies, connecting players behind home routers (relays, or Steam's networking), dedicated server builds, and a lag and packet-loss simulator for testing.

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

**Lighting approach (to decide):** baked lightmaps, light probes, or real-time global illumination. It shapes a lot, so decide early.

---

## 8. Worlds, zones and streaming

- **Scenes** are text files, merge-friendly in git (a selling point: Unity and Godot scenes are notorious for conflicts).
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

- **Layout:** a standard 3D editor. Entity list and inspector on the right, the library (assets and code) on the left or bottom and collapsible, a terminal, and Build and Build-and-Play in the top bar with tools.
- **Theme:** dark, but not the overdone near-black. Contrasting mid-greys with a few accent colours, possibly as gradients. Tried out as we go, and shared with Aventail and Halberd where it fits.
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

**Later:**
- **Navigation and AI:** navmeshes, flowfields (moving many units), behaviour trees or state machines.
- **Steam:** achievements, cloud saves, Workshop, Steam Deck.
- **Localization and accessibility:** subtitles, text scaling, colourblind options.
- **Modding** for games made with Solis (separate from editor scripting).
- **Cutscene timeline.**

**Decisions:**
- **License:** MIT proposed (the indie-engine norm; Godot and Bevy use it). **(to decide)**
- **Platforms:** PC only. Consoles need NDAs that clash with open source, as with Godot.
- **Documentation, templates and tutorials** are a pillar of their own for a beginner-focused engine.

---

## 15. Roadmap

**Solis 0.1 within about a year,** if Lust comes first and Solis waits its turn among the other projects.

| Months | Focus |
| --- | --- |
| 1–3 | LustScript and native Lust. For Solis: this document and its decisions. |
| 4–6 | Rust foundation, renderer, editor shell. |
| 7–9 | Components in Lust, physics, sound, the scripting window. |
| 10–12 | The flagship sample, lockstep netcode, polish, 0.1. |

**The flagship sample:** a small space battle. Two capital ships, a carrier launching fighter squadrons, a few hundred lasers in the air, over the network. It tests instancing, components, VFX and lockstep at once.

**Not in 0.1:** open-world streaming, the full Solis Eclipse toolkit, the UI editor, rollback, advanced animation.

**Risks:**
1. **Native Lust is on the critical path.** If it slips, Solis slows but doesn't stop: the foundation, renderer and editor shell don't need it.
2. **Time.** The pace is set by decisions, testing and review, not by writing code. Too many projects at full speed turns a year into three.

---

## 16. Working together

Tandem work, with issues tagged by difficulty so there's always something the right size.

- **The project lead:** editor panels and the theme, the asset library, built-in components, the sample game's logic, and LustScript editor tools.
- **Claude:** the native Lust compiler, the Rust foundation and its bindings, the renderer, component storage, deterministic netcode and world streaming.

Both sides review each other's pull requests.

**Builds:** the repo is private, so builds don't run on every update. See [decision 0001](docs/decisions/0001-builds-on-demand.md).

---

## 17. Decision log

- 2026-10-20: Solis is 3D only, aimed at stylized, strategy, online co-op/PvP and open-world games.
- 2026-10-20: Lust for the engine and game code; LustScript/Lua only for editor scripting.
- 2026-10-20: A thin Rust foundation behind a C interface, with the engine in Lust.
- 2026-10-20: Entities and components with typed events; no node tree or signals.
- 2026-10-20: Scale target: Empire at War-style battles (§2). Game-specific systems stay out of the engine.
- 2026-10-20: [0001](docs/decisions/0001-builds-on-demand.md): builds and checks don't run on every push.
