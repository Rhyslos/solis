# Lights and shadows: the sun, point lights and spotlights

> Light your models with a sun that casts shadows, and with lamps you place yourself or bring in from Blender.

## When to use it
Light decides how a model reads: which side is bright, where its shadows fall, what mood the scene has. Use it to check how a model looks in daylight, under a warm lamp, or in a dark room lit by one torch.

- The **sun** ([concepts](concepts.md#light)) lights everything from one direction, like daylight, and casts shadows. A new scene has one.
- A **point light** shines every way from one spot, like a bare bulb or a candle.
- A **spotlight** shines a cone, like a torch, a car headlight or a stage light.

Everything also gets soft **sky light**, so shadows are never pitch black.

Every light is an [entity](concepts.md#the-scene) with a **Light** [component](concepts.md#the-scene); where it is and which way it points come from its **Transform**. Until saving scenes arrives later in Phase 2, lights aren't kept when you close Solis, and the sun starts as it was.

## How to use it

**Change the sun**
1. In the **Scene** panel, click **Sun**.
2. In **Properties**, under **Transform**, drag the first **Rotation** number (X) to tilt the sun: 0° shines straight down (noon), 90° skims the horizon (sunrise), so a bigger tilt gives longer shadows. Drag the third number (Z) to turn it around the sky.
3. Under **Light**, click the **Colour** swatch for a warmer or cooler light, and drag **Strength** to brighten or dim it. Untick **Shadows** to turn them off.

**Add a lamp**
1. Fly the camera to where the light should be ([Moving around the viewport](navigating-the-viewport.md)).
2. Choose **Entity > Add point light** to place a bulb 2 m above the point the camera looks at, or **Entity > Add spotlight** to place a spotlight at the camera, shining where the camera looks (like holding a torch).
3. The lamp appears as a small cross in its colour (a spotlight also has a line the way it shines). It's listed in the Scene panel and selected, with its settings in Properties.
4. To move it, fly somewhere else and click **Move here** in Properties. **Delete** removes it.

**Bring lamps from Blender:** export with **Punctual Lights** ticked (under Include in Blender's glTF export), then [import the model](importing-models.md). Its lamps come in as entities under the model in the Scene panel, and are deleted with it. A sun in the file sets the scene's sun.

## Settings

**Transform** (every light has one):

| Setting | What it does |
|---|---|
| Position | Where the light is, in metres (X, Y, Z). Drag a number to move it. A sun's position doesn't matter: it's infinitely far away. |
| Rotation | Which way it shines, in degrees. With all three at 0 a light shines straight down. **X** tilts it (90° is level with the ground), **Z** turns it round. Point lights shine every way, so their rotation doesn't matter. |
| Scale | Not used by lights. |

**Light:**

| Setting | What it does |
|---|---|
| Kind | **Sun**, **Point** or **Spot**. |
| Colour | The light's colour. Slightly warm white is daylight; orange is evening. |
| Strength | Point lights and spotlights: in watts, as in Blender, so numbers carry over: 1000 W lights a room, 100 W is a small lamp close by. The sun: in watts per square metre; 3 is the editor's clear day, 0 turns it off for a night scene lit by lamps. |
| Range | 0: the light fades on forever, getting weaker with distance, as real light does. More: it stops at this many metres. Games use ranges to keep many lamps cheap. |
| Cone (spotlights) | How wide the light reaches, in degrees from the middle of the cone. |
| Soft edge (spotlights) | How gradually the light fades at the cone's edge: 0° for a hard circle, more for a soft one. |
| Shadows | Whether it casts shadows. For now only the sun does. |

## Tips and common problems
- **Problem:** a lamp lights through a wall. **Why:** for now only the sun casts shadows; lamps don't. Shadows from lamps come with the lighting choices of [decision 0008](../decisions/0008-lighting-is-a-project-choice.md).
- **Problem:** a lamp seems to have no effect. **Why:** it may be too weak for its distance: light gets four times weaker each time the distance doubles. **Fix:** raise its strength, or move it closer.
- **Problem:** everything is washed out near a lamp. **Why:** the lamp is very strong for its distance. **Fix:** lower its strength, or the exposure under [Environment](sky-fog-and-exposure.md).
- **Problem:** lamps far from the camera stop lighting. **Why:** up to 16 lamps light the view at once, the ones nearest the camera.
- **Problem:** shadows look blurry in a big scene. **Why:** one shadow map covers every model, so in a large scene each of its pixels covers more ground.
- **Problem:** I added lamps, closed Solis, and they're gone. **Why:** scenes can't be saved until later in Phase 2.
- **Problem:** I deleted the sun and everything went dark. **Why:** the sun was the only light. **Fix:** add lamps, or add an entity and give it a Light of kind Sun (adding components arrives later in Phase 2; for now, import a model with a sun, or start Solis again).

## Related
- [Looks](looks.md): both the standard and stylized looks are lit; the unlit look ignores light.
- [Sky, fog and exposure](sky-fog-and-exposure.md): a daylight sky that follows the sun, and how bright the picture is.
- [Importing models](importing-models.md)
- [Entities and components](entities.md): how lights, models and the environment are built.
- Concepts: [sun, lamp, watts, range, shadow, sky light](concepts.md#light)
