# Concepts

> The ideas this manual uses, in plain words. Other pages link here the first time they use a term.

## The editor and the engine

- **Engine:** the program a game runs on. It draws the picture, plays the sound, runs the physics and the game's own code. When a game ships, the engine goes with it.
- **Editor:** the window you build a game in. Games ship without it. In Solis the engine is written in Rust; game code can be written in Lust, Rust or Lua (arriving in a later phase).

## The window

- **Panel:** one area of the editor with one job, such as the Viewport or the Console. Every panel has a **tab** with its name.
- **Tab group:** panels that share one space. Click a tab to bring that panel to the front; the front tab is underlined in the accent colour.
- **Docking:** dragging a panel's tab to a new place. Drop it on the edge of another panel to put them side by side, or in the middle to make them one tab group. See [Interface tour](interface-tour.md).
- **Layout:** where all the panels are. Solis remembers yours, and has three ready-made ones (Window > Layouts).
- **Accent colour:** the one colour the editor uses to mark what's chosen or in front: the front tab's underline, the chosen tool, Play.

## The 3D view

- **Viewport:** the panel that shows the 3D world as the editor's camera sees it.
- **Z is up.** Solis measures height along Z. The ground is the flat X and Y directions.
- **Axes:** the three directions in 3D. Solis colours them the same everywhere: **X red, Y green, Z blue**. The red and green lines across the ground are the X and Y axes; they cross at the origin (the point 0, 0, 0).
- **Metres:** one unit in Solis is one metre. The grid's small squares are 1 metre across.
- **Grid:** lines on the ground to judge size and distance by. Brighter lines mark every 10 and 100 metres. The grid fades out with distance so it doesn't clutter the horizon.
- **Fly camera:** the way you move around the viewport: you steer the camera like a drone. See [Moving around the viewport](navigating-the-viewport.md).
- **Tool:** what a click in the viewport does: Select, Move, Rotate, Scale or Transform. The tools work once there are things in a scene to use them on (Phase 2).
