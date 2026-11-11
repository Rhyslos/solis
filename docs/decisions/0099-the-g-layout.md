# 0099. The G layout: the editor's frame, to the project lead's own design

- **Status:** Accepted (the project lead, 2026-11-11; carries out [0098](0098-ideas-become-milestones.md)'s "the editor's UI, remade"; replaces the layout of [0004](0004-editor-layout.md) and the settings window of [0061](0061-settings-window-and-no-layout-presets.md); changes [0085](0085-the-editor-overhaul.md), [0086](0086-the-launcher.md) and [0090](0090-always-through-the-launcher.md); its theme is [0100](0100-one-theme-five-accents.md) and its placeholders [0101](0101-unfinished-parts-as-placeholders.md))
- **Date:** 2026-11-11

## Problem

Decision 0098 put "the editor's UI, remade" first in the plan, to a design the project lead would bring. On 11 November they brought it: a specification of 1,192 lines and thirteen screens, the "G layout", and asked for all of Solis's interface to be remade to it, with a placeholder wherever the engine lacks a feature the design shows, and one setting that hides everything unfinished.

The design is not a restyling of today's editor. Today's frame is a one-row top bar with menus and dockable panels (decision 0004), arranged into workspaces (decision 0085). The design has no menus and no docking: areas of the engine are ribbon tabs, the middle is one or two editor groups, and everything else sits in a shelf of tabs along the bottom.

## Options

1. **Restyle the docked editor** in the design's colours and stop there. Cheap, and not what was asked: the layout is the design.
2. **Keep the docking library for the middle** and write only the ribbon and the shelf. The library can't draw the shelf, a group with a header instead of tabs, or anything at the right end of a tab strip, and its saved layouts are a free-form tree where the design remembers four small values for each tab (the plan's inventory, part e.1).
3. **Build the design's frame as Solis's own** (chosen): every panel keeps its contents and is given a home in the new frame, then each screen is made to match its picture.

## Decision

The project lead's design in [`docs/design/g-layout/`](../design/g-layout/README.md) is the source of truth for how the editor looks. Where it and the code differ, the code is wrong or a decision says why.

- **The frame**, top to bottom: a **tab bar** (the logo, the ribbon's tabs, the command search, the project and area), a **ribbon** for the tab in front, one or two **editor groups** with document tabs, a **shelf** of tabs with columns, and a **status bar**. The **File** tab is a full page instead: Recent projects, Preferences, Project settings, Input, Learn, Updates, Exit.
- **Ribbon tabs** are how one moves between areas of the engine: Home, Library, Shading, Animate, **UI**, VFX, Audio, Debug, Build. The UI tab is Solis's addition, for the game-screen editor, between Animate and VFX.
- **Built as four small phases, Shared 1a to 1d**, sixteen milestones ([plan](../plans/shared-1-ui-remake.md)): the look and the frame; the everyday screens; 2D and its editors; the rest, and the finish. Decision 0097 keeps phases to two to six milestones, so the one phase 0098 named is split, each part with its own gate and the last with the gate 0098's phase was given.
- **Nothing becomes unreachable.** After every milestone the editor works and everything that existed before can still be reached; a test walks the list.
- **Today's keys stay** (the project lead: "Keep today's keys"): Q select, W move, R rotate, S scale, T all three, and Space opens the search, whose box shows that key. The design's own keys are used only for things that are new (Ctrl + Space folds the shelf). Any other new key or mouse action is still asked first (CONTRIBUTING.md, rule 17).
- **Solis's words on screen:** Area, Entity, Properties, Model, Picture, assembly, component, game code, where the design's samples say otherwise. The design's names are used where Solis had none: Preferences, Project settings, Recent projects, Exit, "Search or run a command", "+ Component".
- **The running game is a Game tab** beside the area's tab in the same group, brought to the front when Play is pressed, so the area can still be watched and changed while the game runs.
- **No play tint.** Decision 0085 made playing unmistakable by colouring the window: the bars and the viewport's edge turn blue while the game runs and amber while it is paused. The design shows none, and the project lead chose the design as drawn. What says the game is running is now this, and only this: **Play becomes Stop**, with **Pause** and **Step** beside it, and the Game tab is in front. No bar, border or status bar changes colour.
- **The logo** is today's, in the design's 20 × 20 light tile, as one picture that can be swapped. A new logo is designed with the project lead later (the ideas list), outside this work.
- **Parts the design doesn't draw** (Solis's 2D editors above all, and smaller things such as the Play group while playing) are drawn in the design's language on its screens and approved by the project lead before they are built. Approvals are recorded in the plan.
- **Code opens in the user's code editor**, as today. The design's built-in code editor is a later feature.

## What this replaces and changes

| Decision | What goes | What stands |
| --- | --- | --- |
| [0004](0004-editor-layout.md), the editor's layout | The one-row top bar with its menus, every panel dockable, floating panels, the standard arrangement, and the theme (see 0100). The docking library leaves when the dock does (Shared 1a's last milestone), with a decision of its own then. | The tools and their keys (Q, W, R, S, T), dragging from the library to place, the Fly camera and F to frame. |
| [0061](0061-settings-window-and-no-layout-presets.md), the Settings window | The window itself: its pages become **File › Preferences** (this computer) and **File › Project settings** (this project) in Shared 1b. Reset layout becomes resetting a tab's shelf and split. | No layout presets; a setting is a row with a name and a line saying what it does; settings are searched; they are never undo steps. Until Shared 1b the window stays and is opened from the File page. |
| [0085](0085-the-editor-overhaul.md), the editor overhaul | Workspaces (they become ribbon tabs: Level is Home, and Animate, UI and Code have their tabs), the Library drawer (the Library is a ribbon tab and a shelf column; Ctrl + Space folds the shelf), "docking stays", and the colouring while playing (above). "Nothing is shown before it works" is changed by 0101. | Space opens the search; one search for everything; the status bar; changes made while playing are kept or undone, never lost; hints. |
| [0086](0086-the-launcher.md), the launcher | The launcher as a page of its own look: it becomes **File › Recent projects**, with Learn and Updates as File pages (Shared 1b). | Everything it does: the list of known projects, pin, search, rename, copy, remove from list, scan, locate, the build that last saved a project. |
| [0090](0090-always-through-the-launcher.md), always through the launcher | Nothing yet. Whether Solis still starts in a small window of its own, or on the page at the editor's size, is asked before Shared 1b's milestone that builds the page. | Solis always starts on the list of projects, and never reopens one by itself. |

## Consequences

- The look arrives first (decision 0100): today's editor, still docked and with its menus, takes the design's colours, sizes and type in one step. The frame follows in two milestones (the top, then the middle); until then today's frame, its workspaces and its play tint are as they were.
- Saved panel arrangements can't be carried into a frame without docking: when the dock goes, every tab starts from the design's standard and the Console says so once. The old settings keys are left alone, so an older build still finds its arrangement.
- The specification's comparisons to another product were reworded in the repository's copy (CONTRIBUTING.md, rule 20); nothing else in it was changed, and its pictures are never edited, because tests sample their colours.
- Picture tests join the whole-window tests: measured layout against the specification, sampled colours against the design's pictures, and Solis's own approved pictures.
- The manual's interface tour, keyboard page and every "File >" and "Edit >" path change as the milestones land; each milestone rewrites the pages for what it changed.
- CLAUDE.md's mock-up rule now names this design as the source of truth.
