# 0114: The tools float in the view

- **Status:** Accepted (the project lead asked for it and approved the mock-ups, 2026-11-14)
- **Date:** 2026-11-14

## Problem

The G layout ([decision 0099](0099-the-g-layout.md)) put the area's tools in a toolbox: a 44 px column down the left edge of the viewport group, with Select, Move, Rotate, Scale and Transform, then four tools that aren't built (Pose bones, Motion path, Add key, Measure). Before the remake, the tools were icon buttons in one rounded frame floating at the foot of the view.

The project lead asked (2026-11-14): "Can you make the move, rotate, scale buttons the same as they were before the huge UI update. I liked when it floated inside the viewport, and then you can add buttons next to them for point, edge and faces too."

## Options

1. **Keep the column**, and put Points, Edges and Faces under the tools in it while a brush is edited.
2. **A bar floating at the foot of the view**, as before the remake, in the design's floating-panel look, with Points, Edges and Faces after it while a brush is edited (chosen; mock-ups approved).
3. **Both:** the column and a floating bar of the brush's modes.

## Decision

1. **The tools float in the view:** a rounded frame in the pills' look (their fill and border), centred at the foot of the view in use, with icon buttons for Select, Move, Rotate, Scale and Transform, each with its key small in its bottom-right corner (Q, W, R, S, T). The tool in use is filled with the accent and its icon is dark, the design's active look. Pointing at a button shows its name and key. As drawn in the approved mock-ups [`07-floating-tools-editing.png`](../design/tools-1/07-floating-tools-editing.png) and [`08-floating-tools-plain.png`](../design/tools-1/08-floating-tools-plain.png).
2. **While a brush's insides are edited**, a short line and then **Points, Edges and Faces** (1, 2, 3) follow, the mode in use lit; a click changes the mode as the key does. They show only while editing.
3. **It keeps clear** of the bottom-right pills (Lock, World, the camera's speed) and of the left of the view: moved left of the pills when they would meet; its buttons shrunk (to 20 px) to stay on their row; else above them, full size; and in a view narrower than the bar its buttons shrink to fit it. A strip of keys at the foot of the view (drawing a brush out, editing one, painting tiles) sits just above it. Every view the layouts give at the standard window has room for this; at the least window (960 × 600) some views are under 114 px tall, too short for three rows, and the bar overlaps the bottom-right pills there.
4. **This replaces the toolbox column of decision 0099 for the area's views.** The views are that much wider. The UI canvas keeps its own toolbox column, which is a different editor.
5. **Show ▾ › Tools** (it was "Toolbox") shows or hides the bar, as does File › Preferences › Viewport › Tools. The saved setting keeps its name (`tools` in the viewport's overlays), so a saved "off" stays off with no upgrader.
6. **With two or four views**, the bar is in the view in use only, as the old bar was and as the bottom-right pills are; it follows the view a click or the wheel makes the one in use.
7. **The four unbuilt tools go with the column, for now** (Pose bones, Motion path, Add key, Measure). They stay in the table of unfinished parts and the plan's inventory, drawn nowhere, and come back where their own milestones put them (3D 1, Shared 4, Shared 10; Measure isn't planned).
8. A tile map's painting tools stay on their own ribbon (HOME TOOLS · Tile map); while one is in use, no transform tool on the bar is lit, and clicking one puts the painting tool down. While placing (the Place shelf tab in front) there is no bar: the ribbon's build form has the tools.
9. No new keys or mouse actions: the bar's buttons are clicks for what the keys already do.

## Consequences

- `toolbox.rs` keeps only the UI canvas's column; the bar is `viewport_tool_bar.rs`, drawn with the view's pills.
- The editor's approved pictures and the guide's pictures were taken again; the manual describes the bar at the foot of the view.
- The design's specification keeps its toolbox lines, with a note pointing here.
