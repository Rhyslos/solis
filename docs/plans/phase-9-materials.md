# Phase 9 plan: Materials and shaders (draft)

**Draft for the project lead.** Materials and shaders got a phase of their own on 2026-10-26, between light (Phase 8) and effects (Phase 10, Solstice): [decision 0048](../decisions/0048-materials-and-shaders-before-effects.md). Light and effects meet in materials: how a surface takes light, glows, gets wet or sways in the wind.

This draft lists the choices still to make, as questions for the project lead, then a plan for every milestone. Once the questions are answered, the answers become decisions, and this plan is kept up to date as milestones are built.

**The principle (the project lead's):** build **general tools people can use in creative ways, not features for one need.** A plant that glows as night falls is one use of a tool that lets any material value follow anything in the world. A lamp that flickers, a shield that shimmers when hit and stone that darkens in rain are others, made by the game maker, not built into Solis one by one.

**Gate:** without writing code, the project lead makes four materials in the material showcase:
- a plant that glows as night falls;
- stone that darkens and shines when it rains;
- a toon character with its own outline;
- a tree that sways in the wind.

The benchmark's target level still runs at 60 fps, with its 2,000 ships each tinted differently.

## The idea, in plain words

A **material** says what a surface looks like: its colour, how rough or metal it is, whether it glows, how see-through it is. It also says how it takes light: standard, stylized (cel bands), the light ramp, or unlit. Today materials come only from model files, and the look is chosen for the whole view (decision 0013). This phase makes them Solis's own.

Every value in a material can be **plugged into an input** instead of being a fixed number. Inputs are general:
- the light around the surface (from Phase 8);
- the time;
- the distance to the camera;
- the height;
- the weather's wetness;
- noise;
- a value the game sets.

Each passes through curves and colour gradients. "Glow = 1 when it's dark around me, fading in over 10 seconds" is just glow plugged into the light around the surface through a curve.

A **node graph** does the same with boxes and wires, for anything the simple view can't: the boxes are inputs, maths, textures, noise and gradients, and the wires carry values to the material's outputs. That is a shader, made without code. For experts, a box can hold a few lines of shader code.

## Choices to make before any code

Each question has a recommended answer first, then the alternatives, in plain words.

1. **How materials are edited.**
   - **Recommended: two views of the same file.** A **simple view** in Properties (each value fixed or plugged into an input) for most materials, and the **graph** for the rest. A simple material opens in the graph as a few boxes. A graph that the simple view can't show opens in the graph only.
   - Alternative: graph only (one way, but heavy for "make this red and shiny").
   - Alternative: settings only in this phase, the graph after 0.1 (DESIGN.md §7's old plan; less creative freedom).

2. **How a graph becomes a shader.**
   - **Recommended: the graph fills in Solis's own lit shader, it doesn't replace it.** A graph computes the surface's outputs:
     - colour, normal, roughness, metal and glow;
     - opacity;
     - an offset for its corners (for wind).

     Solis's shader then lights the surface with everything Phase 8 built: shadows, the sky's light, bounced light and fog. So every custom material still gets the whole lighting for free, in every look. The graph is turned into WGSL (the shader language wgpu uses), checked by wgpu's own checker (naga), and saved in the cache. A change shows in about a second, like game code.
   - Alternative: also allow fully custom shaders that do their own lighting (more freedom, but they'd miss every lighting improvement and break the quality levels). Possible later as an "unlit, I'll do it myself" output.

3. **Shader code for experts.**
   - **Recommended: a code box holding a WGSL function,** with typed inputs and outputs as boxes. It's checked before it runs, and a mistake shows in the Problems panel on that box, never a crash. Loops must have a fixed limit, so a mistake can't freeze the graphics card.
   - Alternative: no code boxes (safer, less freedom).
   - Alternative: shader code written in Lust later (one language for everything; a large piece of work in Lust itself).

4. **What game code can change.**
   - **Recommended: per-copy values.** A material names some of its values as **settable** (say `glow`, `tint`, `wear`), and each entity using it can set its own: from Properties, or from game code with `setMaterialValue(entity, name, number)` and `setMaterialColour(entity, name, colour)`. They live on a component, so they're saved and undone like any field. They're drawing only, so they don't change the game or its checksum.
   - Alternative: game code can only swap whole materials (simpler, far less creative).

5. **Many copies, each different.**
   - **Recommended:** each copy carries a small number of settable values (up to eight numbers) in its instance data, so 2,000 ships with 2,000 different tints still draw together, in one batch.
   - Alternative: a separate draw per changed copy (simpler, slow at the scale target).

6. **Imported materials.**
   - **Recommended: imported materials stay as the model file says**, and **Make editable** creates a `.solmat` beside the model that takes over. Re-importing the model never overwrites it.
   - Alternative: every import writes `.solmat` files at once (more files, most never edited).

7. **The file.**
   - **Recommended: `.solmat`** (already reserved by decision 0015), Lust-style readable text like areas (decision 0020), so it merges in Git. The graph is saved in the same file (decision 0039's rule: graphs saved as text).

8. **Moving corners (wind, waves, breathing).**
   - **Recommended: allowed, with a promised limit.** A material that moves its corners says how far at most, so culling and shadows grow each copy's bounds by that much and nothing pops out of view.
   - Alternative: no moving corners in this phase.

9. **Controls (CONTRIBUTING rule 17).**
   - **Recommended: the graph editor has the same controls as the animator's graph (Phase 7 plan, choice 7)**, since they share the widget:
     - **wheel** zooms;
     - **middle-drag** pans;
     - **drag from a pin** makes a wire;
     - **right-click or Space** opens the box search;
     - **Delete** removes.
   - Alternative: whatever Phase 7 settles, kept identical.

## Where things stand

- **Materials** are glTF's metal-and-roughness materials from model files, kept in the import cache. `.solmat` is reserved (decision 0015) but unused.
- **The look** is chosen for the whole view (decision 0013): standard, stylized, unlit, and the light ramp from Phase 8.
- **Instances** carry a 52-byte placement each (Phase 5), drawn in batches per mesh and level of detail.
- **Phase 8 provides the inputs:** the light reaching each surface, wetness, the time of day, glowing surfaces that light the world.
- **The graph widget** comes from Phase 7's animator, and Phase 10's effects use it too.

## The rules this phase keeps

- **General tools first.** A request for a specific effect becomes a new input, box or example material, never a special case in the renderer.
- **Every material gets all the light.** Custom materials stay inside Solis's lighting, at every quality level.
- **Never a crash, never a freeze.** Bad graphs and bad code show in Problems; the material falls back to a plain pink "broken" look until fixed.
- **The scale target holds.** Per-copy values keep batching; a scale case renders the benchmark with every ship's material different.
- **AMD first, when a choice must be made** ([decision 0049](../decisions/0049-amd-first-when-a-choice-must-be-made.md)): the WGSL that graphs turn into is written and measured for AMD's cards first, and stays fast on others.
- **Drawing never changes the game.** Material values are output, outside the checksum.

## Milestones

### M1. Materials as assets
- **The file:** `.solmat` materials, made with **Asset > New material** or **Make editable** on an imported one.
- **Each has its own look:** standard, stylized, light ramp or unlit. This completes decision 0013, whose viewport menu becomes "as the materials say", plus overrides for checking.
- **The simple view** in Properties.
- **A preview tab:** a ball, a cube or the model, under the area's sky and a time-of-day slider.
- **Manual:** a page on materials.
- **Fuzz test** for the file reader.

### M2. Inputs that drive any value
- **Any value in the simple view** can be fixed, or plugged into an input through a curve or a colour gradient.
- **The inputs:**
  - the light around the surface;
  - the time (game time or real time);
  - the distance to the camera;
  - the height;
  - the time of day;
  - the weather's wetness;
  - noise;
  - a settable per-copy value.
- **Smoothing:** each input can be eased (fade in over N seconds).
- **Per-copy values:** a `MaterialValues` component, the natives `setMaterialValue` and `setMaterialColour`, and up to eight values in the instance data.

### M3. The graph
- **The shared graph editor** shows a material as boxes and wires, with box search, groups, comments and previews on each box.
- **Outputs:** colour, normal, roughness, metal, glow, opacity and corner offset.
- **Turned into WGSL,** checked by naga, cached, and live in about a second.
- **Mistakes** show in Problems, on the box.
- **Box kinds:** maths, textures, noise, gradients, curves, the inputs of M2, the fresnel rim, and the UV and world-position tools.

### M4. Code boxes
- **What a code box is:** a WGSL function with typed pins.
- **Checking:** done before it runs; loops need a fixed limit.
- **Errors** are mapped back to the box and line.
- **The manual** shows three small examples.

### M5. Stylized building blocks and motion
Boxes for the looks stylized games are made of:
- **Light:** light ramps and bands, rim light, hatching, dithering and halftone, and pixel snapping.
- **Outlines:** set per material (width, colour, which side).
- **Mapping:** triplanar mapping (textures with no unwrap).
- **Motion:** wind sway (with the area's wind), and ripples, inside the corner-offset limit.

### M6. At scale, and the material showcase
- **Scale cases:**
  - the benchmark with per-ship tints and glows;
  - a field of 10,000 swaying plants.
- **Help > Make the material showcase…** writes a project with the gate's four materials made as graphs to open and copy, and a few more:
  - a lamp that flickers;
  - a shield that shimmers;
  - water that ripples;
  - a force field that fades with distance.
- **Manual pages:** the graph, the inputs, and code boxes.
- **End of the phase:**
  - the three reviewers (decision 0027);
  - the scale check;
  - the gate.

## What Phase 10 (effects) takes from this

Particles are drawn with these materials. A spark, a puff of smoke or a trail is a material with the particle's own inputs added:
- age;
- speed;
- a per-particle random number.

So an effect's look is made the same way as a surface's, in the same graph. The effects plan's own particle shading becomes a set of example materials.
