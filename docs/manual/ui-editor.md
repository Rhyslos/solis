# The UI editor: drawing a screen on the canvas

> Opens a [screen](concepts.md#screen) on a canvas, so you can pick its pieces, move them, size them and change how they look, while Solis rewrites its Lust file for you.

## When to use it
A [screen](screens.md) (a HUD, a pause menu) is a Lust file in your project's `ui/` folder. You can always write it by hand, but placing and styling pieces is quicker when you can see them: drag a button above another, stretch a panel, try another colour, and see each change at once.

The canvas never takes your file away from you. It changes only the letters it has to (the `gap 14` becomes `gap 20`, and nothing else moves), so comments, spacing and code you wrote by hand stay exactly as they were. You can keep the file open in a code editor beside Solis: a change saved there shows on the canvas within a second.

## How to use it
1. In the Library, open the `ui` folder and **double-click a screen's file** (`pauseMenu`). Solis switches to the **UI** workspace and draws the screen on the **Canvas**.
2. **Pick a piece**: click it on the canvas, or its row in **Elements** on the left. Shift-click (Ctrl-click in Elements) picks more; drag a box round pieces on an empty part of the canvas to pick everything inside it.
3. **Change it in Properties** on the right: its words, its width and height, how what's inside it lines up, its padding, colour, corners and opacity. Every change is saved straight away, and `Ctrl + Z` over the canvas undoes it.
4. **Move it**: drag it on the canvas to another place among its neighbours, or into another column or row; a blue line shows where it will land. In Elements, drag its row above or below another row, or onto the middle of a column's row to put it inside.
5. **Size it**: pick one piece and drag the small square on its right edge, bottom edge or corner.
6. Check the screen on other screen sizes with the size list at the top left (Full HD, 4K, ultrawide, a phone). The dashed line is the **safe area**: keep text and buttons inside it, since some TVs cut the edges off.
7. **Open as code** (top right) opens the file in your code editor, for anything the canvas doesn't edit.

## Settings
| Setting | What it does |
|---|---|
| Words | The text on a `text` or a `button`. Text worked out by code (`"Score " + score.text()`) shows as code and is changed in the file. |
| Width, Height | **set** a size in the screen's pixels (on a 1920 × 1080 design), **fill** the room it has, or **auto**: as big as what's in it. |
| Align | Where what's inside it sits: a 3 × 3 grid, from top left to bottom right. On a column or a row this places its pieces; on text, the words. |
| Padding | Room inside its edge, all round. |
| Gap | Room between the pieces inside a column or a row. |
| X, Y | Where a piece sits inside a `canvas`. Dragging it on the canvas changes these. |
| Colour | A button's or shape's colour, text's colour, or a column's background, from Lust's colour words (`blue`, `gray9`, `red5`). |
| Corners | How round its corners are, in pixels. |
| Opacity | 1 is solid, 0 is invisible. |
| Text size, Bold | For text and buttons. |
| Hover | A look while the pointer is over it. **+ Add a hover look** dims it a little; edit it in **All its settings**. |
| Disabled | Greyed, and clicks do nothing. |
| When clicked | What a button does, as written in its `{ }`: change it as code. |
| All its settings | Everything in the piece's `[ ]`, as written. Type anything Lust has (`nowrap`, `spread`, `minWidth 200`); press Enter to keep it. If it doesn't read, it isn't changed. |
| Safe area | Shows the guide 5% in from each edge. |
| Snap | Sizes snap to whole pixels and to other pieces' edges. |

## Shortcuts
| Action | Shortcut |
|---|---|
| Pick more (canvas) | `Shift + click` |
| Pick more (Elements) | `Shift + click` or `Ctrl + click` |
| Delete what's picked | `Delete` (over the canvas or Elements) |
| Copy what's picked, just after itself | `Ctrl + D` |
| Pick nothing | `Esc` |
| Undo, redo | `Ctrl + Z`, `Ctrl + Y` (over the canvas or Elements) |
| Zoom | Mouse wheel |
| Move the view | Middle mouse button drag |
| Fit the canvas | Double-click an empty part, or click the zoom button |

## Tips and common problems
- **Problem:** a piece has a yellow **code** badge and can't be picked. **Why:** code made it: a `loop` or an `is …? then` in the view. The canvas draws it from the screen's starting values, but its letters are code. **Fix:** Open as code and change it there.
- **Problem:** the canvas says it "can't follow" the file. **Why:** the file has a mistake the canvas can't read past, such as a `[` never closed. **Fix:** Problems says where; fix it as code, and the canvas follows again.
- **Problem:** "Updating…" shows after a piece was moved, added or deleted. **Why:** the canvas waits for the game's code to be checked again, which takes a moment. **Fix:** nothing: it goes in about a second.
- **Problem:** a bar or a number shows as empty. **Why:** the canvas draws the screen for an entity whose values are all their starting ones (health 0, say). **Fix:** press Play to see it with the game's real values in the Game panel.
- **Problem:** dragging a piece doesn't move it where expected. **Why:** in a column or a row, pieces go one after another: a piece moves *between* its neighbours, not to any spot. **Fix:** to place pieces freely, put them in a `canvas` and drag them there.

## Related
- [Game screens](screens.md): what a screen's Lust file holds, and how the game shows it.
- [Atlas Crop](atlas-crop.md): pictures cut from a sheet, for `image` pieces.
- [Concepts](concepts.md): [screen](concepts.md#screen), [canvas](concepts.md#canvas).
