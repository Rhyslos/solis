# 0004. The editor's layout: Halberd's panels, Unreal's tabs, everything dockable

- **Status:** Accepted
- **Date:** 2026-10-21

## Problem

The editor needed a look and a layout before Phase 0 builds its shell. Three rounds of mockups were tried: modern and rounded (too basic), dense pro-tool layouts inspired by Aventail, Halberd and Unreal Engine, and finally mixes of Halberd and Unreal. The open question in the last round was where the library and console go: on the left suits an ultrawide screen, at the bottom suits a small one.

## Options

1. **A fixed layout** with the library at the bottom, or on the left.
2. **A layout picker** with a few fixed arrangements.
3. **Dockable panels:** every panel can be dragged anywhere, so any arrangement is possible, with a few presets to start from.

## Decision

Option 3, on the look chosen in the mockups ("Solis editor · layout picker" on the mockup canvas). The library-on-the-left and library-at-the-bottom layouts turned out to be nearly the same layout with panels in different places, which is exactly what docking solves.

**Top bar, one row:**
- Menus on the left (File, Edit, View, Entity, Build, Help), as in Halberd.
- **Document tabs in the centre,** as in Unreal: scenes, prefabs and code files open as tabs, each with an icon and ×, and + opens a new one.
- On the right: Play, Play with two players, Stop, Build ▾ and Build & Play.

**Panels:**
- Headers mix the two: Halberd's ▼ to collapse, and Unreal's rounded tabs with a small icon.
- **Every panel is dockable.** Drag a tab to any edge to split there, onto another panel to tab them together, or out of the window to float it (on a second monitor, say).
- The arrangement is saved per computer, so an ultrawide and a laptop each keep their own.
- **Window → Layouts** has presets: *Default* (library and console side by side under the viewport), *Wide screen* (library in a column on the left), *Small screen* (library and console as tabs in one bottom panel). People can save their own, and *Reset layout* always brings back the default.

**The default arrangement:**
- **Right, full height:** Scene and Layers (a tree, with search), above Properties. Properties shows the entity's components as category bars with their language (Lust, Rust or Lua), and X/Y/Z fields colour-coded.
- **Centre:** the viewport, with Viewport and Game tabs. Unreal-style buttons over the top left (≡, Perspective, Lit, Show) and snap values top right (grid, angle, camera speed); Halberd's tool strip ("Tool: Select · Place · Move W · Rotate R · Scale S · All T") and camera strip (Default, Orbit, Fly) over the view.
- **Bottom, under the viewport:** the library (folder tree, search, thumbnails with a coloured stripe per kind of asset) beside the console (Info, Warnings and Errors filters, search, timestamps).

**Theme:** dark neutral greys (the window #141414, panels #252525, headers #1a1a1a, fields #151515) with one accent colour, chosen in the settings. Petrol blue is the default; red, green and plum are the other choices, as in Aventail.

## Consequences

- **Docking comes from a library:** `egui_dock` (MIT) gives egui draggable tabs, splits and floating windows. Phase 0 builds the shell on it.
- **The earlier theme idea changes:** the design had "contrasting mid-greys with accent gradients". The chosen mockups are darker and use a single solid accent; gradients are dropped.
- Saved layouts need a format that survives new panels being added in later versions; an unknown or missing panel falls back to the default place.
- The two-player Play button needs the netcode (ROADMAP.md Phase 8); until then it's hidden.
