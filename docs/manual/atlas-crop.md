# Atlas Crop: pictures cut out of a sprite sheet

> Cut named pictures out of a PNG or an Aseprite sheet (a sword, a heart, a button's corner), so one sprite or a few can be used on their own, without changing the picture.

## When to use it

Artists often draw many small pictures on one sheet: every item in a game, every icon of a HUD. Atlas Crop marks each one as a **crop** with a name, so game code and screens can use just that part of the sheet: `Picture.Sprites.Tools.Axe`. The picture itself is never changed, so the artist keeps drawing on the same sheet and every crop follows.

For a sprite that plays clips, use [sprites](sprites.md) and [sprite animation](sprite-animation.md) instead; for ground made of tiles, a [tileset](tilesets.md).

## How to use it

1. In the Library, right-click a picture (a PNG, JPEG or Aseprite file) and choose **Atlas Crop…**. The **Atlas Crop** tab opens beside the viewport, showing the picture, and Solis makes a file beside it to keep the crops: `sprites/tools.png` gets `sprites/tools.atlas.lst`.
2. Set the **Grid** to the size the pictures are drawn on (16 × 16 for most pixel art). Crops snap to it, so a quick drag catches a whole tile.
3. **Drag on the picture** to draw a crop. It's named `Crop`, `Crop2`… at first: type a better name in **Name** and press **Rename** (or Enter). A name starts with a letter and holds only letters, digits and `_`.
4. Or press **Crop every tile**: a crop for every grid cell with something drawn in it, named by its column and row (`Tile_3_0`).
5. Click a crop to pick it; **Delete crop** (or Delete with the pointer over the tab) removes it. Every change is saved at once, and **Ctrl+Z** over the tab undoes it.
6. Later, double-click the `.atlas.lst` file in the Library to open its picture's crops again.

Game code names each crop by its picture's path and its name, from `solis/project.lst`: `fetch "solis/project":Picture;` then `Picture.Sprites.Tools.Axe` for `sprites/tools.png`'s Axe. A [screen](screens.md) shows one with `image "sprites/tools.png#Axe";`: the picture's path, `#`, and the crop's name.

## Settings

| Setting | What it does |
|---|---|
| Grid | The size, in the picture's pixels, crops snap to when drawn and that **Crop every tile** cuts by. |
| Name | The picked crop's name, as game code uses it. |

## Shortcuts

| Action | Shortcut |
|---|---|
| Draw a crop | Drag with the left button on the picture |
| Pick a crop | Click it |
| Delete the picked crop | `Delete`, with the pointer over the tab |
| Zoom | Mouse wheel over the picture |
| Move the picture | Drag with the middle button |
| Undo, redo | `Ctrl + Z`, `Ctrl + Y` (pointer over the tab) |

## Tips and common problems

- **Problem:** the tab says the picture isn't in the project. **Why:** an atlas file belongs to the picture beside it, by its file name, and that picture was moved or renamed. **Fix:** move the `.atlas.lst` file beside the picture, or rename it to match (`tools.png` and `tools.atlas.lst`).
- **Problem:** a crop isn't offered to game code. **Why:** its name, or a folder or picture name in its path, can't be a Lust name (the Console says which). **Fix:** rename it: start with a letter, and use only letters, digits and `_`.
- **Problem:** Crop every tile made fewer crops than there are cells. **Why:** empty cells (every pixel see-through) are left out, and so are cells a crop already covers exactly. That's on purpose.

## Related

- [Sprites](sprites.md), [Tilesets](tilesets.md), [Game screens](screens.md).
- [Concepts](concepts.md): [atlas crop](concepts.md#2d).
