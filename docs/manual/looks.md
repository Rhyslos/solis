# Looks: standard, stylized and unlit

> Choose how models are shaded: realistic, cel-shaded with outlines, or plain colours.

## When to use it
- **Standard** shows a model as most engines and Substance Painter would: shiny metal, rough plastic, bumps from its normal texture. Use it to check a model's materials.
- **Stylized** is a cartoon or "cel-shaded" look, like many stylized games: light in a few flat bands, crisp highlights and dark outlines. It's the look Solis is built for.
- **Unlit** shows colours and textures exactly as painted, with no light at all. Use it to check a texture.

For now the look applies to every model in the view. Once materials become assets (Phase 2), each material can have its own.

## How to use it
1. At the bottom left of the viewport, click the look button (it shows the look in use, such as **Standard ▴**).
2. Choose **Standard**, **Stylized** or **Unlit**. The menu stays open, so you can compare.
3. For the stylized look, choose how many **Bands of light** (2, 3 or 4), and whether to draw **Outlines**.
4. Click anywhere else to close the menu. Solis remembers your choice.

To try the looks without a model of your own, choose **Help > Add the sample area**: a checker cube, a row of plastic spheres from glossy to rough, a row of metal ones, and a wall with grooves and a glowing stripe.

## Settings
| Setting | What it does |
|---|---|
| Bands of light | How many steps the stylized light goes in, from shadow to full light. 2 is bold and graphic; 4 is softer. 3 is a good start. |
| Outlines | Dark edges around models in the stylized look, about 1.5 points wide. |

What the standard look reads from a model's [material](concepts.md#models):

| Material setting | What it does |
|---|---|
| Colour and colour texture | The surface's colour. |
| Metallic | 0 for paint, plastic and wood; 1 for bare metal, which reflects its surroundings tinted by its colour. |
| Roughness | 0 mirror-smooth with a tiny sharp highlight; 1 chalky with no shine. |
| Normal texture | Small bumps and grooves that catch the light, without extra triangles. |
| Occlusion texture | Darkens creases and corners. |
| Glow (emissive) | Light the surface gives off itself: screens, lamps, engines. It glows in every look. |

## Tips and common problems
- **Problem:** metal looks dark or dull. **Why:** metal shows what it reflects, and for now it reflects only the sky's soft light and the highlights of the sun and lamps. **Fix:** add a lamp near it ([Lights and shadows](lights.md)), or switch to the daylight sky ([Sky, fog and exposure](sky-fog-and-exposure.md)), which metal reflects.
- **Problem:** white surfaces look light grey rather than pure white. **Why:** the AgX tone mapping, as in Blender, keeps room above white for brighter light. **Fix:** raise the exposure, or choose Standard tone mapping ([Sky, fog and exposure](sky-fog-and-exposure.md)).
- **Problem:** no outline on part of a model. **Why:** outlines are drawn only on solid, one-sided surfaces; see-through and double-sided ones are left without.
- **Problem:** everything looks the same in Unlit. **Why:** that's the point: no light, just colour.

## Related
- [Importing models](importing-models.md)
- [Concepts](concepts.md): material, texture, metallic, roughness, normal texture.
