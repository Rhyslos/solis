# 11 · Shared 9: weather and the finished picture

ROADMAP.md (from Phase 13; both for 2D and 3D): **the finished picture, part two** (bloom, white balance, colour grading); **weather and moods** (weather files keyed by time of day, blended, with one overcast control, bringing their effects, rain and snow, from Shared 8). **Gate:** a 2D scene and a 3D scene each go from a clear morning to a rainy evening, blended smoothly, with bloom on the lamps.

Two milestones, possibly three (bloom and white balance; grading; weather). The detail is in `docs/plans/light.md` L5 and L7 and `docs/plans/effects.md` (post chain, bloom, grading).

## What exists

- **The last pass** (`post_fs` in `viewport.wgsl`): exposure (fixed), tone mapping (AgX default, ACES, none; decision 0017), sRGB; the image's alpha marks the lit world so the editor's backdrop and lines are never tone mapped.
- **Day and night** (decision 0092): Environment colour stops by hour, `World:timeOfDay` as game state; Light sprites; lamps Only at night.
- **The pixel stage** for 2D (drawn small, scaled up); sprites write alpha 0 (colours exactly as painted, never lit or tone mapped unless Light sprites).
- **The sky:** the painted gradient that follows the sun; distance fog.
- Shared 8 provides rain and snow effects; Shared 7 a wetness input waiting for a source.

## M1. Bloom and white balance (the post chain)

**Techniques for bloom:**
| Way | Notes |
| --- | --- |
| **Downsample/upsample pyramid without threshold** (Jimenez, SIGGRAPH 2014: 13-tap down filter, Karis average on the first step to stop single bright pixels flickering, tent-filter up, blended gently) | The modern standard; energy-conserving-ish, stable, cheap (≈0.3–0.5 ms at 1080p) |
| **Threshold + Gaussian blur** | Older; a hard threshold makes things pop in and out |
| **FFT convolution bloom** (lens-shaped kernels) | Beautiful streaks, much costlier; a later "cinematic" option |

**Recommendation:** Jimenez's pyramid in fragment passes (works on GL), as both plans say, with the alpha mask respected and a **stylized bloom** option that bands the glow. For 2D, bloom runs at the game's resolution before the pixel stage scales up, so the glow stays on the pixel grid (light.md L20).

**White balance:** "pick what white becomes" (a colour picker on a neutral surface), temperature and tint, applied in a linear colour space before tone mapping (a von Kries / chromatic adaptation matrix, e.g. the Bradford transform). **Night:** an optional Purkinje shift (bluer, less colourful in the dark) driven by exposure, which comes with 3D 2's automatic exposure; until then driven by the time of day's darkness.

**Tone mapping:** AgX stays; add a neutral base for heavy grading (the light plan names "Tony McMapface", MIT-licensed lookup-table tonemapper: check its licence before adding its table).

**Lens extras** (off by default): lens flare, film grain, chromatic aberration at the edges, heat haze.

*Size: medium.*

## M2. Colour grading

Sliders (warmth, tint, contrast, saturation, shadows/midtones/highlights lift-gamma-gain) baked on the CPU into a **32³ 3D lookup table** when they change, plus an optional **LUT image** (the common 1024 × 32 strip) folded into the same table; one lookup per pixel after tone mapping. "Make a plain LUT…" writes the neutral strip to grade in a paint program. **Palette limiting** for pixel games (2D 2 built a pass for it) becomes a grading-table option (nearest palette colour, optional ordered dithering). Settings live on the Environment and can be keyed by weather (M3). Fuzz the LUT image reader (rule 9). *Size: small to medium.*

## M3. Weather and moods

**What it gives:** a weather (clear, golden, overcast, storm, desert, night) holds, for sunrise, day, sunset and night: the sky's tint, fog colours, the sun's and ambient light's strength and colour, the grade, an exposure bias, how cloudy, how wet, and its effects (rain, snow, fog banks). Changing weather blends over a set time. One **overcast** control moves sky, clouds, shadow softness and light together. A **mood** holds a moment of the day regardless of the clock.

**Design:**
- **A weather file** (`<name>.weather.lst`, decision 0081; the light plan wrote `.solweather`): blocks per time-of-day key with the values above; the Environment picks one; `World:setWeather(Weather.Storm, seconds)` (a project state, rule 3) blends. Every value is a number or colour, so blending is linear (colours in linear space; tone-mapping-sensitive values like exposure blended in stops).
- **Is the weather game state?** Light never changes the game (light.md's rule), but **rain can**: crops watered by rain, slippery paths. **Recommended:** the *chosen* weather and the blend's progress are game state (saved, checksummed, set by game code, like the time of day), and everything drawn from them is output. Game code asks `World:weather()` and `World:wetness()`. Random weather changes are the game's own code (with the game's seed), not the engine's.
- **Effects:** the weather names Shared 8 effects that follow the camera (rain around the view, not over the whole world), their intensity from the blend; in 2D the pixel set's rain and snow.
- **Wetness:** a value (0 to 1, rising while it rains, drying over time) on the Environment feeding Shared 7's wetness input (darker, glossier surfaces); puddles and real wet reflections are 3D 4.
- **Before the physical sky:** 3D 2 brings Hillaire's sky later. Until then, a weather's sky values drive the painted sky and day colours; when 3D 2 lands, the same values become the tint and fog ramp over the physical sky (light.md L3's "art direction over the physics"). Design the file so both read it.
- **Ready-made weathers:** clear, golden, overcast, storm, desert, night.

**Editor UI (mock-up first):** a Weather tab: four columns (sunrise, day, sunset, night) of values, a preview strip across 24 hours, blend-time setting; the viewport's preview time slider (exists) shows any hour under any weather. No new keys.

**Tests:** a blend halfway gives halfway values; saving mid-blend and loading resumes; drawing never changes checksums; weather choice does; fuzz the file. **Scale case:** rain in the benchmark at the target. *Size: medium.*

## Dependencies and order

- Needs Shared 8 (rain, snow), Shared 7 (wetness input), decision 0092's clock.
- Feeds 3D 2 (sky tint from weather), 3D 3 (clouds' amount from weather), 3D 4 (wet reflections, fog volumes).
- Automatic exposure is in 3D 2 (light.md L2); bloom and grading don't need it.

## Risks and questions for the project lead

1. **Weather as game state** (so rain can water crops): **recommended** the chosen weather and wetness are game state; alternative: decoration only.
2. **File name:** **recommended** `<name>.weather.lst`; alternative: `.solweather`.
3. **Bloom on by default** (effects plan's answer: on, gentle, everywhere): keep.
4. **Lens extras:** off by default; which ones at all? Recommended the four listed.

## Crates

None needed.

## Size

Phase: medium.

## Sources

- [Jimenez, Next generation post processing in Call of Duty: Advanced Warfare (SIGGRAPH 2014)](https://www.iryoku.com/next-generation-post-processing-in-call-of-duty-advanced-warfare/); [LearnOpenGL, Physically based bloom](https://learnopengl.com/Guest-Articles/2022/Phys.-Based-Bloom).
- Chromatic adaptation: [Bruce Lindbloom, chromatic adaptation](http://www.brucelindbloom.com/index.html?Eqn_ChromAdapt.html).
- LUT strips: [Unity, User LUT (format)](https://docs.unity3d.com/2018.3/Documentation/Manual/PostProcessing-UserLut.html).
- Tone mapping: [Tony McMapface (h3r2tic)](https://github.com/h3r2tic/tony-mc-mapface); [AgX](https://github.com/sobotka/AgX).
- `docs/plans/light.md` L5, L7, L20; `docs/plans/effects.md` post-processing.
