# Sky, fog and exposure

> Choose what's behind your models, add fog for distance, and set how bright the picture is.

## When to use it
- **Sky:** the **editor backdrop** is a calm grey that keeps your attention on the model; it's the default for working. The **daylight sky** shows your model outdoors: blue at noon, orange at sunset, dark at night, always matching the [sun](lights.md). Its light also colours the shadows, so a sunset gives warm light and cool shadows.
- **Fog** makes distant things fade into the sky, as haze or mist does. It gives large scenes a sense of depth, and is a classic way to hide where a level ends.
- **Exposure** and **tone mapping** decide how bright light is shown on your screen ([concepts](concepts.md#light)). Solis works out light brighter than white, as the real world is, and squeezes it onto the screen the way Blender does. Raise the exposure for a darker scene, lower it if things look washed out.

## How to use it
1. In the **Scene** panel, click **Environment** (under Sun).
2. In **Properties**, choose the **Sky**: **Editor backdrop** or **Daylight**.
3. For fog, tick **Fog: On**, then set **Half hidden at**: the distance at which half of a thing disappears into the fog.
4. Adjust **Exposure** and choose the **Tone mapping** under Picture.

Solis remembers these settings between sessions. With the daylight sky, try dragging the sun's **Height** (select **Sun**) from 60° down to 0° and below to watch the day turn to sunset and night.

## Settings

| Setting | What it does |
|---|---|
| Sky | **Editor backdrop:** the calm grey you work against, with a neutral soft light. **Daylight:** a sky that follows the sun, with the sun's disc and a glow around it. |
| Strength (daylight) | How bright the sky and its light in the shadows are. 1 is natural; lower gives darker, more dramatic shadows. |
| Fog | Turns fog on. |
| Half hidden at | In metres. At this distance half of a thing is hidden; at twice the distance, three quarters. 50 m is thick mist, 500 m light haze. |
| Colour | **Match the sky** takes the horizon's colour, so things fade into the sky. Untick it to choose a colour of your own, such as green for a swamp. |
| Exposure | In stops (EV), as on a camera: +1 is twice as bright, -1 half. 0 is a good start. |
| Tone mapping | **AgX** (default, as in Blender): bright light rolls off softly towards white, with natural colours. **ACES**: the film look many games use, with more contrast and colour. **Standard**: no mapping; anything brighter than white is cut off. |

## Tips and common problems
- **Problem:** white surfaces look light grey. **Why:** AgX keeps room above white for brighter light, as in Blender. **Fix:** raise the exposure a little, or choose Standard.
- **Problem:** the daylight sky is black. **Why:** the sun is below the horizon (night), or its strength is 0. **Fix:** raise the sun's Height or Strength.
- **Problem:** everything disappears into the fog. **Why:** the half-hidden distance is short compared with the scene. **Fix:** raise it.
- **Problem:** the unlit look ignores exposure. **Why:** unlit shows colours exactly as painted, by design.
- The grid and the editor backdrop are never changed by exposure or tone mapping, so the editor stays easy on the eyes whatever the scene looks like.

## Related
- [Lights and shadows](lights.md): the sun, which the daylight sky follows.
- [Looks](looks.md)
- Concepts: [HDR, exposure, tone mapping, fog](concepts.md#light)
