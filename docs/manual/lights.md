# Lights and shadows: the sun, point lights and spotlights

> Light your models with a sun that casts shadows, and with lamps you place yourself or bring in from Blender.

## When to use it
Light decides how a model reads: which side is bright, where its shadows fall, what mood the scene has. Use it to check how a model looks in daylight, under a warm lamp, or in a dark room lit by one torch.

- The **sun** ([concepts](concepts.md#light)) lights everything from one direction, like daylight, and casts shadows. There is always one.
- A **point light** shines every way from one spot, like a bare bulb or a candle.
- A **spotlight** shines a cone, like a torch, a car headlight or a stage light.

Everything also gets soft **sky light**, so shadows are never pitch black.

Until scenes arrive (Phase 2), lamps you add by hand aren't saved when you close Solis. The sun's settings are remembered.

## How to use it

**Change the sun**
1. In the **Scene** panel, click **Sun** (always at the top of the list).
2. In **Properties**, drag **Heading** to turn the sun around the sky, and **Height** to raise or lower it. A low sun gives long evening shadows; 90° is noon, straight overhead.
3. Click the **Colour** swatch for a warmer or cooler light, and drag **Strength** to brighten or dim it. Untick **Shadows** to turn them off.

**Add a lamp**
1. Fly the camera to where the light should be ([Moving around the viewport](navigating-the-viewport.md)).
2. Choose **Entity > Add point light** to place a bulb 2 m above the point the camera looks at, or **Entity > Add spotlight** to place a spotlight at the camera, shining where the camera looks (like holding a torch).
3. The lamp appears as a small cross in its colour (a spotlight also has a line the way it shines). It's listed in the Scene panel and selected, with its settings in Properties.
4. To move it, fly somewhere else and click **Move here** in Properties. **Delete** removes it.

**Bring lamps from Blender:** export with **Punctual Lights** ticked (under Include in Blender's glTF export), then [import the model](importing-models.md). Its lamps come in with it and are removed with it. A sun in the file becomes the scene's sun.

## Settings

The sun:

| Setting | What it does |
|---|---|
| Heading | Which way round the sky the sun is: 0° north (+Y, the green axis), 90° east (+X, the red axis), 180° south. Shadows point the other way. |
| Height | How high above the horizon, from 0° (sunrise, very long shadows) to 90° (noon, short shadows). |
| Colour | The light's colour. Slightly warm white is daylight; orange is evening. |
| Strength | In watts per square metre, as in Blender. 3 is the editor's clear day; 0 turns the sun off, for a night scene lit by lamps. |
| Shadows | Whether the sun casts shadows, on models and on the ground. |

Lamps:

| Setting | What it does |
|---|---|
| Name | What the Scene panel calls it. |
| Colour | The light's colour. |
| Strength | In watts, as in Blender, so numbers carry over: 1000 W lights a room, 100 W is a small lamp close by. |
| Range | Off: the light fades on forever, getting weaker with distance, as real light does. On: it stops at this many metres. Games use ranges to keep many lamps cheap. |
| Cone (spotlights) | How wide the light reaches, in degrees from the middle of the cone. |
| Soft edge (spotlights) | How gradually the light fades at the cone's edge: 0° for a hard circle, more for a soft one. |
| Position | Where the lamp is, in metres (X, Y, Z). Drag a number to move it. |

## Tips and common problems
- **Problem:** a lamp lights through a wall. **Why:** for now only the sun casts shadows; lamps don't. Shadows from lamps come with the lighting choices of [decision 0008](../decisions/0008-lighting-is-a-project-choice.md).
- **Problem:** a lamp seems to have no effect. **Why:** it may be too weak for its distance: light gets four times weaker each time the distance doubles. **Fix:** raise its strength, or move it closer.
- **Problem:** everything is washed out white near a lamp. **Why:** colours are cut off at white until tonemapping arrives (milestone 5). **Fix:** lower its strength.
- **Problem:** lamps far from the camera stop lighting. **Why:** up to 16 lamps light the view at once, the ones nearest the camera.
- **Problem:** shadows look blurry in a big scene. **Why:** one shadow map covers every model, so in a large scene each of its pixels covers more ground.
- **Problem:** I added lamps, closed Solis, and they're gone. **Why:** lamps aren't saved until scenes arrive in Phase 2.

## Related
- [Looks](looks.md): both the standard and stylized looks are lit; the unlit look ignores light.
- [Importing models](importing-models.md)
- Concepts: [sun, lamp, watts, range, shadow, sky light](concepts.md#light)
