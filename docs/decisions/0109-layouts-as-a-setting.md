# 0109: Layouts as a setting

- **Status:** Accepted (the project lead, 2026-11-13; changes [0099](0099-the-g-layout.md)'s and [0104](0104-docking-is-solis-s-own.md)'s fixed middle for Home's three lists, and narrows [0061](0061-settings-window-and-no-layout-presets.md)'s "no layout presets"; its place in the plan is [0108](0108-shape-brushes-and-sculpting-come-next.md)'s first step)
- **Date:** 2026-11-13

## Problem

After trying the remade interface, the project lead wrote: "Not being able to rescale certain elements is annoying. Also having the Area list as well as asset list in such a compact view on the bottom, I think it's better if we move those up to a more default looking layout. I didn't want to have that, but it's way more convenient."

Under 0099 and 0104, Home's Area list, Properties and Assets sit side by side on the shelf's Entity tab, under the area, at fixed widths; only the shelf's height and the split between two groups can be dragged. The lists are short and the columns can't be widened. The project lead wanted to try other layouts before choosing one, and every edge resizable, and all of it fast.

## Options

1. **One new fixed layout** in place of the shelf's: the quickest, but the project lead asked to try them before choosing.
2. **Free docking again** (any panel anywhere): what 0104 took away, with its saved trees and checks; more than was asked.
3. **A setting with four layouts** (recommended): three that take the lists off the shelf into columns of their own, and the design's shelf as it is, switched at once in Preferences; and every border between panels draggable, remembered per tab and per layout.

## Decision

**Option 3.**

- **The setting:** File › Preferences › Appearance › **Panel layout**, four choices:
  - **Area left, Assets bottom** (the standard from now on): the Area list a tall column at the left of the editor groups, Properties a tall column at the right, Assets a wide strip under the groups.
  - **Area and Assets on the left:** the Area list over Assets in one column at the left; Properties at the right.
  - **Area left, Assets right:** the Area list at the left; Properties over Assets at the right.
  - **The shelf (as designed):** the middle as 0099 and 0104 have it.
- **It applies to Home**, the tab whose shelf had these three columns. In the three side layouts the shelf keeps its other tabs (Place, Console, Problems, Map, the unfinished Animation) and its fold (**Ctrl + Space**); its Entity tab goes, since everything on it is in view. The shelf runs the window's whole width under the columns, which reach from the ribbon down to it. Every other tab is unchanged.
- **Placing** stays the **Place** shelf tab in front (answer 20), in every layout. In a side layout the Place tab holds the **Placement** column alone, and the Assets list takes its placing form wherever it is: in the strip, in the left column or in the right one.
- **Every border drags:** the shelf's top (and its grip), the line between two editor groups, the side columns' edges, the Assets strip's top, the line between two lists stacked in a column, and the lines between a shelf tab's columns. The pointer shows a resize arrow over each; a **double-click** puts one back to its standard size. These are the only new mouse actions (the project lead's answer); there are no new keys.
- **Remembered per tab and per layout,** also after closing Solis: in the frame's saved state (`shell`), each tab's sizes under the layout's name and then the border's, every field with a default, anything unknown or out of its limits passed over. The shelf's height and the split in the shelf layout keep the keys they had, so an earlier build still reads them. Tabs other than Home look the same in every layout and keep one set of sizes.
- **Sizes have least values,** and when the window narrows things give way in a stated order: the center first (to 240 points), then the right column, then the left one (each to 180); narrower still, the lists go back on the shelf until there is room. On the shelf, the column that takes what's left gives way first, then the others from the right, then they go under each other.
- **Fast:** where everything goes is worked out each frame from a handful of remembered numbers and the window's size; nothing walks the area or the files to lay the frame out. The scale check has a case for it: an editor frame in each layout with 10,000 entities and 10,000 files listed.
- **Not presets:** 0061 removed named arrangements saved by the user (Wide screen, Small screen). This is one setting with four fixed layouts; there is still nothing to save or name, and **Reset layout** still puts the tab in front back as it started (now with all its sizes, in every layout).

## Consequences

- Home looks unlike the design's first screens by default; the design's own middle is one choice away and keeps its approved picture.
- The approved pictures gain one of Home in each layout; `editor.png` shows the new standard.
- `shell_reach_tests.rs` checks every panel can be reached in every layout.
- The frame's saved state has a new field (`sizes`) in each tab's part; the interface look a new field (`layout`). Settings from earlier builds open in the new standard layout.
- The window test's positions follow the standard layout.
