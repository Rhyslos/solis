# 0022. Undo and redo as reversible edits to the scene document

- **Status:** Accepted
- **Date:** 2026-10-23

## Problem

Phase 2 asks for undo and redo for every edit. That includes edits to components made at run time (Lua, Lust), prefab overrides, and drags that send a new value every frame.

## Options

1. **Hand-written commands,** each with its own undo. Every new component needs code; Lua and Lust components can't take part.
2. **Snapshots of the whole scene for each step.** Robust, but slow and memory-hungry at thousands of entities.
3. **A few reversible edits on the scene document's values** (decision 0019), with user-level commands built from them. This is how Aventail works ("tools produce operations and commands").

## Decision

Option 3.

- **Edits** (`scene::Edit`) are `SetField`, `AddComponent`, `RemoveComponent`, `Spawn`, `Despawn` (a whole subtree, saved in full), `Rename` and `Reparent`.
  - Applying an edit returns its inverse.
  - The same edits will serve the Git panel's summaries ("Health.maxHp 4000 → 5000"), prefab overrides, the merge tool and editor scripting.
- **Commands** in the editor (set a field, delete, duplicate, reparent, import…) turn into edits. Nothing in the editor changes the scene directly. Properties shows copies of values and sends commands.
- **History:**
  - Steps with a label, their edits, and the selection before and after.
  - A whole drag is one step, merged by (widget, start of the interaction). A drag that ends where it started leaves no step.
  - A step that undo has returned to never merges with a new one.
  - History is limited by memory, not by a count.
- **Selection** is not an undo step itself, but undo and redo bring back the selection the step had, so undo takes you back to what you were editing. It's never saved in files.
- **Not undo steps:**
  - Re-importing an asset (history holds IDs, not data, so it stays valid).
  - Settings of the editor itself (layout, look, camera).
- **A scene reloaded from disk** with no unsaved changes becomes one undoable step.

## Consequences

- Components from Lust and Lua are undoable with no extra code.
- Every editing feature from now on is written as a command, never as a direct change.
