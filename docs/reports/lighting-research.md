# Research: light, sky and atmosphere for Solis

**2026-10-26.** The project lead asked for "all the fancy lighting solutions working well", with the sky, lighting and colour as Solis's selling point, and never the flat, sterile "Backrooms" look. This report covers the techniques found, what they cost and which open-source code may be reused. The plan built on it is [docs/plans/phase-8-light.md](../plans/phase-8-light.md). The project lead keeps the reference games and the name checks; they are not repeated here. Numbers come from the sources unless marked as an estimate.

## In one paragraph

Skies that feel right share one recipe: **a physically based sky and haze underneath, with art direction on top.**
- **The physics** gives colours that feel right without anyone painting them: sunlight scattering in air, ozone, clouds lit by the sun, and haze that tints distance.
- **The art direction** gives the mood: white balance, colour grading, colour by time of day, and stylized clouds.

Almost all of it can be built on wgpu with compute shaders and no ray tracing. The sky and haze together cost a few milliseconds a frame.

Real-time bounced light (global illumination) is the one part that is still hard everywhere. Solis gets there in layers: baked probes first, then real-time probes, each kept within a cost budget.

## Why lighting looks "Backrooms"

- **Ambient light from no direction.** Light that's the same from every side makes everything flat; nothing shows its shape. The fixes:
  - ambient light that changes with the way a surface faces (from the sky above, the ground below);
  - ambient occlusion, which darkens creases and corners where light can't reach.
- **Grey shadows.** Outdoors, shadows are lit by the blue sky while the sun is yellow, so they look blue or purple, most of all at sunrise and sunset. A warm sun against cool shadows is half of a sunset's feeling.
- **Tone curves that twist colour.** The common ACES film curve turns bright reds and greens orange and bright blues purple, and washes out bright saturated colours. AgX, Solis's default, keeps colours more honest.
- **Exposure that jumps with the scene.** Measuring brightness from the picture makes it lurch when a dark wall fills the view. Metering the light arriving instead avoids it.
- **No depth in the air.** Without haze, far hills look pasted on.
- **Light that is even, colourless and comes from nowhere** (fluorescent ceilings, no sun direction, one colour temperature). This is our reading of the "Backrooms" look itself; no source measures it.

## Sky and atmosphere

| Part | What it adds | Cost (frame, 1080p) | Size to build |
| --- | --- | --- | --- |
| **Physical sky** (Hillaire 2020) | Real sky colours at every sun height, the orange low sun, ozone's blue twilight, haze that tints distance (aerial perspective) | about 0.3 ms (measured on a GTX 1080), under 1 MB | small–medium |
| **Light in real units, auto exposure** | Noon is about 100,000 lux and starlight 0.0001: 30 stops, so exposure has to follow the scene, as eyes do | about 0.1 ms | medium |
| **Night and time of day** | Sun and moon from place, date and hour; stars from a catalogue; moon phases | negligible | small |
| **Clouds: a flat layer** | Clouds on the sky, lit from below at sunset by the same sky | under 0.2 ms | small |
| **Clouds: volumetric** (Schneider 2015) | Clouds with depth, silver linings, shadows on the ground | about 1–2 ms with quarter-size tracing and reuse over frames | large |
| **Volumetric fog and light shafts** (froxels; Wronski 2014, Hillaire 2015) | Fog that light passes through, god rays, fog volumes | about 0.5–1.1 ms, 10–15 MB | medium |

**The physical sky** builds four small tables each frame and reads them when drawing:
- the light lost along a path;
- light that bounced many times;
- the sky as seen from here;
- a box of haze in front of the camera.

Bruneton's older method looks as good, but takes about 250 ms to rebuild when the sun moves. It is used only as a reference to test against.

**Clouds need reuse over frames.** Drawn in full every frame, they cost about 11.6 ms at 1080p on a GTX 1080. Schneider's method draws at quarter size and refreshes a sixteenth of the pixels each frame. His noise textures come to about 8 MB.

**Physics with art direction:** keep the physical result, and steer it:
- a "hazier than physics" control over distance;
- a tint over the sky;
- grading through a colour table, at well under 0.1 ms.

## Light, shadow and bounce

| Part | What it adds | Cost (frame, 1080p) | Size to build |
| --- | --- | --- | --- |
| **Depth and normals drawn first** (prepass) | What every screen effect reads | about 0.3–0.8 ms (estimate) | small–medium |
| **Sky light from the real sky** | Ambient light from the sky above and the ground below, and the sky reflected in shiny surfaces, instead of one flat colour | about 0.1 ms | small |
| **Ambient occlusion** (visibility bitmask, Therrien 2023) | Soft darkening in creases and contact; one bounce of nearby light almost for free | about 0.5–1 ms (GTAO, its parent, Jimenez 2016: 0.5 ms on a console) | medium |
| **Cascaded shadows** | Sharp shadows near the camera, and shadows far away (today one map covers 200 m) | about 1–3 ms, depending on the scene (estimate) | medium |
| **Contact shadows** | Small shadows where things touch, which maps miss | about 0.2–0.5 ms | small |
| **Soft shadows** (PCSS) | Sharp at the base, softer further away | +0.3–1 ms | small |
| **Many lamps** (clustered shading, Olsson 2012) | Hundreds of lamps at once instead of 16; a few with shadows | about 0.2–0.5 ms plus shadows | medium |
| **Reflection probes** | Local reflections that move correctly as you walk (parallax corrected) | about 0.1–0.3 ms | medium |
| **Baked light probes** (irradiance volumes) | Light that bounces around a level, on everything, including thousands of moving units | about 0.1–0.3 ms | large (the baker) |
| **Reflections on the screen** (SSR) | Mirror-like water, floors and wet streets | about 1–2 ms (estimate) | medium |
| **Real-time bounced light** (probes traced against distance-field bricks; DDGI, Majercik 2019; Brixelizer GI, GDC 2024) | Bounce that follows moving lamps and the time of day | about 2–4 ms (1.7 ms reported on a mid-range card) | large |
| **Radiance cascades** (Sannikov) | Noise-free soft light from every source | 3D versions are research: 2–26 ms with flicker (2025 study) | research |

**Why nothing depends on ray tracing:**
- Ray tracing in wgpu is still experimental: its docs warn of "major bugs".
- The scene's structure is rebuilt every frame, which suits 20,000 moving things badly.
- The open-source engines' general dynamic bounced-light systems are either being rewritten or tied to one maker's cards.

So screen and probe techniques come first, and real-time bounce is traced against a simplified copy of the world, not on ray-tracing hardware.

**Temporal anti-aliasing (TAA)** hides noise by averaging frames, but blurs and leaves trails behind fast things, such as 20,000 lasers. So Solis keeps its sharp multisampling, and gives each noisy effect its own small history.

## The colour pipeline

| Step | Recommended | Why |
| --- | --- | --- |
| Tone mapping | **AgX** stays the default | Honest colours; bright light goes to white as in a camera |
| Bloom | Jimenez's method (SIGGRAPH 2014): no threshold, gentle, a pyramid of blurs | Bright things spread light as through a real lens |
| Exposure | **Automatic**, from a histogram, with separate speeds for brightening and darkening; or fixed | As eyes adjust |
| White balance | "Pick what white becomes", plus temperature and tint | Easier for artists than a temperature slider alone |
| Grading | Sliders plus an optional look-up table image | The common way to grade a game |
| Night | A Purkinje shift: bluer and less colourful in the dark | Night looks like night |

## What it costs together

At 1080p on a mid-range card (estimates built from the sources):

| Setting | Includes | Total |
| --- | --- | --- |
| **Default** | Prepass, sky light, ambient occlusion, cascaded and contact shadows, many lamps, reflection probes, physical sky, flat clouds, auto exposure, bloom, grading | about **4–7 ms** |
| **High** | Default, plus volumetric clouds and fog, soft shadows, reflections on the screen, real-time bounced light and bounce from the screen | about **+6–9 ms** |

A frame at 60 fps is 16.7 ms, and the benchmark's target level already uses part of it. So every part gets quality settings, and the scale check measures them.

## Code that may be reused

All permissively licensed. Each is checked again and recorded in `licenses/` when used.
- Hillaire's sky and atmosphere demo (MIT).
- Bruneton's precomputed atmospheric scattering (BSD-3).
- An open-source Rust engine's WGSL atmosphere (MIT or Apache-2.0).
- AMD FidelityFX Brixelizer GI (MIT).
- AMD FidelityFX FSR 3 (MIT), for later.

## Sources

**Sky, atmosphere, clouds and fog**
- Hillaire, "A Scalable and Production Ready Sky and Atmosphere Rendering Technique" (EGSR 2020): https://diglib7.eg.org/bitstream/handle/10.1111/cgf14050/v39i4pp013-022.pdf
- Hillaire's demo (MIT): https://github.com/sebh/UnrealEngineSkyAtmosphere
- Bruneton, precomputed atmospheric scattering (BSD-3): https://github.com/ebruneton/precomputed_atmospheric_scattering
- Schneider, real-time volumetric cloudscapes (SIGGRAPH 2015, Advances in Real-Time Rendering): https://www.advances.realtimerendering.com/s2015/
- Hillaire, physically based sky, atmosphere and clouds (2016) and unified volumetric rendering (2015): https://www.ea.com/news/physically-based-sky-atmosphere-and-cloud-rendering, https://www.ea.com/news/physically-based-unified-volumetric-rendering-in-frostbite
- Atmosphere and clouds in Vulkan, costs measured (CESCG 2023): https://cescg.org/wp-content/uploads/2023/04/Sakmary-Real-time-Rendering-of-Atmosphere-and-Clouds-in-Vulkan.pdf
- Wronski, volumetric fog (GDC 2014): https://bartwronski.com/publications/
- Jensen et al., a physically-based night sky: https://cseweb.ucsd.edu/~henrik/papers/nightsky/
- Ozone and the blue hour: https://en.wikipedia.org/wiki/Chappuis_absorption

**Light, shadow and bounce**
- wgpu ray tracing status: https://wgpu.rs/doc/wgpu/documentation/extensions/ray_tracing/index.html
- Majercik et al., DDGI (2019): https://cim.mcgill.ca/~derek/files/DDGI-lowres.pdf
- Brixelizer GI (GDC 2024): https://gpuopen.com/gdc-presentations/2024/GDC2024_GI_with_AMD_FidelityFX_Brixelizer.pdf
- Radiance cascades: https://arxiv.org/abs/2505.02041v1, https://odr.chalmers.se/items/9e42007a-bb60-440f-ba28-04410822758f/full
- Jimenez et al., GTAO (2016): https://www.iryoku.com/downloads/Practical-Realtime-Strategies-for-Accurate-Indirect-Occlusion.pdf
- Therrien et al., visibility bitmask (2023): https://arxiv.org/abs/2301.11376v2
- Stable shadow maps: https://learn.microsoft.com/en-us/windows/win32/dxtecharts/common-techniques-to-improve-shadow-depth-maps
- Olsson et al., clustered shading: https://www.cse.chalmers.se/~uffe/clustered_shading_preprint.pdf
- PCSS: https://developer.download.nvidia.com/shaderlibrary/docs/shadow_PCSS.pdf
- Stochastic screen-space reflections: https://frostbite.com/frostbite/news/stochastic-screen-space-reflections
- Yang, Liu and Salvi, a survey of temporal anti-aliasing (2020): https://research.nvidia.com/labs/rtr/publication/yang2020survey

**Colour**
- AgX in Blender 4.0: https://wiki.blender.org/release_notes/4.0/color_management/
- Jimenez, next-generation post-processing (bloom, SIGGRAPH 2014): https://www.iryoku.com/
- Khronos PBR Neutral: https://www.khronos.org/news/permalink/khronos-pbr-neutral-tone-mapper-released-for-true-to-life-color-rendering-of-3d-products
