# Research: light, sky and atmosphere for Solis

**2026-10-26.** Asked for by the project lead: "all the fancy lighting solutions working well", with the sky, lighting and colour as Solis's selling point, inspired by the feeling great game sunsets give, and never the flat, sterile "Backrooms" look. This report is what was found; the plan built on it is [docs/plans/phase-8-light.md](../plans/phase-8-light.md). Sources are listed at the end; numbers are from them unless marked as an estimate.

## In one paragraph

The games with loved skies share one recipe: **a physically based sky and haze underneath, with art direction on top.** The physics (sunlight scattering in air, ozone, clouds lit by the sun, haze that tints distance) gives colours that feel right without anyone painting them; the art direction (white balance, colour grading, colour by time of day, stylized clouds) gives the mood. Almost all of it can be built on wgpu with compute shaders, with no ray tracing, and the sky and haze together cost a few milliseconds a frame. Real-time bounced light (global illumination) is the one part that is still hard everywhere, even for Godot and Bevy; Solis should get there in steps, starting with baked light probes.

## What makes a sunset feel right

**The Any Austin video** is "The (Objectively) Best Video Game Sunsets" (17 October 2026), which ranks sunsets with a scoring formula; GTA V's Los Santos is on it. The rest of the list couldn't be confirmed (the article listing it is paywalled and YouTube couldn't be reached from here), so it's left out rather than guessed. The project lead's own list of favourites, from the video, would make good reference pictures for the gate.

**What the loved skies do:**

- **Ghost of Tsushima** (Sucker Punch, SIGGRAPH 2021), the most useful single source. "Stylized realism": a physical sky drives the sky, clouds, haze and fog, from 10 cm to 100 km; artists can push it away from correct. Exposure follows the light arriving, not what the camera sees, so dark or bright surfaces don't pump the brightness. They found common tone curves push reds towards yellow (worse at sunset) and fixed it. White balance is "pick what white should become", which artists found easier than a temperature slider. Night gets a "Purkinje shift" (bluer, less colourful), so it looks like night rather than dark day. One wind moves grass, smoke, cloth and particles.
- **Horizon Zero Dawn** (Guerrilla, SIGGRAPH 2015): ray-marched clouds lit by the sun: light fading through the cloud (Beer's law), a bright "silver lining" near the sun, and darker edges facing it ("powder"). Time of day lights everything with nothing baked. About 2 ms on a PS4 after drawing at quarter size and updating a sixteenth of the pixels each frame.
- **Sea of Thieves** (Rare, 2018): stylized on purpose. Clouds are shapes artists model (even skulls), lit cheaply, crisp at a distance; the sea mixes a deep colour with a glowing one where sun shines through waves.
- **Firewatch** (GDC 2015): bold colours in distinct layers that give depth, fog for distance, a "colour script" tying palettes to the story, and a sky that can fill half the screen.
- **Journey:** colour and light follow the story: cold and grey as the sun sets, flooded with light above the clouds, ending at sunrise.
- **Red Dead Redemption 2** (SIGGRAPH 2019): one joined-up system for sky, clouds and fog over 24 hours. Details couldn't be reached.

## Why lighting looks "Backrooms"

- **Ambient light from no direction.** Light that's the same from every side makes everything flat; nothing shows its shape. The fixes are ambient light that changes with the way a surface faces (from the sky above, the ground below) and ambient occlusion (darkening in creases and corners where light can't reach).
- **Grey shadows.** Outdoors, shadows are lit by the blue sky while the sun is yellow, so they look blue or purple, most of all at sunrise and sunset. A warm sun against cool shadows is half of a sunset's feeling.
- **Tone curves that twist colour.** The common film curve (ACES) turns bright reds and greens orange and bright blues purple, and washes out bright saturated colours; Unreal has a "blue correction" setting just for it. AgX (Solis's default, Blender's since 4.0) keeps colours more honest.
- **Exposure that jumps with the scene.** Measuring brightness from the picture makes it lurch when a dark wall fills the view.
- **No depth in the air.** Without haze, far hills look pasted on.
- **Even, colourless, sourceless light** (fluorescent ceilings, no sun direction, one colour temperature) is the Backrooms itself. That last point is our reading; no source measures it.

## Sky and atmosphere: what to build

| Part | What it adds | Cost (frame, 1080p) | Size to build |
| --- | --- | --- | --- |
| **Physical sky (Hillaire 2020)** | Real sky colours at every sun height, the orange low sun, ozone's blue twilight, haze that tints distance (aerial perspective), the planet seen from space | about 0.3 ms (GTX 1080), under 1 MB | small–medium |
| **Light in real units, auto exposure** | Noon is about 100,000 lux and starlight 0.0001: 30 stops, so exposure has to follow the scene, as eyes do | about 0.1 ms | medium |
| **Night and time of day** | Sun and moon from place, date and hour; stars from a catalogue; moon phases; the sky's colours keyed by the sun's height for art direction | negligible | small |
| **Clouds: a flat layer** | Clouds on the sky, lit from below at sunset by the same sky | under 0.2 ms | small |
| **Clouds: volumetric (Horizon's method)** | Clouds with depth, silver linings, shadows on the ground | about 1–2 ms with quarter-size tracing and reuse over frames | large |
| **Volumetric fog and light shafts** | Fog that light passes through, god rays, fog volumes (a misty valley, a smoky hall) | about 0.5–1.1 ms, 10–15 MB | medium |

- **The physical sky** builds four small tables each frame (the light lost along a path, light that bounced many times, the sky as seen from here, and a box of haze in front of the camera) and reads them when drawing. Bruneton's older method looks as good but takes about 250 ms to rebuild when the sun moves, so it is used only as a reference to test against.
- **Ready references with licences Solis may use:** Hillaire's own demo (MIT), Bevy's atmosphere in WGSL, the shader language Solis uses (MIT or Apache-2.0), and Bruneton's (BSD-3, notice kept).
- **What other engines have:** Godot 4's sky is an older, simpler model with no ozone, twilight or haze over distance, so Solis can do better than Godot here. Unreal and Bevy use Hillaire's method.
- **Clouds need reuse over frames.** Drawn fully every frame they cost about 11.6 ms at 1080p on a GTX 1080, and that cost is why the Horizon trick matters. Their noise textures are about 8 MB at Horizon's sizes.
- **Stylized games** keep the physics and steer it:
  - Unreal has a "make the distance hazier than physics" setting and lets a custom material take over the sky.
  - Genshin Impact grades its physical fog through a colour table, at under 0.05 ms.
  - For Solis, today's painted gradient sky can live on as a stylized preset over the physical one.

## Light, shadow and bounce: what to build

| Part | What it adds | Cost (frame, 1080p) | Size to build |
| --- | --- | --- | --- |
| **Depth and normals drawn first** (a prepass) | What every screen effect below reads | about 0.3–0.8 ms (estimate) | small–medium |
| **Sky light from the real sky** | Ambient light that comes from the sky above and the ground below, and reflections of the sky in shiny surfaces, instead of one flat colour | about 0.1 ms | small |
| **Ambient occlusion (visibility bitmask, 2023)** | Soft darkening in creases and contact; one bounce of nearby light almost for free | about 0.5–1 ms (GTAO, its parent: 0.5 ms on PS4) | medium |
| **Cascaded shadows** | Sharp shadows near the camera and shadows far away (today one map covers 200 m) | about 1–3 ms, depending on the scene (estimate) | medium |
| **Contact shadows** | Small shadows where things touch the ground, that maps miss | about 0.2–0.5 ms | small |
| **Soft shadows (PCSS)** | Shadows sharp at the base and soft further away, as real ones are | +0.3–1 ms | small |
| **Many lamps (clustered lighting)** | Hundreds of lamps at once instead of 16, so every explosion and muzzle flash can light its surroundings; a few lamps with shadows | about 0.2–0.5 ms plus shadows | medium |
| **Reflection probes** | Local reflections that move correctly as you walk (parallax corrected), baked in the editor | about 0.1–0.3 ms | medium |
| **Baked light probes (irradiance volumes)** | Light that bounces around a level, on everything including thousands of moving units; baked by a baker Solis would write, which needs no ray-tracing hardware | about 0.1–0.3 ms | large (the baker) |
| **Screen-space reflections** | Mirror-like water and floors | about 1–2 ms (estimate) | medium |
| **Real-time bounced light** | Bounce light that follows moving lamps and time of day | about 2–4 ms (AMD's Brixelizer GI, MIT: 1.7 ms on an RX 7600 XT) | large |

- **Ray tracing in wgpu is still experimental.** Its docs warn of "major bugs", and the scene structure is rebuilt every frame, which with 20,000 moving things is a poor fit. So nothing in Solis should depend on ray tracing yet.
- **Bevy's ray-traced lighting (Solari)** in practice runs only on NVIDIA cards, because its noise remover is DLSS.
- **Godot's dynamic bounced light (SDFGI)** is its slowest option, leaks light and is being rewritten, years after it shipped: a sign of how big a general system is.
- **The lesson:** do the screen and probe pieces first, as Bevy did, and real-time bounce light later, on probes traced against a simplified copy of the world (AMD's MIT-licensed approach) rather than ray tracing hardware.
- **Temporal anti-aliasing (TAA) smears.** It hides noise by averaging frames, but it blurs and leaves ghost trails behind fast things like 20,000 lasers. Solis keeps its sharp multisampling, and gives each noisy effect its own small history instead.
- **Temporal upscaling** (AMD's FSR 3, MIT-licensed) can come after Solis has a TAA of its own.

## The colour pipeline

| Step | Recommended | Why |
| --- | --- | --- |
| Tone mapping | **AgX** stays the default | Honest colours. Bevy defaults to Tony McMapface, an even plainer curve meant to leave the look to grading, and Godot added AgX in 4.4. |
| Bloom | The Call of Duty method: no threshold, gentle, a pyramid of blurs | Bright things spread light as through a real lens (already planned in the effects plan) |
| Exposure | **Automatic**, from a histogram, with separate speeds for brightening and darkening; or fixed | As eyes adjust. Ghost of Tsushima adds metering by the light arriving |
| White balance | "Pick what white becomes", plus temperature and tint | Ghost of Tsushima's artist-friendly way |
| Grading | Sliders plus an optional look-up table image | Unity and Unreal do the same |
| Night | A Purkinje shift: bluer, less colourful in the dark | Night looks like night |

## What it costs together

At 1080p on a mid-range card (estimates built from the sources):

| Setting | Includes | Total |
| --- | --- | --- |
| **Default** | Prepass, sky light, AO, cascaded and contact shadows, many lamps, reflection probes, physical sky, flat clouds, auto exposure, bloom, grading | about **4–7 ms** |
| **High** | Default, plus volumetric clouds, volumetric fog and soft shadows | about **+3–4 ms** |
| **Ultra** | High, plus reflections on the screen and real-time bounce light | about **+3–6 ms** |

After the project lead's answers, the plan puts reflections on the screen and real-time bounced light into High (about +6–9 ms over Default in all), with no Ultra for now.

A frame at 60 fps is 16.7 ms, and the benchmark's target level already uses part of it. So each part gets quality settings, and the scale check measures them.

## The project lead's reference games

Named on 2026-10-26: Firewatch, Skyrim, Cyberpunk 2077, Mad Max (2015) and Arma 3, in that order, then Minecraft and Subnautica. Two things were checked against their sources: Skyrim's weather records (the Creation Kit wiki) and Firewatch's sky (Campo Santo's blog and the GDC 2015 write-up). The rest is from general knowledge of the games and their engines, found by search but not opened, and is marked *(unchecked)*.

- **Firewatch** (Unity):
  - **The sky** is built from a three-colour gradient (top, middle, bottom), a sun halo for the haze around the sun, and a separate horizon halo (Campo Santo).
  - **Colour does the work:** bold colours in distinct layers give depth, fog adds depth and distance, and "the biggest chunk of colour is actually determined by the sky" (GDC 2015 write-up).
  - **Gradient fog:** a fog colour taken from a painted strip by distance is widely described *(unchecked)*. Solis's fog colour ramp (plan L3) is built in that spirit.
- **Skyrim** (Creation Engine). A weather record holds:
  - Ambient, Fog, Horizon, Sky-Lower/Upper, Sun, Sunlight and Stars colours for sunrise, day, sunset and night;
  - up to 16 cloud layers, each with its own texture and speed;
  - one ImageSpace (its colour grading);
  - directional ambient per time slot (a colour, direction and specular per axis);
  - wind, fog distances, precipitation, thunder and lightning, and an Aurora setting (Creation Kit wiki).

  The Special Edition added volumetric god rays (E3 2016 coverage). Two moons with phases *(unchecked)*.
- **Cyberpunk 2077** (REDengine 4) *(unchecked)*:
  - **Without ray tracing:** neon and many lamps, haze lit by them, wet streets with screen-space reflections and a puddle material.
  - **With ray tracing:** ray-traced reflections, shadows and bounced light.
  - **RT Overdrive (2023):** path tracing with NVIDIA's denoiser (GDC 2023 and 2024).
  - **For Solis:** the look survives without ray tracing if there are screen-space reflections, wet surfaces and glowing surfaces that light the haze.
- **Mad Max (2015)** (Avalanche):
  - a physically based renderer with time of day and weather;
  - moving sandstorms with dense dust and lightning *(unchecked)*;
  - how the sky itself is made wasn't found.
- **Arma 3** (Real Virtuality 4) *(unchecked)*:
  - HDR with eye adaptation, and very long views;
  - sun, moon and stars placed for the date and the map's latitude, with moon phases, so moonless nights are truly dark;
  - one "overcast" value driving clouds, light and rain;
  - a 2016 visual upgrade to lighting, tone mapping, sky and clouds (Bohemia's "OPREP – Visual Upgrade").
- **Minecraft** *(unchecked)*:
  - **Light levels:** every block holds a sky light and a block light from 0 to 15, falling by one a block.
  - **Colour:** a small table turns each pair into a colour (warm torches, sky light dimming at night).
  - **Shading:** smooth lighting darkens corners, and faces are shaded by direction (tops brightest).
  - **Sky:** the fog turns sunrise-coloured towards the sun, and the sun and moon are square.
  - **Light has meaning in play** (where monsters spawn).
  - Bedrock's Vibrant Visuals (2025) added deferred lighting, sun shadows, volumetric fog and better water while keeping the pixel textures. Shader packs (BSL, Complementary) add shadows, volumetric light, coloured light and bloom.
  - **The identity is simplicity and readability:** dark reads as dark.
- **Subnautica** (Unity) *(unchecked; only forum threads found)*:
  - water fog coloured per biome by absorption;
  - light shafts and moving caustics near the surface;
  - deep areas lit by glowing plants and creatures and the player's own lights.
  - **For Solis:** fog volumes with an absorption colour, a caustics projector, and glowing surfaces feeding the bounced light are enough; no special underwater system is needed.

**What the plan gained from them:**
- weathers and moods (Skyrim, Firewatch, Arma);
- a tint over the physical sky and a fog colour ramp (Firewatch, Mad Max);
- glowing surfaces that light the world, area lights and lightning (Cyberpunk, Subnautica, storms);
- moving fog volumes with a colour, spotlights in fog, caustics (Mad Max, Subnautica);
- reflections on the screen and wet surfaces, moved up from "after 0.1" (Cyberpunk);
- real sky positions, several moons, the aurora (Arma, Skyrim);
- long shadow and haze distances (Arma);
- lens extras (Cyberpunk, Mad Max);
- the light ramp look (Minecraft).

## Names for the effects suite

Not legal advice; checked from public trademark copies (US only) and web searches.

- **Eclipse:** highest risk. The Eclipse Foundation holds a live US trademark ECLIPSE for software development tools (2017), which a game engine arguably is. A particle-heavy "Eclipse Engine" is being built for *Star Wars: Eclipse*, and there is an "Eclipse Origins" game maker. The "Solis" prefix helps little: the strong word is still Eclipse.
- **Ember:** high risk in this field. **EmberGen** (JangaFX) is a leading real-time fire and smoke tool for game effects, and "Solis Ember" next to fire effects invites confusion. Ember.js also holds the EMBER software trademark.
- **Solstice:** lowest risk. Mersive's SOLSTICE trademark is for meeting-room screen sharing, a different market, and no game engine or effects tool by that name was found. "Solis Solstice" echoes the "of the sun" theme.
- **Other clear-looking names** (quick checks only): Parhelion (a "sun dog"), Aureole (a ring of light round the sun), Sunspot.
- **Aura** (suggested by the project lead, checked afterwards): crowded in exactly this field. Ramen VR's Aura is an AI agent that makes Niagara particle effects for Unreal and Unity; Aura 2 is an award-winning volumetric lighting and fog tool for Unity; ASUS's Aura Sync is lighting software. **Aureole** shares its root and was clear.
- **Names to avoid:** Corona (Chaos Corona renderer), Glimmer (shader packs), Flare, Lumen (Unreal's lighting), Halo.

A trademark professional's search would be needed before relying on any of this. EU, UK and WIPO registers weren't checked.

## Sources

**Sunsets and colour**
- Any Austin, "The (Objectively) Best Video Game Sunsets": https://www.youtube.com/watch?v=IRaRU-O6hIU
- Boing Boing on the video: https://premium.boingboing.net/p/the-very-best-sunsets-in-gaming
- Patry, Ghost of Tsushima (SIGGRAPH 2021): https://www.advances.realtimerendering.com/s2021/jpatry_advances2021/index.html
- PlayStation Blog on Ghost of Tsushima: https://blog.playstation.com/?p=345372
- Schneider, Horizon Zero Dawn clouds (2015): https://www.advances.realtimerendering.com/s2015/The%20Real-time%20Volumetric%20Cloudscapes%20of%20Horizon%20-%20Zero%20Dawn%20-%20ARTR.pdf
- Guerrilla, Nubis cubed: https://www.guerrilla-games.com/read/nubis-cubed
- Ang, The Technical Art of Sea of Thieves (2018): https://history.siggraph.org/wp-content/uploads/2022/09/2018-Talks-Ang_The-Technical-Art-of-Sea-of-Thieves.pdf
- Firewatch at GDC 2015: https://www.thumbsticks.com/gdc-2015-the-art-of-firewatch
- Journey's colour: https://epiloguegaming.com/visualizing-the-narrative-journey/
- Flat ambient light: https://www.leadwerks.com/community/topic/11170-why-is-ambient-light-flat
- The Backrooms' look: https://nofilmschool.com/six-design-tricks-backrooms and https://musemagazine.it/articles/backrooms-en/
- Coloured shadows: https://www.pressreader.com/uk/the-artist/20210601/283330410142945
- Blender 4.0 colour management (AgX): https://wiki.blender.org/release_notes/4.0/color_management/
- Godot 4.4 (AgX): https://godotengine.org/article/dev-snapshot-godot-4-4-beta-1/
- Bevy's tone mappers: https://doc.qu1x.dev/bevy_trackball/bevy/core_pipeline/tonemapping/enum.Tonemapping.html
- Khronos PBR Neutral: https://www.khronos.org/news/permalink/khronos-pbr-neutral-tone-mapper-released-for-true-to-life-color-rendering-of-3d-products
- Gran Turismo 7's colour: https://www.gtplanet.net/why-gran-turismo-7-graphics-look-20250902/
- Unreal, grading and tone mapper: https://dev.epicgames.com/documentation/en-us/unreal-engine/color-grading-and-the-filmic-tonemapper-in-unreal-engine
- Unreal, auto exposure: https://dev.epicgames.com/documentation/en-us/unreal-engine/auto-exposure-in-unreal-engine
- Jimenez, Call of Duty bloom: https://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare/
- Unity URP tone mapping: https://docs.unity.com/en-us/engine/6000.3/manual/post-processing-and-full-screen-effects/urp/effect-list/tonemapping

**Sky, atmosphere, clouds and fog**
- Hillaire, sky and atmosphere (EGSR 2020): https://diglib7.eg.org/bitstream/handle/10.1111/cgf14050/v39i4pp013-022.pdf and slides https://blog.selfshadow.com/publications/s2020-shading-course/hillaire/s2020_pbs_hillaire_slides.pdf
- Hillaire's demo (MIT): https://github.com/sebh/UnrealEngineSkyAtmosphere
- Bruneton's precomputed scattering (BSD-3): https://github.com/ebruneton/precomputed_atmospheric_scattering
- Bevy's atmosphere: https://doc.qu1x.dev/bevy_trackball/src/bevy_pbr/atmosphere/mod.rs.html, https://bevy.org/news/bevy-0-18/
- Godot's PhysicalSkyMaterial: https://raw.githubusercontent.com/godotengine/godot/master/doc/classes/PhysicalSkyMaterial.xml
- Unreal's Sky Atmosphere: https://dev.epicgames.com/documentation/en-us/unreal-engine/sky-atmosphere-component-in-unreal-engine
- Unreal's physical light units: https://dev.epicgames.com/documentation/en-us/unreal-engine/using-physical-lighting-units-in-unreal-engine
- Frostbite sky and clouds (2016): https://www.ea.com/news/physically-based-sky-atmosphere-and-cloud-rendering
- Atmosphere and clouds in Vulkan, costs measured: https://cescg.org/wp-content/uploads/2023/04/Sakmary-Real-time-Rendering-of-Atmosphere-and-Clouds-in-Vulkan.pdf
- Wronski, volumetric fog in Assassin's Creed IV: https://bartwronski.com/wp-content/uploads/2014/03/ac4_gdc.pdf
- Frostbite unified volumetrics (2015): https://www.ea.com/news/physically-based-unified-volumetric-rendering-in-frostbite
- Godot fog volumes: https://godotengine.org/article/fog-volumes-arrive-in-godot-4
- A physically based night sky: https://cseweb.ucsd.edu/~henrik/papers/nightsky/
- Ozone and the blue hour: https://en.wikipedia.org/wiki/Chappuis_absorption
- Bevy's exposure and auto exposure: https://doc.qu1x.dev/bevy_trackball/bevy/camera/struct.Exposure.html, https://doc.qu1x.dev/bevy_trackball/bevy/post_process/auto_exposure/struct.AutoExposure.html
- Genshin Impact's fog and grading (Unity Japan talk): https://docswell.com/s/UnityJapan/KWRPQ5-210617-unity-dojo20211mihoyozhenzhongyi

**Light, shadow and bounce**
- wgpu ray tracing status: https://wgpu.rs/doc/wgpu/documentation/extensions/ray_tracing/index.html
- Bevy 0.15 to 0.20 release notes: https://bevy.org/news/bevy-0-15/, https://bevy.org/news/bevy-0-19/, https://bevy.org/news/bevy-0-20/
- Bevy Solari: https://jms55.github.io/posts/2025-12-27-solari-bevy-0-18
- Godot's global illumination guide: https://docs.godotengine.org/it/4.5/_sources/tutorials/3d/global_illumination/introduction_to_global_illumination.rst.txt
- Godot's rendering priorities: https://godotengine.org/article/rendering-priorities-january-2024/
- DDGI (Majercik et al. 2019): https://cim.mcgill.ca/~derek/files/DDGI-lowres.pdf
- AMD Brixelizer GI (GDC 2024): https://gpuopen.com/gdc-presentations/2024/GDC2024_GI_with_AMD_FidelityFX_Brixelizer.pdf
- Lumen's performance guide: https://dev.epicgames.com/documentation/en-us/unreal-engine/lumen-performance-guide-for-unreal-engine
- Lumen and virtual shadow maps in Fortnite: https://www.unrealengine.com/tech-blog/lumen-brings-real-time-global-illumination-to-fortnite-battle-royale-chapter-4, https://www.unrealengine.com/tech-blog/virtual-shadow-maps-in-fortnite-battle-royale-chapter-4
- Radiance Cascades: https://arxiv.org/abs/2505.02041v1, https://odr.chalmers.se/items/9e42007a-bb60-440f-ba28-04410822758f/full
- Voxel cone tracing in The Tomorrow Children: https://cgvr.informatik.uni-bremen.de/theses/finishedtheses/VoxelConeTracing/TheTechnologyOfTomorrowsChildrenFinal.pdf
- Jimenez, GTAO (2016): https://www.iryoku.com/downloads/Practical-Realtime-Strategies-for-Accurate-Indirect-Occlusion.pdf
- Therrien, visibility bitmask (2023): https://arxiv.org/abs/2301.11376v2
- Microsoft, stable shadow maps: https://learn.microsoft.com/en-us/windows/win32/dxtecharts/common-techniques-to-improve-shadow-depth-maps
- Clustered shading (Olsson et al.): https://www.cse.chalmers.se/~uffe/clustered_shading_preprint.pdf
- NVIDIA PCSS: https://developer.download.nvidia.com/shaderlibrary/docs/shadow_PCSS.pdf
- Frostbite stochastic SSR: https://frostbite.com/frostbite/news/stochastic-screen-space-reflections
- TAA survey (Yang, Liu, Salvi 2020): https://research.nvidia.com/labs/rtr/publication/yang2020survey
- FSR 3 open source: https://www.gamingonlinux.com/2023/12/amd-fidelityfx-super-resolution-3-fsr3-is-now-open-source/page=1/

**The reference games**
- Campo Santo on Firewatch's sky: https://blog.camposanto.com/post/112703721804/this-blog-post-is-an-in-detail-explanation-of-a
- Skyrim weather (Creation Kit wiki): https://ck.uesp.net/wiki/Weather and the file format https://en.uesp.net/wiki/Skyrim_Mod:Mod_File_Format/WTHR
- Skyrim Special Edition's god rays (E3 2016): https://www.mcvuk.com/development/e3-2016-skyrim-remaster-has-volumetric-god-rays-dynamic-dof-new-shaders-and-more
- Cyberpunk 2077 RT Overdrive: https://www.nvidia.com/en-au/geforce/news/cyberpunk-2077-ray-tracing-overdrive-update-launches-april-11/
- Mad Max (2015): https://en.wikipedia.org/wiki/Mad_Max_(2015_video_game)
- Arma 3's visual upgrade: https://dev.arma3.com/post/oprep-visual-upgrade
- Minecraft's light: https://minecraft.wiki/w/Light-emitting_blocks; Bedrock deferred lighting: https://learn.microsoft.com/en-us/minecraft/creator/documents/deferredlighting
- Subnautica's water (forum): https://discussions.unity.com/t/subnautica-water-shaders-and-effects/1637842

**Names**
- Eclipse Foundation trademarks: https://www.eclipse.org/legal/trademarks/, https://trademarks.justia.com/862/46/eclipse-86246654.html
- The Star Wars: Eclipse engine: https://foro3d.com/en/2026/mayo/el-motor-de-eclipse-fotorrealismo-y-particulas-masivas-en-la-galaxia.html
- Eclipse Origins: https://www.rpgmaker.net/engines/eclipseor
- EmberGen: https://www.cgchannel.com/2023/03/jangafx-releases-embergen-1-0/
- EMBER (Tilde, Ember.js): https://trademarks.justia.com/owners/tilde-inc-2646260
- SOLSTICE (Mersive): https://trademarks.justia.com/884/36/solstice-88436079.html
- Aura by Ramen VR: https://www.tryaura.dev/documentation/vfx-agent
- Aura 2 for Unity: https://discussions.unity.com/t/best-artistic-tool-unity-awards-2019-aura-2-volumetric-lighting-fog/733761
- ASUS Aura Sync: https://www.asus.com/campaign/aura/us/index.php
