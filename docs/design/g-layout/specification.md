# Engine Editor — G Layout: UI Specification
*Copied from the project lead's document of 2026-11-11; two comparisons to another product were reworded, CONTRIBUTING rule 20.*<br>
Nov 11, 2026 · @Rhy

## Overview

The G layout is one editor shell — a ribbon on top, editor groups in the middle, a tabbed shelf at the bottom, a status bar — reused by every area of the engine. Switching ribbon tabs is how the user moves between areas; there is no permanent left-hand panel. Everything is dark, neutral gray (no warm or cool tint), with one accent color for selection and focus.

The design lives on the canvas **Engine Editor — G Layout** as thirteen 1440 × 900 desktop screens. Each screen below is described panel by panel: what it shows, how it behaves (built or intended), and the exact visual details (sizes in px, hex colors).

| # | Screen | Ribbon tab | Center area | Shelf opens on |
| --- | --- | --- | --- | --- |
| 1 | Level editor | Home | 3D viewport + code editor (code editor is a later feature) | Object |
| 2 | Level design (placing) | Home (build mode) | Full-width 3D viewport, drag in progress | Place |
| 3 | Library | Library | Full-width asset grid | Details |
| 4 | Material | Shading | Node graph + live preview | Parameters |
| 5 | Sequencer | Animate | Camera preview + curve editor | Sequencer |
| 6 | Profiler & console | Debug | Frame-time graph, frame timeline, statistics | Console |
| 7 | Recent projects | File | Full-page File view (no ribbon) | — |
| 8 | Preferences | File | Full-page File view (no ribbon) | — |
| 9 | Model viewer | Library › Mesh tools (contextual) | Large model viewport + UV layout | Overview |
| 10 | Audio mixer | Audio | Mixer console + bus effect (EQ) | Sound events |
| 11 | VFX editor | VFX | Effect preview + emitter stack | Module |
| 12 | Project settings | File | Full-page File view, General tab | — |
| 13 | Input & keybinds | File | Full-page File view, Project settings › Input | — |

**How to read this spec.** “Built” means the mockup already behaves that way (only the Level editor’s shelf tabs are clickable). Everything else marked “intended” is the planned behavior for the real engine. Sample names such as Harbor\_Prototype, Docks\_Night and Crane\_Arm\_02 are placeholder content, not features.

**Coding happens through Ledger at first.** The built-in code editor shown on the Level editor is a planned feature, not part of the first release. Until it ships, game logic is written through Ledger; Project settings › Scripting is set to “Ledger”, and the code group on the Level editor stays hidden unless the feature is turned on. Where this spec describes the code editor, read it as the target design for that later feature.

## Visual design system

The whole editor uses pure neutral grays (R = G = B), never black (#000000) and never tinted. Depth comes from lighter surfaces, not shadows: the further “forward” a surface sits, the lighter its gray. One accent color marks selection, the active tool and focus; every other color carries a fixed meaning (axis, asset type, data type, status).

### Surfaces and lines

| Token | Hex | Used for |
| --- | --- | --- |
| App background | #0E0E0E | Window root, tab bar, segmented-control wells, dark text on accent fills |
| Field | #121212 | Text inputs, number fields, console background |
| Shelf bar / graph | #141414 | Shelf tab bar, node-graph background, File views |
| Code / status | #161616 | Code editor group, status bar, column header strips |
| Ribbon | #181818 | Ribbon body and the active ribbon tab |
| Panel | #1A1A1A | Shelf columns (scene list, inspector, assets) |
| Viewport | #1C1C1C | 3D viewport background |
| Card | #1F1F1F | Component cards inside the shelf |
| Group divider | #1E1E1E | 1 px gap between side-by-side editor groups |
| Hover | #242424 / #262626 | Hovered buttons and rows; #262626 is also the active shelf tab |
| Line | #262626 | All panel borders and 1 px separators between shelf columns |
| Field border | #2C2C2C | Inputs; cards use #2A2A2A, floating pills #2E2E2E |
| Strong line | #333333 / #3A3A3A | Dashed “add” buttons, chips, shelf drag grip |

### Text

| Role | Hex | Examples |
| --- | --- | --- |
| Emphasis | #FFFFFF | Selected row text, active tab, object name in shelf header |
| Primary | #E6E6E6 | Values, headings, field contents |
| Body | #C8C8C8 | Tree rows, list rows, ribbon button labels |
| Label | #A6A6A6 | Field labels, inactive tabs, breadcrumbs |
| Meta | #8C8C8C | Counts, types, hints, ribbon group labels |
| Faint | #6E6E6E / #5E5E5E / #5A5A5A | Log timestamps / code line numbers / path separators |

All text meets at least 4.5:1 contrast on its surface; #8C8C8C is the lowest gray used for readable text.

### Accent and meaning colors

| Group | Colors | Rule |
| --- | --- | --- |
| Accent (user choice) | #5C9CFF default; options #D4D4D4 (no color), #F2994A, #4FC2A8, #B48CFF | Selection, active tool, active tab underline, focus ring, toggles on, playhead, progress |
| Selection fill | accent mixed 16–24% over the surface | Selected rows and tiles; text turns #FFFFFF |
| Links | #7FB2FF, hover #A9CBFF | Inline links (“Open Library”, “Bake now”, file:line) |
| Status | success #4FC28A, warning #E8B04A, error #F06A6A (error text #F08A84) | Status dots, log levels, problem icons; always paired with a word or icon |
| Axes (gizmo) | X #E5534B, Y #6CC24A, Z #4C8DFF | Gizmo arrows, axis widget |
| Axes (field letters) | X #F08A84, Y #8FD47A, Z #7FB2FF | Lighter versions for small letters in number fields |
| Asset types | Folder #C9A35A, Mesh #5FC4D6, Material #6CC28A, Texture #E57F6A, Animation #B48CFF, Audio #E8B04A, Prefab #7FB2FF | 7 × 7 px rounded square beside the type word |
| Node pin types | Float #9A9A9A, Vector3 #E8C95A, Color #C38CFF, Texture #E57F6A | Pin rings and the wires they feed; texture pins are square |
| Code syntax | keyword #8FB8F0, string #B8CF9A, comment #7A7A7A, number #E0B878, default #D4D4D4 | Script editor |
| Play | button #E6E6E6 with #0E0E0E label; record #F08A84 | The one light button on screen is Play |

Color never carries meaning alone: every colored dot or icon has a word next to it, and colors that must be told apart also differ in lightness.

### Type

| Use | Font | Size / weight |
| --- | --- | --- |
| UI default | Inter | 12 px / 400 |
| Ribbon tabs, shelf tabs | Inter | 12 px; active 500 |
| Ribbon button labels | Inter | 10 px under an 18 px icon |
| Ribbon group labels | Inter | 10 px, #8C8C8C, letter-spacing 0.04 em |
| Column / panel headers | Inter | 11.5 px / 600, #C8C8C8 |
| Rows, labels | Inter | 11.5 px |
| Numbers, paths, timecodes, shortcut keys | JetBrains Mono | 10.5–11 px |
| Code | JetBrains Mono | 11.5 px, line height 19 px |
| File-view page title | Inter | 22 px / 700 |
| Section caps (settings) | Inter | 10.5 px / 600, uppercase, letter-spacing 0.08 em |

### Sizes and spacing

| Element | Size |
| --- | --- |
| Tab bar | 34 px tall; tabs 30 px, 12 px side padding |
| Ribbon | 66 px tall; large button 50 × 44 px; small row 20 px |
| Editor tab strip | 30 px |
| Shelf bar | 34 px; drag grip 36 × 4 px, radius 2 |
| Shelf column header | 30 px |
| Tree / list rows | 22–24 px |
| Fields | 24 px tall, radius 5, 7 px side padding |
| Icon buttons | 26 × 26 px, radius 5; toolbox buttons 34 × 34 px, radius 7 |
| Floating pills (viewport overlays) | 24 px tall, radius 5, background #121212 at 85% |
| Status bar | 24 px |
| Radii | 4–5 px controls, 6 px tab tops and tiles, 8 px cards, 10 px project cards |
| Panel padding | 10–12 px; card gap 10 px; column separators 1 px |

### Icons and states

- **Icons:** inline line icons on a 16 px grid, 1.3–1.6 px stroke, current text color; 11–18 px on screen. No emoji. Icon-only buttons always have a text label for screen readers.
- **Hover:** background steps to #242424 (buttons) or #222222 (rows); text to #E6E6E6.
- **Selected:** accent-tinted fill + #FFFFFF text; tiles also get a 2 px accent inner outline.
- **Active tool / primary action:** solid accent fill with #0E0E0E icon or label.
- **Edited or focused value:** field border turns accent (for example Rotation Y = 35).
- **Toggles:** 28–32 × 16–18 px pill; on = accent with dark knob on the right, off = #333333 with #8C8C8C knob on the left.
- **Checkboxes:** 12–14 px rounded squares; on = accent fill, off = #3A3A3A outline.
- **Unsaved:** 6–7 px amber dot (#E8B04A) after a document name.

## Global shell

Every editor screen stacks the same five bands from top to bottom: tab bar (34 px), ribbon (66 px), center editor area (flexible, about 400–470 px), shelf (34 px bar + content, about 290–360 px), status bar (24 px). Only the File views replace the ribbon, center and shelf with one full page.

### 1. Tab bar (34 px, #0E0E0E)

- **Left:** app logo, a 20 × 20 px light-gray (#E6E6E6) rounded square with a dark cube outline, 8 px gap after it.
- **Ribbon tabs:** File · Home · Library · Shading · Animate · VFX · Audio · Debug · Build. Each is 30 px tall, bottom-aligned, 12 px Inter, #A6A6A6. The active tab is white text on #181818 (the ribbon’s color) with 6 px rounded top corners and a 2 px accent underline, so it reads as the tab of the ribbon below it.
- **Center:** command search, 24 px tall, up to 380 px (max 36% of the width), #181818 with a #262626 border, radius 6. Search icon, placeholder “Search or run a command”, a “Ctrl K” hint in mono on the right.
- **Right:** project and level path “Harbor\_Prototype / Docks\_Night” (project #A6A6A6, slash #5A5A5A, level #E6E6E6), then a collapse-ribbon chevron button.
- **Behavior (intended):** clicking a tab switches the ribbon and the whole workspace (center + shelf) to that area. Ctrl K opens a command palette that searches commands, assets, scene objects and settings. Collapsing the ribbon leaves only the tab bar, freeing 66 px.

**Contextual tabs.** When an asset opens in its own view, a temporary tab appears right after its parent tab, the way a set of picture tools appears in a ribbon only while a picture is selected. Example: opening a mesh from the Library adds “LIBRARY TOOLS · Mesh” after Library. It is styled as active, with a small gray caps prefix (9.5 px / 600) and a faint accent line along its top edge, and it disappears when the viewer closes.

### 2. Ribbon (66 px, #181818, bottom border #262626)

- Built from **groups**. Each group is a column: buttons on top, a 10 px gray label underneath (“Create”, “Snapping”…), groups separated by a 1 px #262626 line, 4 px top / 10 px side padding.
- **Large buttons:** 50 × 44 px, 18 px icon above a 10 px label, radius 6, hover #242424. The active one gets an accent-tinted fill and white text.
- **Small rows:** 20 px lines stacked two or three high, used for toggles (12 px checkbox + label, e.g. “Grid · 0.25”) and dropdowns (“Quality: Preview ▾”).
- **Play group** (editing tabs only) sits at the far right, pushed there by a spacer, with a separator on its left: **Play** (the only light button: #E6E6E6 fill, dark label, filled triangle) and **Simulate** (circle with triangle).
- Overflow: the ribbon scrolls sideways on narrow windows instead of wrapping.

### 3. Center editor area

- One or two **editor groups** side by side, separated by 1 px #1E1E1E. Each group has its own 30 px document tab strip on #0E0E0E: the active document tab is #1C1C1C with white text and a 1 px accent line on top; inactive tabs are #141414, #8C8C8C.
- Groups can split (viewport + code, graph + preview, camera + curves). Intended: drag a document tab to split or merge groups; a split remembers its ratio per workspace.

### 4. Shelf (bar 34 px + content)

- **Shelf bar** (#141414, borders top and bottom #262626): a 36 × 4 px drag grip on the left (#3A3A3A, radius 2), then shelf tabs (26 px tall, radius 6; active = #262626 fill, white, weight 500; inactive #A6A6A6), a spacer, and screen-specific tools on the right (transport controls, counts, hints).
- **Content:** columns laid side by side with 1 px #262626 gaps. Each column has a 30 px header (11.5 px / 600 title, optional gray count on the right) and #1A1A1A body.
- **Behavior:** shelf tabs switch the shelf contents (built on the Level editor). Intended: drag the grip to resize; Ctrl+Space collapses or expands it; each workspace remembers its last tab and height; selecting something opens the shelf on the tab set in Preferences (“Shelf opens on”).
- Columns wrap underneath each other on narrow windows.

### 5. Status bar (24 px, #161616, top border #262626, 11 px #A6A6A6)

- **Left:** green dot + “Ready”, “main · 4 changes” (branch and uncommitted count), error and warning counts as colored dots + numbers.
- **Right:** context text that changes per screen (cursor line and column, snap settings, placement status, recording state), then the engine version “v0.1.0” in mono.
- **Intended:** clicking the counts opens the Problems tab; clicking the branch opens version control.

## Screen 1 — Home · Level editor

The everyday editing screen: the 3D viewport and the selected object’s script side by side, with the scene list, object settings and assets in the shelf. It is the only screen where the shelf tabs already click (Object, Animation, Console, Problems).

### Ribbon (Home)

| Group | Controls | Intended behavior |
| --- | --- | --- |
| Create | Large: Mesh, Light, Camera, Volume (volume icon is a dashed square) | Adds that object type at the viewport center or at the mouse; opens a type submenu on long press |
| Snapping | Small: “Grid · 0.25” (on, accent box), “Angle · 15°” (off) | Toggles snapping; clicking the value edits the step |
| Lighting | Large: Bake (sun icon). Small: “Quality: Preview ▾”, “Light map view” | Bakes lighting at the chosen quality; light map view swaps the viewport shading |
| Project | Large: Save, Build | Save all; Build opens build targets |
| Play | Play (light button), Simulate | Play runs the game in the viewport; Simulate runs physics and scripts without the player |

### Center, left group: viewport (about 60% of the width)

- **Document tabs:** “Docks\_Night.level” (active), “Docks\_Day.level”. Several levels can be open at once.
- **Toolbox** (44 px column, #141414, right border #262626), 34 × 34 px buttons, top to bottom: Select, **Move (active: accent fill, dark icon)**, Rotate, Scale, a 22 px divider, Pose bones, Motion path, Add key, Measure. Intended shortcuts: Q / W / E / R for the first four. *(Solis's note: the area's tools float in a bar at the foot of the view instead of this column, as the project lead asked on 2026-11-14, [decision 0114](../../decisions/0114-the-tools-float-in-the-view.md); the UI canvas keeps its toolbox column.)*
- **Viewport scene:** sky gradient #141414 → #2A2A2A down to the horizon (44% height), floor #202020 with perspective grid lines in white at 5% opacity. Objects are flat-shaded grays (lit faces lighter). The selected crane has a 1.5 px accent outline; its animation path is a dashed accent curve with an accent start dot and a light end dot.
- **Move gizmo:** three 2.5 px rounded arrows from the object’s pivot — X red (#E5534B) to the right, Y green (#6CC24A) up, Z blue (#4C8DFF) toward the viewer.
- **Overlays:** top left, pills “Perspective ▾” and “Lit ▾” (view and shading menus); top right, a mono pill “6.9 ms · 144 fps”. Pills are 24 px, #121212 at 85% with a #2E2E2E border.

### Center, right group: code editor (about 40%)

*Later feature.* The code editor ships after the first release; at launch, coding happens through Ledger and this group is hidden, giving the viewport the full width. Everything below is the target design for when it arrives.

- **Document tabs:** “CraneController.cs” (active), “HookPhysics.cs”.
- **Breadcrumb** (22 px, 11 px gray): Scripts › Harbor › **CraneController.Tick** (current function in #C8C8C8).
- **Code** (JetBrains Mono 11.5 / 19 px): line numbers in #5E5E5E, right-aligned in a 30 px gutter. Line 84 carries a runtime error: 10% red background, 2 px red bar on the left, red line number, and an inline message “● Target was destroyed (frame 1832)” under it.
- **Intended:** the editor opens the script of the selected object’s component; runtime errors from Play mode are pinned to the line with the frame number; the group closes when no script is open, and Level design mode hides it automatically.

### Shelf bar

Tabs: **Object** · Animation · Console · Problems · 2. Right side, always visible: previous key, play animation, next key (26 px icon buttons) and the frame counter “0096 / 0240” in mono.

### Shelf — Object tab (default)

| Column | Width | Contents |
| --- | --- | --- |
| Scene | 220–240 px | Header “Scene” + count “214”. Tree: Props ▾ expanded with Crane\_Arm\_02 (selected: accent-tinted row, white text), Crate\_Stack\_A, Crate\_Stack\_B, Bollard\_x12; collapsed Lighting ▸, Terrain ▸, Gameplay ▸. Children indent 28 px. |
| Inspector | Takes all remaining width | Header: accent cube icon, “Crane\_Arm\_02” in white, “Static mesh · Props” in gray, and a dashed “+ Component” button on the right. Component cards below, wrapping as space allows. |
| Assets | 280–320 px | Header “Assets” + current folder “Harbor”. 4-column tile grid (see below). |

**Inspector cards** (#1F1F1F, 1 px #2A2A2A border, radius 8, 10 px padding, min 180 px wide):

- **Transform:** rows Pos / Rot / Scale × three fields. Each field shows a colored axis letter (X #F08A84, Y #8FD47A, Z #7FB2FF) before its value: Pos 12.5, 0, -4.7; Rot 0, **35** (accent border = value just edited), 0; Scale 1, 1, 1.
- **Mesh Renderer:** Mesh “SM\_Crane\_Arm”, Material “M\_Steel”, Shadows toggle (on).
- **CraneController** (a script): Speed 0.8, Target “None” in red text (broken reference, matching the code error), Loop checkbox (on).
- **Intended:** drag a number field’s label left or right to scrub the value; right-click a card for reset, copy, remove; “+ Component” opens a searchable list.

**Asset tiles** (square, radius 6, #242424): SM\_Crane (2 px accent inner outline = the selected object’s mesh), SM\_Crate (cube icon), M\_Steel (gray sphere), T\_Rust (diagonal gray stripes), AN\_Swing (purple curve), SFX\_Creak (amber speaker), Import (dashed border, “+”). Names under each tile, 10.5 px, cut off with “…”. Intended: drag a tile into the viewport to place it, or onto a card field to assign it.

### Shelf — Animation tab

- **Tracks** (220 px): header “SEQ\_Crane\_Intro”; group rows (#1E1E1E, bold) Crane\_Arm\_02 and Harbor\_Lamp\_01 with child tracks Rotation Y (selected, value “35.0°”), Location, Intensity; an Audio track.
- **Dope sheet:** a Keys / Curves switch (Keys active); a frame ruler labeled 0, 60, 120, 180 with lines every 30 frames; group summary keys as small gray diamonds (8 px), track keys as light diamonds (10 px), the selected key at frame 96 in accent with a ring; the audio track shows a gray clip “SFX\_Crane\_Creak”; the playhead is a 2 px accent line with a “96” flag on top.
- **Key details** (240 px): Frame 96, Value 35.0°, Ease = Step / Linear / **Smooth**.

### Shelf — Console tab

Mono log lines: time (#6E6E6E), level word (INFO gray, WARN amber, ERROR red), message. The error line has a faint red background and a link “CraneController.cs:84” that opens the code editor at that line.

### Shelf — Problems tab

One row per problem with an icon and a word: error “Missing reference: SM\_Pier\_Plank on Pier\_Segment\_07” (level name in gray), warning “Lighting is out of date (2 lights moved)” with a “Bake now” link.

### Status bar (right side)

“Ln 84, Col 9” (code cursor), “Snap 0.25 · World” (snap step and transform space), “v0.1.0”.

## Screen 2 — Home · Level design (placing)

The Home workspace tuned for building a level: the viewport takes the full width, the code editor is hidden, and the shelf opens on **Place** so assets are dragged straight up into the scene. The mockup freezes the moment a crate is mid-drag.

### Ribbon (Home, build mode)

| Group | Controls | Intended behavior |
| --- | --- | --- |
| Place | Large: Mesh, Light, Camera, Volume | Same as Create; the new object follows the mouse until clicked |
| Snapping | Small: “Grid · 0.25”, “Angle · 15°”, “Snap to surface” — all on | Applies to dragging, placing and moving |
| Paint | Large: Brush, Scatter | Brush paints the selected asset along strokes; Scatter fills an area with random spacing, rotation and scale |
| Transform | Large: **Move** (active, accent-tinted), Rotate, Scale | Moved here from the toolbox because there is no toolbox in this mode |
| Play | Play, Simulate | As everywhere |

### Viewport (full width, 430 px tall)

- **Same scene** as the Level editor; nothing is selected because a placement is in progress.
- **Ghost object:** the dragged crate is drawn at its snapped spot as three faces with accent fill at 22% and a 1.5 px dashed accent outline (5 px dash, 3 px gap).
- **Footprint:** a diamond on the floor under the ghost, accent fill at 16% with an accent outline at 80%.
- **Snap guides:** three dashed accent lines (55% opacity) running from the footprint along the grid to the edges of the view.
- **Drag trail:** a faint dashed white curve (35% opacity) from the bottom of the viewport — where the shelf is — up to the cursor, showing where the drag came from.
- **Cursor:** white arrow with a dark outline.
- **Label next to the cursor:** a 208 × 52 px dark box (#181818 at 95%, #333333 border, radius 7) with the asset name “SM\_Crate\_Large” (12 px / 600, white) and “x 18.25  z -6.50  · grid” in mono gray: live coordinates plus what it is snapping to (grid, surface or free).
- **Hint strip** (bottom center, 30 px, #181818 at 94%, radius 8): keys in small mono chips — **Release** place · **Shift** keep placing · **Alt** free, no snap · **R** rotate 15° · **Esc** cancel.
- **Overlays:** top left “Perspective ▾”, “Lit ▾”, “Show ▾”; top right “6.9 ms · 144 fps” and “Code editor hidden while building · Show” (Show is a link that brings the code group back).

### Shelf bar

Tabs: **Place** · Object · Animation · Console · Problems · 2. Right side: hint “Drag into the scene, or select and click to place”.

### Shelf — Place tab

| Column | Width | Contents |
| --- | --- | --- |
| Scene | 220–240 px | The scene tree. While dragging, a temporary row “+ SM\_Crate\_Large (placing)” appears in accent italics where the object will land. Footer: “New objects go into Props ▾” — picks the folder new objects are filed under. |
| Placement | 250–280 px | Align to: Surface / Grid / Free (Surface active). Rotation: “Keep · 0° ▾”. Random yaw: toggle (off). Scale: 1.00. Physics: “Settle on drop ▾” (object falls and rests when placed). |
| Assets | Remaining width | Filter chips: **★ Favorites** (active), Recent, Harbor ▾ (folder), Lights. Search “Find something to place”. Link “Open Library”. Tile grid (min 92 px tiles). |

**Place tiles:** SM\_Crate\_Large (being dragged: 55% opacity, dashed accent border, small accent “dragging” badge bottom left), SM\_Crate\_Small, SM\_Bollard, SM\_Pier\_Plank, SM\_Crane\_Arm, PF\_Harbor\_Lamp, PF\_Crane\_Full, SM\_Barrel, SM\_Rope\_Coil, SM\_Fence\_2m, SFX\_Gulls\_Loop, Point light. Meshes show a gray cube icon, prefabs a blue three-block icon, audio an amber speaker, lights an amber bulb. Favorites carry an amber ★ in the top right corner (SM\_Crate\_Large, SM\_Bollard, PF\_Harbor\_Lamp).

### Placement behavior (intended)

1. Press on a tile and drag upward; the tile dims and the ghost appears as soon as the cursor enters the viewport.
2. The ghost snaps live according to the ribbon and Placement settings; the label shows coordinates and the snap source.
3. Release to place. The object is created in the chosen folder and **selected immediately**, so the Object tab can adjust it.
4. Shift while releasing keeps the asset “loaded”: every further click places another copy.
5. Without dragging: click a tile once, then click in the scene to place it. A shortcut drops the selected asset under the mouse.
6. Esc or dragging back onto the shelf cancels with nothing created.

### Status bar (right side)

“Placing SM\_Crate\_Large · Snap 0.25 · Surface”.

## Screen 3 — Library

The Library is the asset manager: importing, organizing, tagging, checking references and version control. It is deliberately **not** where objects are placed in the world — that happens in Level design — but it links straight there with **Place**. The ribbon has no Play group.

### Ribbon (Library)

| Group | Controls | Intended behavior |
| --- | --- | --- |
| Add | Large: Import, Create, Folder | Import opens a file picker (dropping files anywhere also imports); Create makes a new material, prefab, script etc.; Folder adds a subfolder here |
| View | Small: Grid (on), List (off), Show folders (on) | Grid vs list view; whether folders appear as tiles |
| Filter | Small: “Type: All ▾”, “Tags ▾”, “Missing refs” (off) | Filters the grid; Missing refs shows only assets with broken references |
| Source control | Large: Check in, History | Commit selected assets; show their change history |
| Use | Large: **Place** (accent-tinted). Small: “Show in level” | Place jumps to Level design with the asset loaded on the cursor; Show in level selects every placed copy in the open level |

### Center: browser (full width, 430 px, #161616)

- **Toolbar** (40 px, wraps on narrow windows): Back and Up-one-folder buttons; breadcrumb “Content / Environment / **Harbor ▾**” (earlier parts are gray links, the current folder is a dropdown to jump to siblings); type chips — All (active, #262626 with #3A3A3A border), Mesh, Material, Texture, Animation, Audio, Prefab, each with its colored dot; search box “Search Harbor” (260 px, right-aligned).
- **Grid:** tiles fill the width (each at least 108 px), 4 px gap. A tile = square thumbnail (radius 6, #242424, #2C2C2C border), name (11 px, cut off with “…”), and a type line (colored 7 px dot + type word, 10.5 px gray).
- **Thumbnails by type:** folders show a large gold outline folder; meshes a gray wire cube; materials a shaded gray sphere; textures their pattern (T\_Rust\_Mask diagonal stripes, T\_Planks\_Normal vertical stripes); animations a purple curve; audio an amber speaker; prefabs three blue blocks.
- **Contents shown (18):** Cranes, Docks, Vehicles (folders); SM\_Crane\_Arm, SM\_Crane\_Base, SM\_Crate\_Large, SM\_Bollard, SM\_Pier\_Plank; M\_Steel\_Painted, M\_Wood\_Weathered, M\_Rust, M\_Water\_Harbor; T\_Rust\_Mask, T\_Planks\_Normal; AN\_Crane\_Swing; SFX\_Crane\_Creak, SFX\_Gulls\_Loop; PF\_Crane\_Full.
- **Selected tile** (SM\_Crane\_Arm): accent-tinted tile background and a 2 px accent inner outline on the thumbnail.
- **Problem badge** (SM\_Pier\_Plank): a 16 px dark-red circle with a red “!” in the thumbnail’s top right corner, tooltip “Missing reference”.

### Shelf bar

Tabs: **Details** · Used by · 3 · Import queue · 1 · History. Right: “48 items · 1 selected”.

### Shelf — Details tab

| Column | Width | Contents |
| --- | --- | --- |
| Preview | 300–340 px | Dark box (#111111, radius 8) with a rendered preview of the mesh on a ground line with a soft shadow. Pills bottom left: Lit, Wire, UV (shading modes). Intended: drag to orbit. |
| Details | Remaining width | Name “SM\_Crane\_Arm” (14 px / 600); “Static mesh · imported from crane\_arm.fbx” with the mesh color dot. Two key–value columns: Path (mono) Content/Environment/Harbor · Triangles 12,418 · Vertices 7,902 · LODs 3 (auto) · Materials M\_Steel\_Painted, M\_Rust · Bounds 4.0 × 22.0 × 2.0 m · Disk size 1.8 MB · Modified Today, 10:04. Tag chips: metal, hero-props, and a dashed “+ Tag”. |
| Used by | 280–320 px | Header “Used by 3”. Rows: Docks\_Night (level), PF\_Crane\_Full (prefab), Docks\_Day (level), type word right-aligned in gray. Footer: import progress “pier\_planks\_4k.png 68%” with a 4 px accent bar. |

### Intended behavior

- Drag tiles onto a folder tile or a breadcrumb part to move them; references update automatically.
- F2 renames; Ctrl/Shift-click multi-selects; Delete warns when something still uses the asset.
- Double-click opens the asset in its editor (material → Shading, animation → Animate, script → code editor).
- Dragging a tile from the Library onto the Home tab switches to Level design with the asset on the cursor.

### Status bar (right side)

“Drag files anywhere to import”.

## Screen 4 — Shading · Material

A node-graph material editor with a live preview beside it and the material’s exposed parameters in the shelf. Wires are colored by data type, so a wrong connection is visible at a glance. No Play group in this ribbon.

### Ribbon (Shading)

| Group | Controls | Intended behavior |
| --- | --- | --- |
| Material | Large: **Apply** (accent-tinted), Save | Apply compiles and pushes changes to every object using the material; Save writes the asset |
| Add node | Large: Texture, Math, Param | Opens that category’s node list; the node lands at the mouse |
| Graph | Small: “Auto-layout”, “Comment frame”, “Wires: Curved ▾” | Tidy the graph left to right; wrap the selection in a labeled frame; wire style (curved, straight, right-angled) |
| Preview | Small: “Shape: Sphere ▾”, “Lit” (on), “Live update” (on) | Preview mesh, lighting, recompile on every change |

### Center, left group: node graph (about 75%)

- **Document tabs:** M\_Steel\_Painted (green material dot, active), M\_Rust, MF\_Triplanar (a material function).
- **Canvas:** #141414 with a dot grid (1 px #2A2A2A dots every 20 px). Intended: middle-drag pans, wheel zooms, box-select with left drag.
- **Comment frame** “Surface textures” around the left column of nodes: white fill at 3%, 1 px #333333 border, radius 10, label sitting just above its top edge (11 px / 600, #A6A6A6).
- **Node anatomy:** body #222222, 1 px #363636 border, radius 8, soft drop shadow; header 28 px, #2A2A2A, with a type dot and the title (11.5 px / 600). Pin rows 24 px. Pins are 10 px rings with a 2 px border in the type color; **filled = connected, hollow = unconnected**. Inputs sit on the left edge, outputs on the right. Unconnected inputs can show a default value in gray mono (Metallic 1.0, Roughness 0.62).

| Node | Pins | Notes |
| --- | --- | --- |
| Texture Sample (rust) | Out: RGB (Vector3), R, A (Float) | 46 px thumbnail of the texture inside the node (gray diagonal stripes) |
| Texture Sample (steel) | Out: RGB, R, A | Thumbnail: light-to-dark gray gradient |
| Parameter “Rust\_Amount” | Out: Float | Green dot = parameter; exposed in the shelf |
| Multiply | In: A, B · Out: Result |  |
| Lerp (selected) | In: A, B (Vector3), Alpha (Float) · Out: Result | Selected = accent border + 1 px accent ring + deeper shadow |
| Material Output | In: Base Color, Metallic, Roughness, Normal, Emissive | Header slightly lighter (#303030) with a sphere icon |

- **Wires** (2 px curves in the source pin’s color): rust RGB → Lerp B; steel RGB → Lerp A; rust A → Multiply A; Rust\_Amount → Multiply B; Multiply → Lerp Alpha; Lerp Result → Base Color. Wires leaving the selected node are drawn in accent at 2.5 px.
- **Hint pill** (bottom center): “Tab Add node · F Frame selection” with keys in mono chips.

### Center, right group: preview (about 25%, #111111)

- Header “Preview”. A shaded steel sphere (highlight upper left, dark lower right) with soft darker rust patches and a contact shadow.
- Footer: green dot + “Compiled · 64 instructions · 2 samplers”. Intended: turns amber while compiling and red with the error text when compilation fails.

### Shelf bar

Tabs: **Parameters** · Node · Lerp · Palette · Instances · 6. (“Node · Lerp” shows the selected node’s settings; “Palette” is the searchable node list; “Instances” lists material instances that inherit from this one.)

### Shelf — Parameters tab

- **Parameters column** (wide): header “M\_Steel\_Painted · 5 parameters”, cards grouped by the parameter’s group name:
  - **Surface:** sliders Rust\_Amount 0.35 and Roughness 0.62. Slider = 4 px #2C2C2C track, accent fill up to the value, 12 px light knob, value in mono at the top right.
  - **Color:** Paint\_Tint field with a 12 px swatch and “#C9C9C9”; Two-sided checkbox (off).
  - **Tiling:** Rust\_Tiling 4.0; UV set “UV0 ▾”.
- **Pin types column** (280–320 px): a legend — Float (gray ring), Vector3 (yellow ring), Color (purple ring), Texture (orange square).

### Status bar (right side)

“Lerp selected”.

## Screen 5 — Animate · Sequencer

The animation and cinematics workspace: the camera shot and a curve editor side by side, with the full sequencer (tracks, keys, clips, playhead) filling a taller shelf. Playback controls live in the shelf bar so they stay visible whatever the shelf shows.

### Ribbon (Animate)

| Group | Controls | Intended behavior |
| --- | --- | --- |
| Keys | Large: Add key (diamond icon), **Auto key** (red record icon, label in #F08A84) | Add key keys the selected properties at the playhead; Auto key records a key whenever a value changes (the viewport gets a red frame while it is on) |
| Interpolation | Small: Step (off), Linear (off), **Smooth** (on, accent box) | Sets the ease of the selected keys and of new keys |
| Tracks | Large: Track, Curves | Track adds a track for the selected object or property; Curves switches the shelf’s dope sheet to the curve view |
| Playback | Small: “Loop” (on), “30 fps ▾”, “Range 0–240” | Loop playback; frame rate; the play range (dragging its ends on the ruler also sets it) |
| Play | Play, Simulate | As everywhere |

### Center, left group: camera preview (about 60%, #111111)

- **Document tabs:** “CAM\_Shot\_010” (active), “Viewport” (switch between looking through the shot camera and the free editor camera).
- **Frame:** a 2.39:1 cinema frame centered with 14 px padding, a 1 px #2C2C2C outline around it. Inside: night shot — gradient sky #121212 → #2E2E2E, a light-gray moon top right, the crane as a near-black silhouette, two amber lamp dots on the right, and the crane’s motion path as a dashed accent curve with an accent start dot.
- **Timecode pill** top right: “00:00:03:06” in mono (hours:minutes:seconds:frames).

### Center, right group: curve editor (about 40%, #151515)

- **Header** (30 px): “Curves · Rotation Y” and a Frame-all button (grid icon) on the right.
- **Graph:** grid lines #1E1E1E every quarter. The selected curve (Rotation Y) is a 2 px accent line; a second curve in #6E6E6E at 1.5 px for context. The selected key sits at the curve’s peak: accent diamond (10 px, rotated square) with two light tangent handles (3 px dots) on dashed gray tangent lines. A faint vertical accent line marks the playhead.
- **Intended:** drag keys and handles; Alt-drag breaks tangents; F frames the selection; the visible curves follow the tracks selected in the shelf.

### Shelf bar

Three tabs: **Sequencer**, “Key · Rotation Y”, Console. Right side: transport — previous key, **play** (light #E6E6E6 button with dark triangle), next key (26 px icon buttons) — then the frame counter “0096 / 0240” in mono #A6A6A6.

### Shelf — Sequencer tab (about 360 px tall)

**Tracks column** (260 px, #1A1A1A):

- Header “SEQ\_Crane\_Intro” with a “+” add-track button; a blank 22 px row so the tracks line up with the ruler.
- Rows are 26 px with a #1E1E1E line under each. Top-level tracks are bold #E6E6E6 on #1E1E1E; child tracks are indented 18 px. Values sit right-aligned in gray mono.
- Tracks in order: Camera Cuts; ▾ Crane\_Arm\_02 → **Rotation Y** “35.0°” (selected: accent-tinted row, white text), Location; ▾ Harbor\_Lamp\_01 → Intensity “8.0”; Audio; Events.

**Dope sheet** (remaining width, #161616):

- **Toolbar** (30 px): Keys / Curves switch (Keys active, #2C2C2C), “Snap: frames” on the right in gray.
- **Ruler** (22 px, mono 10 px #8C8C8C): labels 0, 30, 60 … 210, with a #2C2C2C line every 30 frames (every 12.5% of the 240-frame range).
- **Lanes** (26 px each, top-level lanes on #1A1A1A, the selected lane accent-tinted at 8%):
  - Clips (Camera Cuts, Audio, Events): 18 px rounded bars, #2C2C2C fill, #3A3A3A border, label inside in 10.5 px #C8C8C8 — CAM\_Shot\_010 (frames 0–120), CAM\_Shot\_020 (120–220), SFX\_Crane\_Creak (30–198), ShakeCam (96–126), FadeOut (200–228).
  - Group summary keys (on parent tracks): 8 px gray (#8C8C8C) diamonds at every child key’s frame.
  - Track keys: 10 px light (#D4D4D4) diamonds — Rotation Y at 0, 48, 96, 150, 210; Location at 0, 150; Intensity at 60, 72, 180.
  - Selected key (Rotation Y, frame 96): accent diamond with a 2 px dark gap and a 1 px accent ring.
- **Playhead:** 2 px accent line through all lanes at frame 96, topped by a 32 × 18 px accent flag with “96” in dark bold mono.

### Shelf — Key · Rotation Y tab (not shown open)

The selected key’s details: frame, value, and Step / Linear / Smooth ease, as in the Level editor’s Animation tab.

### Intended behavior

1. Click the ruler to move the playhead; Space plays and pauses; arrow keys step one frame; Up/Down jump between keys.
2. Click a key to select it, box-drag for several, drag to retime (snapping to frames), Alt-drag to duplicate.
3. Drag clip edges to trim and clip bodies to move; double-click a camera clip to look through that camera.
4. Selecting a track shows its curve in the curve editor; selecting an object in the viewport scrolls the tracks to it.
5. Auto key on: any change in the viewport or the Object tab writes a key at the playhead.

### Status bar (right side)

“30 fps · 240 frames”.

## Screen 6 — Debug · Profiler & console

The performance and debugging workspace: a frame-time graph across the top, a per-thread breakdown of the selected frame with statistics beside it, and the console in the shelf. The mockup inspects frame #1832, a 22.4 ms spike caused by the same CraneController error shown on the Level editor.

### Ribbon (Debug)

| Group | Controls | Intended behavior |
| --- | --- | --- |
| Capture | Large: Record (red record icon, label in #F08A84), Pause, Save | Start/stop recording frames from the running game; Save writes the capture to disk for later comparison |
| Target | Small: “Play-in-editor ▾”, “Frames: last 300”, “Deep profile” (off) | Pick what to profile (editor play session, a standalone build, a device); how many frames to keep; deep profile records every function call (slower) |
| Show | Small: CPU (on), GPU (on), Memory (off) | Which tracks appear in the graph and timeline |
| Tools | Large: Debugger, Memory | Open the script debugger (breakpoints, stepping) or a memory snapshot view |
| Play | Play, Simulate | As everywhere |

### Center, top: frame-time graph (130 px, #141414)

- **Y-axis labels** (mono, 10 px, #8C8C8C) in a 38 px gutter on the left: “33ms”, “16ms”, “0”. The scale tops out at 40 ms.
- **Budget lines:** dashed horizontal lines at 33.3 ms (30 fps budget, #4A4A4A) and 16.6 ms (60 fps budget, #3A3A3A).
- **Bars:** the last 120 frames as thin bars, 2 px gaps, 1 px rounded tops. Normal frames (about 6–9 ms) are #4A4A4A; frames over the 16.6 ms budget are lighter #A6A6A6 (only frame 40, at 18.2 ms; the small periodic bumps to about 11–13 ms stay #4A4A4A); the **selected frame** (#84, 22.4 ms) is drawn in accent.
- **Top right label:** “Frame #1832 · 22.4 ms” (10.5 px, #A6A6A6).
- **Intended:** click a bar to select that frame; drag to select a range (statistics then show averages); hover shows the frame number and time.

### Center, bottom left: frame timeline (wide, #161616)

A 100 px name column (#1A1A1A) and a time ruler (mono, 10 px) marked 0 ms, 5, 10, 15, 20. A dashed #5A5A5A vertical line at 16.6 ms marks the 60 fps budget inside the frame. Each thread is a 26 px row; blocks are 20 px tall, radius 3, mono 10 px labels, cut off when narrow.

| Thread | Blocks (start → duration, color) |
| --- | --- |
| Main | Input 0–1.2 ms (#2E2E2E) · Physics.Step 1.2–9.1 ms (#5A5A5A) · **CraneController.Tick** 9.1–13.2 ms (accent mixed 45% into #222222 — highlighted as the culprit) · Anim.Evaluate 13.2–15.8 ms (#3A3A3A) · GC.Collect 15.8–18.0 ms (dark red #4A2A2A, pink text #FFD6D3) · Render.Submit 18.0–22.4 ms (#333333) |
| Physics | Broadphase 1.3–4.9 ms (#444444) · Solver (8 iter) 5.0–8.9 ms (#4E4E4E) |
| Render | Frame N-1 RHI 0.4–6.4 ms (#2E2E2E) · Visibility 18.2–22.2 ms (#333333) |
| Worker 1 | Cull 2.0–4.4 ms (#2A2A2A) · Skinning 9.5–12.5 ms (#383838) |
| Worker 2 | Cull 2.2–4.0 ms (#2A2A2A) · Particles 13.4–15.4 ms (#333333) |
| GPU | Shadows 0.8–3.2 ms (#2E2E2E) · GBuffer 3.2–8.8 ms (#3A3A3A) · Lighting 8.8–11.2 ms (#444444) · Post 11.2–12.6 ms (#2E2E2E) |

- Block text is #E6E6E6 unless noted. Blocks are grayscale on purpose; only the selected/culprit block uses the accent and only garbage collection uses red.
- **Intended:** click a block to select it (statistics and the shelf’s “Hottest functions” jump to it); double-click a script block to open its code at that function; scroll to zoom the time axis.

### Center, bottom right: Statistics (260–300 px, #1A1A1A)

Header “Statistics”, then four metric rows, each a label (#A6A6A6) with a mono value on the right and a 4 px bar underneath (#262626 track):

| Metric | Value | Bar |
| --- | --- | --- |
| CPU · game | 22.4 ms | 100%, #A6A6A6 (over budget) |
| GPU | 11.8 ms | 53%, accent |
| Draw calls | 1,942 | 39%, accent |
| Memory | 2.31 / 8 GB | 29%, accent |

### Shelf bar

Tabs: **Console** · Hottest functions · Memory snapshot. Right side: level filter buttons styled like shelf tabs — “Info 128” (gray), “Warnings 3” (amber text), “Errors 1” (red text #F08A84). Intended: each toggles that level on or off.

### Shelf — Console tab (#121212)

Mono 11 px lines, 3 px vertical padding, each: timestamp with milliseconds (#6E6E6E), a 44 px level word, the message.

| Time | Level | Message | Row style |
| --- | --- | --- | --- |
| 10:21:04.112 | INFO (#A6A6A6) | \[Assets\] Imported pier\_planks\_4k.png → T\_Pier\_Planks | plain |
| 10:21:06.480 | INFO | \[Play\] Entered play mode · Docks\_Night | plain |
| 10:21:07.031 | WARN (#E8B04A) | \[Physics\] Crane\_Arm\_02 collider overlaps Crate\_Stack\_A at spawn | faint amber background (6%) |
| 10:21:12.204 | ERROR (#F06A6A) | NullReference in CraneController.Tick() — target hook was destroyed | faint red background (8%); link “CraneController.cs:84” right-aligned |
| 10:21:12.390 | WARN | \[Render\] Frame time 22.4 ms exceeded 16.6 ms budget | faint amber background |

- **Command line** (32 px, #0E0E0E, top border #262626): an accent “›” prompt, input pre-filled with “physics.debug\_draw ” in mono, and an autocomplete hint on the right “contacts | colliders | off” (10.5 px gray).
- **Intended:** Tab completes commands and arguments; Up/Down walk history; clicking a file:line link opens the code editor at that line; clicking a log line that has a frame number selects that frame in the graph.

### Intended behavior

- Recording starts automatically when Play starts if “profile on play” is set; the graph scrolls live while recording and freezes on Pause.
- Errors and warnings logged during a frame are marked on that frame’s bar, so spikes and errors line up.
- Saved captures can be loaded side by side to compare two builds.

### Status bar (right side)

“Recording · 300 frames”.

## Screen 7 — File · Recent projects

Clicking the **File** tab replaces the ribbon, center area and shelf with one full-page view, a backstage page that takes over the whole window. Only the tab bar and status bar stay. This screen is the project hub: recent projects, new projects and templates.

### Layout

- **Page background:** #141414.
- **Left:** the File menu column, 220–240 px wide, #181818, with a 1 px #262626 right border and 16 px / 10 px padding.
- **Right:** the page content, with 28 px top/bottom and 36 px side padding and 22 px between blocks.
- **Narrow windows:** the menu column stacks above the content.

### File menu column

- **Item style:** 36 px tall, 14 px side padding, radius 7, 13 px / 500 Inter, #A6A6A6, a 15 px line icon and a 10 px gap. Hover = #1E1E1E fill with #E6E6E6 text. The active item = #1E1E1E fill with #FFFFFF text.
- **Order, top to bottom:**
  1. **Back to editor** (left chevron, #C8C8C8, 8 px space below). It returns to the last editing tab.
  2. **Recent projects** (clock). This item is active on this screen.
  3. **New project** (plus).
  4. **Open…** (open folder).
  5. **Save all** (disk).
  6. *Separator:* 1 px #262626 line with 8 px / 6 px margin.
  7. **Project settings** (gear).
  8. **Preferences** (sliders). This is Screen 8.
  9. **Plugins** (puzzle piece).
  10. *Separator.*
  11. **Learn** (open book).
  12. **Exit** (door arrow).

### Page header

| Element | Details |
| --- | --- |
| Title | “Recent projects”, 22 px / 700 |
| Subtitle | “Pick up where you left off, or start from a template.”, 12.5 px, #8C8C8C |
| Search | 32 × 260 px, #0E0E0E fill, #2C2C2C border, radius 7, 14 px search icon, placeholder “Search projects” |
| Open… | 32 px button, #262626 fill, #333333 border, radius 7, weight 500 |
| New project | 32 px button with solid accent fill, #0E0E0E label at weight 600 and a plus icon. It is the page’s main action. |

The header wraps onto two lines on narrow windows.

### Project cards

- **Grid:** cards fill the width, each at least 240 px wide, with a 16 px gap.
- **Card:** #1A1A1A fill, 1 px #2A2A2A border, radius 10, content clipped to the corners.
- **Thumbnail:** 140 px tall, #1C1C1C, with a flat gray illustration of the project’s scene.
- **Type badge:** top left of the thumbnail, “3D” or “2D”, 10.5 px / 500, #C8C8C8 on #2A2A2A, radius 4, padding 2 × 7 px.
- **Body** (12 × 14 px padding, 5 px gap):
  - Project name: 13 px / 600.
  - Folder path: mono 10.5 px, #8C8C8C.
  - Last line: when the project was last opened on the left (11.5 px, #8C8C8C) and the engine version on the right (mono).
- **Open project:** the project that is already open gets a border of 60% accent mixed into #2A2A2A.
- **Outdated version:** a project made with an older engine shows its version and “upgrade” in amber (#E8B04A).

| Card | Badge | Path | Last opened | Version | Thumbnail |
| --- | --- | --- | --- | --- | --- |
| Harbor\_Prototype (open, accent border) | 3D | D:/Projects/Harbor\_Prototype | Open now | v0.1.0 | Crate stack and crane on a gray floor |
| Neon\_Courier | 2D | D:/Projects/Neon\_Courier | Yesterday | v0.1.0 | Building silhouettes with a light figure |
| Glacier\_Sim | 3D | E:/Work/Glacier\_Sim | 3 days ago | v0.0.9 · upgrade (amber) | Mountain ridge with pale peaks |
| Tiny\_Tactics | 2D | D:/Projects/Tiny\_Tactics | Last week | v0.1.0 | Grid board with two round tokens |

### Template strip

- **Header:** “Start from a template” (13 px / 600, #C8C8C8) on the left and an “All templates” link on the right.
- **Strip:** one row that scrolls sideways, with a 14 px gap.
- **Template:** 150 px wide. Its thumbnail is 84 px tall, #1A1A1A, radius 8, with a centered 24 px #8C8C8C icon. The label below it is 12 px / 500.
- **Templates, in order:**
  1. Blank 3D (cube)
  2. Blank 2D (framed picture)
  3. First-person (camera)
  4. Top-down (grid)
  5. Platformer (stepped lines)
  6. VR / XR (eye)
- **Border:** the two blank templates have a **dashed** #2C2C2C border because they start empty. The other four have a solid border.

### Intended behavior

- Clicking a card opens that project and returns to the editor on its last workspace. Clicking the open project simply goes back to the editor.
- Right-click a card for Pin, Show in folder, Remove from list and Upgrade.
- Opening an outdated project offers to upgrade a copy first.
- Search filters the cards by name and path as you type.
- Clicking a template, or New project, asks for a name and folder, then creates and opens the project.
- Esc or **Back to editor** leaves the File view.

### Status bar (right side)

“Update \[X.Y\] available” (the version number is a placeholder), then “v0.1.0”.

## Screen 8 — File · Preferences

Personal editor settings, shown in the File view with “Preferences” active in the left menu. Every change applies instantly — there is no Save or OK button. The File view’s shell (left menu, no ribbon, no shelf) is the same as on Recent projects (Screen 7).

### Header

- **Title:** “Preferences” (22 px / 700).
- **Subtitle** (12.5 px, #8C8C8C): “Saved for you on this machine. Project settings are shared with the team.” — makes the split clear: Preferences = personal, Project settings (a separate File menu item) = shared.
- **Search** (right, 280 × 32 px, #0E0E0E, #2C2C2C border, radius 7): “Search all settings”.

### Sub-tabs

A row under the header with a 1 px #262626 bottom line: General · **Appearance** (active: white, weight 500, 2 px accent underline) · Viewport · Input · Shortcuts · Source control. Inactive tabs are #A6A6A6, 12.5 px, 18 px apart.

### Setting row pattern

Every setting is one row: a two-column grid (text on the left, a 260 px control column on the right), 12 px top and bottom padding, a 1 px #222222 divider under it. The left side has the setting name (12.5 px / 500) and a one-line description under it (11.5 px, #8C8C8C). The control is right-aligned. Groups of rows sit under small section caps (10.5 px / 600, uppercase, letter-spacing 0.08 em, #8C8C8C). The settings column is at most 760 px wide.

### Theme

| Setting | Description | Control · value |
| --- | --- | --- |
| Base theme | Neutral gray surfaces with no color tint. | Dropdown (200 px field) “Neutral dark” |
| Accent color | Used for selection, the active tool and focus. | Five 22 px rounded swatches: #5C9CFF (selected), #D4D4D4, #F2994A, #4FC2A8, #B48CFF. The selected one has a 2 px gap ring in the background color, then a 2 px ring in its own color. |
| Interface scale | Scales text and controls. Useful on 4K screens. | Dropdown “100%” |
| Density | Compact fits more rows; comfortable is easier to click. | Segmented control: Compact · **Default** (active, #2C2C2C) · Comfortable |

### Layout

| Setting | Description | Control · value |
| --- | --- | --- |
| Show ribbon labels | Turn off to show icons only and save 20 px of height. | Toggle, on |
| Shelf opens on | Which shelf tab is active when you select something. | Dropdown “Object” |
| Code editor split | Show scripts next to the viewport when a script is open. | Toggle, on |

### Live preview (right column, 260–320 px)

Under a “Preview” section cap: a miniature of the editor (radius 10, #2A2A2A border) that redraws as settings change. Top to bottom:

1. **Tab bar** (14 px, #0E0E0E) with one active tab (#181818) underlined in the accent.
2. **Ribbon** (22 px, #181818).
3. **Editor area** (92 px): a viewport (#202020, 3 parts) with a gray bar for an object outlined in the accent, beside the code group (#161616, 2 parts).
4. **Shelf bar** (10 px, #141414).
5. **Shelf** (#1A1A1A) with three small cards (#1F1F1F).

Under it: a full-width “Reset to defaults” button (30 px, #262626 fill, #333333 border, radius 6).

### Intended behavior

- Every change applies to the whole editor at once; the preview is only there to show it before you look away from this page.
- Search filters across **all** sub-tabs, not just the open one: matching rows show grouped under their tab name, and the matched words are highlighted.
- Reset to defaults asks which scope (this tab or all preferences) before resetting.
- Preferences are stored per user and machine; they never go into the project or version control.

### Status bar (right side)

“Changes apply instantly”.

## Screen 9 — Library › Model viewer

Double-clicking a mesh in the Library opens it in its own full view so a model can be inspected properly: a large studio viewport with measurements, a UV layout beside it, and LODs, materials and import details in the shelf. A contextual tab appears in the tab bar while it is open. The ribbon has no Play group.

### Tab bar

The Library tab stays visible but inactive. Right after it sits the contextual tab **“LIBRARY TOOLS · Mesh”**: active style (white text on #181818, 6 px rounded top, 2 px accent underline), a small gray caps prefix “LIBRARY TOOLS” (9.5 px / 600, letter-spacing 0.06 em, #8C8C8C, 6 px gap before “Mesh”), and a faint 2 px accent line inside its top edge (accent at 55%). It disappears when the last model tab closes.

### Ribbon (Mesh tools)

| Group | Controls | Intended behavior |
| --- | --- | --- |
| View | Large: **Lit** (active, accent-tinted), Unlit, Wire, Normals, UV check | Shading mode of the viewport; UV check draws a checker texture to reveal stretching |
| Show | Small: Bounds (on), Pivot (on), Collision (off) | Overlays: dashed bounds box with measurements, pivot gizmo, collision shapes |
| Lighting | Small: “Studio ▾”, “Rotate light”, “Background: Gray ▾” | Lighting preset (Studio, Outdoor, Night, scene lighting); rotate the key light; backdrop color |
| LOD | Small: Auto (off), “LOD 0 ▾”, “Show tri density” (off) | Auto switches LOD by distance like in game; otherwise a fixed LOD; tri density tints dense areas |
| Tools | Large: Reimport, Pivot, Shot | Reimport from the source file; edit the pivot point; save a screenshot / thumbnail |
| Use | Large: **Place** (accent-tinted). Small: “Show in level” | Place jumps to Level design with this mesh on the cursor; Show in level selects every placed copy |

### Document tab strip (30 px, #0E0E0E)

- **SM\_Crane\_Arm** (active: #1C1C1C, white text, 1 px accent line on top, cyan mesh dot, close × in #8C8C8C) and **SM\_Crate\_Large** (inactive) — several models can be open as tabs.
- Right end: link **“‹ Back to Library”** (11.5 px, link blue) returns to the Library grid with the asset selected.

### Center, left group: model viewport (about 75%, #181818)

- **Backdrop:** radial gradient #2E2E2E at the center (slightly above middle) fading to #151515 at the edges — a neutral photo-studio look.
- **Ground:** soft black contact shadow (ellipse, 35% opacity) under the model and three faint white floor lines (6% opacity).
- **Mesh:** SM\_Crane\_Arm drawn large and centered, flat-shaded grays (lit faces #8E8E8E–#A6A6A6, shaded #565656–#626262).
- **Bounds box:** dashed accent rectangle (5 px dash, 4 px gap, 80% opacity) hugging the model.
- **Dimension lines:** 1 px #A6A6A6 lines with end ticks — one under the model for width, one to its right for height — each with a label chip (#121212, radius 4) in 11 px JetBrains Mono #E6E6E6: “4.0 m” and “22.0 m”.
- **Pivot gizmo** at the base: short axis lines X #E5534B, Y #6CC24A, Z #4C8DFF (2.2 px) from a white 3.5 px dot.
- **Overlays:**
  - Top left: breadcrumb pill “Library › Environment › Harbor › **SM\_Crane\_Arm**”.
  - Top right, stacked: mono pills “LOD 0 · 12,418 tris” and “7,902 verts · 2 materials”.
  - Bottom right: camera presets as a segmented group (#121212 at 90%, #2E2E2E border, radius 6): Front · Side · Top · **Perspective** (active, #2C2C2C, white).
  - Bottom left: hint pill with an amber sun icon — “Shift-drag rotates the light · Drag orbits · Wheel zooms”.

### Center, right group: UV layout (280–360 px, #141414)

- Header (30 px): “UV layout” and a “UV0 ▾” dropdown (UV channel).
- The 0–1 UV square: #121212 with a #333333 border and a 4 × 4 grid in #222222; seven islands outlined in #C8C8C8 (1 px) with an accent fill at 10%. The island under the mouse / matching the selected face is outlined in accent at 1.6 px.
- Footer: “7 islands · 0 overlaps” and a small “Stretch” toggle (off) that colors islands by distortion.

### Shelf bar

Tabs: **Overview** · LODs · Materials · Collision · Sockets · 2 · Import settings. Right: “crane\_arm.fbx · imported today 10:04”.

### Shelf — Overview tab

| Column | Contents |
| --- | --- |
| Levels of detail | Header + “screen size” hint. Rows (28 px): **LOD 0** 100% · 12,418 tris · 7,902 verts (selected, accent-tinted); LOD 1 45% · 5,120 tris · 3,210 verts; LOD 2 18% · 1,480 tris · 960 verts. Dashed “+ Generate LOD” button. |
| Material slots | Header + count 2. Cards (#1F1F1F, radius 7): 30 px sphere preview, name and slot use — M\_Steel\_Painted “Slot 0 · arm, tower”; M\_Rust “Slot 1 · hook, cable” — each with an “Open” link to the Shading tab. |
| Import | Source Art/Harbor/crane\_arm.fbx (mono) · Scale 1.0 · Up axis Y up · Normals Imported · Bounds 4.0 × 22.0 × 2.0 m · Warnings “None” in green (#8FD4A8). |

### Intended behavior

- Drag orbits around the model, middle-drag pans, wheel zooms, F frames it; Shift-drag rotates the key light independently of the camera.
- View modes and overlays change only this viewer, never the level.
- Clicking an LOD row (or the LOD dropdown) shows that LOD; Auto switches by simulated distance and shows which LOD is active.
- Reimport re-reads the source file with the saved import settings and keeps material assignments.
- Hovering the UV square highlights the matching faces on the model and the reverse.
- Place and Show in level link to the Level design and Level editor screens.

### Status bar (right side)

“Viewing SM\_Crane\_Arm · LOD 0 · Studio lighting”.

## Screen 10 — Audio · Mixer

A console-style mixer: one channel strip per bus, the selected bus’s effect (EQ) beside it, and the game’s sound events in the shelf. Meters stay gray until a signal gets hot, so color only appears when something needs attention.

### Ribbon (Audio)

| Group | Controls | Intended behavior |
| --- | --- | --- |
| Mixer | Large: Bus, Send, Snapshot | Add a bus (routes to Master by default); add a send from the selected bus to a return; save the current fader and effect state as a snapshot |
| Monitor | Small: “Clear solo”, “Meters: Peak + RMS ▾”, “Listen from: Camera ▾” | Unsolo every bus; meter style; where 3D sound is heard from while editing (editor camera, player, or a selected object) |
| Effects | Large: EQ, Compress, Reverb, Limiter | Adds that effect to the selected bus’s next free insert slot |
| Play | Play, Simulate | As everywhere; the mixer keeps metering live during play |

### Center, left group: mixer console (about 60%, #141414, 440 px tall)

- **Document tab:** “MX\_Harbor\_Main” with an amber speaker icon. Several mixers can be open (for example a separate one for cutscenes).
- **Strips** sit in a row, 8 px apart with 10 px padding, scrolling sideways when they don’t fit.
- **Strip anatomy** (104 px wide, #1A1A1A, 1 px #262626 border, radius 8), top to bottom:
  1. **Header:** bus name (12 px / 600, #E6E6E6, cut off with “…”) and routing below it (10 px #8C8C8C, e.g. “Bus → Master”), on #161616 with a line under it.
  2. **Insert slots:** 20 px rows, #222222, radius 4, 6 px side margin. An active effect shows a 5 px green dot (#4FC28A) before its name; an empty slot reads “+ Insert” in #6E6E6E.
  3. **Reverb send:** an 18 px knob (#121212 face, #3A3A3A ring, white pointer line), the label “Rev” and the send level in mono on the right. Buses without a send leave the space empty.
  4. **Pan:** a 3 px #2C2C2C track between “L” and “R” (10 px gray) with a 2 × 9 px white marker.
  5. **Fader and meters** (at least 190 px tall): a dB scale on the right of the fader area (+6, 0, −6, −12, −24, −48 in mono 9 px #6E6E6E, each with a faint #262626 line), a 3 px #121212 fader track, and a 26 × 16 px fader cap (radius 3, gradient #5A5A5A → #3A3A3A, #6E6E6E border, white center line, small drop shadow). The travel runs from +6 dB at the top to −60 dB at the bottom.
  6. **Stereo meters** beside the fader: two 5 px bars on #121212, filled from the bottom with a gradient from #6E6E6E to #9A9A9A, and a 2 px white peak-hold line just above the level. When a meter goes above about 86% of its range the fill tops out in amber (#E8B04A).
  7. **Readout:** the fader value in mono 11 px, centered (“−3.0 dB”, or “0.0 dB”).
  8. **M / S buttons:** two 22 px buttons sharing the width, bold 11 px. Off = #262626 with #A6A6A6 text. Mute on = amber fill with dark text; Solo on = accent fill with dark text. Master has Mute only.
- **Selected strip:** accent border plus a 1 px accent ring, header tinted with the accent (16%), name and readout in white.
- **Muted strip:** the whole strip drops to 55% opacity, so it reads as “out” at a glance.

| Strip | Routing | Inserts | Rev send | Pan | Fader | Meters (L / R) | State |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Music | Bus → Master | EQ, + Insert | −18 dB | slightly left | −8.0 dB | 48% / 52% | — |
| SFX | Bus → Master | EQ, Comp | −12 dB | center | −3.0 dB | 78% / 74% | Selected |
| Ambience | Bus → Master | Low cut, + Insert | −6 dB | center | −10.0 dB | 40% / 44% | — |
| Dialogue | Bus → Master | Comp, + Insert | −24 dB | center | −2.0 dB | silent | — |
| UI | Bus → Master | + Insert | none | center | −6.0 dB | 22% / 22% | Muted (dimmed, amber M) |
| Reverb | Return | Reverb, + Insert | none | center | −9.0 dB | 36% / 38% | — |
| Master | Output | Limiter, + Insert | none | center | 0.0 dB | 88% / 84% (amber, hot) | No Solo button |

### Center, right group: bus effect (about 40%, #161616)

- **Header** (30 px, #0E0E0E): “SFX › **EQ**” (the selected bus and effect) and an EQ / Comp switch on the right (EQ active) to flip between that bus’s inserts.
- **Graph:** a #121212 panel (radius 6, #222222 border) with a #222222 grid and a slightly brighter #333333 line at 0 dB. The EQ response is a 2 px accent curve with an 8% accent fill underneath.
- **Band handles:** four points on the curve, dark fill with a light 1.5 px ring. The selected band (the bell at 2.4 kHz) is larger and filled in accent.
- **Axis labels** (mono 9.5 px #6E6E6E): 20, 100, 500, 1k, 5k, 10k, “20k Hz” along the bottom; “+12 dB” top left.
- **Band cards** (4 equal columns under the graph; #1F1F1F, #2A2A2A border, radius 6): type in gray over the value in mono — Low shelf “120 Hz · −3”, **Bell “2.4 kHz · +4”** (selected: accent border, accent tint, white value), Bell “6 kHz · −1”, High cut “14 kHz”.
- **Intended:** drag a handle sideways for frequency and up or down for gain; the mouse wheel over a handle changes its width (Q); double-click the curve adds a band.

### Shelf bar

Tabs: **Sound events** · Snapshots · 3 · Console. Right: “14 / 64 voices” (sounds playing now / voice limit).

### Shelf — Sound events tab

A table on #1A1A1A with a 28 px header row (10.5 px / 600 caps, #8C8C8C, on #161616) and 26 px rows. A 6 px green dot before the name means the event is playing right now.

| Event | Bus | Volume | Space | Playing |
| --- | --- | --- | --- | --- |
| SFX\_Crane\_Creak (selected, playing) | SFX | −4 dB | 3D | 1 voice |
| SFX\_Crate\_Impact | SFX | −2 dB | 3D | 0 voices |
| SFX\_Gulls\_Loop (playing) | Ambience | −8 dB | 3D | 3 voices |
| AMB\_Harbor\_Night (playing) | Ambience | −6 dB | 2D | 1 voice |
| MUS\_Harbor\_Theme (playing) | Music | −5 dB | 2D | 1 voice |
| VO\_Foreman\_01 | Dialogue | 0 dB | 3D | 0 voices |
| UI\_Click | UI | −10 dB | 2D | 0 voices |

The selected row is accent-tinted with white text; “0 voices” is dimmed to #6E6E6E.

**Event detail column** (320–380 px): header “SFX\_Crane\_Creak” with a light **Preview** button (#E6E6E6, dark label and play triangle) on the right. Below it a 40 px waveform on #121212 drawn as thin bars — the part already played is in accent, the rest #5A5A5A. Then fields: Attenuation “2 m → 40 m · log”, Pitch variation “± 5%”, Max voices “4 · steal oldest”, Used by “CraneController, SEQ\_Crane\_Intro”.

### Intended behavior

- **Solo and mute:** Solo plays only the soloed buses (several can be soloed); Mute silences a bus and dims its strip. “Clear solo” resets all. Neither is saved into the game — they are for listening only.
- **Snapshots:** a snapshot stores fader, send and effect values. The game can blend to a snapshot (for example “Underwater” or “Pause menu”) over a set time; the Snapshots tab lists them and previews a blend.
- **Listen from:** chooses the listener for 3D sounds while editing — the editor camera, the player, or a selected object — so attenuation can be checked in place.
- **Routing:** drag a strip’s routing line onto another strip to send its output there; dragging an event row onto a strip moves the event to that bus.
- **Metering:** meters run live in edit and play mode, showing peak with RMS behind it (per the Monitor setting); the peak line holds for about a second; clicking a meter resets its peak.
- Double-clicking a fader resets it to 0 dB; Alt-drag fine-tunes.

### Status bar (right side)

“48 kHz · 14 / 64 voices · audio CPU 1.2%”.

## Screen 11 — VFX · Effect editor

A **suggested design** for building particle effects: a live preview on the left and the effect’s emitters as side-by-side stacks of modules on the right, with the selected module’s editor in the shelf. The mockup edits “FX\_Crane\_Sparks” — sparks, smoke and a glow coming off the crane hook.

### Ribbon (VFX)

| Group | Controls | Intended behavior |
| --- | --- | --- |
| Emitter | Large: Emitter (+), Duplicate | Adds a new emitter column to the effect; Duplicate copies the selected emitter with all its modules |
| Add module | Large: Spawn, Velocity, Color, Size, Collide | Inserts that module into the selected emitter at the right stage (Spawn → Spawn, Velocity → Initialize, Color/Size → Update, Collide → Update) |
| Simulation | Small: “Loop” (on), “Speed: 1× ▾”, “Show bounds” (off) | Loop the preview; slow motion (0.1×–2×); draw the effect’s bounding box |
| Preview | Small: “Background: Scene ▾”, “Time of day: Night ▾”, “Follow emitter” (on) | Preview against the current level, a gray studio or black; lighting preset; keep the camera on a moving emitter |
| Play | Play, Simulate | As everywhere |

### Center, left group: effect preview (about 40%, #111111)

- **Document tabs:** “FX\_Crane\_Sparks” (active, small amber sun icon), “FX\_Harbor\_Mist”.
- **Scene:** dark night sky (#101010 → #242424) over a #1A1A1A floor with a faint perspective grid (white at 4%). The crane cable and hook are near-black (#0A0A0A) coming down from the top.
- **Sparks:** about 70 short streaks (1.6 px, rounded ends) bursting downward from the hook in a cone and arcing with gravity. Color by distance from the source: near = pale #FFF2C8, middle = #F2C46D, far = #E57F6A, fading in opacity as they travel. A bright #FFF8E0 core dot sits at the hook with a soft amber radial glow (#F2C46D at 45% → 0).
- **Smoke:** five soft gray (#8C8C8C) circles rising and growing above the hook, opacity falling from 18% to 5%.
- **Overlays:** top left pills “Lit ▾” and “Overdraw” (view modes); top right, stacked mono pills “842 particles” and “GPU 0.31 ms”.
- **Playback bar** (bottom, 30 px, #121212 at 90%, radius 7): Restart and Pause icon buttons, a 4 px timeline with accent fill to the current time (38%), and “0.76 / 2.0 s” in mono.

### Center, right group: emitter stack (about 60%, #151515)

- **Header** (30 px, #0E0E0E): “Emitters 3” on the left, “Modules run top to bottom” in gray on the right.
- **Columns:** one per emitter, side by side (each at least 190 px), 10 px gaps.
- **Column header** (32 px, radius 7, #181818): emitter name (12 px / 600), a tiny badge for where it simulates (“GPU” or “CPU”, 9.5 px, #2C2C2C), and a small enable toggle on the right. The selected emitter’s header is accent-tinted with white text.
- **Stage labels** between groups of cards: SPAWN, INITIALIZE, UPDATE, RENDER (10 px / 600 caps, #8C8C8C).
- **Module card:** #1F1F1F, 1 px #2A2A2A border, radius 7, 8 × 10 px padding. Title (11.5 px / 600) with a 10 px accent square on the right meaning “enabled”; below, key–value rows (keys #8C8C8C, values mono #E6E6E6) and, where useful, a thumbnail.
- **Selected module** (Sparks › Color over life): accent border, 1 px accent ring and an accent tint at 8%.

| Emitter | Stage › Module | Values / thumbnail |
| --- | --- | --- |
| Sparks (GPU, selected) | Spawn › Burst | Count 40 at 0.0 s |
|  | Spawn › Rate | 120 per second |
|  | Initialize › Particle | Lifetime 0.4–0.9 s · Speed 4–8 m/s · Cone 35° down · Size 0.02–0.05 |
|  | Update › Gravity & drag | Gravity −9.8 · Drag 0.6 |
|  | Update › **Color over life** (selected) | 12 px gradient strip: #FFF8E0 → #F2C46D → #E57F6A → transparent, over an alpha checkerboard |
|  | Update › Collision | Bounce 0.3 · Kill after 2 hits |
|  | Render › Sprite | Blend Additive · Material M\_Spark |
| Smoke (CPU) | Spawn › Rate | 12 per second |
|  | Initialize › Particle | Lifetime 2–3 s · Speed 0.4 m/s up |
|  | Update › Size over life | Small curve thumbnail: rises fast, then levels off (#C8C8C8 line) |
|  | Update › Color over life | Gradient: gray at 60% → transparent, over checkerboard |
|  | Render › Sprite | Blend Alpha · Material M\_Smoke\_Soft |
| Glow (CPU) | Spawn › Single | Count 1, follows hook |
|  | Render › Light | Color #F2C46D · Radius 3 m · Flicker 0.2 |

- A dashed “+ Module” button (28 px, radius 7, #3A3A3A dashed border) closes the Glow column.

### Shelf bar

Tabs: **Module · Color over life** · Parameters · 4 · Performance. Right side: the module’s location “Sparks › Update” in gray. The first tab is named after whatever module is selected.

### Shelf — Module tab: gradient editor

- Title: “Color and alpha over the particle’s life” (12 px / 600).
- **Gradient bar:** 28 px tall, radius 6, #333333 border, drawn over a 12 px gray checkerboard so transparency is visible.
- **Stops:** 14 px diamonds under the bar at 0% (#FFF8E0), 30% (#F2C46D, **selected**: accent border), 65% (#E57F6A, 80% alpha) and 100% (#E57F6A, 0% alpha). Unselected stops have a light #E6E6E6 border.
- **Axis** under the stops (mono 10 px, #6E6E6E): birth · 25% · 50% · 75% · death.
- **Selected stop fields:** position 30%, color swatch + “#F2C46D”, Alpha 100%, Intensity × 4.0 (HDR brightness for glow).

### Shelf — Performance column (300–360 px)

- Header “Performance” with “budget 1,000” on the right.
- **Graph** (64 px): particle count over the effect’s duration as an accent line with a 14% accent fill; the 1,000-particle budget is a dashed #4A4A4A line across the top. The count spikes at the burst and settles.
- **Stats:** Peak particles 842 · GPU time 0.31 ms · Overdraw 2.1× · Memory 1.4 MB.

### Intended behavior

- Modules run in the order shown, top to bottom inside each stage; drag cards to reorder, or to another emitter to move them.
- Click a module to edit it in the shelf; Delete removes it; the enable square switches it off without deleting.
- Any value can be exposed as a **parameter** (Parameters tab), so gameplay can change it at runtime — for example spark count driven by crane speed.
- The preview restarts on every edit unless paused; Speed slows it down for checking motion.
- Budget warnings: when peak particles or GPU time cross the effect’s budget, the Performance tab and the status bar turn amber, and the costliest emitter is named. Intended LOD settings reduce spawn rates with distance.

### Status bar (right side)

“FX\_Crane\_Sparks · 3 emitters · looping”.

## Screen 12 — File · Project settings

Shared settings for the whole project, shown in the File view with **Project settings** active in the File menu. Unlike Preferences (personal, per machine), these are stored in the project’s /Config folder and go through version control, so every change reaches the team on check-in. The mockup shows the General tab.

### Shell

Same File view as Recent projects and Preferences: tab bar with File active, no ribbon, no shelf; the File menu column on the left (Back to editor, Recent projects, New project, Open…, Save all, Project settings (active: #1E1E1E fill, white text), Preferences, Plugins, Learn, Exit); page content on #141414 with 28 / 36 px padding.

### Header

| Element | Details |
| --- | --- |
| Title | “Project settings”, 22 px / 700 |
| Subtitle | “Shared with everyone on Harbor\_Prototype. Stored in /Config and tracked in version control.” — 12.5 px #8C8C8C; “/Config” in mono #C8C8C8 |
| Changes pill | 26 px pill, radius 13, accent mixed 14% into #141414, a 6 px accent dot and “4 changed from defaults” (11.5 px #E6E6E6) |
| Search | 280 × 32 px, #0E0E0E, #2C2C2C border, radius 7, placeholder “Search project settings” |

### Sub-tabs

General (active: white, weight 500, 2 px accent underline) · Rendering · Physics · Input · Audio · Platforms · Build · Plugins. 12.5 px, inactive #A6A6A6, 18 px apart, on a 1 px #262626 bottom line. Input is its own designed page (Screen 13).

### Setting rows and the “changed” marker

Same row pattern as Preferences: name (12.5 px / 500) and one-line description (11.5 px #8C8C8C) on the left, a 260 px right-aligned control column, 12 px vertical padding, 1 px #222222 divider. Groups sit under 10.5 px uppercase section caps with 18 px space above.

A setting that differs from the engine default gets a **6 px accent dot** in the left margin (14 px left of the name, the column is indented 14 px to make room) and a small **Reset** link right after its name.

### Identity

| Setting | Description | Control · value | Changed |
| --- | --- | --- | --- |
| Project name | Shown in the window title, builds and the Project hub. | Text field (260 px) “Harbor\_Prototype” |  |
| Company | Used for save-file folders and build metadata. | Text field “\[STUDIO NAME\]” |  |
| Version | Your game’s version, not the engine’s. Builds stamp it. | Mono field (120 px) “0.3.0” | Yes |
| Project icon | Window, taskbar and build icon. PNG, at least 256 × 256 px. | 44 px icon tile (#262626, radius 10, cube icon) + “Change…” button |  |

### Startup

| Setting | Description | Control · value | Changed |
| --- | --- | --- | --- |
| Game start level | The first level loaded when the built game starts. | Dropdown “MainMenu” |  |
| Editor start level | Opened when the project opens in the editor. | Dropdown “Docks\_Night” | Yes |
| Default game mode | Rules, player and camera used when a level sets none. | Dropdown “GM\_Harbor” |  |
| Target frame rate | Frame-rate cap for the built game. Players can override it. | Segmented: 30 · **60** · 120 · Unlimited | Yes |

### Engine

| Setting | Description | Control · value | Changed |
| --- | --- | --- | --- |
| Engine version | The engine this project is built with. | “v0.1.0” (mono) · “up to date” (green #8FD4A8) · “Check now” link |  |
| Scripting | How game logic is written. Coding happens through Ledger for now; the built-in code editor comes later. | Dropdown **“Ledger”** |  |
| Units and axes | World units and which axis points up. Changing this re-imports assets. | Dropdown (110 px) “Meters” + segmented **Y up** · Z up | Yes |

### Right column (280–330 px)

- **Changed from defaults** card (#1A1A1A, #2A2A2A border, radius 10, 14 px padding): one line per change — accent dot, setting name, old → new in gray mono: Version 0.1.0 → 0.3.0 · Editor start level none → Docks\_Night · Target frame rate 30 → 60 · Physics › Gravity -9.81 → -12.0, the last with a small “Physics” chip (#262626, 10 px) because it lives on another tab. Intended: clicking a line jumps to that setting.
- **Version control** card: branch icon + “Version control”; “Config/Project.ini has changes that are not checked in. Teammates get them after you check in.” (file name in mono); buttons **Check in** (accent fill, dark label) and **Review** (#262626).
- **Footnote** (11 px #8C8C8C): “A dot before a setting marks a value changed from the engine default. Personal settings such as theme and shortcuts live in Preferences.”

### Personal vs project

|  | Preferences (Screen 8) | Project settings (this screen) |
| --- | --- | --- |
| Who it affects | Only you, on this machine | Everyone on the project |
| Stored in | User profile | /Config, in version control |
| Examples | Theme, accent, density, shortcuts | Name, start levels, frame rate, scripting, input, physics |

### Intended behavior

- Changes save to /Config immediately; there is no Save button. The status bar and the Version control card show they are not checked in yet.
- Search covers all sub-tabs; matches are grouped under their tab name with the words highlighted.
- Reset returns one setting to the engine default; the dot and the line in “Changed from defaults” disappear.
- Changing Units and axes warns first that assets will be re-imported and lists how many.
- Check in commits the config files with an auto-written message (“Project settings: Version, Target frame rate”); Review shows a side-by-side diff first.
- When a teammate’s settings change arrives, a notice offers to reload the affected systems.

### Status bar (right side)

“Saved · not checked in”, then “v0.1.0”.

## Screen 13 — File · Project settings › Input

The game’s input actions and their keybinds, shared by the whole team through Project settings. It defines what the *player* presses; the editor’s own shortcuts are personal and live separately in Preferences › Shortcuts. The page uses the File view shell with “Project settings” active in the File menu and the **Input** sub-tab selected (same header as Screen 12: title, shared-settings subtitle, “4 changed from defaults” pill, search).

### Toolbar (one row, wraps on narrow windows)

- **Context chips** (26 px, radius 13): **Gameplay** (active: #262626 fill, #3A3A3A border, white, weight 500), Crane (vehicle), Menus, Photo mode (inactive: #A6A6A6 text, #2C2C2C border), then a dashed “+ Context” chip. A context is a set of actions that is active at the same time (on foot, driving the crane, in a menu).
- **Device filter** (segmented): All devices (active) · Keyboard & mouse · Gamepad.
- **Search** (28 × 220 px): “Find action or key” — matches action names and bound keys (typing “E” finds everything on E).
- **Add action** (accent fill, dark label, plus icon) — the page’s main action.

### Bindings table (left, takes the remaining width)

- Panel: #1A1A1A, 1 px #262626 border, radius 10.
- **Header row** (32 px, #141414): Action · Type · Keyboard & mouse · Gamepad · Trigger, in 10.5 px / 600 uppercase gray caps.
- **Columns:** Action (min 150 px) · Type (88 px) · Keyboard & mouse (widest) · Gamepad · Trigger (92 px); 10 px gaps, 14 px side padding.
- **Rows:** at least 38 px tall, 1 px #222222 line under each. Action name 12 px / 500; type and trigger 11 px gray.
- **Group header rows** (28 px, #161616): MOVEMENT, INTERACTION, CRANE in 10.5 px uppercase caps. The Crane header adds “· also in context ‘Crane (vehicle)’” in normal-case #6E6E6E.
- **Keycaps** (keyboard and mouse): 22 px tall, min 22 px wide, radius 5, #262626 fill, #3A3A3A border with a **2 px bottom border** so they read as physical keys, JetBrains Mono 10.5 px. Alternatives are separated by a gray “or”; axis halves by “− / +”.
- **Gamepad chips:** #1E1E1E fill, #444444 border, mono 10.5 px. Face buttons (A, B, X, Y) are **round** 22 px circles; sticks, shoulders, triggers and clicks (Left stick, LB, RT, L3) are **pills** with 6 px radius. Generic labels only, no console brand colors.
- **Add binding:** a 22 px dashed square “+” after the last chip (shown on Release load).

| Group | Action | Type | Keyboard & mouse | Gamepad | Trigger |
| --- | --- | --- | --- | --- | --- |
| Movement | Move | Axis 2D | W A S D, or arrow keys | Left stick | Continuous |
| Movement | Look | Axis 2D | Mouse move | Right stick | Continuous |
| Movement | **Jump** (selected) | Button | *Listening:* “Press a key… Esc cancels” | A | Pressed |
| Movement | Sprint | Button | Left Shift | L3 | Held |
| Movement | Crouch | Button | C, Left Ctrl | B | Toggle |
| Interaction | Interact ⚠ | Button | E (conflict) | X | Pressed |
| Interaction | Enter crane | Button | F | Y | Held 0.4 s |
| Crane | Swing ⚠ | Axis 1D | Q − / + E (conflict) | LB, RB | Continuous |
| Crane | Hook up / down | Axis 1D | Wheel | LT, RT | Continuous |
| Crane | Release load | Button | Space, Mouse 2 | D-pad ↓, + | Released |

- **Selected row (Jump):** accent tint at 12% and a 2 px accent bar on the left edge; name in white.
- **Listening state:** the keyboard cell shows one wide keycap “Press a key…  Esc cancels” with a **dashed accent border**, accent-tinted fill (20% into #1A1A1A) and white text. The next key pressed becomes the binding.
- **Conflict state:** a key bound twice in the same context shows as an **amber keycap** (#2E2617 fill, #8A6A2A border, #F2D08A text), and the action name gets a 13 px amber warning triangle (tooltip “E is also bound to Swing right”).
- **Conflict banner** (bottom of the table, #1F1B12 fill, #3A3020 top line): amber warning icon + “E is bound to both **Interact** and **Swing** in Gameplay. Both fire on one press.” in #E8D2A2, and a fix link on the right, “Move Swing to Crane context”.

### Right column (290–330 px, stacked cards, radius 10, #1A1A1A)

**Action details — Jump**

- Header: “Jump” (14 px / 600), a “Button” type badge (#262626), and the context “Gameplay” in gray on the right.
- **Bindings:**
  - Keyboard row (30 px): dashed accent border, accent tint, “Listening…” in white, and “was Space” keycap on the right so the old key is visible while rebinding.
  - Gamepad row (30 px, #1F1F1F): round A chip, “south button” in gray, and a remove (×) button.
  - “+ Add binding” (26 px, dashed #3A3A3A).
- **Fires on:** segmented Pressed (active) · Held · Released.
- **Buffer:** 0.12 s — a press this early still counts (input buffering).
- **Block lower contexts:** toggle, on — when this action fires, contexts underneath don’t also receive the key.

**Device settings**

- Stick dead zone: slider at 0.15.
- Mouse sensitivity: slider at 1.0.
- Invert look Y: toggle, off.
- Players can rebind: toggle, on — exposes these actions to the game’s own controls menu.

**Test input** (#121212 card): a 44 px circle showing the left stick’s live position (accent dot, dashed inner dead-zone ring), plus mono readouts “Move  x 0.62  y 0.41” and “Last: Left Shift → Sprint”.

### Intended behavior

- Click any keycap or gamepad chip to listen for a new input; Esc cancels and keeps the old binding; Backspace clears it.
- Conflicts are checked per context: the same key in two different contexts is fine, the same key twice in one active context is flagged.
- Contexts stack with a priority order (e.g. Menus above Crane above Gameplay); “Block lower contexts” stops a key from falling through.
- The device filter hides the other device’s column and adds device-specific options (e.g. per-stick dead zones).
- Everything here is exported to the game, so a player rebinding menu can be built straight from these actions, with “Players can rebind” controlling which ones appear.
- Changes save immediately into Config and show up as uncommitted in version control, like every Project setting.

### Status bar (right side)

“11 actions · 4 contexts · 1 conflict”.

## Cross-screen behavior

All screens share one selection, one command palette and one set of shortcuts; the ribbon tab decides the workspace, the shelf adapts to it. Only the Level editor’s shelf tabs are built in the mockup; everything else here is intended.

There is no permanent left panel. Its old contents moved: the scene list → the Scene column in the Object and Place shelf tabs; Assets → the Library tab and the shelf’s asset column; Changes → the status bar (“main · 4 changes”) and Library › Source control; Settings and Plugins → File; project name → the right end of the tab bar.

### Keyboard shortcuts

| Action | Shortcut | Where |
| --- | --- | --- |
| Command palette (commands, assets, objects, settings) | Ctrl K | Everywhere |
| Select / Move / Rotate / Scale tool | Q / W / E / R | Viewports |
| Frame selection | F | Viewports, node graph |
| Add node | Tab | Node graph |
| Collapse or expand the shelf | Ctrl+Space | Every screen with a shelf |
| Place and keep the asset loaded | Shift + release | Level design |
| Place without snapping | Alt while dragging | Level design |
| Rotate the ghost 15° | R while dragging | Level design |
| Cancel placement | Esc | Level design |
| Rename asset | F2 | Library |
| Multi-select | Ctrl / Shift + click | Library, scene list, dope sheet |
| Play animation / previous key / next key | Space / buttons in the shelf bar | Level editor, Sequencer |

Shortcut keys appear in the UI as small mono chips (#2C2C2C, 10 px). Intended: all shortcuts are editable in Preferences › Shortcuts.

### Links between screens

| From | Action | Goes to |
| --- | --- | --- |
| Library | Place (ribbon) or drag a tile onto the Home tab | Level design, asset loaded on the cursor |
| Library | Show in level | Level editor, every placed copy selected |
| Library | Double-click an asset | Its editor: material → Shading, animation → Animate, script → code editor |
| Level design | Open Library (link in the Place column) | Library, same folder |
| Level editor | Placed object › Show in library | Library, asset selected |
| Console / Problems | Click “CraneController.cs:84” | Code editor at that line |
| Problems | Bake now | Starts a light bake (Home › Lighting) |
| Status bar | Click error / warning counts | Problems shelf tab |
| Status bar | Click the branch | Library › History |
| Viewport overlay | “Show” (code editor hidden) | Level editor with the code group back |
| File | Back to editor | The last workspace, exactly as left |

Until the built-in code editor ships, every link that would open code (file:line in Console and Problems, double-clicking a script block in the Profiler, double-clicking a script in the Library) opens that script in **Ledger** instead.

### Selection

- One selection for the whole editor. Clicking in the viewport, the scene list or a Library tile selects the same object everywhere.
- The selected object gets a 1.5 px accent outline in the viewport, an accent-tinted row with white text in the scene list, and its name in white in the inspector header.
- Selecting opens the shelf on the tab chosen in Preferences › “Shelf opens on” (default Object).
- Once the code editor ships, it follows the selection: it opens the selected object’s script component, if it has one.
- A newly placed object is selected immediately.

### Drag and drop

- **Asset → viewport:** places it (see Level design). The shelf sits right under the viewport, so the drag is short and upward.
- **Asset → inspector field:** assigns it (mesh, material, sound).
- **Asset → folder tile or breadcrumb:** moves it; references update.
- **Files from the desktop → anywhere:** imports into the current Library folder; progress shows in Library › Import queue.
- **Document tab → editor area edge:** splits or merges editor groups.
- **Shelf grip:** drag to resize the shelf.

### Play mode

- Play runs the game in the viewport; Simulate runs physics and scripts without a player.
- Changes made while playing are temporary. A note says so in the inspector, changed values are highlighted, and “Keep changes” copies them back before stopping.
- Runtime errors land in Console and Problems and pin to their code line with the frame number (“● Target was destroyed (frame 1832)”).
- The Play button turns into Stop while running.

### Persistence (what each workspace remembers)

- Active shelf tab and shelf height.
- Editor group splits and their ratios.
- Open documents and the active one.
- Ribbon collapsed or expanded.
- Level design: Placement settings, the last asset filter (Favorites, Recent, folder) and the target folder for new objects.
- Global, per user (Preferences): accent color, density, interface scale, “Shelf opens on”, code-editor split on or off.

### Responsiveness

- The ribbon scrolls sideways on narrow windows instead of wrapping.
- Toolbars (Library filter chips, Level editor toolbars) wrap onto a second line.
- Shelf columns and inspector cards wrap underneath each other; cards keep a 180 px minimum width.
- Editor groups stack vertically when there is no room side by side.
- Asset grids reflow: tiles keep a minimum width (108 px Library, 92 px Place) and add columns as the window grows.

## Placeholders and open questions

Everything in brackets is a placeholder to replace, and all names and numbers on screen are sample content to make the mockups readable — none of them are features or targets.

### Placeholders and sample content

| Item | Where it appears | What it stands for |
| --- | --- | --- |
| \[ENGINE\] | Not on the G screens (the logo stands alone); used in the earlier variants | The engine’s real name (not decided yet) |
| \[X.Y\] | Update notice in Recent projects / status bar | The next engine version number |
| v0.1.0 | Right end of every status bar, project cards | Current engine version; illustrative |
| Harbor\_Prototype, Neon\_Courier, Glacier\_Sim, Tiny\_Tactics | Tab bar path, Recent projects | Sample project names |
| Docks\_Night, Docks\_Day | Tab bar path, document tabs, Used by | Sample level names |
| Crane\_Arm\_02, Crate\_Stack\_A/B, Bollard\_x12, Harbor\_Lamp\_01 | Scene lists, inspector, profiler, console | Sample scene objects |
| SM\_, M\_, T\_, AN\_, SFX\_, PF\_ names | Library, shelf asset grids, Place tiles | Sample assets; the prefixes show a naming convention (Static Mesh, Material, Texture, Animation, Sound effect, Prefab), not a rule the engine enforces |
| CraneController.cs, HookPhysics.cs, line 84 error | Code editor, console, Problems | Sample script and runtime error |
| SEQ\_Crane\_Intro, CAM\_Shot\_010/020, frame 96 / 240 | Animation shelf, Sequencer | Sample sequence, shots and timing |
| 6.9 ms · 144 fps, 22.4 ms frame, 1,942 draw calls, 2.31 / 8 GB | Viewport pills, Profiler | Illustrative performance figures |
| 12,418 triangles, 7,902 vertices, 1.8 MB, 64 instructions | Library details, Material preview | Illustrative asset statistics |
| main · 4 changes, 1 error, 1 warning | Status bar | Sample version-control and problem counts |

Added with screens 9–13: **\[STUDIO NAME\]** (Project settings › Company) is a placeholder; Harbor\_Prototype version 0.3.0, the MX\_Harbor\_Main mixer, FX\_Crane\_Sparks and all input actions are sample content.

### How the design got here

- Six layout directions (A–F) were explored first; **A** (classic docked), **C** (icon rail + sidebar) and **F** (bottom shelf) were shortlisted and each got two extra variants.
- A neutral gray theme was chosen over warm or cool tinted themes; the accent stays a single user-chosen color.
- **G** combines F1 (full-width viewport + bottom shelf), F2 (toolbox + animation shelf), F3 (ribbon) and C3 (split viewport and code editor, command search).
- The left-hand panel was removed. Its contents moved: scene list → shelf, project name → tab bar, Settings and Plugins → File, changes count → status bar.
- Asset management (Library) was split from placing assets in the world (Level design), linked by “Place” and “Show in level”.
- All earlier variants are kept on the original canvas, **Engine Editor — Dark UI Concepts**; the G screens live on **Engine Editor — G Layout**.

### Open questions

- [ ] Should the scene list also be available as a slide-out panel from the left edge, in addition to the shelf column?
- [ ] Build tab: its ribbon and workspace (targets, platforms, build log) are not designed yet.
- [ ] How exactly does the ribbon collapse — icons only, tabs only, or auto-show on hover?
- [ ] Multi-monitor use: can editor groups and the shelf be undocked into separate windows?
- [x] A light theme is not planned; confirm it stays dark-only.
- [ ] Accessibility: UI scale range, keyboard-only navigation, a high-contrast option.
- [ ] Density vs click targets: the compact 22–26 px controls suit mouse use but are below the 44 px touch guideline; is touch or pen support needed?
- [ ] Code editor auto-hide in Level design: when exactly does it hide and come back, and is the split ratio remembered?

* [ ] Ledger hand-off: how the editor opens a script in Ledger (from an error, a component card, the Library) and whether changes reload live while playing.
* [ ] Audio and VFX: confirm the mixer bus list and the module stages before these editors are built.
