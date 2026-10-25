# Phase 7 plan: Animation (draft)

The working plan for the animation half of Phase 7 (ROADMAP.md, "Phase 7: Sound and animation"; DESIGN.md §10). Sound has its own plan; this one says only what animation needs from it (a way to play a named sound event). It lists the choices to make before any code and a plan for every milestone, so a later session can carry on where this one stopped. Update it as milestones are built: tick them off here and in the README tracker, and change a milestone's plan if its design changes.

**Gate:** a character walks and runs with smooth blending, footsteps play on the right frames, and a reverb zone changes how they sound (the reverb part is the sound plan's).

## Answers (the project lead, 2026-10-25)

Recommended for every choice except two:

- **Blend spaces: 1D and 2D both in Phase 7** (not 2D later). A4 grows to include two-number blend spaces (strafing).
- **The gate's character: only the project lead's own** (no Solis-made robot in the playground). Automatic tests still need a small skinned model, which Solis makes in code for the tests only (kept out of samples and the playground).

The rest as recommended: live poses near, baked clips far; cues per clip in the model's `.solmeta`; clips from other files matched by bone names; no root motion in Phase 7; animators saved as `.solanim` on an `Animator` component; the graph editor's controls as listed (middle-drag pans, wheel zooms, F frames); the viewport animates only with Preview animation on; the natives as listed, bone positions later; glTF only, FBX later.

## Choices to make before any code

Each is a question for the project lead, with a recommended answer first. Answers that settle something lasting become decisions (a line in DESIGN.md §17 and a file in `docs/decisions/`, numbered from 0041).

1. **How do thousands of units animate cheaply?** Working out a full pose (where every bone is) for 10,000 units every frame, and sending it to the graphics card, costs too much: about 24 MB a frame at 60 bones each.
   - **Recommended: two ways, chosen by Solis per unit, by size on screen, like LODs.** Units close to the camera get a **live pose**: Solis works out every bone, blending as many animations as the state machine says, and sends it each frame. Units further away play **baked clips**: each animation is worked out once, when the model loads, as a pose for every 30th of a second, and kept on the graphics card; a far unit only says "clip, frame, and how far to the next frame", and the card does the rest. Nothing has to be set up by the game maker. This is NVIDIA's "skinned instancing" (almost 10,000 animated characters at 30 fps on 2007 hardware) plus what Unreal does with far characters (updating them less often).
   - Alternative: live poses only. Simplest, best-looking, but likely misses the stress level (10,000 units at 30 fps).
   - Alternative: baked clips only (as Unity's Animation Instancing does). Fastest, but no proper blending between animations up close, which the gate needs, and IK later would be hard.
   - Ruled out: **vertex animation textures** (every corner's position baked per frame). Very fast, but memory grows with the triangle count, there's no blending, and LODs would need their own bakes. Better suited to effects (Solis Eclipse) than characters.
2. **Where are footstep moments (cues) written?** glTF files can't carry them reliably (Blender's markers aren't exported).
   - **Recommended: in the model's `.solmeta` sidecar, per clip,** edited in Solis on a clip timeline. They belong to the clip (a walk has its footsteps wherever it's used), move with the model when it's renamed, and are text in Git.
   - Alternative: in each state machine file. Then two state machines using the same walk each need the footsteps added.
3. **Blend spaces:** blending clips by a number (speed: idle at 0, walk at 1.5 m/s, run at 5 m/s).
   - **Recommended: one-number (1D) blend spaces now; two-number (2D, for strafing: forwards and sideways) later,** as an issue. The gate needs only 1D.
   - Alternative: both now (more to build and to test).
4. **Clips from other files.** Mixamo and many artists export one file per animation, all for the same skeleton.
   - **Recommended: yes, matched by bone names.** A state machine can use a clip from any model whose bones have the same names; missing bones keep their resting place, and Problems names them. Retargeting between different skeletons stays later (DESIGN.md §10).
   - Alternative: only clips inside the model's own file (simpler, but artists must merge files in Blender first).
5. **Does animation move the character (root motion)?**
   - **Recommended: not in Phase 7** (DESIGN.md §10 lists it as later). Game code moves the character, as now, and tells the state machine its speed; animation is how it looks, plus its cues. That keeps animation out of physics and netcode.
   - Alternative: root motion now, which would have to run in the tick and be strictly deterministic, and makes feet that never slide but characters harder to steer.
6. **The state machine's file and name.**
   - **Recommended: an "animator", saved as `.solanim`** (decision 0015's pattern; added to RENAMING.md), placed on an entity by a built-in `Animator` component.
   - Alternatives: `.solstates`, or `.solgraph` for every graph editor (state machines, behaviour trees, shaders) with the kind inside.
7. **The graph editor's mouse and keys** (CONTRIBUTING rule 17). Recommended, as Unity's and Unreal's graph editors do, with no clashes inside the graph tab:
   - left-drag a state to move it; left-drag on empty space draws a selection box;
   - drag from a state's edge to another state, or right-click a state > Make transition, to add a transition;
   - right-click empty space: Add state, Add blend space;
   - middle-drag pans; the wheel zooms around the pointer; F frames everything; Delete removes; double-click a state to edit its clip.
   - Alternative for panning: right-drag (as in Unity's Animator window). Easy to confuse with the viewport, where holding the right button flies the camera (decision 0009), so not recommended.
   - Alternative: Space + left-drag to pan, for laptops without a middle button (could be added alongside).
8. **Previews in the editor.** The editor draws only when something changes (decision 0026), and an animating model changes every frame.
   - **Recommended: the viewport stays still (the resting pose, or the entry state's first frame) unless the viewport's ≡ menu has "Preview animation" on; the animator tab has its own small preview that always plays, with sliders for the parameters.** While the game is playing, the animator tab highlights the selected entity's current state and transition.
   - Alternative: always animate in the viewport (simple, but the editor never goes Idle in an area with characters).
9. **Game code's names.** Lust reserves some short words (`save`, `load`), and Phase 6 used `saveGame` and `loadGame`. Recommended natives (file `solis/animation`), with distinctive names that won't clash: `setAnimNumber(entity, name, float)`, `setAnimSwitch(entity, name, bool)`, `triggerAnim(entity, name)`, `setAnimSpeed(entity, float)`, `jumpToAnimState(entity, state, float blendSeconds)`, `animStateOf(entity)` and `animTimeOf(entity)`; and an event the engine sends, `AnimationCue(string name)`.
   - Also to choose: **can game code ask where a bone is** (`jointPositionOf(entity, "hand_r")`, to fire from a barrel or a hand)? Recommended: later, as an issue, with attaching things to bones. It's cheap and deterministic (worked out from the tick's state), but not needed for the gate.
10. **The gate's character.** Recommended: Solis makes its own simple robot in code (`assets::sample`, MIT, like the benchmark's ship), with idle, walk, run and jump clips, for tests and the playground; the project lead also tries a character of their own (from Blender or Mixamo; Mixamo's files can't go into the repository).
11. **FBX.** DESIGN.md §10 says glTF and FBX; ROADMAP says glTF. Recommended: glTF only in Phase 7, and DESIGN.md §10 updated to say FBX is later; Blender converts FBX to glTF. The one good permissive FBX reader (ufbx, MIT) is a C library, which would need a C build and an `unsafe` decision.

## Where things stand

- **The glTF reader ignores skins and animations** (`assets::reader`, `geometry`): a skinned character imports in its resting pose. `gltf` 1.4.1 (MIT or Apache-2.0, already used) parses skins (`Skin::joints`, inverse bind matrices) and animations (channels, samplers; step, linear and cubic-spline keys); Solis reads the buffers itself, within limits, so the same checks extend to them.
- **A corner is 32 bytes** (position, normal, texture place); big meshes are split to fit the card's buffers (`geometry::split`), so skin data must split with them. **LODs share their mesh's corners** (`lods.rs`), so every level gets skin data for free.
- **An instance is 52 bytes** (three rows of its placement, and a pick number); batches are per model, mesh, LOD and mirroring; culling uses one ball per mesh worked out at upload. A moving skinned mesh can leave its resting ball.
- **On OpenGL (Mesa, CI's software GPU), egui-wgpu asks for WebGL2's limits:** no storage buffers in shaders and no compute shaders. So bone matrices must live in a **floating-point texture** read in the vertex shader (it works everywhere, Direct3D 12 included), and skinning happens in the vertex shader, not a compute pass.
- **The tick's order** (`Workbench::run_game`): land spawns, remember places, projectiles, input, game code's updates and rounds of events (changes landed in a fixed order, decision 0033), land spawns, physics.
- **Events are declared by game code** (`event Hit(entity by, float damage);`); the `solis/` files hold only natives so far. An event the engine sends (`AnimationCue`) needs declaring in a generated file too.
- **Saves are text in sections** (`solis save 1`, `editor::save_game`); `World::checksum` covers places, values, projectiles, physics and the seed.
- **Sound** arrives in the same phase (sound plan, first two milestones): sound events by name ("footstep" picks a clip with some variation), which is what cues play.

## Determinism: what animation is to the game

- **The state machine is game state; poses are not.** Which state each animator is in, how far through it, any crossfade in progress, its parameters and waiting triggers move on **once per tick** (1/60 s), from parameters game code set, and are in the checksum and in saves. Game code can read them and cues reach it, so they must come out the same on every PC and in every replay.
- **Every animator ticks every tick,** however far away or hidden: it's a few numbers each (well under 1 ms for 10,000 on several cores, written in jobs that each fill their own slice, decision 0031). Only drawing is cut down by distance.
- **Times are plain additions and comparisons** (no sines or square roots in what's decided per tick), in a fixed order, so they repeat exactly (Windows only, decision 0012).
- **Poses are only for drawing.** Worked out after the ticks, from the state moved part way to the next tick by `World::progress`, as positions are (Phase 6, M1); paused, exactly at the tick. Live or baked, a unit's cues and state are the same, because they never come from a pose.

### Where animation goes in the tick

1. Land spawns (as now); remember places, **and each animator's times** (for drawing between ticks).
2. Projectiles.
3. **Animation** (new): each animator takes the parameters that landed at the end of the last tick, moves on by one tick times its speed, checks its transitions (in the order listed in the file; the first whose conditions hold wins), moves crossfades on, and collects the **cues** it passed this tick. Cues are put in one list in a fixed order: by entity ID, then time within the tick, then the clip's cue order.
4. Input; game code's updates. Game code reads this tick's animation state (it was settled before game code began); `setAnimNumber` and friends are writes, landing at the end with the others (decision 0033), used by the next tick's step 3.
5. **Rounds of events:** the tick's `AnimationCue` events go first in the first round (they happened before game code ran), delivered only to entities with a component that has `on AnimationCue`, then the events sent during the updates. They count toward the existing event limits (10,000 a tick).
6. Land changes, land spawns, physics (as now).
7. **Sound:** a cue with a sound plays it from the entity's place, handed to the sound system after the frame's ticks (sound is output, not game state, so it isn't checksummed). With several ticks in one frame, each sound starts a tick later than the one before if the sound library allows a short delay, otherwise at once.

So a footstep reaches both: the sound system plays "footstep" by itself (no code needed), and game code that wants to (dust, a noise that wakes guards) handles `AnimationCue("footstep")`.

**Which cues fire when blending:** in a blend space, clips stay in step by their share of the cycle (walk and run each place the left foot at 0% and the right at 50%, so blending never shuffles the feet), and only the clip with the most weight fires its cues. During a crossfade, only the state with more than half the weight fires. So one footstep sounds, never two.

## Safety (CONTRIBUTING rules 8 to 10)

- **Limits on what's read from model files:** at most 256 bones a skin and 64 skins a file, 4 bones per corner (a second set is dropped with a warning, weights shared out again), 1,000 clips a file, clips up to 10 minutes; key counts are checked like accessors are now, before anything is allocated. Bad numbers are tidied: times out of order are sorted (warning), NaN keys dropped, quaternions made unit length, a corner whose weights are all zero follows the skin's first bone, a bone index past the skin refuses that mesh's skin (warning, drawn unskinned). Cubic-spline keys are worked out into the steady 30-a-second frames at import, as all clips are, so there's one way to play them.
- **Limits on `.solanim` files:** 1 MB, 256 states, 1,024 transitions, 64 parameters, 8 conditions a transition, 16 clips a blend space; a file past them, or damaged, is refused with a message, and the entity is drawn in its resting pose.
- **Limits on the graphics card:** baked clips share a budget (256 MB by default), past which more units get live poses and the Console says why; the live poses sent each frame are limited too (the nearest units first).
- **Fuzz tests** for everything read: skins and clips in glTF (thousands of damaged files), `.solanim` files, cues in sidecars, the animation section of saves, the import cache's new layout.
- **No panics in the frame:** a missing clip, a state machine changed while playing, a state that no longer exists after a hot reload: the animator goes back to its entry state, with a note in the Console.

## The shape of the code

| Crate | Layer | What it gains | Milestone |
|---|---|---|---|
| `assets` | 1 | Skins, skinned corners (joints and weights), clips resampled to 30 frames a second, each clip's bounds; the sample robot | A1 |
| `asset_db` | 1 | Clips and skins as parts in sidecars (stable IDs); cues per clip | A1, A6 |
| `animation` (new) | 1 | Sampling and blending poses, the state machine, `.solanim` read and written, cues collected. No GPU, no editor | A2, A4, A6 |
| `area` | 0 | Built-in `Animator` component (animator file, speed) | A2, A4 |
| `world` | 1 | Each entity's animator state, the animation step in the tick, checksums, times remembered for drawing | A4 |
| `render` | 1 | The bone texture, skinned versions of the model, shadow, pick and outline pipelines, instances that say where their pose is, baked clips, animated bounds | A2, A3 |
| `script` | 1 | The natives above, and `event AnimationCue` in `solis/animation.lst` | A4, A6 |
| `editor` | 2 | The animator tab (graph editor, preview), the clip timeline for cues, the tick's new step, the saves' animation section, clips in the Library | A2–A6 |
| `app` | 3 | The animated crowd in the benchmark; the playground's walking robot | A3, A6 |

New crates beyond these aren't needed: sampling and blending are a few hundred lines on `glam`. **ozz-animation-rs** (0.11, the deterministic Rust port of ozz) was looked at and is MPL-2.0, which `deny.toml` doesn't allow; **bevy_animation_graph** (0.11, MIT or Apache-2.0) is tied to Bevy's engine, but its design (state machines as graph nodes, blend spaces, event tracks for syncing) is a good reference. For the graph editor, **egui-snarl** (0.12, MIT or Apache-2.0, egui 0.36 like Solis) draws node graphs with pins, which suits shader graphs later; a state machine is boxes and arrows, so a small canvas of Solis's own is recommended now.

## Milestones

### A1. Skins and clips from glTF ☐

- The reader keeps skins (bones, their resting placements, inverse bind matrices, turned to Z up like nodes) and each skinned corner's 4 bones and weights (kept in a second list beside the corners, so unskinned models cost nothing extra), split with the mesh when it's split. Clips are read per bone (move, turn, size), resampled to 30 frames a second, and their bounds measured (the box the model stays in while playing it).
- Clips and skins become parts in the sidecar (decision 0021), so renaming a clip in Blender keeps references; the import cache's layout version goes up.
- The Library shows a model's clips; importing says how many (and warns about morph targets, which wait).
- `assets::sample` gains the robot (two legs, two arms, a head; idle, walk, run, jump).
- Tests: the robot's bones and clips read back as made; a clip's pose at a time matches glTF's own keys (step, linear and cubic spline); the LODs keep skin data; limits (bones, clips, length); damaged skins and clips are refused or tidied (fuzz); re-import keeps clip IDs; a file of clips only (Mixamo-style) imports.

### A2. Skinned drawing, playing one clip ☐

- **Drawing:** skinned meshes get a second vertex stream (bones and weights) and skinned versions of every pipeline that draws models (lit, shadow, pick, outline), so shadows and picking follow the pose. Bone matrices for every live unit go into one floating-point texture each frame (3 texels a bone); each instance gains where its pose starts (instances grow from 52 to 64 bytes). Culling uses each clip's bounds.
- **The `Animator` component,** first version: a model on the same entity plays one chosen clip, looping, at a speed. Live poses only.
- **In the editor:** "Preview animation" in the viewport's ≡ menu (off by default, decision 0026); Properties chooses the clip.
- Tests (GPU tests on Mesa's OpenGL and Direct3D 12): a bent arm's pixels are where the pose puts them; the shadow and pick image follow the pose; an unskinned model draws exactly as before (pixels compared); the robot plays the same frame at the same time at 30, 60 and 144 fps. Scale case: 2,000 robots, live, CPU and GPU time.

### A3. Crowds: baked clips and animation LOD ☐

- Each clip in use is baked into the bone texture once, when its model loads (after A1's resampling: a pose per 30th of a second). Far units say "clip, frame and the share to the next" and the vertex shader blends the two frames; during a far unit's crossfade, the two states' frames are blended the same way. Near units keep live poses.
- **Choosing** which units are live: the nearest by size on screen, up to a number that keeps the frame's live work in budget (as Unreal's animation budget does), with a little overlap so a unit at the boundary doesn't flick between the two; live poses for units further out may be updated every second or third frame.
- **The benchmark** gains an animated area: 2,000 walking robots at the target level and 10,000 at stress (Help > Make the benchmark…).
- Tests: a baked frame draws the same pixels as the live pose at that time (within a small tolerance); switching between live and baked doesn't jump; the budget limits are kept. Scale cases: target and stress with animated units, live and baked shares reported; the ceiling run with robots.

### A4. State machines and blending ☐

- **The `.solanim` file** (decision 0039: a graph saved as readable text), Lust-style blocks like areas (decision 0020), one per parameter, state and transition, with stable random IDs so two people's changes merge, and each state's place in the graph:

  ```
  -- Solis animator. Written by Solis; safe to edit by hand.
  animator version 1;
  model = "4WQ0B7M2KD9TS";  -- characters/robot.glb

  parameter speed { kind = "number"; start = 0.0; }
  parameter jump { kind = "trigger"; }

  state 3KD8V0Q2MX7TA {
      name = "Moving";
      entry = true;
      place = [120.0, 80.0];
      blend_by = "speed";
      clips = [["Idle", 0.0], ["Walk", 1.5], ["Run", 5.0]];
  }

  state 7PQ2B5N8WK1ZC { name = "Jump"; place = [320.0, 80.0]; clip = "Jump"; loop = false; }

  transition 9XM1R4T6HD2QE { from = "Moving"; to = "Jump"; when = ["jump"]; blend = 0.15; }
  transition 2CF7K9P3AW5YN { from = "Jump"; to = "Moving"; when = ["finished"]; blend = 0.2; }
  ```
- **States** play one clip or a 1D blend space; **transitions** have conditions (a number compared with a value, a switch on or off, a trigger, "finished", "from any state"), all of which must hold, and a crossfade time. A crossfade can be interrupted by a transition out of the state it's going to; at most three states blend at once (the oldest drops out), so the state stays small and saveable.
- **In the tick** as above; **natives** as in choice 9, in Lust, Lua (`solis.setAnimNumber`) and Rust; the state in the checksum; **saves** gain an `animation` section (each animator's state, parameters and crossfades, numbers by their exact bits; the format becomes `solis save 2`, still reading 1). A save whose animator file has changed since loads that animator at its entry state, with a note.
- Tests: transitions fire on the right tick; crossfade weights add to 1; the blend space keeps feet in step; the file round trips and merges (two people adding states on two branches); damaged files refused (fuzz); the same recorded inputs give the same animation states and checksums on 1 core and 4; save, load, the same checksum; hot reload while playing.

### A5. The animator graph editor ☐

- **An animator tab** in the document tabs (decision 0004): states as boxes, transitions as arrows (two ways between the same states shown side by side), the entry state marked; the controls chosen in choice 7. The parameters listed beside the graph; Properties edits the selected state's clip or blend space, or the transition's conditions and crossfade.
- **Preview:** a small view of the model playing the animator, with a slider or switch per parameter; while the game is playing, the selected entity's current state and transition are lit.
- Edits are undoable (decision 0022) and saved to the `.solanim` file like areas (through a temporary file, CONTRIBUTING rule 11).
- Tests: every edit and its undo; the text the editor writes is what a hand-written file would be (round trip); a file changed on disk reloads; the editor's speed with 256 states.

### A6. Cues, footsteps and the playground ☐

- **The clip timeline:** in the animator tab (or the Library), a clip plays with a timeline under it; cues are added, named, moved and given a sound event. Saved in the model's sidecar (choice 2).
- Cues collected in the tick and delivered to game code (`AnimationCue`) and the sound system, as above.
- **Help > Make the playground…** (Phase 6) gains the robot with an animator (idle, walk, run in a blend space by speed; jump), footsteps with sound, and a reverb zone (sound plan), driven by the Phase 6 character controller, which sets `speed` and triggers `jump`.
- **Docs:** the manual's new pages (animation, the animator editor, cues) and concepts (skeleton, clip, pose, blend space, state machine, cue).
- Tests: a cue fires exactly once per pass, on the right tick, including when a loop wraps and when several ticks run in one frame; one footstep (never two) while blending walk and run; the cue order is the same on 1 core and 4; a replay gives the same cues; cues in sidecars round trip and survive damage (fuzz).

### End of the phase

- The review (decision 0027): three reviewers, the fixes in pull requests by kind, the scale check with all three levels, now with animated units.
- The gate, tried by the project lead on their PC.

## Risks

- **The stress level with animation.** 10,000 animated units add GPU work (4 bone lookups a corner, shadows too) and upload; A3 is where this is proven, so it comes before the state machine.
- **OpenGL and CI's software GPU** can't use storage buffers, so the bone texture is the only way; it works on every card but is slower to fill. GPU tests on Mesa will be slow with many skinned units: keep their counts small.
- **Baked frames blend matrices in a straight line,** which can pinch a joint turning a long way between two frames 1/30 s apart; acceptable far away, never used up close.
- **The roadmap's count changes:** README lists two animation milestones; this plan has six (A2 and A3 could be one if wanted).
- **Engine-declared events** in the `solis/` files are new; Lust's checker must accept an event declared there and handled in game code.

## Sources

- NVIDIA, [Skinned Instancing white paper](https://developer.download.nvidia.com/SDK/10/direct3d/Source/SkinnedInstancing/doc/SkinnedInstancingWhitePaper.pdf) (2007): bone matrices for every frame in one texture, about 10,000 characters at 30 fps in ~13 draws; and [GPU Gems 3, ch. 2](https://developer.nvidia.com/gpugems/GPUGems3/gpugems3_ch02.html).
- Unity, [Animation Instancing](https://blog.unity.com/engine-platform/animation-instancing-for-skinnedmeshrenderer): baked animation textures, 5–6× more characters, but no transitions or blending.
- Epic, [Animation Budget Allocator](https://dev.epicgames.com/documentation/en-us/unreal-engine/animation-budget-allocator?application_version=4.27): far characters update less often, within a budget in ms, interpolating between updates.
- Bevy, [animation graph example](https://bevy.org/examples/animation/animation-graph/) (graphs saved as RON text); [bevy_animation_graph](https://github.com/mbrea-c/bevy_animation_graph) (0.11, MIT or Apache-2.0: state machines, 2D blend spaces, event tracks, a graph editor).
- [ozz-animation-rs](https://docs.rs/ozz-animation-rs) (0.11, MPL-2.0, not allowed by deny.toml): a deterministic Rust rewrite of ozz-animation's runtime, reading `.ozz` files only.
- [gltf](https://docs.rs/gltf/latest/gltf/animation/index.html) (1.4.1, MIT or Apache-2.0): skins and animation channels.
- [egui-snarl](https://docs.rs/egui-snarl/) (0.12, MIT or Apache-2.0, egui 0.36).
- wgpu-types 30 and egui-wgpu 0.36 source (local): on OpenGL, `Limits::downlevel_webgl2_defaults()` has no storage buffers in shaders.
