# 0014. Lights placed two ways, in Blender's units

- **Status:** Accepted
- **Date:** 2026-10-22

## Problem

Phase 1's fourth milestone brings the sun, point lights and spotlights, and shadows from the sun. But lights are things you place in a scene, and scenes and entities don't arrive until Phase 2. Where do lights come from until then, and what units are their strengths in?

## Options

1. **Lights from files only:** Blender exports its lamps into glTF (`KHR_lights_punctual`), so a model can bring them in. Nothing to place by hand until Phase 2.
2. **Lights placed by hand only**, with a small editor for them now.
3. **Both** (the project lead's choice): lamps come in with a model's file, and can be added by hand from the Entity menu. Both kinds are listed in the Scene panel and edited in Properties.

For units, glTF stores candela and lux; Blender, which most of Solis's users model in, uses watts and watts per square metre.

## Decision

Option 3, in Blender's units.

- **The sun** is one per scene: its direction (heading round the sky and height above the horizon), colour, strength (W/m², 3 by default) and whether it casts shadows. It's listed first in the Scene panel. Its settings are remembered between sessions; a sun in an imported file replaces them.
- **Lamps** (point lights and spotlights) come in with a file (removed along with its model), or from **Entity > Add point light** (above the point the camera looks at) and **Add spotlight** (at the camera, shining where it looks). **Move here** in Properties moves one the same way. Strength in watts; colour; an optional range; for spotlights the cone and its soft edge. They aren't saved until scenes arrive in Phase 2, when they become entities with a light component.
- **Units:** strengths are Blender's, so a 1000 W lamp lights a scene about as brightly in both (light falls off as P / 4π²d², as in Blender). glTF's candela and lux are converted with the 683 lumens per watt Blender's exporter uses.
- **Shadows:** the sun's shadow map (2048 × 2048, softened over nine texels) covers the models in view and the ground below them, which darkens too. Lamps don't cast shadows yet; they light through walls. Up to 16 lamps light the view at once, nearest the camera first.
- Each lamp shows in the viewport as a small cross in its colour (and a line the way a spotlight shines).

## Consequences

- Lamps added by hand are lost on closing until Phase 2; the manual and the Scene panel say so.
- Shadows from lamps, and more than 16 lamps, wait for the lighting choices of decision 0008 (baked, real-time and global light per project).
- The sun's shadow map is fitted to every model, so a very large scene gets blurrier shadows; cascades (several maps, sharper near the camera) can follow when scenes get big.
