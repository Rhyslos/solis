# Concepts

> The ideas this manual uses, in plain words. Other pages link here the first time they use a term.

## The editor and the engine

- **Engine:** the program a game runs on. It draws the picture, plays the sound, runs the physics and the game's own code. When a game ships, the engine goes with it.
- **Editor:** the window you build a game in. Games ship without it. In Solis the engine is written in Rust; game code can be written in Lust, Rust or Lua; see [Game code](game-code.md).

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
- **Flat view (orthographic):** a view from straight above, the front or the side, where things keep their size however far away they are, so edges and heights line up exactly. The usual view, where far things look smaller, is **perspective**. See [Moving around the viewport](navigating-the-viewport.md).
- **View:** one picture in the viewport, with its own camera. The viewport can show one, two or four at once.
- **Fly camera:** the way you move around the viewport: you steer the camera like a drone. See [Moving around the viewport](navigating-the-viewport.md).
## The area

- **Area:** everything in one place of your game (a level, a menu, a room), made of entities. Other engines call this a *scene*. Each area is saved as one `.solarea` text file; see [Saving areas](saving-areas.md). See [Entities and components](entities.md).
- **Entity:** one thing in an area: a model, a light, the sky. On its own it's only a name; its components make it something.
- **Component:** one part of what an entity is, with settings. A **Transform** says where it is; a **Light** makes it shine; a **Model** draws an imported model. Mixing components makes different things, the way one Lego brick goes into many builds.
- **Transform:** an entity's position, rotation and scale.
- **Assembly:** a group of entities saved once and placed many times; each placed **copy** follows the assembly but can have its own changes. Other engines call it a *prefab*. See [Assemblies](assemblies.md).
- **Unpack:** turning a copy of an assembly, or an imported model, into separate ordinary entities.
- **Parent and child:** an entity can belong to another, like a lamp to the room model it came with. A child moves with its parent, and deleting the parent deletes its children. A child's position is measured from its parent, so moving the parent carries it along.
- **Instance:** one copy of a mesh drawn somewhere. A mesh used many times (a crate in a hundred places) is stored once and drawn as many instances, which is much faster than storing a hundred crates.

## Projects

- **Project:** a folder holding one game: its assets, settings and areas. See [Projects and the Library](projects-and-the-library.md).
- **Asset:** a file the game uses: a model now, textures, sounds and code later.
- **Sidecar:** the small `.solmeta` text file beside each asset holding its ID, so renaming the file breaks nothing. Commit it with the asset.
- **Library:** the panel showing the project's files, folder by folder, with thumbnails; drag from it to place things.
- **Thumbnail:** a small picture of a file, so you can see what a model or picture is without opening it.
- **Recycle Bin:** where deleted files wait until you empty it; deleting in the Library sends files there, so nothing is lost by accident.
- **Cache:** files Solis makes to work faster (models it has already read). Safe to delete: they're made again.
- **Hot reload:** a changed file shows in Solis straight away, without restarting.

## Models

- **Model:** a 3D object made in a modelling program (Blender, Aventail): its shape, and how its surface looks. See [Importing models](importing-models.md).
- **glTF:** the standard file format for game models, in two forms: `.glb` (everything in one file) and `.gltf` (a text file, with its data and textures in files beside it).
- **Mesh:** a model's shape, made of triangles.
- **Triangle:** the smallest piece of a 3D shape. Graphics cards draw everything as triangles; a game character has a few thousand to tens of thousands.
- **Level of detail (LOD):** a simpler version of a mesh, with fewer triangles, drawn when a copy is so small on screen that the difference can't be seen. Solis makes them on import, or uses a model's own (named `_LOD1` and so on). See [Levels of detail](levels-of-detail.md).
- **Material:** how a surface looks: its colour, how shiny, how see-through.
- **Texture:** an image wrapped around a model, giving it detail (paint, wood grain, panels).
- **Metallic and roughness:** the two numbers that decide how a surface shines. Metal reflects its surroundings; rough surfaces spread their shine out until it disappears.
- **Normal texture:** a texture that stores small bumps and grooves as directions, so a flat surface catches the light as if it were carved. It looks light blue, because "facing straight out" is stored as blue.
- **Look:** how models are shaded in the viewport: standard, stylized (cel-shaded) or unlit. See [Looks](looks.md).

## Light

- **Sun:** light from very far away, arriving in parallel from one direction everywhere, like daylight. One per area. See [Lights and shadows](lights.md).
- **Lamp:** a light in the area. A **point light** shines every way from one spot, like a bare bulb; a **spotlight** shines a cone, like a torch or a stage light.
- **Watts (W):** how strong a lamp is, in the same units as Blender, so numbers carry over. The sun's strength is in watts per square metre (W/m²).
- **Range:** how far a lamp reaches. Real light fades on forever, getting weaker with distance; a range makes it stop sooner, which games use to keep many lamps cheap.
- **Shadow:** where something blocks a light. Solis draws the sun's shadows with a **shadow map**: the area drawn once more from the sun's side, so each point can be checked for whether it sees the sun.
- **Sky light (ambient light):** the soft light that comes from the whole sky rather than from the sun, so shadows are never black.
- **HDR (high dynamic range):** working out light brighter than a screen's white, as the real world is: a sunlit wall can be many times brighter than a shaded one.
- **Exposure:** how bright the picture is made, in stops (EV) as on a camera: +1 doubles the brightness, -1 halves it.
- **Tone mapping:** squeezing HDR light into what a screen can show, so bright highlights roll off softly instead of turning flat white. Solis uses AgX, as Blender does, unless you choose otherwise.
- **Fog:** distant things fading into a colour, as in haze or mist. It gives a sense of distance and depth.

## Tools

- **Tool:** what a click in the viewport does: Select, Move, Rotate, Scale or Transform. See [Selecting and moving things](selecting-and-moving.md).
- **Selection:** the entities you're working on. Selected things are outlined in the accent colour in the viewport and highlighted in the Area panel; Properties shows one of them.
- **Gizmo:** the coloured arrows, rings and boxes drawn on the selection, which you drag to move, turn or resize it. Each handle works along one axis, in its colour.
- **Pivot:** the point a selection turns and grows around: the middle of the selected entities.
- **Snapping:** making moves, turns and sizes jump in steps (to the grid lines, 15° at a time, 10% at a time) so things line up exactly.
- **Hidden and locked:** ways to keep things out of your way while you work: a hidden entity isn't drawn in the editor, and a locked one can't be clicked in the viewport. The game still has them both.
- **Axis lock:** stopping drags from changing one axis, such as Z, so a drag slides along the floor without lifting.
- **World and object axes:** the world's X, Y and Z are the same everywhere; an object's own axes turn with it. The gizmo can follow either.
- **Console:** the panel where the editor reports what it's doing, and where you can type commands. See [The Console](console.md).
- **Command:** a short instruction typed in the Console's command line, such as `select Crate` or `set Transform.position 0 0 2`. Commands do the same things as the menus, so each change can be undone.
- **Problem:** a mistake in the area that Solis can't fix by itself, such as a model whose file is gone. The Problems panel lists them. See [Problems](problems.md).

## Playing

- **Play mode:** running the area as a game inside the editor. Press Play to start and Stop to finish; everything changed while playing is undone by Stop. See [Play mode](play-mode.md).
- **Game:** what players see and do. In the editor it's shown in the **Game** panel, through the game's camera, without the editor's grid and markers.
- **Camera:** an entity the game looks through. The game uses the first camera in the area. Its **field of view** is how wide it sees.
- **World:** the game's own copy of the area while it runs, kept in a form a game can run over quickly. It's made when you press Play and thrown away on Stop.
- **Frame:** one picture on screen. Games aim for 60 **frames a second** (fps), which leaves about 16.7 milliseconds (thousandths of a second) for each; 30 fps still feels playable. See [The Profiler](profiler.md).
- **CPU and GPU:** the computer's processor, which runs game code and the editor, and its graphics card, which draws. They work at the same time; the slower one sets the frame rate.
- **Culling:** leaving out what the camera can't see before drawing. Each copy of a model is checked against the view (a ball around it is enough), so a huge area costs only what's in view. Things just out of view are still drawn into the sun's shadow map when their shadow can fall into it.

## Game code

- **Game code:** the part of a game written as code: its rules, and what things do on their own. See [Game code](game-code.md).
- **Lust:** a programming language made for games like Solis's, which reads like plain English. Solis checks it and runs it. Lust can also become Rust, which runs fastest, for the finished game.
- **Native method:** a method Lust knows the name and values of, but whose work is done by the engine, such as `moveTo`. The engine's are listed in the `solis` folder Solis writes into each project.
- **Mistake (in code):** something the checker finds before the code runs, such as a misspelt name or text where a number should be. Solis lists each one with its file, line and column (how far along the line).
- **Projectile:** something small and fast a game fires in its thousands (a laser, a bullet), which isn't an entity: Solis moves it every tick until its time is up, and draws all of them in one go. Game code fires one with `fire`. See [Game code](game-code.md#projectiles).
- **Tick:** one step of the running game. Solis runs 60 a second, whatever the frame rate, and game code's `update` runs once each tick, so the game behaves the same on a fast or slow computer. During a tick, game code sees the world as it was when the tick started (plus its own changes), so it can run on every core at once and still come out the same.
- **Event:** a message one component sends to an entity, such as "you were hit, for 2.5". Each of that entity's components that cares reacts to it. Events let components work together without knowing about each other, even when they're written in different languages.
