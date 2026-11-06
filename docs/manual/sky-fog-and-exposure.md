# Sky, fog and exposure

> Choose what's behind your models, add fog for distance, and set how bright the picture is.

## When to use it
- **Sky:** the **editor backdrop** is a calm grey that keeps your attention on the model; it's the default for working. The **daylight sky** shows your model outdoors: blue at noon, orange at sunset, dark at night, always matching the [sun](lights.md). Its light also colours the shadows, so a sunset gives warm light and cool shadows.
- **Fog** makes distant things fade into the sky, as haze or mist does. It gives large areas a sense of depth, and is a classic way to hide where a level ends.
- **Exposure** and **tone mapping** decide how bright light is shown on your screen ([concepts](concepts.md#light)). Solis works out light brighter than white, as the real world is, and squeezes it onto the screen the way Blender does. Raise the exposure for a darker area, lower it if things look washed out.

## How to use it
1. In the **Area** panel, click **Environment**.
2. In **Properties**, choose the **Sky**: **Backdrop** or **Daylight**.
3. For fog, tick **Fog**, then set **Half hidden at**: the distance at which half of a thing disappears into the fog.
4. Adjust **Exposure** and choose the **Tone mapping**.

**Day and night** (for a game with a clock: a farm, a town):

5. Tick **Day and night**. Set how long **A day lasts** (in minutes of play: 24 is an hour of the day a minute) and the hour it **Starts at**.
6. **Colours** shows the day as a strip, from midnight to midnight. Each colour has an hour: the world is tinted by them, smoothly from one to the next. Click a colour to change it, drag its hour (or type `19:30`), **+ Add a colour** for another (it goes in the longest stretch of the day), and **Remove** to take one out.
7. Tick **Preview time** and drag it to see the area at any hour in the viewport: a dusk tint, the street lamps on. The preview isn't saved, and the game always starts at **Starts at**.

The time of day runs with the game's time, so a paused game's day stands still, and save games keep it. Game code reads it and sets it ([Game code](game-code.md)): `World:timeOfDay()` gives the hour (19.5 is half past seven), and `World:setTimeOfDay(6)` after a night's sleep moves the day on to six in the morning.

**Light sprites** lights a 2D game's sprites and tile maps with the day's colour and the [lamps](lights.md): a lamp shines on the ground beneath it, in a pool that's brightest right under it. The light is worked out for each of the game's own (big) pixels, so it stays crisp. **Light steps** draws it in bands (3 is a good stylized look; 0 is smooth). New areas have Light sprites on; areas made before it have it off, so they look as they did. With no lamps and day and night off, a lit sprite looks exactly as drawn.

These settings belong to the area (the Environment is an [entity](concepts.md#the-area) like any other), so they're [saved with it](saving-areas.md); a new area starts with the usual ones. With the daylight sky, try selecting **Sun** and dragging its **Rotation** X from 30° up to 90° and beyond to watch the day turn to sunset and night.

## Settings

| Setting | What it does |
|---|---|
| Sky | **Backdrop:** the calm grey you work against, with a neutral soft light. **Daylight:** a sky that follows the sun, with the sun's disc and a glow around it. |
| Sky strength (daylight) | How bright the sky and its light in the shadows are. 1 is natural; lower gives darker, more dramatic shadows. |
| Fog | Turns fog on. |
| Half hidden at | In metres. At this distance half of a thing is hidden; at twice the distance, three quarters. 50 m is thick mist, 500 m light haze. |
| Fog matches sky | Takes the horizon's colour, so things fade into the sky. Untick it to use **Fog colour**, such as green for a swamp. |
| Fog colour | The fog's own colour, when it doesn't match the sky. |
| Exposure | In stops (EV), as on a camera: +1 is twice as bright, -1 half. 0 is a good start. |
| Tone mapping | **AgX** (default, as in Blender): bright light rolls off softly towards white, with natural colours. **ACES**: the film look many games use, with more contrast and colour. **Standard**: no mapping; anything brighter than white is cut off. |
| Day and night | The day's colours tint the world as the game's time of day runs: the sun and the sky's light in 3D, and lit sprites in 2D. |
| A day lasts | How long a whole day lasts, in minutes of play. |
| Starts at | The hour the game starts at (8 is eight in the morning). |
| Colours | The day's colours at their hours (up to 8). A lamp with **Only at night** is on while the tint is darker than half the brightest colour: from dusk to dawn, whatever the colours. |
| Preview time | Shows the area at another hour in the viewport. The editor's alone: never saved. |
| Light sprites | Sprites and tile maps are lit by the day's colour and by lamps. |
| Light steps | 0 for smooth light on sprites; 2 to 8 for bands of light. |

## Tips and common problems
- **Problem:** white surfaces look light grey. **Why:** AgX keeps room above white for brighter light, as in Blender. **Fix:** raise the exposure a little, or choose Standard.
- **Problem:** the daylight sky is black. **Why:** the sun is below the horizon (night), or its strength is 0. **Fix:** lower the sun's Rotation X below 90°, or raise its Strength.
- **Problem:** everything disappears into the fog. **Why:** the half-hidden distance is short compared with the area. **Fix:** raise it.
- **Problem:** the unlit look ignores exposure. **Why:** unlit shows colours exactly as painted, by design.
- **Problem:** at night my sprites are too dark to see. **Why:** the night colour is dark on purpose, and lit sprites take it. **Fix:** make the night colour lighter (a deep blue rather than near black), or add lamps.
- **Problem:** a lamp lights nothing on my sprites. **Why:** **Light sprites** is off (an area made before it), or the lamp is too weak or too high. **Fix:** tick Light sprites; a lamp of 100 to 300 W, 1 to 2 m up, makes a good pool.
- **Problem:** the colours jump instead of changing smoothly. **Why:** two colours sit close together in the day. **Fix:** move them apart, or take one out.
- The grid and the editor backdrop are never changed by exposure or tone mapping, so the editor stays easy on the eyes whatever the area looks like.

## Related
- [Lights and shadows](lights.md): the sun, which the daylight sky follows, and lamps that shine only at night.
- [Sprites](sprites.md): what Light sprites lights.
- [Looks](looks.md)
- Concepts: [HDR, exposure, tone mapping, fog](concepts.md#light)
