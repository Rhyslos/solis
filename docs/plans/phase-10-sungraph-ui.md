# Phase 10 plan: Sungraph UI (draft)

The working plan for Phase 10, game UI (ROADMAP.md, "Phase 10: Game UI"; it was the game-UI half of Phase 9 until [decision 0046](../decisions/0046-light-before-netcode.md) gave light a phase and effects and UI a phase each; [decision 0039](../decisions/0039-game-ui-is-drawn-in-solis.md)). Effects (Phase 9) have their own plan, and post-processing (bloom, grading) moved to light's Phase 8 plan; **Sharing with effects** below says where the two meet so nothing is built twice. Update this file as milestones are built: tick them off here and in the README tracker, and change a milestone's plan if its design changes.

> **Draft.** Every item under "Choices to make" is a question for the project lead, with a recommended answer. Nothing here is decided until it is answered and written up as a decision.

**Gate:** a health bar and a pause menu, drawn in Sungraph UI with their values tied to game code, are on screen. Tried by hand: open the UI sample, draw a change to the health bar in the canvas, press Play, watch the bar follow the player's health, press Pause, move through the menu with the keyboard, the mouse and a gamepad, and resume.

## Answers (the project lead, 2026-10-25)

Recommended for every choice except one, and one left open:

- **Canvas tools: number keys 1 to 7** (select, box, ellipse, line, pen, text, image), not Figma's letters, **plus Aventail's radial menu on E** (Aventail decision 0037: a ring of the tools around the pointer, the whole slice towards a tool is its target), adapted from Aventail's code (decision 0007). The rest of the canvas controls as recommended: wheel zooms, middle-drag pans, Shift keeps proportions, Alt draws from the centre, arrows nudge, Ctrl+D, Delete, Ctrl+G, Enter edits text, Esc stops.
- **Ties: a tie word on a screen's fields, as recommended; the keyword itself is still to be chosen** by the project lead (`tied` until then). Passing values in from code keeps working alongside.

The rest as recommended: Solis's own drawing (shader shapes, `lyon`, `fdsm` text); screens are Lust files in `assets/ui/`; elements named with a `name` word; a `Screen` component plus code shows them; pausing stops game time, not game code (`setGameSpeed`); the menu actions table (arrows, Enter/Space, Backspace, P; d-pad, A, B, Start); a screen can ask for the pointer; designed at 1920 × 1080 scaled by height; Inter plus a pixel font the project lead picks; glow and outline per element, pixelation for the whole UI; Text size 75–200% and a separate UI size.

## What Sungraph UI is, in plain words

- **A screen** (the HUD, the pause menu, a shop) is one file. Inside it is Lust: a `view` that says what's on the screen, the values it shows, and the small bits of code that work out text such as "15 / 20".
- **You draw it** in a canvas inside Solis: boxes, circles, lines, curves and text, placed on the screen and pinned to its corners or edges so it fits any screen shape. **Or you type it**: the canvas and the code are two ways of looking at the same file, and either one can be used at any time.
- **Solis draws every line and letter itself**, every frame, at the size of the screen it's on. Nothing is a picture, so nothing goes blurry at 4K, and effects such as glow or a pixel-art look apply to everything at once.
- **Values are tied to the game.** A tie says "this number follows the player's `Health.hp`". The bar's width is worked out from it, so the bar moves on its own; no game code has to "update the UI".
- **The UI only looks; it never touches.** It reads the game as the last tick left it. When the player clicks Resume, that click is sent to the game the way a key press is, at the next tick, so recordings, replays and (Phase 11) networked games stay exactly the same on every PC.

## Choices to make before any code

Each is a question for the project lead, with the recommendation first. Those marked **(controls)** add keys or mouse actions and must be asked (CONTRIBUTING rule 17).

1. **How Solis draws shapes and text underneath.**
   - **Recommended: Solis's own drawing, from small proven parts.** Boxes, rounded boxes, circles, rings and lines are drawn by a shader as exact shapes (each a "signed distance": the GPU works out how far every pixel is from the edge), so they're razor sharp at any size and changing a health bar's width costs one tiny update. Free curves (the pen tool) are cut into triangles once by `lyon` and kept until they change. Letters come from the font's own outlines as distance images (multi-channel signed distance fields, made by `fdsm`), so text stays sharp at any size and outlines and glow on text are almost free. Works on every graphics card Solis already runs on, including the software driver used in tests.
   - *Alternative: Vello* (Linebender's GPU vector renderer, 0.11, which already uses Solis's wgpu 30). The best quality and the most features (gradients, blurs, clipping) for the least code, but it needs compute shaders, re-encodes the picture each frame, and ties Solis's wgpu version to three projects instead of two (egui, Vello, Solis); its software-driver support is unproven.
   - *Alternative: femtovg* (canvas drawing in the style of HTML's canvas, 0.27, wgpu 30). Simpler than Vello, but a smaller project with its own text system, and less control over caching.
2. **What a screen's file is.**
   - **Recommended: an ordinary Lust file (`.lst`) in the project's `assets/ui/` folder**, holding one `assembly … like Screen` with a `view`. It's real Lust, so the checker checks it, the Code panel opens it, and it reads like the rest of a game. The Library shows it with a UI icon and opens it in the canvas.
   - *Alternative:* a new kind of file (`.solui`) written Lust-style like areas (decision 0020): simpler for Solis to read and write, but not real Lust, so the "code behind every element" promise of decision 0039 is lost.
3. **How a value is tied to the game.** Ties need a new word in Lust (the project lead's language).
   - **Recommended: a `tied` word on a screen's fields**, with the field's starting value used as the preview in the canvas:
     `float hp = 20 tied "Player".Health.hp;` ("this number follows the `hp` of the `Health` on the entity called Player"). `me` ties to the entity the screen belongs to (choice 5): `tied me.Health.hp`. A method works too: `tied "Player".Health:fraction()`.
   - *Alternative:* the Lust way for windows today: a component passes values in (`Hud { hp = hp };` inside the component's own view). No new word, but every tie needs game code, and the canvas can't make ties on its own.
   - *Alternative:* other wording: `follows`, `from`, `<-`.
4. **How elements are named,** so code, the canvas and Git can find each one.
   - **Recommended: a `name` style word,** `box [name healthFill, …];`, unique within the file. The canvas gives new elements names like `box3`, renamed in Properties. Readable in code and in Git.
   - *Alternative:* random IDs as areas use (stable, but unreadable in code); or no names (elements found by their place, which breaks when one is moved).
5. **How a screen gets on screen.**
   - **Recommended: both a built-in `Screen` component and code.** Add a `Screen` component to an entity (the player, or a "Game" entity) and pick the file: the screen shows while the entity exists and `Screen.shown` is on, and `me` in its ties means that entity. Game code can show and hide it by setting `Screen.shown`, like any other field, so which screens are open is part of the game: saved, replayed and the same on every PC.
   - *Alternative:* only from code (`show("ui/pause")`), or a list of screens in the project's settings.
6. **What "paused" means.**
   - **Recommended: pausing stops game time, not game code.** A `setGameSpeed(0)` native (`1` is normal speed, `0.5` slow motion): time, physics, projectiles and `dt` stop, but components still run each tick, so they can hear "Resume" and react. Simple to explain, and replays stay exact.
   - *Alternative:* everything stops and only the UI runs. Then the Resume button needs a special path around the game, and anything animated on the pause screen that comes from game code freezes.
7. **(controls) Moving through menus.** New ready-made actions in `input.solinput` (decision 0037), rebindable like the others:

   | Action | Keyboard | Gamepad |
   |---|---|---|
   | UI Move (up, down, left, right) | arrow keys | d-pad and left stick |
   | UI Choose | Enter and Space | A |
   | UI Back | Backspace (Esc in exported games) | B |
   | Pause | P (Esc in exported games) | Start |

   **Recommended: the table above.** Esc stays the editor's while playing in the editor (decision 0037), which is why Back and Pause use other keys there. While a menu has the focus, the game's own actions are held back, so pressing A to choose doesn't also jump.
   *Alternatives:* W A S D also moves through menus; Tab and Shift+Tab move between buttons (as in Lust's windows) as well as the arrows.
8. **(controls) The mouse in menus.**
   - **Recommended: a screen can ask for the pointer** (`view [pointer]`). While one is shown, the game gives the pointer back (visible and free inside the Game panel), clicks go to the UI first, and clicks on nothing go to the game. When it closes, the game holds the pointer again.
   - *Alternative:* the game draws its own pointer, moved by the mouse or a stick; or menus are keyboard and gamepad only.
9. **(controls) The canvas's keys and mouse.** Only while the canvas has the focus:
   - **Recommended:** tools Q select (as in the viewport), R box, O ellipse, L line, P pen, T text, I image (Figma's keys); wheel zooms towards the pointer and middle-drag pans (like the viewport's middle-drag); drag to draw, Shift keeps proportions (squares, circles, 45° lines), Alt draws from the centre; arrows nudge by 1, Shift+arrows by 10; Ctrl+D duplicates, Delete removes, Ctrl+G groups; Enter on a text element edits it, Esc stops.
   - *Alternatives:* number keys 1 to 7 for the tools (no clash with the viewport's R and T); no tool keys at all (buttons only); Figma's navigation (wheel scrolls, Ctrl+wheel zooms, Space+drag pans).
10. **The screen size you design for.**
    - **Recommended: design at 1920 × 1080, scaled with the screen's height,** so a 1280 × 720 screen shows everything at two thirds and a 21:9 screen gets more room at the sides, where anchored elements move out to the edges. The canvas previews 1920 × 1080, 1280 × 720, 2560 × 1440, 3440 × 1440 (ultrawide), 1280 × 800 and 1024 × 768 (4:3), with a "safe area" guide.
    - *Alternatives:* scale to fit the width; or no scaling (sizes in real pixels, bigger screens see smaller UI).
11. **The fonts a new project gets.**
    - **Recommended: Inter** (already in Solis, OFL) **and one pixel font** for pixel-art games (an OFL font the project lead picks). Any `.ttf` or `.otf` in the project can be used too.
    - *Alternative:* Inter only.
12. **Styles over one element as well as the whole UI.**
    - **Recommended: glow and outline can be set on the whole UI and on any element; pixelation on the whole UI only** (it has to line up with the screen's pixels).
    - *Alternative:* the whole UI only, as decision 0039 first describes.
13. **Text size for players** (accessibility).
    - **Recommended: a player setting, Text size, from 75% to 200%,** kept with their rebinding, which grows text and lets rows and columns make room; and a separate UI size. Game code can read both.
    - *Alternative:* UI size only (everything grows together).

Once answered, these become decisions (likely three: the file and ties, choices 2 to 5; how it's drawn, choices 1 and 12; and the controls, choices 7 to 9) and a line each in DESIGN.md §17.

## Where things stand

- **No game UI yet.** The Game panel draws the world through the first camera into its own image (`ViewportView::image`, `GAME_IMAGE`), and egui writes the play state's notes over it (`editor/src/game_panel.rs`).
- **The picture is finished in one last pass:** the world is drawn in HDR, then `post_fs` (`render/src/viewport.wgsl`) applies exposure and tone mapping into an 8-bit image. The editor's own lines are never tone mapped (decision 0017); game UI follows the same rule and is drawn after.
- **No bloom or screen passes yet.** Bolts glow by being brighter than white, but nothing spreads the glow. The stylized look's outlines draw each model again from behind (an "inverted hull"), not as a pass over the screen. Both are the effects plan's to build, and the UI shares them.
- **Input is named actions** (`input` crate, `input.solinput`, one `Frame` sampled per tick, recordings). There are no menu actions, and no way for anything but keys and buttons to put something into a tick.
- **Lust already has views,** in its own windows (LUST.md §17): `column`, `row`, `stack`, `scroll`, `space`, `text`, `button`, `input`, `checkbox`, `slider`, `image`, `every`, styles in `[ ]` like Tailwind (`pad 16`, `gap 8`, `width fill`, `round 12`, `background blue`), pieces of your own (assemblies used inside views), and a test mode. Lust's Lua output already turns a view into a tree of tables (`__view()`, one `{k, t, s, c}` per piece), which is exactly what Solis can read from Luau. Lust draws its windows on the CPU (fontdue, softbuffer), so Solis can't use that renderer in its GPU frame, but it keeps Lust's **layout rules**, so a view lays out the same in both.
- **Lust's checker is strict about view words** (`check_styles`), so new drawing words and `tied` are changes in the Lust repository, done by Claude as in Phase 4.
- **Game code** runs as Luau on every core and reads the world as the tick started (decision 0033). There's no native to read another component's field: ties need one (in Rust, not Luau; below).
- **Fonts:** egui already brings `skrifa` 0.44 and `harfrust` (font reading and shaping) into the build, and Inter and JetBrains Mono are in the editor's assets with their licences.

## The UI reads, never writes

What keeps decision 0033 (and Phase 6's determinism) whole:

- **When a screen reads:** ties are read once per tick, after the tick lands, from the world as it then stands: the same world game code will see at the start of the next tick. Between ticks the values don't change, so the UI isn't worked out again unless the screen size, the pointer or the focus changed.
- **Ties to fields are read by Rust,** straight from the world (`Game::values` for Lust and Lua components, the area values for built-in ones), with no Luau call. The entity is found by name once and found again only when entities are added or removed.
- **Ties to methods and the screen's own methods** (formatting "15 / 20") run in a **UI Luau of their own**, never in the game's. It holds a copy of the values they read; anything they change in that copy is thrown away. Only reading natives are there (`time`, `positionOf`, `nameOf`, `held`…); `moveTo`, `send`, `spawn`, `fire`, `random`, `saveGame` and the like aren't, and calling one says so in plain words.
- **The checker helps:** in a `like Screen` assembly, assigning to a `tied` field is a mistake, and `get "solis/entity":[moveTo]` in a screen file is a mistake that says "a screen can't move things; send an event instead".
- **How the UI changes the game:** a button's block may (1) change the screen's own untied fields (which tab is open: the screen's own memory, as in Lust's windows), (2) `send` an event to an entity, or (3) `press` a named action. (2) and (3) become **UI events in the next tick's input `Frame`**: recorded in recordings, counted in its checksum, and in Phase 8 sent over the network like any input. A replay takes them from the recording, not from the UI, so a replay plays the same with no UI at all.
- **What isn't game state:** hover, focus, the pointer's place and transitions (a button fading in) live only in the UI and use real time. They never reach the game except through (2) and (3).

## Sharing with effects (Solis Eclipse and post-processing)

The effects plan builds bloom, colour grading and stylized effects in the same phase. To build each thing once:

| Shared piece | Used by effects for | Used by UI for | Built in |
|---|---|---|---|
| **A chain of screen passes** (`render::post`): blur by shrinking and growing the image (the usual "dual filter" bloom), applied to any HDR image with a threshold and strength | bloom on the world | glow on the UI layer | the effects plan's bloom milestone; UI M6 reuses it |
| **Pixelate:** draw at 1/N size, enlarge with hard edges, optional palette | a pixel-art look for the world | pixelating the UI at the **same pixel size**, so their pixels line up | whichever comes first; one "pixel size" setting in the project's look |
| **Outline from a mask** (a jump-flood pass: finds every pixel's distance to the nearest edge in a few steps) | outlines around effects and selected things | an outline around the whole UI, or a group | UI M6 if effects don't need it sooner |
| **Shape shader functions** (rounded box, circle, ring, line as distances, in one WGSL file) | stylized particles: rings, shockwaves, sparks without textures | every UI shape | UI M1 |
| **Pictures from the project** (upload, atlas) | flipbooks | `image` | whichever comes first |
| **Easing curves** (ease in, out, bounce; a small shared type) | values over a particle's life | transitions | whichever comes first |

**The order of the finished picture,** agreed by both plans: world (HDR) → bloom → tone mapping → colour grading → **UI layer** (drawn into its own image, with its own glow and pixelation) → laid over the result. The UI is never tone mapped or colour graded, so its colours are exactly what was picked.

## Safety (CONTRIBUTING rules 8 to 10)

- **Limits:** a screen file is read within 1 MB; at most 10,000 elements and 64 levels deep per screen, 10,000 points per curve, 64 KB of text per element; the glyph atlas at most 4096 × 4096 (least recently used letters leave first); fonts within 32 MB and images within the asset limits.
- **Files read get fuzz tests:** screen files (Solis's reader), fonts (skrifa on damaged files), saved player settings (text size), and the ties' paths.
- **No panics reach the frame:** a tie that gives NaN or infinity, a size below zero or a curve that crosses itself is drawn as nothing (or the last good value) with one message, never a crash. lyon is given only finite, capped points.
- **Time limits:** the UI Luau has a memory limit and a deadline (4 ms for all of a frame's methods); past it the screen shows its last values and the Console says which method was slow.
- **The frame never waits:** fonts are read and letters made into distance images on a worker; until a letter is ready it's drawn from a smaller size, so text appears at once.

## The shape of the code

| Crate | Layer | What it gains | Milestone |
|---|---|---|---|
| `ui` (new) | 1 | A screen as a tree of elements; reading and writing the drawable part of a screen file with each element's place in the text; layout (anchors, rows, columns, stacks, with `taffy`); shapes as instances and curves tessellated (`lyon`), cached; text laid out (`parley`) with letters as distance images (`fdsm`); hit testing and focus moving. No GPU, no window | M1–M5 |
| `render` | 1 | `ui.rs` and `ui.wgsl`: the UI layer drawn in one or two draw calls, after tone mapping; the styles (sharing `render::post`) | M1, M6 |
| `script` | 1 | The UI Luau (reading natives only); a view's tree read from Luau; ties' paths resolved; `solis/ui.lst` (`press`, `setGameSpeed`, `gameSpeed`, `textScale`) | M2, M3 |
| `input` | 1 | The UI actions; UI events in `Frame` and recordings; game actions held back while a menu has the focus | M3 |
| `area` | 0 | The built-in `Screen` component (file, shown, order on screen) | M2 |
| `editor` | 2 | The canvas tab, the element list, Properties for elements, the tie picker, undo for screens, the Game panel's pointer for menus | M2–M5 |
| `app` | 3 | Screen files read and saved, fonts and pictures loaded, the UI sample (Help > Make the UI sample…) | M2, M7 |

Crate candidates (all allowed by `deny.toml`; versions as of 2026-10-25):

| Crate | Version | Licence | For | Why |
|---|---|---|---|---|
| `lyon_tessellation` | 1.0.23 | MIT or Apache-2.0 | free curves into triangles | the standard in Rust for ten years; done once per shape and kept |
| `taffy` | 0.15 | MIT | layout | flexbox and grid, used by Bevy, Dioxus and Zed; Lust's rows, columns, `fill`, `grow` and `gap` map onto it, and anchors onto its absolute placing |
| `parley` | 0.12 | Apache-2.0 or MIT | text layout | line breaking, right-to-left text and shaping (harfrust and skrifa inside), from the Linebender group |
| `fdsm` | 0.8 | MIT | letters as distance images | pure Rust multi-channel distance fields: sharp letters at any size, outlines and glow in the shader |
| `skrifa` | 0.44 (already in the build) | MIT or Apache-2.0 | reading fonts | the same one egui uses |
| `etagere` | 0.3 | MIT or Apache-2.0 | packing letters into the atlas | used by glyphon and WebRender |
| *considered:* `vello` | 0.11 | Apache-2.0 or MIT | everything | see choice 1 |
| *considered:* `femtovg` | 0.27 | MIT or Apache-2.0 | everything | see choice 1 |
| *considered:* `cosmic-text` + `glyphon` | 0.19, 0.12 | MIT or Apache-2.0 (glyphon also Zlib) | text | good, but letters as bitmaps at each size, so outlines and glow on text would need extra passes; parley is lighter for Solis's use |
| *considered:* `tiny-skia` | 0.12 | BSD-3-Clause | CPU drawing | too slow for a HUD redrawn at 4K each frame, but useful for thumbnails of screens in the Library |

## How others do it (what Solis takes from each)

- **Figma:** constraints (left, right, left-and-right, centre, scale) pin a layer to its frame's edges; auto layout makes rows and columns. Solis's anchors and Lust's rows and columns are the same two ideas.
- **Rive:** "view models" hold the values a design shows, and bindings link them to properties; code sets the values. Solis's `tied` fields are the view model, written in the screen itself.
- **Unreal UMG:** binding a property to a function polls it every frame, which is known to be slow; the newer Viewmodel plugin updates widgets only when a value changes ("field notify"). Solis reads ties once per tick in Rust and redraws only what changed, which gets the second without asking game code to notify.
- **Unity UI Toolkit:** UI is a tree described in a file (UXML) with styles in another (USS), bound to data at runtime; it's retained (kept between frames), not rebuilt. Solis keeps one file, and also keeps its tree.
- **Noesis:** vector UI from XAML files with data binding, drawn by the GPU at any size: the closest to Sungraph's idea, but proprietary.

## Milestones

### M1. The drawing layer: shapes and text, sharp at any size

The Rust layer DESIGN.md §12 asks for first, which Lust's `view` is read into later.

- The `ui` crate: elements `box` (with `round`), `ellipse`, `line`, `path` (straight and curved pieces), `text` and `image`, grouped by `stack`, `row` and `column`; fill, border, opacity, rotation. Placed by **anchors** (a point or an edge of the parent: top left, top, …, stretch left-to-right) plus an offset, or laid out by rows and columns with Lust's rules (`pad`, `gap`, `fill`, `grow`, `spread`, `center`). Design size and scaling as choice 10.
- **Drawing:** boxes, ellipses and lines as instances (one 64-byte record each) drawn by the shape shader; curves tessellated by lyon and kept until they change; text laid out by parley, letters as distance images in an atlas made on a worker. Everything in one vertex buffer, in drawing order, in one or two draw calls.
- `render` draws the layer over the Game panel's picture after tone mapping (the "order of the finished picture" above), at the image's own size.
- A screen can be built in Rust for now; the Game panel shows a test HUD from a Console command (`ui test`), so it can be seen before Lust can make one.
- Tests: layout at every preview size (anchored elements keep their distance to their edge; stretched ones grow); a 1-pixel line and 8-point text drawn at 4K and at 720p (GPU pixel tests, also on the software driver); nothing re-tessellated when only a box's width changes; a curve with NaN or a million points refused; the atlas fills and frees; fuzz the font reader. Scale case: 1,000 elements with 200 texts changing each tick, CPU and GPU times in the Profiler.

### M2. Screens in Lust, with values tied to the game

- **In Lust** (a pull request in the Lust repository, then Solis pins it): the drawing words (`box`, `ellipse`, `line`, `path`; `name`, `anchor`, `at`, `rotate`, `glow`, `outline`, colours with transparency; a shape is filled with Lust's own `background`), `tied` fields, `like Screen`, the screen rules in the checker (above), and a trailing comma allowed in `[ ]` so each style can sit on its own line (see Saving and Git). Lust's existing words keep their meanings (`size` is text size, `background`, `round`, `border`, `width`, `height`, `opacity`).
- **A screen file** (example below) is checked with the rest of the game's code; its mistakes go to Problems with file and line.
- **Reading a view:** when the program is loaded, each screen's view runs in the UI Luau and its tree of tables becomes the `ui` tree, kept between frames: an element is matched by its `name` (or its place, inside loops by the row's key, as in Lust), so only what changed is laid out and drawn again.
- **Ties:** resolved by Rust after each tick (above); methods in the UI Luau.
- **The `Screen` component** (choice 5) puts a screen in the Game panel; several are drawn in their `order`.
- In the editor (before the canvas exists), a screen file opens in the Code panel and shows in the Game panel while playing; new code is swapped in while playing, like components (decision 0028).
- Tests: a tied field follows a component's value tick by tick; a method tie gives the same text on 1 core and 4; a method that tries to change a value changes only the UI's copy; `moveTo` in a screen is a mistake; the view is read again only after a tick or a resize; a tie to a missing entity or field shows the preview value and one message; round trip of the Lust (Luau and, when Phase 10 allows, Rust). **First half of the gate:** a health bar, written by hand, follows the turret sample's drone's `Health.hp`.

### M3. Menus: clicks, focus, gamepads and pausing

- **Buttons:** any element can be a `button` (a block that runs when chosen); hover and pressed looks (`hover:[ … ]`, `pressed:[ … ]`), and `disabled`.
- **Focus:** the first button of a screen that takes focus gets it (or the one marked `focus`); UI Move goes to the nearest button in that direction (or the one named with `next up healthButton`); a focus ring drawn by Solis, styled in the screen.
- **The actions** of choice 7, sampled with the others; while a screen that takes focus is shown, the game's own actions are held back.
- **The pointer** (choice 8): a screen with `[pointer]` frees it inside the Game panel; clicks hit the frontmost element under it.
- **UI events into ticks:** `send` and `press` from buttons become part of the next tick's `Frame`, recorded and replayed.
- **Pausing** (choice 6): `setGameSpeed`, `gameSpeed` natives; the Game panel's corner note says "Game speed 0".
- Tests: spatial focus picks the expected button in grids and odd layouts; a click and a gamepad A choose the same button; a replay with menu clicks matches its checksums with no UI drawn; the game doesn't jump when A chooses; editor keys stay off; damaged recordings with UI events are refused (fuzz). **Second half of the gate, by code:** a hand-written pause menu pauses, resumes and quits to a "Paused" note.

### M4. The canvas, part 1: selecting, moving and Properties

- **A screen opens as a document tab** (decision 0004) with the **canvas** in the centre in place of the viewport: the screen drawn by the real renderer at a preview size (choice 10), with the safe-area guide, panned and zoomed (choice 9).
- **Selecting:** click, Shift-click, drag a box; handles to move and resize; snapping to edges, centres and other elements, with guides.
- **The element list** (like the Area panel): the tree in drawing order, drag to reorder or regroup, rename, hide and lock while editing.
- **Properties for elements:** name; place, with an **anchor picker** (a 3 × 3 grid plus stretch); size; fill, border, corners; text, font, size; opacity, rotation.
- **Undo** for every change, merged per drag (decisions 0022 and 0025).
- **Saving changes the least text it can** (below): what the canvas doesn't understand is kept exactly as written. A screen changed on disk (in the Code panel or another editor) reloads within a second.
- **Code and canvas:** "Open as code" jumps to the selected element's line; a part of the view the canvas can't edit (a loop, a question, an expression) is drawn from the preview values and shown with a "code" badge, selectable but edited in code.
- Tests: drawn with egui_kittest as the other panels: selecting, dragging (one undo step), anchors at every preview size; saving after a move changes only that element's lines; a file with comments and hand-written code survives a hundred canvas edits unchanged elsewhere; fuzz the reader.

### M5. The canvas, part 2: drawing tools and ties

- **Tools** (choice 9): box, ellipse, line, pen (click for corners, drag for curves, handles to reshape), text (click to type), image (from the Library, with fill or fit).
- **The tie picker:** a small chain button beside every value in Properties. It lists the screen's fields, and to make a new tie: an entity (by name, or "the screen's own entity"), then one of its components from the registry, then a field or method that gives the right kind of value. It writes the `tied` field and uses it, for example `width 420 * hp / maxHp`, with a simple "fills a bar" helper for the most common case.
- **Preview values:** each tied field's starting value is what the canvas shows; a slider beside it tries others without saving them, so a designer can see the bar at 10% without playing.
- Tests: each tool makes the expected Lust; the picker lists only fields of the right kind; a tie made in the canvas follows the game in play; preview sliders never change the file.

### M6. Styles, and text size for players

- **Glow, pixelation and outlines** over the whole UI and per element (choice 12), with `render::post` shared as described above; text outlines and glow in the text shader at no extra pass.
- **The screen's look** in the canvas's Properties when nothing is selected: glow strength and spread, pixel size (matching the world's if the project's look has one), outline width and colour.
- **Accessibility basics** (choice 13): Text size and UI size as player settings (natives to read and set them, a menu can offer them), rows and columns making room; the canvas previews at 100%, 150% and 200%.
- Tests: GPU pixel tests of each style; pixelated UI lines up with the world's pixels; 200% text wraps instead of spilling out; the styles' cost in the scale case.

### M7. The UI sample, and the manual

- **Help > Make the UI sample…**: a project with the turret sample's drone and a player, a HUD (health bar, ammo, score) and a pause menu (Resume, Text size, Quit), drawn in the canvas with ties, and a few lines of Lust for pausing.
- **The manual:** Sungraph UI (what it is, drawing a screen, anchors and screen sizes, tying values, menus and gamepads, styles, the screen file as code, in Lust, Rust and Lua where game code is shown), new terms in concepts.md (screen, anchor, tie, focus), keyboard shortcuts.
- Tests: the sample plays the same from a recording, menus included.

### End of the phase

- The review (decision 0027): three reviewers, the fixes in pull requests by kind, the scale check with the UI cases added (`tools/scale/scale-check.sh`).
- The gate, tried by the project lead on their PC.

## A screen file

How the HUD might look (the words depend on choices 3 and 4):

```
-- ui/hud.lst: drawn in Sungraph UI. Safe to edit by hand: the canvas keeps
-- everything it doesn't understand exactly as written.
assembly Hud like Screen {
    has {
        float hp = 20 tied me.Health.hp;         -- 20 is what the canvas shows
        float maxHp = 20 tied me.Health.maxHp;
        int ammo = 12 tied me.Turret.ammo;
    }

    method string hpText() {
        return Maths:floor(hp).text() + " / " + Maths:floor(maxHp).text();
    }

    view [design 1920 1080] {
        stack [
            name healthBar,
            anchor bottomLeft,
            at 48 -48,
            width 420,
            height 36,
        ] {
            box [name healthBack, width fill, height fill, background gray9, round 8,];
            box [name healthFill, width 420 * hp / maxHp, height fill, background red5, round 8,];
            text hpText() [name healthText, center, middle, size 22, bold, white, outline 2,];
        }
        text "Ammo " + ammo.text() [name ammoText, anchor bottomRight, at -48 -48, size 32,];
    }
}

pass Hud;
end
```

And the pause menu's buttons, which only send:

```
column [name buttons, anchor center, gap 16,] {
    button "Resume" [name resume, focus,] { press("Pause"); }
    button "Quit" [name quit,] { send QuitToMenu() to me; }
}
```

## Saving and Git

- **The canvas edits the text, not a copy of it.** Solis's reader keeps where each element and each style word is in the file; a change replaces just those characters, and a new element is written in the canonical shape. Comments, blank lines and hand-written code are never rewritten.
- **The canonical shape is made to merge** (decision 0020's tests): one element per block; an element with more than three styles writes one style per line, each ending with a comma (hence the trailing comma in M2), so two people changing different properties of the same element change different lines. Names, not positions, find elements, so moving one up the list is a small change.
- **Drawing order is file order** (later in front, as in Lust's `stack`): reordering moves a block, which Git handles.
- `.gitattributes` marks `assets/ui/**/*.lst` `text eol=lf`, like areas. The Phase 10 merge tool can learn screens (element by element, by name) later; plain Git is enough for the gate.
- Saving writes a temporary file and swaps it in (rule 11).

## Risks

- **Lust changes:** the drawing words, `tied` and `like Screen` are changes to the project lead's language, so their wording waits for answers. M1 doesn't need them, so the work can start before they're settled.
- **Built games** (Phase 10) run Lust as Rust, whose views today open their own window. Lust's Rust output will need to hand a screen's tree to Solis instead, as its Lua output does. Noted for Phase 10, and designed for now by keeping "a view gives a tree of pieces with values" as the only contract.
- **Editing code with a canvas** is the hardest part. Limiting the canvas to the parts it understands (the "code" badge) keeps it safe; M4 may need splitting in two.
- **Small text from distance images** can look soft below about 12 pixels. If it does, small sizes are drawn from ordinary letter images (still from the font's outlines, so still sharp).
- **The effects plan's order:** if bloom lands after M6 is due, M6 builds `render::post` itself and the effects plan reuses it.
- **Scale:** health bars over hundreds of units (an RTS) are elements placed in the world, not on the screen. They're not in the gate; the instance-based shapes are designed so they can be added without a new renderer.

## Sources

- Vello 0.8 release notes (wgpu 28, March 2026) [skia.googlesource.com](https://skia.googlesource.com/external/github.com/linebender/vello/+/refs/tags/v0.8.0%5E%21/); Vello 0.11 depends on wgpu 30 ([crates.io](https://crates.io/crates/vello)); Vello Hybrid's README [skia.googlesource.com](https://skia.googlesource.com/external/github.com/linebender/vello/+/bf5b0f968356bec23fefe76e715e2f91502893eb/sparse_strips/vello_hybrid/README.md).
- Versions and licences from crates.io: [lyon_tessellation](https://crates.io/crates/lyon_tessellation), [taffy](https://crates.io/crates/taffy), [parley](https://crates.io/crates/parley), [fdsm](https://crates.io/crates/fdsm), [cosmic-text](https://crates.io/crates/cosmic-text), [glyphon](https://crates.io/crates/glyphon), [femtovg](https://crates.io/crates/femtovg), [tiny-skia](https://crates.io/crates/tiny-skia), [etagere](https://crates.io/crates/etagere), [skrifa](https://crates.io/crates/skrifa).
- Rive data binding: [rive.app/docs/editor/data-binding/overview](https://rive.app/docs/editor/data-binding/overview).
- Unreal UMG Viewmodel and field notify: [dev.epicgames.com](https://dev.epicgames.com/documentation/en-us/unreal-engine/umg-viewmodel-for-unreal-engine).
- Unity UI Toolkit: [introduction](https://docs.unity.com/en-us/engine/6000.0/manual/uitoolkits/uielements/introduction-ui-toolkit), [data binding](https://docs.unity.com/en-us/engine/6000.0/manual/best-practice-guides/bpg-uiad-index/data-binding).
- Noesis GUI: [noesisengine.com](https://www.noesisengine.com/noesisgui/).
- Figma constraints: [help.figma.com](https://help.figma.com/hc/en-us/articles/360039957734).
- Lust's views: LUST.md §17 and `src/lua.rs` (`__view`) in the Lust repository; Solis: `crates/editor/src/game_panel.rs`, `crates/render/src/viewport.wgsl`, `crates/script/src/describe.rs`, `crates/input/src/frame.rs`.
