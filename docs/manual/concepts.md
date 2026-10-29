# Concepts

> The ideas this manual uses, in plain words. Other pages link here the first time they use a term.

## The editor and the engine

- **Engine:** the program a game runs on. It draws the picture, plays the sound, runs the physics and the game's own code. When a game ships, the engine goes with it.
- **Editor:** the window you build a game in. Games ship without it. In Solis the engine is written in Rust; game code can be written in Lust, Rust or Lua; see [Game code](game-code.md).

## The window

- **Panel:** one area of the editor with one job, such as the Viewport or the Console. Every panel has a **tab** with its name.
- **Tab group:** panels that share one space. Click a tab to bring that panel to the front; the front tab is underlined in the accent colour.
- **Docking:** dragging a panel's tab to a new place. Drop it on the edge of another panel to put them side by side, or in the middle to make them one tab group. See [Interface tour](interface-tour.md).
- **Layout:** where all the panels are. Solis remembers yours; **Window > Reset layout** puts them back as they started.
- **Accent colour:** the one colour the editor uses to mark what's chosen or in front: the front tab's underline, the chosen tool, the selected row. Amber unless you pick another. Colours that mean something keep their own: Play is blue, Stop red, errors red.

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
- **Asset:** a file the game uses: a model, a picture, a sound, an assembly, code.
- **Sidecar:** the small `.solmeta` text file beside each asset holding its ID, so renaming the file breaks nothing. Commit it with the asset.
- **Library:** the panel showing the project's files, folder by folder, with thumbnails; drag from it to place things.
- **Thumbnail:** a small picture of a file, so you can see what a model or picture is without opening it.
- **Recycle Bin:** where deleted files wait until you empty it; deleting in the Library sends files there, so nothing is lost by accident.
- **Cache:** files Solis makes to work faster (models it has already read). Safe to delete: they're made again.
- **Hot reload:** a changed file shows in Solis straight away, without restarting.
- **Format version:** a number at the top of each file Solis saves, saying how it's laid out. When an update changes the layout, the number goes up and the update brings an **upgrader**, so older files keep opening. Before Solis saves an older file in the new layout, it keeps a copy in `.solis/backups/`. See [Backups and upgrades](backups-and-upgrades.md).

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
- **Drawing between ticks** (interpolation): showing everything part way between where it was at the last tick and where it is now, so a screen faster than the game's 60 ticks a second shows smooth movement. See [Play mode](play-mode.md).
- **Frame:** one picture on screen. Games aim for 60 **frames a second** (fps), which leaves about 16.7 milliseconds (thousandths of a second) for each; 30 fps still feels playable. See [The Profiler](profiler.md).
- **CPU and GPU:** the computer's processor, which runs game code and the editor, and its graphics card, which draws. They work at the same time; the slower one sets the frame rate.
- **Culling:** leaving out what the camera can't see before drawing. Each copy of a model is checked against the view (a ball around it is enough), so a huge area costs only what's in view. Things just out of view are still drawn into the sun's shadow map when their shadow can fall into it.

## Game code

- **Game code:** the part of a game written as code: its rules, and what things do on their own. See [Game code](game-code.md).
- **Lust:** a programming language made for games like Solis's, which reads like plain English. Solis checks it and runs it. Lust can also become Rust, which runs fastest, for the finished game.
- **Native method:** a method Lust knows the name and values of, but whose work is done by the engine, such as `moveTo`. The engine's are listed in the `solis` folder Solis writes into each project.
- **Mistake (in code):** something the checker finds before the code runs, such as a misspelt name or text where a number should be. Solis lists each one with its file, line and column (how far along the line).
- **Projectile:** something small and fast a game fires in its thousands (a laser, a bullet), which isn't an entity: Solis moves it every tick until its time is up, and draws all of them in one go. Game code fires one with `fire`. See [Game code](game-code.md#projectiles).
- **Spawn:** game code making an entity while the game runs (a copy of an [assembly](assemblies.md)); **despawn** takes one away. See [Game code](game-code.md#making-and-removing-entities).
- **Benchmark:** a ready-made test of how much a computer can play at once. Solis's is two fleets of ships firing lasers: see [The benchmark](benchmark.md).
- **Tick:** one step of the running game. Solis runs 60 a second, whatever the frame rate, and game code's `update` runs once each tick, so the game behaves the same on a fast or slow computer. During a tick, game code sees the world as it was when the tick started (plus its own changes), so it can run on every core at once and still come out the same.
- **Random number:** a number game code gets by chance, such as a die roll (`randomInt(1, 6)`). Solis makes them from a **seed**: the same seed gives the same numbers in the same order, so a play can be repeated exactly. Each Play has a new seed unless you choose one (`seed` in the [Console](console.md)).
- **Event:** a message one component sends to an entity, such as "you were hit, for 2.5". Each of that entity's components that cares reacts to it. Events let components work together without knowing about each other, even when they're written in different languages.

## Simulation

- **Physics:** making things move like real objects without writing how: they fall, collide, stack and get pushed. See [Physics](physics.md).
- **Rigid body:** what makes an entity move by physics. **Dynamic** ones are moved by physics, **kinematic** ones only by game code (pushing dynamic ones aside), and **fixed** ones never move.
- **Collider:** the shape physics bumps into (a box, sphere or capsule), which can be simpler than what's drawn. A collider on its own makes a floor or wall that never moves.
- **Impulse (push):** a sudden push, like a kick. The same push moves a light thing more than a heavy one.
- **Ray:** an invisible line from a point in a direction, asking "what's the first thing along here?". Games use them to see whether a character stands on the ground, or what a gun points at.
- **Determinism:** a game coming out exactly the same every time it's given the same inputs, on any computer. Solis keeps to it (decimals rounded the same way, everything done in a fixed order), so a recording replays exactly, and players online can keep their games in step by sending only their inputs.
- **Action:** something the player does, by name (Move, Jump), which game code asks about instead of keys. Its **bindings** are the keys, mouse buttons and gamepad buttons that set it off; **rebinding** changes them. See [Input](input.md).
- **Character controller:** what moves a player's character: it walks, climbs steps and slopes, slides along walls instead of sticking, and stands on moving platforms. It isn't pushed around by physics like a crate; it moves itself, and pushes physics things out of its way. Solis's is the **CharacterBody** component plus the ready-made **CharacterController**. See [Characters and cameras](characters-and-cameras.md).
- **Ready-made components:** components Solis writes into every project's `solis/kit/` folder, in Lust (the character controller and the cameras): use them as they are, or read and copy them to make your own.
- **Combination:** a key pressed while holding Ctrl, Alt or Shift, bound as one input (`Alt+Left`).
- **Axis:** an action that gives numbers instead of on or off: a stick, or four keys, giving which way and how far.
- **Recording:** every tick's input from a play, kept so the play can be run again exactly, checking it comes out the same. See [Input](input.md#how-to-use-it).
- **Save game:** the whole game at a moment, kept in a file: every entity, every component's values, physics, the clock and the random numbers. Loading it carries on exactly as the game would have. See [Save games](save-games.md).
- **Checksum:** one number made from everything in the game at a moment. Two plays that agree on it at the same tick are in the same state; when they don't, something differs. `checksum` in the [Console](console.md) shows it.

## Sound

- **Sound card:** the part of a computer that turns sound into what speakers and headsets play. Most computers have one built in; a USB headset brings its own. With none, Solis keeps working and plays nothing.
- **WAV and Ogg:** the two kinds of sound file Solis reads. WAV keeps every sample as it is (big files, right for short effects from a sound editor); Ogg (Ogg Vorbis) is squeezed to about a tenth of the size with little to hear, right for music and anything long.
- **Sample rate:** how many times a second a sound is measured, such as 44,100 (CD quality) or 48,000. Higher keeps finer detail, in a bigger file.
- **Channels:** how many separate sounds a file holds side by side: one (mono) or two (stereo, left and right).
- **Sound event:** what a game plays, such as "footstep": a few recordings and how to vary them each time. See [Sound events](sound-events.md).
- **Take:** one recording of a sound, as in a recording studio. A footstep event might hold five takes of a foot on gravel.
- **Decibel (dB):** how loudness is measured. Every 6 dB is about twice (or half) as loud, and 0 dB means as recorded.
- **Semitone (st):** one step on a piano keyboard. 12 semitones is an octave: twice the pitch. Playing a sound faster raises its pitch.
- **Bus:** a group of sounds turned up or down together, like a channel on a mixing desk: music, effects, voices, ambience and the interface each have one. Buses sit inside other buses, all inside Master. See [The Mixer](mixer.md).
- **Fader:** a bus's volume control, the slider on its strip in the Mixer.
- **Meter:** a bar showing how loud a bus is right now: green, then yellow, red near the top.
- **Ducking:** turning one bus down automatically while another plays: music under speech.
- **Reverb:** the echo a space adds to every sound in it: short in a small room, long in a cave. See [Reverb zones](reverb-zones.md).
- **Listener:** where the game hears from: a Listener component on a camera or character, or with none the camera. See [Listeners and who hears what](listeners.md).
- **Voice:** one sound playing. A game can have thousands of sounds wanting to play; only so many voices are heard at once, the most important first.
- **Resting (virtual) sound:** a looping sound that isn't heard right now, so it isn't mixed, but keeps its place in time, so it carries on where it would be when it's heard again.
- **Mute and solo:** mute silences a bus; solo plays only the soloed buses, to hear one part on its own.
- **Streaming:** playing a long sound a piece at a time as it's read from its file, instead of reading all of it first, so a long piece of music doesn't fill the memory. Solis streams sounds longer than ten minutes.

## 2D

- **2D in Solis:** flat things lying on the ground, seen from straight above by a camera with no perspective. They're in the same world as 3D, so light, sound, physics, saving and game code work on them too. Height (Z) says what's drawn over what. See [2D: the pixel-perfect view](2d-pixel-perfect.md).
- **Pixel art:** pictures drawn one pixel at a time, at a small size, meant to be shown scaled up with every pixel kept square and sharp.
- **Base resolution:** the size a 2D game is drawn at before it's scaled up to the screen, such as 480 × 270. Smaller shows chunkier pixels and less of the world.
- **Pixels per unit:** how many of the game's pixels make one metre of the world. With 16, a 16-pixel tile is one metre.
- **Pixel-perfect:** every pixel of the game shown as the same whole number of screen pixels, never blurred or uneven. Solis draws the game at its base resolution and scales it up in **whole steps** (×2, ×3, ×4…).
- **Whole steps:** scaling by a whole number. When the screen isn't a whole number of times the base resolution, the rest is black **bars**; **Fill** scales a little further instead, smoothing only the edge of each pixel.
- **2D camera:** a camera whose View is 2D: it looks straight down at the game's base resolution, follows its entity's place, and never turns.
- **Tile:** one square of a 2D world's grid, usually 16 pixels; [tile maps](tile-maps.md) are built from them.
- **Sprite:** a flat picture in a 2D world, such as a character or a crop: one frame of a sprite sheet, lying on the ground at its entity. See [Sprites from Aseprite](sprites.md).
- **Sprite sheet:** a picture holding a sprite's frames, from one Aseprite file. Solis reads `.aseprite` files as they are; nothing needs exporting.
- **Frame:** one picture of a sprite sheet. Showing frames one after another animates it.
- **Tag:** in Aseprite, a named run of frames (*walk*, *swing*) with a direction (forward, reverse, ping-pong) and repeats: a clip to play.
- **Slice:** in Aseprite, a named rectangle on the picture (*feet*, *hand*), used for where a sprite stands, where a held tool goes, or what can be hit.
- **Pivot:** the point a sprite stands on: it sits on its entity's place, so the sprite turns and flips round it. Usually between the feet.
- **Atlas:** one big picture with many frames packed into it, so the graphics card draws a sheet's sprites all at once instead of one at a time.
- **Clip:** a run of a sprite sheet's frames played as an animation (*walk*, *hoe*), each frame shown for its own time, forward, backward or ping-pong (there and back), over and over or a number of times. Aseprite's tags are clips; you can make more in Solis. See [Animating sprites](sprite-animation.md).
- **Clips file:** a `.solframes` file beside an Aseprite file, holding what Solis adds to its clips (speed, events) and clips made in Solis. Open it to edit its clips in the **Clips** and **Frames** tabs.
- **Animator:** a `.solanim` file that chooses which clip a sprite plays, from states, transitions and parameters, edited in the **Animator** tab; an **Animator** component puts one on an entity. See [Animators](animators.md).
- **State:** one thing an animated sprite can be doing (*Idle*, *Walk*), drawn as a box: it plays a clip, or a clip for each facing direction.
- **Transition:** an arrow from one state to another (or from *Any state*), taken when all its conditions hold.
- **Parameter:** a value an animator's transitions look at: a number, a switch (on or off), a trigger (fired for one tick) or a direction (which way it faces).
- **Graph editor:** the canvas of boxes and arrows the Animator tab draws on; materials and effects will use the same one, with the same controls.
- **Frame event:** a name on a frame of a clip (*footstep*). When the clip reaches the frame, a sound event of that name plays and game code hears `AnimationCue`.
- **Tile map:** a grid of cells laid out in a 2D world, saved as a `.soltilemap` file and placed by an entity's **Tile Map** component. Each cell holds a terrain on each layer, and values. See [Tile maps](tile-maps.md).
- **Tileset:** a `.soltileset` file listing the terrains a tile map's cells can be. Every map that uses it draws its terrains the same way.
- **Terrain:** what a cell is (*grass*, *tilled*, *water*), with a name, a colour and the one character a map file writes for it (its **code**, such as `.` for grass).
- **Layer (tile map):** one of a map's stacked grids, such as Ground, Decoration and Above. A layer is drawn **under** sprites (the ground they walk on) or **over** them (roofs they walk behind).
- **Cell value:** something every cell of a map has besides its terrain: a **number** (growth), a **switch** (watered) or a **choice** (crop: wheat or corn). The game changes them as it plays, and saves keep them.
- **Chunk:** a 32 × 32 block of a map's cells. A map is saved and checked chunk by chunk, so a change to one cell touches only its chunk.
- **Autotiling:** picking each cell's tile of art from its neighbours, so edges and corners come out right without choosing tiles by hand. See [Tilesets and autotiling](tilesets.md).
- **Copy (stamp):** cells of a map copied with the tile painter's eyedropper, to paint with: flipped, turned, and repeated like a pattern. See [Painting tiles](painting-tiles.md).
- **Exact tile:** a cell that shows one tile of its tileset as it is, chosen by hand, instead of the one autotiling would pick. Art only: the game reads the cell's terrain. See [Painting tiles](painting-tiles.md#exact-tiles).
- **Kind (terrain):** how a terrain's tiles are picked: plain (one tile), 16-tile, 47-tile, edges (fences, roads) or its own rules.
- **Slot:** one situation a terrain's art has a tile for, such as "tilled to the north and east only". Each kind has its own slots; a slot with no tile shows pink.
- **Dual grid:** drawing art half a cell off the cells, so each tile sits where four cells meet and is chosen by which of them are the terrain. 16-tile terrains use it: 15 drawings cover every edge and corner.
- **Variation:** another tile for the same slot, picked by weight so a field doesn't look repeated. Each cell always gets the same one.
- **Template:** a picture with every slot of a kind laid out at your tile size, to draw your tiles over.

## Getting Solis

- **Build:** one version of the program, made by GitHub each time a change is merged into the main version. Each build has a **build number** that goes up by one or more each time, so a bigger number is newer. Help > About shows yours. See [Installing and updating](installing-and-updating.md).
- **Release:** a build published on GitHub's Releases page for anyone to download. Solis publishes every build as a release called `build-` and its number.
- **Installer:** a program that puts another program on your computer properly: in its own folder, in the Start menu, and in Windows' list of apps so you can uninstall it. Solis's is `solis-setup.exe`.
- **Splash screen:** the picture a program shows while it starts: Solis's logo, name and version, and what it's doing (such as checking for updates).
- **SHA-256:** a 64-character fingerprint of a file's contents. Change one byte and it's completely different, so comparing fingerprints shows a download arrived whole and unchanged.
