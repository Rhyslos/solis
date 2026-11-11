# 0103: Today's keys stay, and what the new frame adds for the hand

- **Status:** Accepted (builds on [0099](0099-the-g-layout.md); keeps [0085](0085-the-editor-overhaul.md)'s search key; follows rule 17 of CONTRIBUTING.md)
- **Date:** 2026-11-11

## Problem

The project lead's design, the G layout, comes with a table of keys of its own, and several of them are already other things in Solis. Its command search opens on a key Solis doesn't use, while Solis's search opens on **Space**. Its tools are on Q, W, E and R, while Solis's are on Q, W, R and S, with T for all three and E for flying up. Its ribbon has two actions for the hand that Solis never had: a long press on a Create button to list the kinds it can make, and a ribbon that scrolls sideways when the window is narrow.

Rule 17 says no key or mouse action is added without the project lead's answer. This record holds the answers given while the frame was built, so the manual, the code and later milestones have one place to read them.

## Options

- **Take the design's keys.** The editor would match its pictures to the letter, and every key a person has learnt would move: Rotate off R, Scale off S, the search off Space, and T and the flying keys would need new homes.
- **Keep today's keys, and use the design's only for what is new.** Nothing learnt is lost; the pictures' key hints read differently in a few places.

## Decision

**Today's keys stay** (the project lead, 2026-11-11: "Keep today's keys").

- **Tools:** Q select, W move, R rotate, S scale, T all three.
- **The search:** Space, with nothing being typed (over the Frames tab Space still plays the clip). The search box in the tab bar shows **Space** as its hint. The key the design shows there is not used.
- **Every other key Solis has** (undo and redo, new, open, save and save as, Delete, duplicate, rename, select all, the axis locks, the fly camera, the tile painter's and the UI editor's keys, the Console's) is as it was. The one table of the editor's shortcuts is `ShellCommand::keys` (`crates/editor/src/shell/command_words.rs`): the search and the ribbon's hover texts read it, and a test presses each key and holds what it does against the command of that name.
- **The design's keys are used only for things that are new.** The first is Ctrl + Space, which will fold the shelf when the shelf is built (it pops the Library drawer up until then).

**Two mouse actions are added with the ribbon** (the project lead, 2026-11-11):

- **The wheel over the ribbon scrolls it sideways** when the ribbon doesn't fit the window. Elsewhere the wheel does what it did.
- **A Create button's list of kinds opens from a small arrow on the button, or from a right-click on the button.** A plain click makes the usual kind (Light makes a point light; its list has the spotlight). There is no long press: it is slow, and it can't be found by looking.

**Also part of the frame, and not new for the hand in kind:** a click on a tab brings it to the front; a double-click on a tab, or the chevron at the tab bar's right end, collapses the ribbon to its tabs and brings it back; while it is collapsed, a click on a tab drops the ribbon over the center until something on it is chosen or the pointer moves below it; Esc leaves the File page, as it left the launcher.

Answers to the keys and mouse actions of later screens (placing, the Library, the mixer, the profiler, Input) were given on the second round of mock-ups and are written into the plan (`docs/plans/shared-1-ui-remake.md`, decision D15); each joins this record's list in the manual when its milestone builds it.

## Consequences

- The search box's hint and every hover text show Solis's keys, so the editor differs from the design's pictures in those words only.
- The toolbox gets a fifth button (all three gizmos, T) when the toolbox is built, which the design doesn't draw.
- A Create button with more than one kind carries a small arrow the design's pictures don't show.
- `docs/manual/keyboard-shortcuts.md` lists the two mouse actions and the search's place in the tab bar.
