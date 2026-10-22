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
## Models

- **Model:** a 3D object made in a modelling program (Blender, Aventail): its shape, and how its surface looks. See [Importing models](importing-models.md).
- **glTF:** the standard file format for game models, in two forms: `.glb` (everything in one file) and `.gltf` (a text file, with its data and textures in files beside it).
- **Mesh:** a model's shape, made of triangles.
- **Triangle:** the smallest piece of a 3D shape. Graphics cards draw everything as triangles; a game character has a few thousand to tens of thousands.
- **Material:** how a surface looks: its colour, how shiny, how see-through.
- **Texture:** an image wrapped around a model, giving it detail (paint, wood grain, panels).
- **Metallic and roughness:** the two numbers that decide how a surface shines. Metal reflects its surroundings; rough surfaces spread their shine out until it disappears.
- **Normal texture:** a texture that stores small bumps and grooves as directions, so a flat surface catches the light as if it were carved. It looks light blue, because "facing straight out" is stored as blue.
- **Look:** how models are shaded in the viewport: standard, stylized (cel-shaded) or unlit. See [Looks](looks.md).

## Light

- **Sun:** light from very far away, arriving in parallel from one direction everywhere, like daylight. One per scene. See [Lights and shadows](lights.md).
- **Lamp:** a light in the scene. A **point light** shines every way from one spot, like a bare bulb; a **spotlight** shines a cone, like a torch or a stage light.
- **Watts (W):** how strong a lamp is, in the same units as Blender, so numbers carry over. The sun's strength is in watts per square metre (W/m²).
- **Range:** how far a lamp reaches. Real light fades on forever, getting weaker with distance; a range makes it stop sooner, which games use to keep many lamps cheap.
- **Shadow:** where something blocks a light. Solis draws the sun's shadows with a **shadow map**: the scene drawn once more from the sun's side, so each point can be checked for whether it sees the sun.
- **Sky light (ambient light):** the soft light that comes from the whole sky rather than from the sun, so shadows are never black.
- **HDR (high dynamic range):** working out light brighter than a screen's white, as the real world is: a sunlit wall can be many times brighter than a shaded one.
- **Exposure:** how bright the picture is made, in stops (EV) as on a camera: +1 doubles the brightness, -1 halves it.
- **Tone mapping:** squeezing HDR light into what a screen can show, so bright highlights roll off softly instead of turning flat white. Solis uses AgX, as Blender does, unless you choose otherwise.
- **Fog:** distant things fading into a colour, as in haze or mist. It gives a sense of distance and depth.

## Tools

- **Tool:** what a click in the viewport does: Select, Move, Rotate, Scale or Transform. The tools work once there are things in a scene to use them on (Phase 2).
