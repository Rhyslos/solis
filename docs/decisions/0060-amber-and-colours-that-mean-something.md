# 0060. Amber by default, and colours that mean something keep theirs

- **Status:** Accepted (changes the standard accent of 0004)
- **Date:** 2026-10-28

## Problem

The editor had one accent colour (petrol blue unless picked otherwise in Window → Accent colour), used for everything coloured: the chosen tool, the front tab, the selected row, Play, Build & Play, the console's level filters, and the icons of models, cameras and the environment in the Area panel. Trying the accents, the project lead wanted amber as the standard. With amber, the console's warnings, its information and its errors all turned the same amber, so a warning looked like something selected, and Play stopped looking like "go".

## Options

1. **Amber for everything coloured,** as before with blue. Simple, but colours lose their meaning.
2. **Amber marks what's chosen; colours that mean something keep their own (chosen).**

## Decision

Option 2, chosen by the project lead:

- **Amber is the standard accent.** It marks the chosen tool, the front tab, the selected row, Pause while paused, the panel icons, links and the drop zone when docking. Anyone who already chose an accent keeps it.
- **Play and Build & Play are always blue** (Solis's petrol blue).
- **Stop is red** while the game runs (it already was).
- **The console's levels have their own colours:** information grey, warnings amber, errors red.
- **Things in the Area panel are coloured by kind, as in the Library:** models blue, a placed assembly yellow, the environment teal, cameras grey, the sun warm yellow, and each lamp its own light's colour. One list of kind colours (`theme::kind`) serves both panels.
- Later colours that mean something follow the same rule: the Mixer's mute red, solo yellow and meters green to red; recording red; axes and colliders as they are.

## Consequences

- The accent menu lists Amber first; Petrol blue moves to its place on the colour wheel.
- Tests check that white text reads on blue and on each level's colour, and that none of them is the standard accent's.
- A new kind of thing in the Library gets one colour in `theme::kind`, and the Area panel uses the same one.
