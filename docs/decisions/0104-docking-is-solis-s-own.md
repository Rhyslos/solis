# 0104: Docking is Solis's own

- **Status:** Accepted (carries out [0099](0099-the-g-layout.md)'s editor groups and shelf; replaces the dockable panels of [0004](0004-editor-layout.md) and the Library drawer and workspaces of [0085](0085-the-editor-overhaul.md); keeps [0061](0061-settings-window-and-no-layout-presets.md)'s Reset layout and no presets)
- **Date:** 2026-11-12

## Problem

Until now the editor's panels were docked with a library (egui_dock): any panel could be dragged to any edge, split, stacked as tabs or floated, and the whole tree was saved. The project lead's design, the G layout, has no free docking. Each tab of the tab bar shows a fixed middle: one or two **editor groups** of document tabs side by side, and under them a **shelf** of tabs whose contents are columns laid side by side, with a status bar under it all (spec 128-164). What each tab remembers is small: its shelf tab, the shelf's height and whether it is folded, the split between its groups, and whether the ribbon is collapsed (spec 1131-1138).

The inventory's part (e.1) looked at styling the library into that shape. It could draw the center's tab strips nearly as designed, but not the shelf (pill tabs with a grip and tools on one bar, and columns that wrap), not anything at the right end of a tab strip, not a group with a header instead of tabs, not groups that stack when the window is narrow, and its saved format is its own tree, which Solis has spent 440 lines checking.

## Options

1. **Keep the library for the center only**, and write the shelf ourselves: two ways of laying out panels, about 200 lines of workarounds, and the saved trees kept.
2. **Write the frame's middle ourselves**: one or two groups, a strip of document tabs each, a divider that a drag moves, the shelf with its grip, tabs, tools and columns. About 900 lines with tests; one dependency fewer; a saved format of four small values per tab that Solis owns.

## Decision

**Option 2.** The editor's middle is Solis's own code (`crates/editor/src/shell/`): `workspaces.rs` is one table of what each tab's groups and shelf hold, `groups.rs` draws the groups, `shelf.rs` the shelf, `status.rs` the status bar, and `saved.rs` the frame's saved state under its own settings key (`shell`). egui_dock leaves the workspace's dependencies.

- **Panels don't move.** Each tab's middle is the design's: Home shows the area (and the Game while it runs) over the shelf's Entity, Console and Problems tabs; Library the Library, with a tileset or a picture's crops opening as a second group; Animate the viewport, with an animator as a second group; UI the canvas; Audio the mixer; Debug the game and the Profiler. A panel that comes with a file (an animator, a tileset, a clips file, a tile map selected) adds its document or shelf tab while the file is open.
- **What a tab remembers:** its shelf tab, the shelf's height and fold, the split; the ribbon's collapse and the tab in front are the frame's. Reset layout puts the tab in front back as the design draws it.
- **Saved arrangements from before are not carried over** (the plan, 4.2): a dock tree means nothing to the frame. The old keys (`panel_layout2`, `workspaces`, `library_pinned`) are never read for what they hold and never written or removed, so an earlier build still finds them; the Console says once that the arrangement wasn't carried over.
- **The Library drawer and its pin go.** Ctrl + Space now folds and unfolds the shelf (the design's key, approved by the project lead), working while typing as the drawer's key did. The Library is a column of Home's Entity shelf tab and the whole of the Library tab.
- **Dragging a document tab to split or merge groups** (spec 151) is built last, in M15, as the plan says; until then groups have fixed splits with a divider that a drag moves.

## Consequences

- One dependency fewer, and with it the advisory exception `deny.toml` kept for a helper it used.
- No panel can be put anywhere the design doesn't put it. Someone who liked a docked arrangement of their own loses it once, and is told so.
- Who the keys belong to is still the panel under the pointer (`key_owner.rs`): the frame notes where it drew each panel, as the dock did.
- Wide panels in narrow columns (Properties beside the Area tree and the Library) look cramped until their screens are rebuilt (M5 onward).
