# Solis: design

> A free, open-source 3D game engine.
>
> **Status:** Phase 1 (seeing things) is built; the progress tracker is in [README.md](README.md). The engine is Rust and games are Lust ([decision 0002](docs/decisions/0002-engine-in-rust-games-in-lust.md)); the build order is in [ROADMAP.md](ROADMAP.md).

Sections marked **(to decide)** are open. Everything else is the current plan, and changes go in the decision log (§17) with a file in `docs/decisions/`.

---

## 1. What Solis is

A lightweight, free alternative to Godot, Unity and Unreal for people getting into game development, aimed at:

- **Stylized games,** with tools for that look out of the box.
- **Strategy games** with large space and ground battles (§2).
- **Medium-poly online co-op and PvP games,** with good netcode built in.
- **Big open worlds,** built from zones and LODs.

**3D, and first-class 2D** ([decision 0054](docs/decisions/0054-first-class-2d.md), 2026-10-27; it was "3D only"). 2D is built as flat things in the same world, so lighting, effects, materials, sound and netcode serve both, with real 2D tools on top: a pixel-perfect view, sprites from Aseprite, tile maps that change as a game runs, autotiling, a tile painter and 2D physics. Focus is still a feature: one engine and one way of working, not a 2D engine beside a 3D one.

**Who it's for:** people who want to make games without learning a huge engine, and without an engine that takes a cut of their money.

### Name

**Solis** (Latin for "of the sun"). No 3D engine by that name was found; the nearest are "Solas", a small .NET engine, and an old 2D game called Solis on SourceForge. The effects suite is **Solstice** (§9, [decision 0047](docs/decisions/0047-the-effects-suite-is-solstice.md)): the sun theme, and clear of the trademarks and products that ruled out Eclipse, Ember, Aura and Aurora.

---

## 2. Scale target

**Large strategy battles:** hundreds of units normally, a few thousand at peak (cheap unit spam), and tens of thousands of projectiles and effects on screen, running smoothly.

**Tested at three levels** ([decision 0030](docs/decisions/0030-scale-is-tested-at-three-levels.md)): the **target**, 2,000 moving units and 20,000 projectiles at 60 fps; **stress**, 10,000 units and 100,000 projectiles holding 30 fps without stutter; and a **ceiling** run that keeps adding until 60 fps ends, and reports where. Games will go past the target (two fleets of fighter squads, a horde of 6,000 zombies), so we test past it.

That's the number the renderer, the component system and VFX are designed against. It is not the tens of thousands of units of the very largest strategy games.

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
- **Lua is quick to change and slower to run:** no compile step, but slower than compiled code, with a garbage collector. Fine for most gameplay; heavy per-unit work at the scale in §2 is better in Lust or Rust. For deterministic netcode (§6), Lua code has to follow a few rules, such as not relying on the order `pairs` walks a table in. How much slower is measured and explained, not guessed ([decision 0055](docs/decisions/0055-lua-speed-measured-and-explained.md)): the manual says which language suits which work, with Solis's own numbers; the profiler points out slow Lua components; and Luau's native code generation is used if it is faster and gives identical game results. Lua isn't translated into Rust: Lust is the typed language that compiles to Rust.
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
- **Sessions:** lobbies, invites, and relays to get through home routers.
- **Foundations:** a fixed simulation timestep, separate from the frame rate; seeded randomness; compact message encoding.
- **Tools:** a lag and packet-loss simulator, network stats, replays, and desync detection.

**One model, optional: lockstep** for strategy games, where only inputs are sent. Strict cross-platform determinism (careful float maths; Rapier's determinism mode for physics) is switched on for lockstep games, and Lust's checker can help keep to its rules.

**Other models plug in through existing Rust crates:** `renet` or `lightyear` for server-authoritative games, `ggrs` for rollback, `matchbox` for peer-to-peer. Solis makes sure they fit and documents how.

**The pitch:** the shared networking layers built in properly, with a strategy-ready model included.

---

## 7. Rendering

wgpu, lightweight, built for medium-poly, stylized games. **AMD first, when a choice must be made** ([decision 0049](docs/decisions/0049-amd-first-when-a-choice-must-be-made.md)): solid and fast on every card, tuned best for AMD's.

**Light is Solis's selling point** (the project lead, 2026-10-26; [decision 0046](docs/decisions/0046-light-before-netcode.md)): skies, sunsets and light that feel right, never flat or sterile. Physics underneath (a physical sky with ozone and haze over distance, clouds lit by the sun), art direction on top (white balance, grading, colour by time of day, a painted sky kept as a style). The research is in [docs/reports/lighting-research.md](docs/reports/lighting-research.md) and the plan in [docs/plans/light.md](docs/plans/light.md).

**0.1:**
- Materials, including a stylized and toon option
- **GPU instancing** (essential for §2), frustum culling, LODs
- Tonemapping, automatic exposure, bloom, white balance and colour grading (much of a stylized look lives here)
- **A physical sky** (Hillaire's method) with an artist's tint and fog colour ramp over it, time of day and night (moons, stars, aurora), weather and moods, flat and volumetric clouds, haze over distance, volumetric fog and light shafts
- **Light with shape:** sky light from the real sky, ambient occlusion, cascaded and contact shadows, hundreds of lamps with a few shadowed, glowing surfaces and area lights, reflection probes and reflections on the screen, and bounced light every way that can be made fast: baked light probes, bounce from the screen, real-time probes traced against a simplified world, and a time-boxed experiment for a better method
- **The light ramp look** for blocky games, beside Standard, Stylized and Unlit
- **Materials and shaders** (Phase 14, [decision 0048](docs/decisions/0048-materials-and-shaders-before-effects.md)): materials as assets with their own look; inputs that drive any material value (the light around a surface, time, distance, height, wetness, noise, values from game code); a node graph for shaders, saved as text; a code node for experts; stylized building blocks (stylized games live on custom shaders)
- Quality levels (Low, Default, High), so the scale target holds on mid-range cards
- The looks aimed for, from the project lead's reference games (kept by the project lead, described in the Phase 13 plan): warm, cosy light; a stylized world of weathers; neon nights; a huge sky gradient; realism; light from living things; and simple, readable light as a game's identity

**Later:**
- Baked lightmaps, TAA and upscaling, hardware ray tracing once wgpu's is stable
- Occlusion culling
- Impostors for distant objects

**Lighting approach:** a choice per project ([decision 0008](docs/decisions/0008-lighting-is-a-project-choice.md)). Real-time lights with sky light came first; baked and real-time bounced light arrive in Phase 13; lightmaps are a further source after 0.1.

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

## 9. VFX: Solstice

Gets a lot of attention: **good enough to be a professional tool in its own right**, while games stay the priority (the project lead, 2026-10-27). Stylized VFX tools out of the box: flipbooks, trails, ribbons, mesh particles, stylized shading, and lots of particles through instancing. It also has:
- **Easy to learn:** the same graph, inputs and keys as materials (decision 0050).
- **A pixel mode** for pixel-art games: pixel-shaped particles snapped to the grid, the game's palette, and ready-made wind streaks, fireflies, bubbles and more (decision 0054).
- **Beginner-friendly:** an extensive library of presets and building blocks with simple settings on top.
- **Custom code** in Lust, Rust or Lua.
- **Interactive light:** emission lights the world, the fog and the bounced light, not just a glow.
- **Baking** to flipbooks, moving-mesh textures and cached playback.
- **Designed from an in-depth study** before it's built (decision 0051).

**Open question:** the wishlist mentions reusing Aventail's VFX system (originally named Gauntlet). Aventail's plans don't contain one so far; "Gauntlet" came up as a rejected name for Aventail itself. So Solstice is probably new design work, built for games first, and Aventail could adopt it later. **(to confirm)**

---

## 10. Animation

**Settled for Phase 7** in [decision 0057](docs/decisions/0057-animation-near-live-far-baked.md): live poses near and baked clips far, `.solanim` animators, blend spaces by one and two numbers, cues per clip, glTF only (FBX later), and one state machine for 2D sprite frames and 3D skeletons.

- Skeletal animation imported from glTF (FBX later; Blender converts it)
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

**Settled for Phase 7** in [decision 0056](docs/decisions/0056-sound-with-kira.md): Kira, WAV and Ogg through permissive readers, `.solsound` events, five buses with Voice ducking Music, reverb by where the listener is, 64 sounds heard at once, and sound that never changes the game.

**Libraries:** Kira (Rust) for playback and mixing; Steam Audio (open source) for 3D sound and occlusion. FMOD and Wwise are proprietary, so they're ruled out.

---

## 12. UI

- **The editor** uses egui, like Aventail and Halberd.
- **Game UI** is drawn in Solis, with **Sungraph UI** ([decision 0039](docs/decisions/0039-game-ui-is-drawn-in-solis.md)): a visual UI editor where UI is drawn canvas style (lines, shapes, curves, text) rather than built from imported images, so it's sharp at any size; any value can be tied live to game code; every element is reachable from code, because a UI file is ordinary Lust (`view { }`); and styles such as glow, pixelation and stylized outlines apply to the whole UI. Lust's plan has its own `view { }` renderer as its biggest item; Solis's game UI renderer could be that renderer, but Solis doesn't wait for it. A Rust UI layer comes first, which `view { }` compiles to later. It's built, whole, in Phase 12, right after 2D and the playtest export (decisions 0046, 0048 and 0054; the first plan had it in Phase 9).
- **Other visual editors** follow the same rule, a graph saved as readable text: state machines (Phase 7), behaviour trees, and shader and material graphs.

---

## 13. The editor

**Uniformity comes first** ([decision 0050](docs/decisions/0050-one-way-to-work-everywhere.md)): every editor in Solis works one way: one graph editor, one Properties layout, one preview tab, one table of shortcuts, the same ideas everywhere. Learn one and you know them all.

**Being remade** ([decision 0099](docs/decisions/0099-the-g-layout.md), Shared 1a to 1d in ROADMAP.md): the editor's frame becomes the project lead's own design, the "G layout" ([`docs/design/g-layout/`](docs/design/g-layout/README.md)): a tab bar, a ribbon whose tabs are the areas of the engine, one or two editor groups, a shelf of tabs along the bottom and a status bar, in one dark neutral theme with five accents ([decision 0100](docs/decisions/0100-one-theme-five-accents.md)). What the engine lacks is shown as a placeholder, and one setting hides them all ([decision 0101](docs/decisions/0101-unfinished-parts-as-placeholders.md)). The look is built; the frame below is today's until the top and the middle are, and each line is rewritten as its part is replaced.

Today's frame, settled in [decision 0004](docs/decisions/0004-editor-layout.md): flat panels with tabs, a one-row top bar, and every panel dockable.

- **Top bar, one row:** menus and document tabs on the left (Unreal's tab system), and Play, Build and Build & Play on the right.
- **Default arrangement:** Area and Layers above Properties on the right, full height; the viewport in the centre, with the tools as icons at its bottom centre (Select, Move, Rotate, Scale, Transform) and its other overlays in the corners, switchable from its ≡ menu; the library (with Code as a second tab) beside the console (with Problems) underneath. The camera is Fly only.
- **Shaped like Aventail:** flat panels with underlined tabs and a small menu row under each title, bordered buttons with small corners, square fields.
- **Dockable:** drag any panel to an edge, onto another panel to tab them, or out to float it. Saved per computer; Window → Reset layout brings back the standard one (no presets since [decision 0061](docs/decisions/0061-settings-window-and-no-layout-presets.md)). Built on `egui_dock`.
- **Theme:** one, Neutral dark: the design's neutral greys with one accent colour from the settings (blue, grey, orange, teal or purple; blue by default) and dark text on accent fills. Colours that mean something, such as warnings, errors and the axes, keep their own, and Play is the one light button ([decision 0100](docs/decisions/0100-one-theme-five-accents.md)).
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
- **Store features** (Steam's achievements, cloud saves, Workshop) are left to each game's developer, not built into the engine ([decision 0098](docs/decisions/0098-ideas-become-milestones.md)).
- **Localization and accessibility:** subtitles, text scaling, colourblind options.
- **Modding** for games made with Solis (separate from editor scripting).
- **Cutscene timeline.**

**Decisions:**
- **License:** MIT ([decision 0007](docs/decisions/0007-mit-licence-and-separate-from-aventail.md)).
- **Platforms:** Windows PCs ([decision 0012](docs/decisions/0012-windows-only.md)): the editor and the games it builds. Consoles need NDAs that clash with open source, as with Godot.
- **Documentation, templates and tutorials** are a pillar of their own for a beginner-focused engine.

---

## 15. Roadmap

**Solis 0.1 in twenty phases (0 to 19),** each ending with a gate the project lead tries by hand, with no timeframe. The full plan is in [ROADMAP.md](ROADMAP.md). In short: groundwork and the editor shell, rendering, the world, the editor, **Lust and Lua join** (Phase 4), scale, simulation, sound, **compatibility**, **2D in two parts** (with animation's state machine), a **playtest export**, game UI, **light, sky and atmosphere**, **materials and shaders**, effects, **3D animation** (after 2D is complete, [decision 0059](docs/decisions/0059-2d-first.md)), netcode, scripting, the full export and Git, and the 0.1 release.

**The flagship sample:** a small space battle. Two capital ships, a carrier launching fighter squadrons, a few hundred lasers in the air, over the network. It tests instancing, components, VFX and lockstep at once.

**Not in 0.1:** open-world streaming, the full Solstice toolkit, rollback, advanced animation.

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
- 2026-10-20: Scale target: large strategy battles (§2). Game-specific systems stay out of the engine.
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
- 2026-10-26: [0046](docs/decisions/0046-light-before-netcode.md): light, sky and atmosphere get Phase 8, chosen by the project lead; effects and game UI get a phase each, and netcode moves after them; bloom and grading move into Phase 8, and bounced light, baked and real-time, arrives there.
- 2026-10-26: [0047](docs/decisions/0047-the-effects-suite-is-solstice.md): the effects suite is called Solstice (Eclipse, Ember, Aura and Aurora were taken in this field).
- 2026-10-26: [0048](docs/decisions/0048-materials-and-shaders-before-effects.md): materials and shaders get Phase 9, between light and effects, as general tools (any material value driven by the world); effects, game UI, netcode, export and 0.1 become Phases 10 to 14.
- 2026-10-26: [0049](docs/decisions/0049-amd-first-when-a-choice-must-be-made.md): Solis supports every card and stays fast on all of them, but when a choice must be made between tuning for one maker or another, it tunes for AMD: AMD cards are the reference for gates and the scale check, compute is sized for AMD's waves, AMD's open-source FidelityFX libraries are first in line, and the backend is chosen by measuring.
- 2026-10-27: [0050](docs/decisions/0050-one-way-to-work-everywhere.md): one way to work everywhere: one graph editor, one Properties layout, one shortcut table (tested), the same inputs, curves and gradients, and the same three languages for custom code, in every editor.
- 2026-10-27: [0051](docs/decisions/0051-understand-then-build.md): understand, then build: each core system starts with an in-depth study and Solis's own design, measured at the scale target; libraries are for what isn't Solis's core.
- 2026-10-27: [0052](docs/decisions/0052-dependency-safeguards.md): safeguards against dependency trouble: a register of every library with its exit plan (checked by a test), walls around each library, a source archive whenever Cargo.lock changes, and a reason for every new library.
- 2026-10-27: [0053](docs/decisions/0053-compatibility-before-2d.md): compatibility comes first (Phase 8): a backup before every upgrade, upgraders for every file format with old examples kept as tests, save games that survive updates, the game-code interface guarded, real projects tested before an update ships, breaking changes labelled.
- 2026-10-27: [0054](docs/decisions/0054-first-class-2d.md): first-class 2D in two parts (Phases 9 and 10), built as flat things in the same world with real 2D tools on top; Rapier2D; each tileset chooses its own kind; then a playtest export (11) and game UI (12); a pixel mode in the effects suite; 2D bounced light in the light phase. Replaces "3D only".
- 2026-10-27: [0055](docs/decisions/0055-lua-speed-measured-and-explained.md): Lua's speed is measured and explained, and Luau's native code is used only if faster with identical results; Lua isn't translated into Rust.
- 2026-10-27: [0056](docs/decisions/0056-sound-with-kira.md): sound with Kira in a `sound` crate; WAV and Ogg through hound and lewton; `.solsound` events; buses Master > Music, Effects, Voice, Ambience, UI with Voice ducking Music; reverb by where the listener is; 64 sounds at once; sound never changes the game (answers of 2026-10-25).
- 2026-10-27: [0057](docs/decisions/0057-animation-near-live-far-baked.md): animation with live poses near and baked clips far; cues per clip in sidecars; blend spaces by one and two numbers; `.solanim` animators; the graph editor's controls, shared by every graph; the project lead's own character for the gate; one state machine for 2D sprite frames and 3D skeletons.
- 2026-10-28: [0058](docs/decisions/0058-listeners-and-who-hears-what.md): a `Listener` component (the camera hears when there's none), and "Heard by" on sound events (everyone, only who caused it, or named groups); nothing about sound crosses the network; Phase 7, S8.
- 2026-10-28: [0059](docs/decisions/0059-2d-first.md): 2D first. Phase 7 is sound only; the animation state machine, the animator graph editor and frame events that play sounds come with 2D's frame animation (Phase 9); 3D skeletal animation is a new Phase 16, after Solstice; netcode, scripting and export, and 0.1 become 17, 18 and 19, and Phases 16 to 19 are "later": not built or planned in detail until Phase 15 is done and the project lead says so. Light and effects stay full engine features, 3D as much as 2D.
- 2026-10-28: [0060](docs/decisions/0060-amber-and-colours-that-mean-something.md): amber is the standard accent and marks only what's chosen; Play and Build & Play are always blue, Stop red while running, the console's levels grey, amber and red, and the Area panel colours things by kind as the Library does.
- 2026-10-28: [0061](docs/decisions/0061-settings-window-and-no-layout-presets.md): Edit → Settings…, a window of pages with a search box, holds the editor's settings (first: the accent colour and Reset layout); the layout presets go, since panels dock freely; Window → Reset layout stays.
- 2026-10-28: [0062](docs/decisions/0062-2d-on-the-ground-drawn-small-scaled-whole.md): 2D lies on the ground, seen from above; a camera's View can be 2D; a project's 2D settings live in `pixels.solpix` (Settings → This project → 2D, 480 × 270 and 16-pixel tiles to start); the game is drawn small and scaled up in whole steps (or to fill), slid by the bit of a pixel the camera moved; the editor gets a 2D view.
- 2026-10-28: [0063](docs/decisions/0063-sprites-read-from-aseprite-cut-out-front-from-the-foot.md): sprites come straight from `.aseprite` files, read by Solis's own reader (fuzzed) and packed into an atlas per file; they're cut out and write depth, so front and behind come from where each stands with no sorting (only see-through sprites blend, back to front); their colours are drawn exactly as painted; a placed sprite stands as drawn on its feet slice, on a whole pixel.
- 2026-10-28: [0064](docs/decisions/0064-frame-animation-clips-files-and-tabs.md): frame animation. Clips are Aseprite's tags plus clips made in Solis, in a sprite's clips file (`.solframes`); a Clips tab beside Properties and a Frames tab at the bottom (frames along a time ruler, events in a lane) come with an open clips file; a Sprite's clip and time are game state, moved on each tick; a frame's event plays the sound event of its name and reaches game code as `AnimationCue`; the viewport animates only with Preview animation on.
- 2026-10-28: [0065](docs/decisions/0065-engine-api-guidelines.md): game code's calls follow the project lead's API guidelines for Lust ([docs/game-api-guidelines.md](docs/game-api-guidelines.md)): a component's own abilities called bare (`this` for the whole thing), methods on other entities and a few engine objects, the project's names as states written by the editor, anything made in an editor reached through a Lust file the editor writes, one shape for names. The existing calls move over once Lust has four changes (states to natives, `entity` methods, bare abilities and `this`, editor-made parts as fields).
- 2026-10-28: [0066](docs/decisions/0066-animator-state-machine.md): animators (`.solanim`) choose a sprite's clip: parameters (number, switch, trigger, direction), states playing a clip or a clip per facing direction (the nearest, keeping its step), transitions with conditions; stepped each tick before sprites, in the checksum and saves (`solis save 4`); edited in the Animator tab on the shared graph editor, as mocked up; game code reaches them through a Lust file the editor writes, with the API milestone.
- 2026-10-28: [0067](docs/decisions/0067-tile-maps.md): tile maps: terrains (name, one-character code, colour) in a tileset file (`.soltileset`); each map its own file (`.soltilemap`, chunks of 32 × 32 as rows of codes, sparse values) placed by an entity's Tile Map; layers and cell values (number, switch, choice) are the map's own, layers under or over sprites; each entity runs its own copy, in the checksum and saves (`solis save 5`); the Map tab beside Properties (mock-up A) and a Values menu over the viewport; game code reaches maps through a Lust file the editor writes, with the API milestone.
- 2026-10-28: [0068](docs/decisions/0068-tile-art-and-autotiling.md): tile art and autotiling: a tileset's art comes from one PNG or Aseprite picture (pictures get IDs); each terrain has a kind (plain, 16-tile on the dual grid, 47-tile, edges, its own rules up to 9 × 9) whose slots its pieces fill, variations picked by weight from the cell's place, animated by columns or Aseprite frames; a missing piece is pink, never another; tileset format 2; drawn chunk by chunk from an atlas; the Tileset tab (mock-up A) with the steps, the picture, each kind's template and a try-it patch.
- 2026-10-29: [0069](docs/decisions/0069-the-tile-painter.md): the tile painter: while a map is selected, Brush (B), Rectangle (U), Line (N), Fill (G) and Eyedropper (I) join the viewport's tool bar with Exact, Art or Meaning (Tab) and the layer painted; the Map tab's terrain swatches are the palette (mock-up B2); left-drag paints, right-drag empties, Alt + click picks; the eyedropper's copy repeats like a pattern, X, Y, Z flip and turn it; Ctrl places exact tiles, kept per chunk in tile map format 2; each stroke one undo step, saved when let go.
- 2026-11-04: [0070](docs/decisions/0070-usable-first.md): usable first: the engine audit chose a revamp over a restart, and a new build order around the project lead's farming game (editor basics, one project file with the rest as Lust, the API milestone, game-wide objects, 2D movement, the camera, game UI and its editor, day and night, the playtest export); blue is the standard accent; nothing is shown that isn't built; New anywhere in the Library; a start page.
- 2026-11-04: [0071](docs/decisions/0071-2d-physics-and-top-down-movement.md): 2D physics and top-down movement, built ahead of the milestones waiting on Lust: a separate 2D world (Rapier2D); solid is a terrain's setting, the game's choice; a map's solid cells merge into walls chunk by chunk, never one per tile; Solis's own mover slides along them without catching on seams; a Collider2D in pixels, ticked Character for game code to walk; the viewport's Walls switch; the kit's TopDownController (8 or 4 directions, never faster diagonally); the arrow keys walk too; triggers wait for the API milestone.
- 2026-11-04: [0072](docs/decisions/0072-lust-alpha-59-and-the-engine-folder.md): Lust moves from alpha 35 to 59; Solis finds its own folder (Lust's host folder) from the first line of the files describing the engine, so the name stays out of the code; the compiler's notes go to the Console; the kit, samples and manual write `this` instead of `me`.
- 2026-11-05: [0073](docs/decisions/0073-world-and-entity-calls-as-methods.md): the world and entity calls move to the API guidelines' shape: an entity's abilities as methods (`position()`, `target:moveTo(p)`, new `face(d)`), `World` and `Saves` objects (`Saves:saveTo`, since `save` is Lust's own word), `solis/project.lst` with the project's assemblies as `state Assembly` (`World:spawn(Assembly.Ships.Fighter, p)`), Lua's `solis.World:say` and `solis.entity.moveTo`; a straight switch, old calls are mistakes that say the new way.
- 2026-11-05: [0074](docs/decisions/0074-input-calls-on-input-with-actions.md): input calls move onto the `Input` object (`Input:pressed(Action.Jump)`, `Input:binding`), with the project's input actions as `state Action` in `solis/project.lst`, written again when they're saved; an action whose name Lust can't take is listed in the Console; Lua's `solis.Input:held("Jump")`.
- 2026-11-05: [0075](docs/decisions/0075-physics-calls-on-the-entity-and-one-ray-call.md): physics calls move onto the entity (`push`, `velocity`, `setVelocity`, `walk`, `grounded`), and a ray is one call, `Physics:castRay`, giving back a `RayHit` (`found`, `distance`, `entity`, `normal`); the `Host` trait says `walk` and `grounded` too.
- 2026-11-05: [0076](docs/decisions/0076-sound-calls-on-audio-with-sounds-and-buses.md): sound calls move onto `Audio` (`Audio:play(Sound.Sfx.Footstep)`, `Audio:playAt`, `Audio:setVolume(Bus.Music, v)`) and the entity (`playSound`, `stopSounds`, `setSoundPlaying`), with the project's sound events and buses as `state Sound` and `state Bus`; names in all capitals are folded (`Bus.Ui`), and the engine finds files and buses ignoring capitals.
- 2026-11-05: [0077](docs/decisions/0077-clips-and-cues-as-names.md): clips and frame events are the project's names (`state Clip`, under each sheet's path, and `state Cue`), written again whenever the sheets change; `animate`, `animateFromStart` and `stopAnimation` on the entity; the events `CueReached(Cue cue)` and `ClipFinished(Clip clip)`.
- 2026-11-05: [0078](docs/decisions/0078-animators-reached-through-their-own-files.md): each animator gets its own Lust file in `solis/animators/`, with a state of its states and a class with a method per parameter (`anim:setMoving(true)`, `anim:current()`, `anim:jumpTo(…)`); a component holds it as a field the engine fills in (`FarmerAnimator anim;`); what's set lands at the end of the tick.
- 2026-11-05: [0079](docs/decisions/0079-tile-maps-reached-through-their-own-files.md): each tile map file gets its own Lust file in `solis/maps/` (states of its layers, terrains and choices; a class with `terrain`, `setTerrain` and a reader and setter per value), held by a component as a field Solis fills in; reads are the map as the tick started, changes land at its end.
- 2026-11-05: [0080](docs/decisions/0080-objects-change-only-in-their-own-update.md): game-wide objects (Lust's passed `object`s) tick once a tick before the components, on the first core, and are copied to the others; only their own update changes them (a write anywhere else stops with a message), and saves keep them.
- 2026-11-05: [0081](docs/decisions/0081-what-solis-makes-is-lust.md): what Solis makes is Lust: every file but `project.solis` becomes a Lust data file named `<name>.<kind>.lst`, read with Lust's own reader; older files move when a project opens, with a backup; tile maps and tilesets first, then the 2D camera and UI, then the rest.
- 2026-11-05: [0082](docs/decisions/0082-the-2d-follow-camera-and-cells-at-places.md): the kit's FollowCamera2D (dead zone, smooth damping, a tile map's bounds, no keys); a map's own class turns places into cells and back (`cellAt`, `placeOf`), `entity:mapBounds()` and `World:viewSize()`.
- 2026-11-05: [0083](docs/decisions/0083-game-ui-on-any-screen.md): game UI fits any screen: elements pinned by anchors (nine points or stretched), a scale rule per screen (grow with the screen, keep its size, whole steps for pixel art), a Screen list of common sizes to preview; pictures cut from sprite sheets kept in a Lust file beside them; the first milestone lays out and draws shapes, text next.
- 2026-11-05: [0084](docs/decisions/0084-game-ui-text.md): game UI text: fonts read by `ab_glyph` (already in the build), each letter made once into a single-channel distance image in one atlas and drawn sharp at any size (hard-edged for whole steps); Inter for now; no wrapping yet.
- 2026-11-05: [0085](docs/decisions/0085-the-editor-overhaul.md): the editor overhaul: the level editor mock-up of the editor UI research is the editor's target, built in seven milestones (the frame, Properties, the Library drawer, one search, playing, workspaces, help in place) before the game UI editor, which lives in its UI workspace.
- 2026-11-05: [0086](docs/decisions/0086-the-launcher.md): the launcher: the start page grows into a full page inside Solis (not a separate program) with Projects (pinned first, search, sort, rename, copy, scan a folder, the build each project was last saved with, recorded as `saved_by` in the project file), Learn and Updates, built in three milestones before game UI.
- 2026-11-05: [0087](docs/decisions/0087-screens-through-the-engine.md): screens are Lust's own views (an `assembly` with a `view` in `assets/ui/`, Lust's own pieces), run in a Luau of their own after each tick, reading the game through the engine (objects copied in, `Ui:entity()`, a reading method per component written by Solis); Lust stays as it is: no `tied` or drawing words.
- 2026-11-05: [0088](docs/decisions/0088-menus-send-events.md): menus: a screen's button sends an event (Lust's `send`), which joins the next tick's input and recordings; game code shows and hides screens with `showScreen(Screen.Pause)` and `hideScreen` on the entity, the screens being project states; pausing is `World:setGameSpeed`.
- 2026-11-06: [0089](docs/decisions/0089-atlas-crop-and-the-ui-editor.md): **Atlas Crop** cuts named pictures from a sheet into `<picture>.atlas.lst` (game code's `Picture.Sprites.Tools.Axe`); the UI editor's layout is approved as mocked up; a screen's glow, outline and pixel size are its own look, Lust's `shadow` a glow per piece; screens show pictures with Lust's `image "file#Crop"`; text and UI size are player settings on `Screens`.
- 2026-11-06: [0090](docs/decisions/0090-always-through-the-launcher.md): Solis always starts on the launcher, in a small window, and a project opens from it (the last one isn't reopened); closing the editor's window returns to the launcher; a newer build is a bar over the projects instead of a question as Solis starts.
- 2026-11-06: [0091](docs/decisions/0091-player-settings-and-the-window.md): the game's own menu offers the player's settings through `Screens:setTextSize`/`setUiSize` and `Window:setFullscreen`/`setResolution`/`useNextResolution`; Alt+Enter switches fullscreen; the settings are the player's (not in saves or recordings), kept from play to play in the editor and in the saved-games folder by an exported game.
- 2026-11-06: [0092](docs/decisions/0092-day-and-night-and-lamps-on-sprites.md): day and night is an Environment setting (a day's length, its start, and time→colour stops) with `World:timeOfDay` and `World:setTimeOfDay`, kept in saves (format 8); Light sprites (on in new areas) lights sprites and tile maps by the day's tint and the lamps, in optional steps; Lights get Only at night.
- 2026-11-06: [0093](docs/decisions/0093-the-playtest-export.md): File > Export for playtesting makes a folder holding Solis renamed after the game, `playtest.txt` and the project in `game/`; started there, it opens straight into the game, with the window following the player's settings; `Window:quit()` closes it; saves, settings, a log and a crash log go in the player's Saved Games/<Game> folder.
- 2026-11-08: [0094](docs/decisions/0094-parallax-layers.md): parallax layers, brought forward from Phase 10 for the project lead's platformer: a Parallax component turns a sprite into a layer (scroll across and up, repeat, in front), drawn where placed when the camera looks at it; layers behind are drawn before everything, slowest first; through a 2D camera each layer is its own small picture, slid by its own bit of a pixel.
- 2026-11-10: [0095](docs/decisions/0095-rules-from-the-video-editor.md): four rules from the project lead's video editor project, merged into Solis's: no name-dropping of any company or product, folder READMEs and CLAUDE.md always current, readable code without heavy commenting, and competing approaches compared for big systems (fast, smooth and robust is the bar).
- 2026-11-10: [0096](docs/decisions/0096-graphite-theme-and-rounded-panels.md): a second theme, Graphite (the project lead's near-black palette), chosen in Settings beside Standard, and rounded panels (cards with gaps, pills) as a separate switch; colours that mean something stay the same in both.
- 2026-11-10: [0097](docs/decisions/0097-three-tracks.md): the rest of the plan in three tracks: 2D finished first, what suits both built once for both (materials, effects, weather and the finished picture, export and Git, netcode), then 3D (animation, light in four steps); big phases split, the same milestones.
- 2026-11-10: [0098](docs/decisions/0098-ideas-become-milestones.md): every idea on the ideas list becomes a milestone in the shared track (Shared 2 to 6 and 10, mods in Shared 11, navmeshes in 3D 1), the editor's UI remade comes first (Shared 1), and photo mode and built-in store features aren't planned.
- 2026-11-11: [0099](docs/decisions/0099-the-g-layout.md): the G layout: the editor's frame becomes a tab bar, ribbon, editor groups, shelf and status bar, to the project lead's own design (`docs/design/g-layout/`), built as Shared 1a to 1d; today's keys stay, Solis's words stay on screen, the running game is a Game tab, there is a UI ribbon tab, and playing is shown by Play becoming Stop (with Pause and Step), not by colouring the window. Replaces 0004's layout and 0061's Settings window; changes 0085, 0086 and 0090.
- 2026-11-11: [0100](docs/decisions/0100-one-theme-five-accents.md): one theme, Neutral dark, with five accents (blue #5C9CFF by default) and dark text on accent fills; Standard, Graphite and rounded panels are removed and old accents mapped; Play is the one light button; interface scale and density are settings. Replaces 0096; changes 0060.
- 2026-11-11: [0101](docs/decisions/0101-unfinished-parts-as-placeholders.md): what the engine lacks is shown where the design puts it, as a quiet placeholder with a hollow square mark, and one setting, Hide unfinished parts, hides them all without leaving gaps. Changes 0070 and 0085 ("nothing that isn't built is shown" becomes "nothing unfinished is shown unmarked").
- 2026-11-11: [0102](docs/decisions/0102-linux-checks-paused.md): the Linux checks on GitHub are paused until the engine is much further along (Ubuntu's package servers were down and blocked every merge); lints, tests and docs run on Windows, formatting and the licence check stay, and the Linux tests and the window test are run by hand before each push. Changes 0018.
