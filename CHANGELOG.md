# Changelog

What changed in each version, for people using Solis. Newest first.

## Unreleased

### Changed

- What other engines call a *scene* is an **area** in Solis: the Area panel, "Untitled area", and `.solarea` files when saving arrives. Reusable things you place many times (prefabs elsewhere) will be **assemblies**.
- Everything in an area is now an entity made of components: the sun, the environment, lights and imported models. Properties shows each entity's name and components (Transform first), drawn the same way for every kind of component. The sun's direction is now its **Rotation** under Transform (X tilts it from straight down, Z turns it round) instead of Heading and Height.
- The sun and the sky, fog and exposure belong to the area now, so they're no longer remembered between sessions on their own; they'll be saved with the area once saving arrives.
- Lights brought in with a model are listed under it in the Area panel, and deleting the model deletes them.

### Added

- **Several views:** the viewport's **≡** menu splits it into one view, two side by side, two one above the other, or four. Each view has its own camera and look; click in one to use it (F, adding things and the tool bar follow it), and **Maximise** shows it alone. Remembered on this computer.
- **Flat views:** each view can be **Perspective**, **Top**, **Front** or **Right**. Flat views show true sizes with the grid in their own plane; drag with the right or middle button to slide them and turn the wheel to zoom towards the pointer. Selecting, gizmos and dropping from the Library work in them.
- **Play mode:** **Play**, **Pause** and **Stop** at the top run the area as a game in the **Game** panel. Everything still works while playing, and changes show in the game at once; **Stop** puts the area, the undo history and the selection back exactly as they were. Saving and opening wait until Stop. A coloured border, a lit Play and a red Stop show that the game is running. Also `play`, `pause` and `stop` in the Console.
- **Cameras:** **Entity > Add camera** adds one where you're looking from. The game looks through the first camera in the area; cameras show as small wire pyramids in the viewport, and can be clicked.
- **Console commands:** press **'** (beside Æ on a Norwegian keyboard) to jump to the Console's new command line. `select`, `find`, `new`, `delete`, `duplicate`, `get` and `set` (any component's setting, on every selected entity at once), `frame`, `undo`, `redo`, `save`, `look`, `tool`, `stats`, `clear` and `help`. Every change is an undo step. **Tab** finishes names, **Up** and **Down** bring back earlier commands (remembered between sessions), and mistakes get a plain answer in red.
- Warnings and errors from the libraries Solis uses (the graphics layer, for example) now show in the Console, named after the library.
- **The Problems panel** lists mistakes in the open area: models or assemblies missing from the project, a Model with no model chosen, components no one knows, and entities whose parent is gone. The tab shows how many; click one to select its entity.
- **The Library** shows the project's folders (a tree you can put away) and every file in the folder chosen: models and pictures as **thumbnails**, areas, assemblies and other files with a coloured stripe per kind, or as a list; a search box finds files in every folder. **Drag a model or assembly into the viewport** to place it on the surface under the mouse, or **onto an entity** in the Area panel to place it under that one. Double-click opens areas, and other files in their own program. Right-click to rename, show in Explorer, or delete to the Recycle Bin; drag a file onto a folder to move it. IDs always follow.
- **The Code tab** lists the project's code files (Lust, Lua and Rust); double-click opens one in your code editor.
- **The Area panel** has a search box, folds (▾/▸ before an entity with children, and View > Fold or Unfold everything), and on each row an **eye** to hide an entity in the editor and a **padlock** so clicks in the viewport pass through it. Its **Edit** and **Entity** menus work (the same as the top bar's, where Edit also gains Duplicate, Delete, Rename, Select all and Select nothing). Selecting in the viewport unfolds the way to it.
- **Properties with several entities selected** shows the components they all have; settings that differ show **—**, and a change applies to every one as one undo step (changing one number of a position or rotation changes only that number on each). Add component and × work on all of them.
- **Copy and paste components' values:** the ⋯ beside a component has Copy values, Paste values (adding the component where it's missing) and Reset to defaults; Properties' Edit menu copies and pastes all of them.
- **Selecting in the viewport:** click a model or a lamp's marker to select it, **Ctrl + click** to add or take away, drag on empty space to draw a box around what to select (**Ctrl** adds), **Esc** to select nothing and **Ctrl + A** to select everything. Selected things are outlined. A copy of an assembly is selected whole first; click again to go inside it.
- **Gizmos:** with Move (**W**), Rotate (**R**), Scale (**S**) or Transform (**T**), drag the arrows, rings, boxes and squares on the selection to move, turn and resize it. A note by the mouse says how far. A whole drag is one undo step; **Esc** or a right click cancels it. **Shift** drags ten times finer.
- **Snapping and axis locks** in the viewport's bottom-right corner: moves onto the grid (1 m to 10 cm), turns in steps (5° to 90°), sizes in 10% steps; hold **Ctrl** while dragging to flip snapping for that drag. **X**, **Y** and **Z** lock an axis so drags leave it alone; **L** switches the gizmo between the world's axes and the entity's own. Remembered on this computer.
- Assemblies: **Entity > Create assembly from selection** saves an entity and everything under it as an assembly in the project; place copies from the Library. Each copy can have its own changes, shown in bold in Properties with ↺ (put back) and ↑ (apply to the assembly, for every copy). Editing the assembly (or a Git pull) updates every copy and keeps their changes. Assemblies can hold other assemblies. Areas store copies as one block plus only what differs.
- **Unpack** in Properties: turns a copy of an assembly into plain entities, or splits an imported model into one entity per part of the file, each drawing only its own part.
- Saving areas: **Ctrl + S**, **Ctrl + Shift + S**, **Ctrl + O** and **Ctrl + N** (also in the File menu). Areas are `.solarea` text files: one block per entity, one line per setting, made so Git merges two people's work cleanly, and safe to edit by hand. The tab shows a • while there are unsaved changes, and Solis asks before losing them. A file changed on disk (a Git pull) is loaded as one step Ctrl + Z can undo, or, with unsaved changes, Solis asks which to keep.
- Projects: **File > New project…**, **Open project…** and **Recent projects**; the last project opens again on start. A project is a folder with an `assets` folder (watched for changes), a `.solis` cache and Git files set up for games. Imported models are copied into it.
- The **Library** lists the project's models; double-click one (or click its +) to place it.
- Renaming or moving a model breaks nothing: each asset's ID lives in a `.solmeta` file beside it, which follows a file renamed in Explorer. Copies get IDs of their own.
- Models saved again in Blender show the new version straight away; if the new file is broken, the old version stays and the Console says why.
- Models read before open much faster: an import cache remembers them, even after a rename. Models are read by a small pool of worker threads.
- **Add component** at the bottom of Properties gives an entity any component (a Light on a model makes it glow like a lamp), and the × beside a component's name takes it off. Game code will register its own components the same way.
- The Area panel stays fast with thousands of entities: only the rows in view are drawn.
- Building the area's tree: drag an entity onto another in the Area panel to put it under it (it stays where it is in the world), or onto empty space to make it stand alone. **Ctrl + click** and **Shift + click** select several; **Delete** deletes them, **Ctrl + D** duplicates them, **F2** or a double-click renames, and **F** points the camera at them. All of it can be undone.
- New entities go to the end of the Area panel's list instead of a random place.
- Models keep their own parts' placements: a mesh used many times in a file is stored once and drawn as copies, so big kits of repeated pieces load faster and use less memory. Mirrored parts (a negative scale) are drawn the right way out.
- Undo and redo for every change to the area: **Ctrl + Z**, and **Ctrl + Y** or **Ctrl + Shift + Z**, or **Edit > Undo** and **Redo**, which name the change. A whole drag is one step, and undo selects what it changed.
- **Entity > Add entity** adds an empty entity at the point the camera looks at.
- A component Solis doesn't know (for example from game code that isn't there) is kept untouched and shown as **Missing component**, with **Remove**.

### Fixed

- **Work could be lost when saving before quitting or starting a new area.** On an untitled area, choosing **Save** in the "unsaved changes" question went straight on to quitting (or the new area) while the Save window was still open. Now everything waits for the Save window.
- After a save was cancelled or failed while quitting, closing the window later closed it without asking about unsaved changes. Now it asks again.
- A file chosen in an Open or Save window opened before **Play** is now opened or saved only after **Stop**, so it never replaces or saves the game's copy. The question about a file changed on disk also waits until Stop.
- A Save name typed without `.solarea`, matching a file that already exists, replaced it without asking. Now it isn't saved, and the Console says how to replace it.
- An assembly that a Git pull changed to hold a new, not yet loaded assembly could detach its copies, and saving made that permanent. Assemblies are now all read before any is used, and the copies keep the version they were made from until the new one can be read.
- A damaged saved panel layout could crash Solis every time it started. A layout that can't be drawn is now replaced by the default one.
- A problem reported by the graphics card no longer closes Solis; it's shown in the Console. Scenes with more model copies than the card can draw at once are skipped with a message instead of crashing.
- Saving now writes the file to disk before it replaces the old one, so a power cut can't leave an empty file. Two saves of the same file at once no longer mix.
- A model file with hundreds of thousands of empty nodes, or a very deep node tree, could freeze or crash the editor. Such files are refused, and very deep trees keep short part names.
- A sidecar using the biggest part number crashed the editor or reused numbers. Part numbers now never repeat, and a sidecar with two parts sharing one is reported as damaged.
- An area file naming one entity twice with different spellings (lower case, or O for 0) silently lost one of them. Now it's reported.
- After many changes, undoing everything could make the area look saved when it wasn't.
- Choosing the open project again from **Recent projects** emptied the viewport. Now nothing happens.
- Opening another project kept the old project's assemblies, so a copy of a project could save its areas against the wrong assemblies.
- Components registered by game code were forgotten when a new area was started or another opened.
- Renaming, moving or deleting the open area in the Library left Solis saving to the old name, making a second file. The open area now follows its file.
- Renaming a file only by changing capitals (`crate.glb` to `Crate.glb`) was refused on Windows.
- The last project now opens again when Solis starts (it was saved but never read back).
- The red and green axes on the ground could show as a large filled wedge on some graphics drivers when the camera was close to them.
- Clicking could select the wrong thing after the area changed or a view was resized, because the answer belonged to an older click. Clicking the very last pixel of a view no longer misses.
- In a Top, Front or Right view, **Add > Camera**, **Add > Spotlight** and **Move here** put the new thing far away, and fog covered the area. They now go where the view looks from, and flat views have no fog.
- The game view and the editor could light an area with two suns differently. Both now use the first one in the Hierarchy.
- **F** framed an unpacked model by its whole size, even when only its parts were selected.
- A thumbnail that was replaced or changed on disk showed the old picture, and its memory wasn't freed. Thumbnails from the last project no longer appear in the next.
- The Problems panel didn't update after a model was reloaded or another project opened. It now also reports an entity that is (through others) inside itself.
- A model added to the folder while the area used it, or a changed one, wasn't reloaded in the viewport.
- **Help > Add the sample area** with a project open added a model the project didn't have, so a saved area that used it said it was missing when opened again. It's now added to the project's assets like an import.
- Placing a model by double-click after an earlier drop of it failed sent it to the old drop point.
- A Git checkout or pull while Solis was open could give a model a new ID if Solis looked between Git writing the model and its sidecar. A new file without a sidecar now waits two seconds for one.
- A model copied (with its sidecar) while Solis was closed could take the original's ID, so areas using the original showed the copy. The original, whose sidecar is older, now keeps it.

- A layout saved before a panel existed keeps its arrangement and gets the new panel beside its usual neighbours, instead of being reset to the default.
- A full Console no longer slows each new message down.
- Damaged or hostile model files can no longer use up all memory: a buffer only reads the bytes it says it holds, a model's files and textures each have an overall budget, a tiny file asking for millions of corners is refused before reading, and one mesh placed too many times is refused. Files are only read from the model's folder.
- A very large mesh no longer crashes Solis: it's split into pieces the graphics card can hold.
- Mirrored parts of a model without normals were lit as if inside out.
- A model needing a glTF extension Solis doesn't read (such as Draco compression) now says which, instead of "refers to parts that don't exist".
- The Import model window no longer freezes the editor while it's open, and an import that fails unexpectedly reports an error instead of "Importing…" forever.
- Distant surfaces no longer flicker against each other (depth is now reversed, keeping precision far away).
- Showing the sun or a lamp in Properties no longer changes values that came from a file (a sun below the horizon, a lamp's long range).
- A model with impossibly large bounds no longer breaks the camera.
- Several viewports no longer share one set of frame data, ready for the game view and split views.

### Changed

- Writing game code in Lust now arrives in Phase 4, right after the editor, instead of Phase 7; the editor's panels say so. Playing the area arrives with the editor, in Phase 3.
- The Console stays fast when full: it lays out only the messages in view (a full Console took about 10 ms a frame, now 0.3 ms). Long messages are cut to one line and show in full when pointed at.
- A camera saved with broken numbers (from a damaged settings file) is repaired when Solis starts, instead of showing a broken view.
- `solis --help` takes the program's name from one place, like the rest of the editor.

- The editor looks more like the final mockup: the area tab sits on a lighter strip with an icon and ×; every panel tab has an icon and a deeper accent underline; the menus are roomier; Play, Play with others and Stop are icon buttons; panel menu rows end in a search icon; the console has compact filter buttons (icon and count), times on every message and JetBrains Mono text; and the viewport fades from dark above to a lighter ground, with a cool grey-blue grid and no hard horizon.

### Added

- The finished picture: light is worked out brighter than white (HDR) and tone mapped as in Blender (AgX by default, or ACES, or Standard), so bright highlights roll off softly instead of flattening to white. Select **Environment** in the Area panel for the sky (the editor backdrop, or a daylight sky that follows the sun from blue noon to orange sunset and dark night), fog, exposure and tone mapping; they're remembered between sessions. The editor's backdrop and grid look as before.

- Lights and shadows: the sun casts shadows on models and the ground, and its direction, colour and strength can be changed (select **Sun** in the Area panel); it's remembered between sessions. **Entity > Add point light** and **Add spotlight** place lamps, listed in the Area panel and edited in Properties (colour, strength in watts as in Blender, range, cone, **Move here**, **Delete**). Lamps and a sun in an imported glTF file (as Blender exports them) come in with the model. Lamps show as small crosses in the viewport, and light both the standard and stylized looks. Selecting a model in the Area panel shows its size in Properties.

- Materials and looks: models now show glTF's full material (metal, roughness, normal, occlusion and glow textures). The viewport's look menu (bottom left) chooses **Standard**, **Stylized** (cel-shaded in 2 to 4 bands of light, with crisp highlights, a rim of sky light and dark outlines) or **Unlit**, remembered between sessions. **Help > Add the sample area** brings in spheres from glossy to rough, plastic and metal, a grooved wall and a glowing stripe for trying them.
- The viewport's bottom-left menus now open upwards and stay open while you change their settings.

- Importing models: **File > Import model…**, or drop a `.glb` or `.gltf` file onto the window. Meshes, colour textures, see-through and double-sided surfaces come in, upright (glTF's Y-up becomes Solis's Z-up), lit by a fixed sun and the sky. The camera frames the new model; the Area panel lists imported models, each with a × to remove it. Models are read in the background, and damaged files are explained, never a crash. Also `solis --import <file>`.

- The user manual (`docs/manual`): getting started, concepts, a tour of the interface, moving around the viewport, the Console and every shortcut.

- A ready-made `solis.exe` for Windows, built on GitHub whenever code is merged (Actions tab, **Windows EXE**). Release builds on Windows open without a console window.

- The Code panel (beside Library) and the Problems panel (beside Console), as in the mockup. They fill in during Phases 7 and 2. Saved layouts from before reset to the default once, to make room for them.

- Flying around the viewport: hold the right mouse button to look; while it's held, W A S D move, Q and E go down and up, the wheel changes speed (shown in the Camera box) and Shift goes faster. The wheel alone moves forward and back; middle drag slides the view. The camera and its speed are remembered.

- The editor window: a one-row top bar with menus, the area tab, and Play, Stop, Build and Build & Play (which say which phase brings them).
- Dockable panels: Viewport, Game, Area, Layers, Properties, Library and Console. Drag a tab to move a panel; the layout is remembered next time. Window > Layouts has Default, Wide screen and Small screen.
- The viewport draws a sky, a ground grid that fades with distance, and the red X and green Y axes. Its corners hold the view menus (≡, Perspective, Lit, Show), the tools (Select, Move, Rotate, Scale, Transform; keys Q, W, R, S, T), grid, angle and camera speed, and the frame rate.
- The console lists messages, with filters by level and a search box.
- Window > Accent colour: petrol blue, red, green or plum.
