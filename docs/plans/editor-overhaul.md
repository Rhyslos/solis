# The editor overhaul

The working plan for the editor's new look ([decision 0085](../decisions/0085-the-editor-overhaul.md)), built ahead of the game UI editor in "Usable first" (ROADMAP.md, step 7). The target is the level editor mock-up from the editor UI research of 4 November (the project lead's document "Editor UI research: what works in engine editors"), confirmed by the project lead on 5 November:

![The level editor: a farm being made](editor-overhaul-level.png)

The research's rules, each from what people praise or complain about in Unreal, Godot, Unity and GameMaker:

1. **Playing is unmistakable, and nothing is lost:** the frame turns a colour while playing; changes made while playing are listed when you stop, with Keep or Undo all.
2. **The game runs inside the editor, and changes reach it at once.**
3. **One search for everything** (Ctrl+K in the mock-up): every action, file, entity and setting, each showing its shortcut.
4. **Workspaces, one per job:** Level, Animate, UI and Code switch the whole layout, each remembered.
5. **Readable first:** neutral dark grey, the blue accent, icons in their kind's colour, text contrast at least 4.5 to 1. Controls don't move between versions.
6. **Properties show values, not buttons:** rare actions in each component's ⋯, a **?** to its manual page, a dot and a reset on a changed value, "shared by 3".
7. **One place per job:** each kind of thing has one tab that does all of it.
8. **The Library is a drawer you can pin.**
9. **A status bar that says what's happening.**
10. **The game UI editor is see-as-you-build** (its own plan, [game-ui.md](game-ui.md)).
11. **Graphs don't turn into spaghetti.**
12. **Assemblies are edited in place.**
13. **Beginners get a way in:** the start page, empty panels that say what to do, tooltips as sentences, nothing shown that isn't built.

## How the work is done

- One milestone per pull request, each drawn onto a real screenshot first when it changes a panel the mock-up doesn't show closely.
- **Nothing shown that isn't built** (decision 0070) still holds: the mock-up's **Playtest build** button, the Layers tab and any workspace tab wait for the milestone that makes them work.
- New keys or mouse actions are asked about first (CONTRIBUTING.md, rule 17), with a recommended choice.
- The docking (decision 0004) stays: the mock-up is the standard layout, and panels can still be moved.

## Milestones

### O1. The frame: top bar, status bar, panel tabs

- **Top bar:** the logo, the menus (File, Edit, View, Entity, Window, Help), then on the right one group of Play (blue), Pause, Step and Stop.
- **Status bar** along the bottom: "Lust checked 0.2 s ago" (or the first mistake), the number of problems (clicking opens Problems), "Saved" or "Unsaved changes", and on the right the area's kind and size (`farm · 2D · 480 × 270`) and whether Ledger is connected.
- **Panel tabs** with an icon each and the accent underline; the panels' standard places as in the mock-up (Area on the left, Properties and Map on the right, Library, Console, Problems, Mixer and Profiler at the bottom).
- The viewport's own strip: Viewport and Game tabs on the left, the view's choices (2D, Lit, Grid, Snap) as small chips on the right, frame time and entity count in the corner.

### O2. Properties: values, not buttons

- Each component a card: its icon and name, **?** (its manual page) and **⋯** (reset, remove, copy, paste, move up or down).
- A changed value shows a dot and its standard value to reset to (`3.5 ↺ 3.0`).
- A file shared by several things says so (`farmer.aseprite · shared by 3`).
- The entity's name with its tags (`player`, **+ tag**).
- **+ Add component…** you can type into to search.
- A game code component shows its file and **Open in Ledger ↗**.

### O3. The Library as a drawer

- Filter chips (All, Sprites, Maps, Code, UI, Sounds) and tags (`#crops`), a search box for the whole project.
- Cards coloured along the bottom by kind, as now.
- A drawer that pops up over the editor and can be pinned to stay docked (the key is asked first; the mock-up shows Ctrl+Space).

### O4. One search for everything

- A search box in the top bar ("Search or do anything…") that finds every menu action, file, entity and setting, with its shortcut beside it, and runs or opens what's picked (the key is asked first; the mock-up shows Ctrl+K).

### O5. Playing is unmistakable

- The editor's frame takes a colour while the game runs (paused: another).
- Changes made to the area while playing are kept in a list; when you stop, **Keep** or **Undo all**, never thrown away silently.

### O6. Workspaces

- Level, Animate, UI and Code in the top bar, each a remembered layout of the panels. Each appears once what it holds is built: Level and Animate first; UI with the game UI editor; Code once it has something to hold (game code is written in Ledger, so what the Code workspace shows is asked first).

### O7. Help in place

- A **Hints** panel that says what the selected thing is and what to do with it, which can be turned off in Settings.
- The viewport's tool bar floating at the bottom middle, as the tile painter's already does, with the tool's choices beside it (`Ground ▾`).
- A readout by the cursor in 2D (`x 192 · y 64 px · cell 12, 4`).
- The Area panel's rows with their kind on the right (`tile map`, `Player`, `assembly`, `follows Farmer`) and an eye to hide each.

After O7, the game UI editor (the game UI plan's M4 onwards) is built inside the UI workspace.

## Open questions for the project lead

- The keys for the search (Ctrl+K) and the Library drawer (Ctrl+Space): asked before O3 and O4.
- What the Code workspace holds: asked before O6.
- The other three mock-ups of 4 November (playing, the UI workspace, the search) shape O4 to O6; they're used as drawn there.
