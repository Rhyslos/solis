# Phase 8 plan: Light, sky and atmosphere (draft)

**Draft for the project lead.** The project lead asked on 2026-10-26 for "all the fancy lighting solutions working well", with sky, lighting and colour as Solis's selling point. The goal is skies and sunsets that give the feeling the best game sunsets do, and never the flat, sterile "Backrooms" look. This phase is new: it comes after sound and animation (Phase 7) and before effects (now Phase 9), game UI (now Phase 10) and netcode (now Phase 11). See [decision 0046](../decisions/0046-light-before-netcode.md), proposed.

The research behind every choice is in [docs/reports/lighting-research.md](../reports/lighting-research.md), with its sources. Nothing here is decided until the project lead answers the questions below. Then the answers become decisions, and this plan is kept up to date as milestones are built, like the other phase plans.

**Gate:** in the light showcase (L1), set a time of day and look. It is judged against screenshots of the project lead's favourite game sunsets. The gate passes when:
- **The day runs from noon to night and back with no jumps.** The sky, sun colour, shadows, clouds and haze all change together. The brightness adjusts as eyes do. Twilight turns blue. Night looks like night: stars and the moon, and dark without going grey.
- **The golden hour feels right.** The sun is low and warm, shadows are cool, the clouds are lit from below, the haze glows towards the sun, and far hills fade into the air.
- **Indoors never looks like the Backrooms.** Inside the showcase's house, sun through a window bounces light around the room, and corners and contacts are soft and darker.
- **The stylized look is as good as the standard one.** The cel-shaded look gets the same sky and colours.
- **The benchmark's target level still runs at 60 fps** at the Default quality level, on the project lead's PC.

## The idea, in plain words

Every loved game sky in the research does the same thing:
- **Physics underneath.** Sunlight is scattered by air (that's why the sky is blue and sunsets are orange). Ozone keeps twilight blue. Clouds are lit from inside, and haze tints whatever is far away. These give colours that feel right without anyone painting them.
- **Art direction on top.** White balance, colour grading, colour that follows the time of day, and stylized clouds give the mood.

Solis does both: a physical sky that is right by default, and controls that let a game push it anywhere, including keeping today's painted sky as a style.

The other half is **light that has a shape.** Today the shadow side of everything gets one flat colour from the sky, and that flatness is the main cause of the "Backrooms" look. This phase fixes it:
- light comes from the sky above and the ground below;
- creases and contacts darken (ambient occlusion);
- shadows stay sharp near the camera and still reach far;
- hundreds of lamps can shine at once;
- light bounces around a room (baked light probes).

## Choices to make before any code

Each question has a recommended answer first, then the alternatives, in plain words.

1. **The order of the phases.**
   - **Recommended:**
     - 7 Sound and animation (as planned; its choices are answered)
     - **8 Light, sky and atmosphere** (this plan)
     - 9 Effects (the effects suite)
     - 10 Game UI (Sungraph UI)
     - **11 Netcode** (moved from 8)
     - 12 Scripting, export and Git
     - 13 Solis 0.1

     Light comes before effects, because effects are lit by it and glow through its bloom. Effects and UI become a phase each, since each is big.
   - Alternative: **light first**, before sound and animation. The selling point arrives sooner, but Phase 7's answered plans wait.
   - Alternative: light, effects and UI as **one** long phase with one gate.

   Moving netcode later is safe; see "Netcode waits safely" below.

2. **The effects suite's name.** "Eclipse" is the riskiest:
   - the Eclipse Foundation holds a live US trademark on ECLIPSE for software development tools;
   - a particle-heavy "Eclipse Engine" is being built for *Star Wars: Eclipse*;
   - the "Solis" prefix helps little.

   "Ember" sits right next to **EmberGen**, a leading fire and smoke tool for game effects.
   - **Recommended: Solstice** ("Solis Solstice"). Its only trademark found is for meeting-room screen sharing, a different market, and no game tool by that name was found.
   - Alternatives: **Parhelion** (a "sun dog") or **Aureole** (a ring of light round the sun), both clear in quick checks but harder to say; or keep Eclipse and accept the risk.

   Not legal advice: a trademark professional's search would be needed before relying on any name.

3. **Light in real units.** Real sunlight is about 100,000 lux and starlight 0.0001: about 30 camera stops. A physical sky only works in real units, with exposure that adjusts as eyes do. Today the sun is in Blender's units (3 W/m² by default; decision 0014), which are far from real.
   - **Recommended:**
     - **The sun's strength and colour come from the sky by default.** Properties says "from the sky", with a manual override in lux.
     - **Lamps keep Blender's watts** (they already convert to real lumens at 683 lm/W, so a 1000 W lamp matches Blender).
     - **Exposure is automatic by default, in the editor and the game alike,** so what you build is what you'll see. It can be fixed in stops (EV) from the viewport's menu or the area's Environment.
     - Areas saved before this open with their sun converted, looking as close to before as they can.
   - Alternative: keep Blender's sun units and scale the sky down to match (simpler, but the sky's brightness is invented, and day to night can't be automatic).

4. **What the time of day is.** If a game's night changes how it plays (guards sleep, zombies come out), the time of day must be game state: saved, in the checksum, the same on every PC.
   - **Recommended:**
     - The time of day is a field on the area's **Environment**, and game code can set or run it (`setTimeOfDay(hours)`, `setDaySpeed(rate)`). It is saved and checksummed like any field.
     - The sky only reads it.
     - Drifting clouds and twinkling stars are decoration on frame time, like effects, and never game state.
   - Alternative: time of day as decoration only (game code can't rely on it, and two PCs could disagree about whether it's night).

5. **Quality levels.** The whole set costs more than a frame can spare at the scale target on a mid-range card. The research estimates, at 1080p:

   | Level | What it adds | Cost |
   |---|---|---|
   | **Default** | The physical sky, sky light, ambient occlusion, cascaded and contact shadows, many lamps, flat clouds, bloom and grading | about 4–7 ms |
   | **High** | Volumetric clouds, volumetric fog and soft shadows | about +3–4 ms |
   | **Low** | Turns things off for older cards; works on any card Solis opens on | — |

   - **Recommended: Low, Default, High, set per project, and changeable by players later** (the settings screen is Sungraph UI's, Phase 10). The editor's viewport has its own setting, so a slow laptop can edit at Low.
   - Alternative: one fixed look (simpler, but either too slow on old cards or too plain on good ones).

6. **Bounced light (global illumination).** Real-time bounced light is the one part still hard everywhere:
   - Godot is rewriting its own.
   - Bevy's ray-traced version runs well only on NVIDIA cards.
   - Ray tracing in wgpu is experimental.
   - **Recommended:**
     - **Baked light probes in this phase.** Solis bakes them on any card (no ray-tracing hardware), and thousands of moving units pick up bounced light from them almost for free.
     - **Real-time bounced light after 0.1**, on probes traced against a simplified copy of the world (AMD's MIT-licensed method, about 1.7 ms), as a further "Ultra" level.
   - Alternative: real-time bounced light in this phase too (about two more months; the riskiest part of the phase).
   - Alternative: also baked lightmaps (the best still pictures, but every model needs a second unwrap; after 0.1).

7. **Clouds.**
   - **Recommended: both kinds in this phase.**
     - A **flat cloud layer** (Default): cheap, lit by the same sky, so clouds glow from below at sunset.
     - **Volumetric clouds** (High), in Horizon Zero Dawn's way: drawn at quarter size, a sixteenth of the pixels refreshed each frame, about 1–2 ms.
   - Alternative: flat clouds only in this phase, volumetric after 0.1.
   - Alternative: artist-shaped cloud models, as Sea of Thieves did (a style that games can do with ordinary models and effects anyway).

8. **The night sky's pictures.**
   - **Recommended:**
     - **Stars from the Yale Bright Star Catalogue**, about 9,000 real stars with their brightness and colour, drawn as points. It's a public scientific catalogue; its exact terms are checked and recorded in `licenses/` before it goes in.
     - **The Milky Way drawn by Solis** from noise, so no photograph's licence is needed.
     - **The moon lit by the sun**, so its phases come by themselves.
   - Alternative: a Milky Way photograph under CC BY (ESO's are), which needs credit in the program's notices.

9. **The look of the sky in stylized games.**
   - **Recommended:** the Environment's sky has a **style**:
     - **Physical**;
     - **Physical with colour curves**: horizon and zenith colours, sunset glow and haze, each set by the sun's height, laid over the physics with a blend;
     - **Painted**: today's gradient, kept.

     The cel-shaded look takes its bands from the direct light and lets ambient occlusion and sky light step into the same bands.
   - Alternative: physical only, with grading as the only control.

10. **Controls (CONTRIBUTING rule 17).**
    - **Recommended: no new keys or mouse actions.** Time of day, quality and exposure are fields and menus. Light probes and reflection probes are placed like any entity, with the existing gizmos.
    - Alternative: a time-of-day scrub in the viewport (hold a key and drag left or right), quick for judging light. T (all gizmos) and L (world or object axes) are taken, so the key would be chosen from the free ones then.

## Where things stand

From `crates/render` and decisions 0008, 0013, 0014 and 0017:

- **Drawing:** a forward renderer into a 16-bit HDR image with multisampling, then exposure and AgX, ACES or no tone mapping in one last pass (`post_fs`). The image's alpha marks the lit world, so the editor's backdrop and lines are never tone mapped.
- **The sun:** one sun, in Blender's units, with one 2048² shadow map fitted to the view out to 200 m, softened over nine texels.
- **Lamps:** up to 16 point and spot lights, nearest first, with no shadows.
- **The sky:** a painted gradient that follows the sun. It also gives the shadow side one flat ambient colour and reflects in metal.
- **Fog:** distance fog only.
- **Indirect light** is already a separate, swappable source (decision 0008), so probes plug in without rewriting materials.
- **The effects plan** (Phase 9) already designs the post chain, bloom and grading. They move here (L5), because light needs them first.

## The rules this phase keeps

- **Light never changes the game.** The time of day can (choice 4); everything drawn from it can't. Clouds, fog, probes and exposure are output, like sound. A test checks that the same game with every quality level gives the same checksums.
- **Nothing depends on ray tracing.** Every part works on any card Solis supports at Default, and Low works on cards without compute shaders and on the software driver the tests use. It is to be checked in L1 whether the software driver has compute; if not, Low is what the tests draw.
- **The scale target holds.** Each milestone adds its cost to the profiler (Phase 5's frame profiler, per pass) and a scale case. The end of the phase runs the full scale check at each quality level.
- **Stylized is first class.** Every milestone is tested and judged in both looks.
- **Reuse with licences kept.** Hillaire's sky demo (MIT), Bevy's atmosphere and effects (MIT or Apache-2.0), Bruneton's reference (BSD-3) and AMD's FidelityFX (MIT) may be adapted, with their notices in `licenses/` and the third-party notices (decision 0044).
- **Tests that look.** Golden pictures (small renders compared with saved ones, with a tolerance), plus number checks: the sky at noon is blue, at sunset the sun's light is orange, twilight has blue overhead, exposure settles within a set time.

## The shape of the code

| Crate | What's added |
|---|---|
| `render` | `sky/`: the four sky tables, the sun's colour, haze over distance, stars and moon<br>`light/`: clusters, the shadow atlas, cascades<br>`probes.rs`: light and reflection probes<br>`clouds/`<br>`fog/`: froxels<br>`post.rs`: exposure, bloom, grading, white balance<br>`quality.rs` |
| `area` | Environment gains the time of day, sky style and curves, clouds, fog, exposure, bloom and grading, and quality.<br>New built-in components: `LightProbeVolume`, `ReflectionProbe`, `FogVolume`. |
| `editor` | Properties for the above<br>the viewport's quality and exposure menu<br>**Bake light** in the Entity menu<br>the light showcase (Help menu) |
| `world` | Time of day as game state; natives in `solis/sky.lst` |

## Milestones

The order is set so that each milestone is visible on its own, and so that what later milestones need comes first.

### L1. Measure first: the light showcase and quality levels
- **The showcase.** **Help > Make the light showcase…** writes a project with:
  - an outdoor valley (hills, a lake, trees, made by Solis in code like the benchmark's ship);
  - a small house with a window;
  - a metal ship;
  - and the benchmark's fleets nearby.

  It is the place every later milestone is judged in, and the gate's scene.
- **Quality levels** (Low, Default, High), with the editor's own setting.
- **Golden-picture tests,** and checking which features the software driver has.
- The profiler shows each new pass.

### L2. Real light and automatic exposure
- Light in real units, with pre-exposure (the 16-bit image tops out at 65,504, so light is scaled by last frame's exposure before it's written).
- Exposure from a 64-step histogram of the picture's brightness, ignoring the darkest and brightest tenth, with separate speeds for brightening and darkening.
- Fixed exposure on request.
- Old areas converted.
- Manual: a page on exposure (why the picture brightens in the dark, as eyes do).

### L3. The physical sky
Hillaire's method:
- **The four small tables:** light lost along a path; light that bounced many times; the sky as seen from here; and a box of haze in front of the camera.
- **Light and colour:**
  - Rayleigh scattering (the blue sky, the orange sunset);
  - Mie scattering (the haze and the glow round the sun);
  - ozone (blue twilight).
- **The sun's light takes its colour from the sky**, so a low sun is orange and its shadows lengthen.
- **Haze over distance replaces distance fog.** Settings: how hazy, how far, and Earth-like by default.
- **Style:** the painted sky stays as a style (choice 9).
- Seeing the planet from above the air comes later.
- **Tests:**
  - colours at noon, sunset and twilight against Bruneton's reference;
  - the tables rebuilt when the sun moves, under 0.5 ms.

### L4. Light from the sky
- The sky is captured into a small cube picture when it changes.
- From it come:
  - ambient light from every direction (spherical harmonics), so the side facing up is lit blue by the sky and the side facing down is lit by the ground;
  - blurred reflections of the sky by roughness.
- This replaces the flat ambient colour, and gives the warm-sun, cool-shadow contrast.

### L5. The finished picture, part two
From the effects plan's M1 and its grading, moved here.
- **The post chain:** bloom (the Call of Duty method: gentle, no threshold, Karis average) and stylized bloom.
- **White balance:** "pick what white becomes", plus temperature and tint.
- **Grading:** sliders plus an optional look-up table image.
- **Night:** a Purkinje shift (bluer and less colourful in the dark).
- **Tone mapping:** AgX stays the default; Tony McMapface is added as a plain base for heavy grading.

### L6. Time of day and night
- **Sun and moon:** from latitude, date and hour, with the moon's phases.
- **Stars and the Milky Way:**
  - stars from the catalogue, turning through the night;
  - Solis's own Milky Way;
  - all dimmed near the horizon by the same air.
- **Colour curves by the sun's height** (choice 9).
- **Game code:** `setTimeOfDay`, `setDaySpeed` and `timeOfDay()`.
- **Manual:** a page on the day and night.

### L7. Clouds
1. **A flat cloud layer** on the sky, with a weather map (how much cloud, what kind) and a cheap light march, lit through the sky's tables.
2. **Volumetric clouds** (High), as in Horizon Zero Dawn:
   - noise textures (about 8 MB);
   - lighting: Beer's law, the silver lining and the powder term;
   - light reddened by the air at each step;
   - drawn at quarter size with a sixteenth refreshed each frame;
   - cloud shadows on the ground.

This may be two pull requests.

### L8. The depth prepass and ambient occlusion
- Depth and normals drawn first.
- Ambient occlusion by the visibility bitmask method, at half size with its own small denoiser, plus specular occlusion, so creases don't glow.
- High adds one bounce of nearby light from the same pass.

### L9. Shadows near and far
- **Cascaded sun shadows:** four cascades, snapped to whole texels so they don't crawl.
  - Far cascades are refreshed less often.
  - Small units are left out of far cascades, which keeps 2,000 units cheap.
- **Contact shadows** for the sun.
- **Soft shadows** at High: sharp at the base, softer further away (PCSS).

### L10. Many lamps
- **Clustered lighting:** hundreds of lamps at once, past decision 0014's 16.
- **Shadows for a budget of lamps:** four to eight, in one atlas, still ones cached.
- This lifts the effects plan's limit on flash lamps, so explosions can light the ships around them.

### L11. Volumetric fog and light shafts
- **Froxel fog:** fog in a box of cells over the view.
  - Light passes through it, so sun shafts fall through windows and trees.
  - Lamps glow in the mist.
- **Height fog,** and **fog volumes** (a misty valley, a smoky hall) that add, or carve fog away.
- Reuses frames, with care for fast lights.
- Low gets cheap screen-space god rays.

### L12. Reflection probes
- A **Reflection probe** entity with a box, baked in the editor.
- Reflections move correctly as you walk past (parallax corrected).
- Blended where boxes overlap.

### L13. Baked light probes
- A **Light probe volume** entity: a box of probes.
- **Bake light** traces light around the area on the graphics card, with Solis's own compute tracer (no ray tracing hardware), aiming at seconds for the showcase.
- **What gets the light:** every surface and every moving unit picks up bounced light from the nearest probes.
- **Saving:** stored next to the area as an asset, and rebaked when asked.
- This is decision 0008's second source of indirect light.

### End of the phase
- Quality levels tuned.
- The three reviewers (decision 0027).
- The scale check at each level.
- The manual's pages on light, sky and quality.
- The gate.

## After 0.1

Kept here so the design leaves room for them:
- **Real-time bounced light (Ultra),** traced against a simplified world: AMD's Brixelizer GI method, MIT, about 1.7 ms.
- **Baked lightmaps.**
- **Reflections on the screen (SSR).**
- **TAA and AMD's FSR upscaling.**
- **Light shafts in the sky** from mountains and clouds.
- **The planet seen from space.**
- **Weather that changes.**
- **Hardware ray tracing,** when wgpu's is stable.

## Netcode waits safely

Netcode moves from Phase 8 to Phase 11. What it needs is already built:
- **Determinism:** a fixed tick, seeded randomness, the start-of-tick world (decision 0033), strict physics (decision 0036) and state checksums.
- **Snapshots:** save games hold everything (decision 0038).
- **Planned to stay out of the game's state:** effects (the effects plan) and UI (the Sungraph plan); this phase's rule keeps light out too.

Two risks come with waiting:
1. **Nobody checks that two different PCs play the same game.** Today's tests compare runs on the same computer. Lua's maths functions, for example, can differ in the last digit between machines (decision 0028).

   **Recommended: a small check now.** A recorded game is played in CI on Linux and Windows, and its final checksum is compared with one saved in the repository. A difference shows up the day it's made, not in Phase 11.
2. **DESIGN.md §1 promises "good netcode built in"**, and the flagship sample plays over the network, so netcode must still come before 0.1. It does, in Phase 11.

The two-player Play button stays hidden until then (decision 0004).

## Sources

All in [docs/reports/lighting-research.md](../reports/lighting-research.md).
