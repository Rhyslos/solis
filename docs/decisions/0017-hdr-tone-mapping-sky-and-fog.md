# 0017. HDR with AgX tone mapping, a daylight sky, and fog

- **Status:** Accepted
- **Date:** 2026-10-22

## Problem

Phase 1's last milestone, "the finished picture": until now every colour was cut off at white as it was drawn. A sunlit white wall, a lamp's hot spot and a glowing stripe all flattened to the same white. There was no sky beyond the editor's grey backdrop, and nothing faded with distance. Four choices had to be made:

- how to keep light brighter than white;
- how to show it on a screen;
- what sky to offer;
- what fog.

## Options

**Tone mapping** (how bright light is squeezed onto a screen):
1. **AgX:** Blender's default since 4.0. Highlights roll off softly towards white, as on film, with natural colours.
2. **ACES:** the film curve many games use. Punchier, more saturated.
3. **None** ("Standard" in Blender): cut off at white, as before.

**The editor's backdrop and lines:** run them through tone mapping like everything else (they'd shift away from the mockup's colours), or leave them out.

## Decision

- **HDR:** the world is drawn into a 16-bit floating-point image (`Rgba16Float`), with multisampling as before. A last pass applies exposure and tone mapping and encodes to sRGB into the 8-bit image egui shows. Bloom and colour grading (Phase 9) will slot in there.
- **Tone mapping:** all three are offered, and **AgX is the default**, so a scene looks as it did in Blender. Exposure is in stops (EV), as in cameras and Blender.
- **The editor's own backdrop and lines are never tone mapped.** The HDR image's alpha channel marks which pixels belong to the lit world, and the last pass leaves the rest exactly as the theme has them. Lines blend so they look as they did when blended in sRGB, matching the mockup.
- **Skies:** the **editor backdrop** (the default, calm and neutral for working), or a **daylight** sky that follows the sun: blue by day, warm at sunrise and sunset, dark at night, with the sun's disc and a glow around it. The daylight sky's colours also light the shadows (the sky light of decision 0008), so a sunset scene has warm light and cool shadows.
- **Fog:** distance fog, set by the distance at which half of something is hidden, taking the sky's horizon colour unless given its own. Things deep in fog over the editor backdrop fade into it exactly.
- The **unlit look** shows colours as painted: never exposed or tone mapped.
- These settings sit under **Environment** in the Scene panel, and are remembered between sessions until scenes arrive (Phase 2), when they become part of the scene.

## Consequences

- AgX makes pure white surfaces show as a soft light grey, as in Blender. People used to "cut off at white" can pick Standard.
- One more full-screen pass per frame, and a 16-bit image instead of an 8-bit one. Both are small next to the scale target.
- The daylight sky is a simple painted gradient, not a physical atmosphere; it reflects in metal but doesn't scatter light around. A physical sky, and height fog, can come later if games need them.
