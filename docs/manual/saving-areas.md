# Saving areas

> Save an area as a file, open it again, and share it through Git without merge trouble.

## When to use it
Save whenever you've built something you want to keep. An [area](concepts.md#the-area) (a level, a menu, a room) is one file ending in `.solarea`, kept in your [project](projects-and-the-library.md) (Solis suggests its `assets/areas` folder).

The file is plain text, written so that two people changing different things in the same area can merge their work in Git without conflicts. It's also safe to read and edit by hand.

## How to use it
1. **Save:** press **Ctrl + S** (or **File > Save area**). The first time, choose a name and a folder.
2. **Save as:** **Ctrl + Shift + S** saves a copy under a new name and carries on with that one.
3. **Open:** **Ctrl + O** (or **File > Open area…**). The models it uses are read from the project, from the cache, so it opens quickly.
4. **New:** **Ctrl + N** starts a fresh area with a sun and an environment.

The tab at the top shows the area's name, with a **•** while it has changes that aren't saved. Solis asks before anything would lose them: a new or another area, another project, or closing. **Save**, **Don't save** or **Cancel**.

**Changed on disk:** if the file changes while it's open (a Git pull, or an edit in a text editor), Solis loads the new version as one step you can undo with Ctrl + Z. If you have unsaved changes too, it asks: **Keep mine** or **Load the file**.

## Shortcuts
| Action | Shortcut |
|---|---|
| New area | `Ctrl + N` |
| Open area | `Ctrl + O` |
| Save area | `Ctrl + S` |
| Save area as | `Ctrl + Shift + S` |

## What the file looks like

```
-- An area. Safe to edit by hand: one block per entity, one line per field.
area version 1;

entity 2B8D4NE6TV0HC {
    name = "Sun";
    order = "a0";
    Light.kind = "Sun";
    Light.strength = 3.0;
    Transform.rotation = [0.25881904, 0.0, 0.0, 0.9659258];
}
```

- One block per entity, sorted by its ID (13 letters and digits that never change), never by the tree or by when it was made, so renaming or moving an entity never moves its block.
- One line per setting: `Component.setting = value;`. Text is in quotes, lists in `[ ]`, and `--` starts a comment.
- A child names its parent with `parent = "…";`.
- Models are named by their asset ID, with the file's path in a comment for people to read; the comment is rewritten on every save and never read.
- Components Solis doesn't know (from game code that isn't there) are kept exactly as written.

## Tips and common problems
- **Problem:** "line 12: …" when opening. **Why:** the file has a mistake, perhaps from a hand edit or a bad merge. **Fix:** the message says the line and what's wrong; fix it in a text editor. Solis never opens half a file.
- **Problem:** "it was saved by a newer version". **Why:** someone saved it with a newer Solis, which may write things this one can't read. **Fix:** update Solis.
- **Problem:** Git reports a conflict in an area file. **Why:** two people added entities in the same spot, or changed lines right next to each other in the same component (both rare). **Fix:** open the file in a text editor and keep both blocks, or the line you want; Solis's own merge tool (Phase 13) will do this for you.
- **Problem:** "Not saved: … already exists". **Why:** the name typed in the Save window had no `.solarea`, and a file with that name and `.solarea` is already there; the window only asked about replacing the name as typed. **Fix:** choose **Save as** again and pick the existing file to replace it, or type another name.
- **Problem:** "Stop playing first" when saving. **Why:** the game is running, and saving now would save its changes, which Stop undoes. **Fix:** press Stop, then save. A Save window opened before Play finishes after Stop.
- Renaming or moving the open area in the Library is safe: Solis saves to its new name from then on. If it's deleted, the next save asks where to put it.
- Saves are written to the disk in full before they replace the old file, so a crash or power cut leaves either the old file or the new one, never half of one.
- Numbers are written as briefly as possible but always read back exactly, so saving without changes doesn't change the file.

## Related
- [Projects and the Library](projects-and-the-library.md)
- [Undo and redo](undo.md)
- [Entities and components](entities.md)
- Concepts: [area, entity, component](concepts.md#the-area)
