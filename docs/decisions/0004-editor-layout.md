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
- **On the left:** the menus (File, Edit, View, Entity, Build, Window, Help), as in Halberd, then the **document tabs** (Unreal's tab system): scenes, prefabs and code files open as tabs, each with an icon and ×, and + opens a new one.
- **In the centre, the tools as icons:** Select (Q, a mouse pointer), Move (W), Rotate (R), Scale (S) and Transform (T, all three at once). There is no separate Place tool: dragging from the library to place things is part of Select.
- **On the right:** Play, Play with two players, Stop, Build ▾ and Build & Play.
- No logo button and no layout buttons: the layout is set by docking (below).

**Panels:**
- **Shaped like Aventail:** flat panels split by thin dark lines; a title bar with Halberd's ▼ to collapse and plain-text tabs, the active one underlined in the accent colour; a small menu row under the title ("≡ Edit View Entity ⌕" for Scene, "≡ Mode Edit Units" for Properties). Buttons have small corners and a thin border, active ones filled with the accent. Fields and search boxes are square-cornered, numbers centred. The document tabs in the top bar use the same underlined style.
- **Every panel is dockable.** Drag a tab to any edge to split there, onto another panel to tab them together, or out of the window to float it (on a second monitor, say).
- The arrangement is saved per computer, so an ultrawide and a laptop each keep their own.
- **Window → Layouts** has presets: *Default* (library and console side by side under the viewport), *Wide screen* (library in a column on the left), *Small screen* (library and console as tabs in one bottom panel). People can save their own, and *Reset layout* always brings back the default.

**The default arrangement:**
- **Right, full height:** Scene and Layers (a tree), above Properties. Properties shows the entity's components under underlined headings with their language (Lust, Rust or Lua), labels and values in two columns, and X/Y/Z fields colour-coded.
- **Centre:** the viewport, with Viewport and Game tabs. Over the view, kept to the corners:
  - **Bottom left:** ≡, Perspective ▴, Lit ▴, Show ▴. The ≡ menu (opening upwards) has checkboxes to show or hide the other overlays, plus grid and camera speed settings.
  - **Bottom right:** grid, angle and camera speed.
  - **Top right:** frame rate and entity count.
- **The camera is Fly only:** one way to move around, no Orbit or Default modes to choose between. A key to frame the selection (F) covers jumping to an object.
- **Bottom, under the viewport:** the library (folder tree, search, thumbnails with a coloured stripe per kind of asset) beside the console (Info, Warnings and Errors filters, search, timestamps).

**Theme:** Aventail's dark neutral greys (the window #1b1b1b, panels #262626, title bars #222222, menu rows #2a2a2a, fields #1c1c1c, lines #161616) with one accent colour, chosen in the settings. Petrol blue is the default; red, green and plum are the other choices, as in Aventail.

## Consequences

- **Docking comes from a library:** `egui_dock` (MIT) gives egui draggable tabs, splits and floating windows. Phase 0 builds the shell on it.
- **It shares a look with Aventail**, so Solis and Aventail feel related, while Unreal's influence stays in how tabs work rather than how they look.
- **The earlier theme idea changes:** the design had "contrasting mid-greys with accent gradients". The chosen mockups are darker and use a single solid accent; gradients are dropped.
- Saved layouts need a format that survives new panels being added in later versions; an unknown or missing panel falls back to the default place.
- The two-player Play button needs the netcode (ROADMAP.md Phase 8); until then it's hidden.
