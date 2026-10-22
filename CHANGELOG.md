# Changelog

What changed in each version, for people using Solis. Newest first.

## Unreleased

### Changed

- Writing game code in Lust now arrives in Phase 4, right after the editor, instead of Phase 7; the editor's panels say so. Playing the scene arrives with the editor, in Phase 3.
- The Console stays fast when full: it lays out only the messages in view (a full Console took about 10 ms a frame, now 0.3 ms). Long messages are cut to one line and show in full when pointed at.
- A camera saved with broken numbers (from a damaged settings file) is repaired when Solis starts, instead of showing a broken view.
- `solis --help` takes the program's name from one place, like the rest of the editor.

- The editor looks more like the final mockup: the scene tab sits on a lighter strip with an icon and ×; every panel tab has an icon and a deeper accent underline; the menus are roomier; Play, Play with others and Stop are icon buttons; panel menu rows end in a search icon; the console has compact filter buttons (icon and count), times on every message and JetBrains Mono text; and the viewport fades from dark above to a lighter ground, with a cool grey-blue grid and no hard horizon.

### Added

- Lights and shadows: the sun casts shadows on models and the ground, and its direction, colour and strength can be changed (select **Sun** in the Scene panel); it's remembered between sessions. **Entity > Add point light** and **Add spotlight** place lamps, listed in the Scene panel and edited in Properties (colour, strength in watts as in Blender, range, cone, **Move here**, **Delete**). Lamps and a sun in an imported glTF file (as Blender exports them) come in with the model. Lamps show as small crosses in the viewport, and light both the standard and stylized looks. Selecting a model in the Scene panel shows its size in Properties.

- Materials and looks: models now show glTF's full material (metal, roughness, normal, occlusion and glow textures). The viewport's look menu (bottom left) chooses **Standard**, **Stylized** (cel-shaded in 2 to 4 bands of light, with crisp highlights, a rim of sky light and dark outlines) or **Unlit**, remembered between sessions. **Help > Add the sample scene** brings in spheres from glossy to rough, plastic and metal, a grooved wall and a glowing stripe for trying them.
- The viewport's bottom-left menus now open upwards and stay open while you change their settings.

- Importing models: **File > Import model…**, or drop a `.glb` or `.gltf` file onto the window. Meshes, colour textures, see-through and double-sided surfaces come in, upright (glTF's Y-up becomes Solis's Z-up), lit by a fixed sun and the sky. The camera frames the new model; the Scene panel lists imported models, each with a × to remove it. Models are read in the background, and damaged files are explained, never a crash. Also `solis --import <file>`.

- The user manual (`docs/manual`): getting started, concepts, a tour of the interface, moving around the viewport, the Console and every shortcut.

- A ready-made `solis.exe` for Windows, built on GitHub whenever code is merged (Actions tab, **Windows EXE**). Release builds on Windows open without a console window.

- The Code panel (beside Library) and the Problems panel (beside Console), as in the mockup. They fill in during Phases 7 and 2. Saved layouts from before reset to the default once, to make room for them.

- Flying around the viewport: hold the right mouse button to look; while it's held, W A S D move, Q and E go down and up, the wheel changes speed (shown in the Camera box) and Shift goes faster. The wheel alone moves forward and back; middle drag slides the view. The camera and its speed are remembered.

- The editor window: a one-row top bar with menus, the scene tab, and Play, Stop, Build and Build & Play (which say which phase brings them).
- Dockable panels: Viewport, Game, Scene, Layers, Properties, Library and Console. Drag a tab to move a panel; the layout is remembered next time. Window > Layouts has Default, Wide screen and Small screen.
- The viewport draws a sky, a ground grid that fades with distance, and the red X and green Y axes. Its corners hold the view menus (≡, Perspective, Lit, Show), the tools (Select, Move, Rotate, Scale, Transform; keys Q, W, R, S, T), grid, angle and camera speed, and the frame rate.
- The console lists messages, with filters by level and a search box.
- Window > Accent colour: petrol blue, red, green or plum.
