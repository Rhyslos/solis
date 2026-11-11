# Undo and redo

> Take back any change to the area, and bring it back again.

## When to use it
Whenever a change didn't turn out as you hoped: a light dragged too bright, an entity deleted by mistake, a model imported into the wrong area. Every change to the area can be undone, so it's safe to experiment.

## How to use it
1. Press **Ctrl + Z** (or choose **Edit > Undo**) to take back the last change. The Edit menu names it, such as *Undo Change Light.strength*.
2. Press again to go further back.
3. Press **Ctrl + Y** or **Ctrl + Shift + Z** (or **Edit > Redo**) to bring a change back.

Undo also selects what the change was about, so you can see what came back.

**What counts as one step:**
- A whole drag of a number is one step, however long you drag. Dragging back to where you started leaves no step at all.
- Typing a name is one step until you click somewhere else.
- An import is one step, with the lights that came with the model.

**Tabs with their own undo:** the Clips and Frames tabs, the Animator, the Tileset tab and the Map tab each keep their own steps: Ctrl + Z with the pointer over the tab undoes there. A tileset's steps belong to its file, not to the tab: they are kept while the project is open, whether the change was made in the Tileset tab, in the tileset's Properties (clicked in the Library) or by ticking **Solid** in the Map tab, and Ctrl + Z over the Tileset tab, or over Properties while it shows the tileset, undoes the last of them. While a [tile painting tool](painting-tiles.md) is in use, Ctrl + Z undoes the last stroke (each stroke, from pressing the button to letting go, is one step). In these tabs, and in the Atlas Crop tab, a whole drag of a number or of a colour in its picker is one step too: the field shows the value as you drag, and the change is made, and the file saved, once, when you let go. A typed number counts when you press Enter or click somewhere else.

**What isn't undone:** selecting things, moving the camera, and the editor's own settings (panels, look, accent colour). They aren't part of the area.

## Shortcuts
| Action | Shortcut |
|---|---|
| Undo | `Ctrl + Z` |
| Redo | `Ctrl + Y` or `Ctrl + Shift + Z` |

## Tips and common problems
- **Problem:** Ctrl + Z changed the text I was typing instead of the area. **Why:** while you type in a field, Ctrl + Z undoes the typing first. **Fix:** click outside the field, then press Ctrl + Z.
- **Problem:** I undid something, made a new change, and now Redo is greyed out. **Why:** a new change starts a new path; what you undid can't come back after that, as in most programs.
- **Problem:** a change I made in a tab was taken back, and the Console says *Couldn't save … It's shown as the file has it, without your change*. **Why:** tile maps, tilesets, clips files, animators and atlases are saved the moment you change them, and this time the file couldn't be written (the disk is full, another program has the file open, or its folder is gone). Solis never shows as saved a change that isn't in the file, so it shows the file as it is, and keeps your change as an undo step. **Fix:** put right what the Console says went wrong, then press **Ctrl + Z** with the pointer over the tab the message names: your change comes back and is saved again.
- Solis keeps the most recent changes, up to about 64 MB of them; the oldest are forgotten first. That's thousands of ordinary changes.

## Related
- [Entities and components](entities.md)
- [Keyboard shortcuts](keyboard-shortcuts.md)
