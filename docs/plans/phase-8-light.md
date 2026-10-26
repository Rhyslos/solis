# Phase 8 plan: Light, sky and atmosphere (draft)

**Draft for the project lead.** The project lead asked on 2026-10-26 for "all the fancy lighting solutions working well", with sky, lighting and colour as Solis's selling point. The goal is skies and sunsets that give the feeling the best game sunsets do, and never the flat, sterile "Backrooms" look. This phase is new: it comes after sound and animation (Phase 7) and before effects (now Phase 9), game UI (now Phase 10) and netcode (now Phase 11). See [decision 0046](../decisions/0046-light-before-netcode.md), proposed.

The research behind every choice is in [docs/reports/lighting-research.md](../reports/lighting-research.md), with its sources. Nothing here is decided until the project lead answers the questions below. Then the answers become decisions, and this plan is kept up to date as milestones are built, like the other phase plans.

**Gate:** in the light showcase (L1), set a time of day and look. It is judged against screenshots of the project lead's reference games (see "The looks we aim for"), above all the warm, cosy one. The gate passes when:
- **The day runs from noon to night and back with no jumps.** The sky, sun colour, shadows, clouds and haze all change together. The brightness adjusts as eyes do. Twilight turns blue. Night looks like night: stars and the moon, and dark without going grey.
- **The golden hour feels right.** The sun is low and warm, shadows are cool, the clouds are lit from below, the haze glows towards the sun, and far hills fade into the air.
- **Indoors never looks like the Backrooms.** Inside the showcase's house, sun through a window bounces light around the room, and corners and contacts are soft and darker. At High, a lamp carried into the dark house lights the room with its bounce as it moves.
- **The stylized look is as good as the standard one.** The cel-shaded look gets the same sky and colours.
- **Each reference has its scene:**
  - the golden valley with a gradient fog (warm, cosy light);
  - a weather change from clear to storm, with the aurora at night (the world of weathers);
  - the rain-wet neon street at night (neon nights);
  - the desert sky and a passing sandstorm (the sky gradient);
  - noon to a moonless night in real units (realism);
  - a dark forest pool where plants and creatures glow as night falls (light from living things);
  - the block cave by torchlight in the light ramp look, where dark still reads as dark (identity).
- **The benchmark's target level still runs at 60 fps** at the Default quality level, on the project lead's PC.

## The looks we aim for

The project lead chose a set of reference games on 2026-10-26 and keeps that list. The plan describes the looks, not the games, ranked as the project lead ranked them:

| # | The look | What makes it | Where in this plan |
| --- | --- | --- | --- |
| 1 | **Warm, cosy light** (the most important; if only one reference could be kept, this one) | A sky built from a three-colour gradient with glows round the sun and the horizon; bold colours in layers; fog that gives depth; hand-set moods for moments of the day | L3 (tint over the sky, the fog colour ramp), L5, L7 (moods) |
| 2 | **A stylized world of weathers** (the overall favourite) | Weathers that each carry their own sky, fog, light and grading for sunrise, day, sunset and night; stacked cloud layers; light by direction indoors; more than one moon and the aurora; god rays | L4, L6, L7, L8, L13 |
| 3 | **Neon nights** | Many lights; glowing signs that light the haze; wet streets that reflect them | L11, L13, L15 |
| 4 | **A huge sky gradient** | A wide desert sky, warm to blue; sandstorms crossing the land | L3, L7, L13 |
| 5 | **Realism** | Real light units; sun, moon and stars where they would really be; truly dark moonless nights; very long views; one overcast control | L2, L6, L7, L10 |

Two more matter just as much:
- **Light from living things.** As the light fades (at night, in caves, deep under water), plants and animals start to glow, and their glow lights the world around them (L11, L13).
- **Identity, the most important of all.** Simple, readable light where dark means danger, warm torches and cold nights, in a blocky world (L12, the light ramp look).

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

## Answers (the project lead, 2026-10-26)

- **The order:** as recommended. Sound and animation stay Phase 7; light is Phase 8, effects 9, game UI 10, netcode 11.
- **Clouds:** as recommended, flat and volumetric.
- **Bounced light: every way that can be made fast.** Baked probes *and* real-time bounced light in this phase, plus a third or fourth way if one can be built or invented, as long as it performs. So bounced light gets four layers (L16 to L19 below), each with a hard cost budget and a scale case, kept only if it earns its place:
  1. **Baked light probes:** free at run time, for everything.
  2. **Bounce from the screen:** one bounce from what's in view, from the ambient occlusion pass (L9).
  3. **Real-time probes:** bounced light that follows moving lamps and the time of day, traced against a simplified copy of the world, with no ray-tracing hardware.
  4. **A Solis method, tried in a time-boxed experiment:** radiance cascades combined with the probes, or another idea found while building 1 to 3. It ships only if it beats layer 3 on cost or looks at the scale target.
- **The effects suite's name:** the project lead suggested **Aura**. It is crowded in exactly this field: an AI tool that makes particle effects for game engines, an award-winning volumetric lighting and fog tool for games, and a hardware maker's lighting software all use it, and "aura" is also a common kind of effect, which makes it hard to search for.

  Still open. Recommended: **Solstice**, or **Aureole** (from the same root as aura: a ring of light round the sun), which was clear in quick checks.
- The other choices are still open; until answered, the plan follows the recommendations.

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
   - it is a registered US trademark for software development tools;
   - another game engine with heavy particle effects is being built under the name;
   - the "Solis" prefix helps little.

   "Ember" sits right next to a leading fire and smoke tool for game effects with a very similar name.
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
   | **High** | Volumetric clouds, volumetric fog, soft shadows, reflections on the screen, real-time bounced light and bounce from the screen | about +6–9 ms |
   | **Low** | Turns things off for older cards; works on any card Solis opens on | — |

   - **Recommended: Low, Default, High, set per project, and changeable by players later** (the settings screen is Sungraph UI's, Phase 10). The editor's viewport has its own setting, so a slow laptop can edit at Low.
   - Alternative: one fixed look (simpler, but either too slow on old cards or too plain on good ones).

6. **Bounced light (global illumination).** Real-time bounced light is the one part still hard everywhere:
   - an open-source engine is rewriting its own, years after shipping it;
   - another's ray-traced version runs well on one maker's cards only;
   - ray tracing in wgpu is experimental.
   - *Answered: every way that can be made fast (see Answers).* The recommendation was:
     - **Baked light probes in this phase.** Solis bakes them on any card (no ray-tracing hardware), and thousands of moving units pick up bounced light from them almost for free.
     - **Real-time bounced light after 0.1**, on probes traced against a simplified copy of the world (an MIT-licensed method measured at about 1.7 ms), as a further "Ultra" level.
   - Alternative: real-time bounced light in this phase too (about two more months; the riskiest part of the phase).
   - Alternative: also baked lightmaps (the best still pictures, but every model needs a second unwrap; after 0.1).

7. **Clouds.**
   - **Recommended: both kinds in this phase.**
     - A **flat cloud layer** (Default): cheap, lit by the same sky, so clouds glow from below at sunset.
     - **Volumetric clouds** (High), after Schneider's method (SIGGRAPH 2015): drawn at quarter size, a sixteenth of the pixels refreshed each frame, about 1–2 ms.
   - Alternative: flat clouds only in this phase, volumetric after 0.1.
   - Alternative: artist-shaped cloud models (a style that games can do with ordinary models and effects anyway).

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
- **Reuse with licences kept.** Open-source code under MIT, Apache-2.0 or BSD-3 (Hillaire's and Bruneton's sky references, an open-source Rust engine's atmosphere, a GPU maker's distance-field lighting) may be adapted, with their notices in `licenses/` and the third-party notices (decision 0044).
- **Tests that look.** Golden pictures (small renders compared with saved ones, with a tolerance), plus number checks: the sky at noon is blue, at sunset the sun's light is orange, twilight has blue overhead, exposure settles within a set time.

## The shape of the code

| Crate | What's added |
|---|---|
| `render` | `sky/`: the four sky tables, the sun's colour, haze over distance, stars and moon<br>`light/`: clusters, the shadow atlas, cascades<br>`probes.rs`: light and reflection probes<br>`clouds/`<br>`fog/`: froxels<br>`post.rs`: exposure, bloom, grading, white balance, lens extras<br>`weather.rs`: weathers and their blending<br>the light ramp look in the shader<br>`ssr.rs`: reflections on the screen<br>`quality.rs` |
| `area` | Environment gains the time of day, sky style and curves, clouds, fog, exposure, bloom and grading, and quality.<br>New built-in components: `LightProbeVolume`, `ReflectionProbe`, `FogVolume`, `AreaLight`.<br>A new asset kind: weathers (`.solweather`). |
| `editor` | Properties for the above<br>the viewport's quality and exposure menu<br>**Bake light** in the Entity menu<br>the light showcase (Help menu) |
| `world` | Time of day as game state; natives in `solis/sky.lst` |

## Milestones

The order is set so that each milestone is visible on its own, and so that what later milestones need comes first.

### L1. Measure first: the light showcase and quality levels
- **The showcase.** **Help > Make the light showcase…** writes a project with:
  - an outdoor valley (hills, a lake, trees, made by Solis in code like the benchmark's ship);
  - a small house with a window;
  - a metal ship;
  - a city street at night, with neon signs and rain-wet ground;
  - a cave and a hut made of blocks, lit by torches;
  - a desert under a wide sky;
  - a forest pool with plants and creatures that glow in the dark;
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
- **Art direction over the physics** (warm, cosy light; the sky gradient):
  - a **tint over the physical sky**: zenith and horizon colours and a glow round the sun (the warm, cosy reference's sky is built from exactly these: a three-colour gradient with a sun halo and a horizon halo);
  - a **fog colour ramp**: a colour strip read by distance, height and angle to the sun, mixed with (or replacing) the physical haze, so far ridges fade through painted colours in layers.
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
- **Ambient by direction** as an override (a colour for each side), for rooms with no sky.

### L5. The finished picture, part two
From the effects plan's M1 and its grading, moved here.
- **The post chain:** bloom (Jimenez's method, SIGGRAPH 2014: gentle, no threshold, Karis average) and stylized bloom.
- **White balance:** "pick what white becomes", plus temperature and tint.
- **Grading:** sliders plus an optional look-up table image.
- **Night:** a Purkinje shift (bluer and less colourful in the dark).
- **Lens extras,** each off by default: lens flare, film grain, colour fringes at the edges (chromatic aberration) and heat haze.
- **Tone mapping:** AgX stays the default; Tony McMapface is added as a plain base for heavy grading.

### L6. Time of day and night
- **Sun and moon:** from latitude, date and hour, where they really would be, with the moon's phases changing how bright moonlight is, so a moonless night is truly dark.
- **More than one moon,** each with its own phase and colour.
- **Stars and the Milky Way:**
  - stars from the catalogue, turning through the night;
  - Solis's own Milky Way;
  - all dimmed near the horizon by the same air.
- **The aurora:** curtains of light in the night sky, shown by weather.
- **Colour curves by the sun's height** (choice 9).
- **Game code:** `setTimeOfDay`, `setDaySpeed` and `timeOfDay()`.
- **Manual:** a page on the day and night.

### L7. Weather and moods
Modelled on the world-of-weathers reference: each weather holds sky, fog, ambient, sun and star colours for sunrise, day, sunset and night, many cloud layers, and its own colour grading.
- **A weather file** (`.solweather`) keys, for each time of day:
  - the sky's tint;
  - the fog colours;
  - the sun's and ambient light's strength and colour;
  - the grade;
  - an exposure bias;
  - how cloudy it is.
- **Blending:** an area's Environment picks a weather, and changes between weathers blend smoothly over a set time.
- **One overcast control** changes the sky, clouds, shadow softness and light together.
- **Game code** can change the weather: `setWeather(file, seconds)`.
- **Moods** (hand-set moments of the day): a weather can be held at a moment rather than following the clock.
- Rain and snow themselves are effects (Phase 9); this milestone gives them their light and sky.
- **Ready-made weathers:** clear, golden, overcast, storm, desert, night.

### L8. Clouds
1. **A flat cloud layer** on the sky, with a weather map (how much cloud, what kind) and a cheap light march, lit through the sky's tables.
2. **Volumetric clouds** (High), after Schneider's method:
   - noise textures (about 8 MB);
   - lighting: Beer's law, the silver lining and the powder term;
   - light reddened by the air at each step;
   - drawn at quarter size with a sixteenth refreshed each frame;
   - cloud shadows on the ground.

This may be two pull requests.

### L9. The depth prepass and ambient occlusion
- Depth and normals drawn first.
- Ambient occlusion by the visibility bitmask method, at half size with its own small denoiser, plus specular occlusion, so creases don't glow.
- High adds one bounce of nearby light from the same pass.

### L10. Shadows near and far
- **Cascaded sun shadows:** four cascades, snapped to whole texels so they don't crawl.
  - Far cascades are refreshed less often.
  - Small units are left out of far cascades, which keeps 2,000 units cheap.
  - Shadows and haze reach 10 km or more for big maps, the far cascades refreshed rarely.
- **Contact shadows** for the sun.
- **Soft shadows** at High: sharp at the base, softer further away (PCSS).

### L11. Many lamps, glowing surfaces and area lights
- **Clustered lighting:** hundreds of lamps at once, past decision 0014's 16.
- **Shadows for a budget of lamps:** four to eight, in one atlas, still ones cached.
- **Glowing surfaces light the world:** a sign's or plant's glow feeds the bounced light and the fog.
- **Glow that wakes in the dark:** a material's glow can follow how dark it is around it, fading in at dusk, in caves and deep under water, so plants and creatures light up as night falls. Game code can also drive it (a creature that flashes when startled).
- **Area lights:** rectangle and tube lamps (neon tubes, screens, windows), shaded cheaply with a standard method (LTC).
- **Lightning:** a short, very bright light from the sky, for storms.
- This lifts the effects plan's limit on flash lamps, so explosions can light the ships around them.

### L12. The light ramp look
For blocky games whose light is part of their identity, simple on purpose: what's dark reads as dark, and dark means danger.
- **A fourth look** beside Standard, Stylized and Unlit. Light reaching a surface is split into **sky light** and **lamp light**, each stepped into levels (16 by default), and the pair is looked up in a **small colour table**: warm torchlight, cold moonlight, a curve set by a brightness slider.
- **Fixed face shading:**
  - tops brightest, sides darker, bottoms darkest;
  - soft darkening where blocks meet (from L9's ambient occlusion).
- **The fog tints towards the sun** at sunrise and sunset.
- **A square sun and moon** as a sky option.
- **The realistic parts switch off:** each of soft shadows, bounced light and haze can be turned off, so a blocky game keeps its readability.
- **What stays in games:** the light-level spreading that decides where monsters spawn is game logic. Solis gives the look; a voxel game writes its own light levels and hands them to the look as the lamp light.

### L13. Volumetric fog and light shafts
- **Froxel fog:** fog in a box of cells over the view.
  - Light passes through it, so sun shafts fall through windows and trees.
  - Lamps glow in the mist.
- **Height fog,** and **fog volumes** (a misty valley, a smoky hall) that add, or carve fog away.
- Fog volumes have an **absorption colour** and can **move**: a sandstorm crossing the desert, a swamp's mist, the colour of water deepening with depth.
- **Spotlights light up fog:** a torch's cone shows in the mist.
- **Caustics:** a moving pattern of light projected onto surfaces (sunlight through water).
- Reuses frames, with care for fast lights.
- Low gets cheap screen-space god rays.

### L14. Reflection probes
- A **Reflection probe** entity with a box, baked in the editor.
- Reflections move correctly as you walk past (parallax corrected).
- Blended where boxes overlap.

### L15. Reflections on the screen and wet surfaces
For rain-wet neon streets; reflection probes can't show moving lights and cars.
- **Screen-space reflections** at High: drawn at half size, reusing rays between pixels, with probes as the fallback where the screen has nothing to reflect.
- **Wetness:**
  - a wetness amount on the Environment (from the weather), darkening and glossing surfaces;
  - puddles where a mask says.
- **Budget:** about 1–2 ms. The known artefacts at the screen's edges fade into the probes.

### L16. Baked light probes
- A **Light probe volume** entity: a box of probes.
- **Bake light** traces light around the area on the graphics card, with Solis's own compute tracer (no ray tracing hardware), aiming at seconds for the showcase.
- **What gets the light:** every surface and every moving unit picks up bounced light from the nearest probes.
- **Saving:** stored next to the area as an asset, and rebaked when asked.
- This is decision 0008's second source of indirect light.

### L17. Real-time bounced light
- **Probes around the camera:** grids of probes in nested sizes (cascades), finer near the camera, that follow it, so open worlds work.
- **Tracing:** each frame a few probes are traced against a **simplified copy of the world**: small blocks of distance fields (how far is the nearest surface), as a published MIT-licensed method does (Brixelizer GI, GDC 2024). Where wgpu's hardware ray tracing is available and stable, it is used instead, behind the same probes.
- **What it gives:** bounced light that follows moving lamps, explosions, opened doors and the time of day.
- **Units only receive:** the thousands of moving units take light from the probes but aren't traced, which keeps the cost the same at 2,000 units or 10,000.
- **Budget:** at most about 2–3 ms at 1080p on a mid-range card, at High. Its authors report 1.7 ms on a mid-range card. Above the budget, it doesn't ship at that level.
- **Blending:** baked probes (L16) are the fallback where the real-time ones haven't settled, and on Default and Low.

### L18. Bounce from the screen, joined up
- One bounce of light from what's on screen (from L9's pass), laid over the probes for small, sharp detail the probes are too coarse for: the red glow of a wall on the floor beside it.
- The two are blended so nothing is counted twice.

### L19. An experiment: a Solis method
- A **time-boxed experiment** (a fixed number of days, decided at its start) to find a better way. The first candidate is **radiance cascades** (Alexander Sannikov's method, already used in 2D in a shipped game), joined with L17's probes and distance fields:
  - it gives soft shadows from every light without noise;
  - 3D versions are still research: a 2025 study measured 2–26 ms with flicker;
  - "split radiance cascades" (2026) is new and unproven in an engine.
- Other ideas found while building L16 to L18 can be tried instead.
- **Shipped only if it wins:**
  - it beats L17 on cost or looks, at the scale target;
  - it has no flicker the gate's viewer can see.
- Otherwise its findings are written up in `docs/reports/` and it stays out. Either way, the project lead sees the comparison.

### End of the phase
- Quality levels tuned.
- The three reviewers (decision 0027).
- The scale check at each level.
- The manual's pages on light, sky and quality.
- The gate.

## After 0.1

Kept here so the design leaves room for them:
- **Baked lightmaps.**
- **TAA and an open-source upscaler.**
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
