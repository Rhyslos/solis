# Projects and the Library

> A project is a folder holding your game's files: models, pictures, areas and code. The Library shows what's in it, with pictures of each model, and is where you place things from.

## When to use it
Make a project as soon as you start something you want to keep. Without one, Solis works as a viewer: models you import are shown but not kept. With one:

- models you import are copied into the project's **assets** folder, so the project has everything it needs, on any computer;
- each [asset](concepts.md#projects) gets an ID that never changes, so renaming or moving a file (in Solis, in Explorer, or through Git) breaks nothing;
- when you save a model again in Blender or Aventail, Solis notices and shows the new version straight away;
- the **Library** shows every file, folder by folder, with a thumbnail of each model and picture, ready to drag into the area.

## How to use it

**Make a project**
1. **File > New project…** and choose an empty folder (make one in the window if you need to). The project is named after the folder.
2. Solis makes `project.solis`, an `assets` folder, and a hidden `.solis` folder for its cache. If you use Git, it also adds `.gitignore` and `.gitattributes` files set up for games.

**Open a project:** **File > Open project…** and choose the project's folder, or pick it from **File > Recent projects**. Solis opens the last project again when it starts.

**Add models:** **File > Import model…**, or drop a `.glb` or `.gltf` file onto the window: it's copied into the project's assets and placed in the area. You can also put files straight into the assets folder (or a folder inside it) from Explorer: they appear in the Library within a moment.

**Find your way around the Library**
- On the left, the **folder tree**: click a folder to see what's in it. (**View > Folder tree** puts it away to make room.)
- On the right, that folder's files as **thumbnails**, each with a coloured stripe saying what kind of file it is: blue for models, gold for assemblies, green for areas, purple for pictures. **View > List** shows them as a list instead. The line above them (*Harbour › props*) shows where you are; click a part of it to go back up.
- **Search** (top right) finds files by name in every folder.
- Point at a file to see its full path and what you can do with it.

**Place a model or assembly**
- **Drag it from the Library into the viewport**: it goes on whatever surface is under the mouse when you let go (a table, the floor), or on the ground. In a flat Front or Right view, dropped on empty space, it goes exactly where you point, at that height. It's selected, ready to move.
- **Drag it onto an entity in the Area panel**: it goes under that entity, at its place.
- Or **double-click** it (or click the **+** on its picture) to place it where the camera looks.

**Open things:** double-click an **area** to open it (Solis asks first if yours has unsaved changes). Double-click a picture or any other file to open it in its own program (your picture editor, say).

**Organise files** (right-click a file or folder):
- **Rename…**: type a new name. The file's ID file goes with it, so nothing using it breaks. Changing only capitals works too. Renaming the open area is fine: Solis saves to the new name.
- **Move**: drag a file onto a folder in the tree.
- **New folder** (also in the **Import** menu): makes "New folder" inside the folder shown.
- **Show in Explorer** opens the folder in your system's file browser.
- **Delete** moves the file (or folder) to the **Recycle Bin**, with its ID file, so you can get it back. Anything in the area still using it keeps showing it until Solis restarts.

A `.gltf` model reads the files beside it, so it can't be moved to another folder on its own: move its whole folder instead. Renaming it in place is fine.

**The Code tab** beside the Library lists the project's code files (Lust `.lst`, Lua and Rust), with a search box. Double-click one to open it in your code editor. Solis checks your Lust whenever you save; see [Game code](game-code.md). The `solis` folder holds files Solis writes, describing the engine to your code.

**Change a model:** export it again over the same file in Blender. Everything using it shows the new version a moment later. If the new file is broken, the old version stays, and the Console says why.

## Settings

| File or folder | What it is |
|---|---|
| `project.solis` | Marks the folder as a project and holds its name. |
| `assets/` | Everything you made or brought in. Watched for changes. |
| `<file>.solmeta` | A small text file beside each asset holding its ID. **Commit it with the asset**, and move or rename them together (Solis copes if you forget, as long as the content didn't change). |
| `.solis/` | Solis's cache: models it has already read, so they open quickly. It keeps within 4 GB, deleting the models used least lately. Safe to delete; it's rebuilt. Never committed. |

## Tips and common problems
- **Problem:** the Console says a sidecar couldn't be read. **Why:** the `.solmeta` file was damaged, perhaps by a bad Git merge. **Fix:** fix it in a text editor, or delete it to give the model a new ID (anything using the old ID will then say the model is missing).
- **Problem:** the Console says a model "is a copy, so it got a new ID". **Why:** a file was copied together with its `.solmeta`; two files can't share an ID. The original (whose `.solmeta` is older) keeps its ID, so nothing using it changes. Nothing is wrong.
- **Problem:** a model I saved into the assets folder from Blender takes a couple of seconds to show in the Library. **Why:** Solis waits two seconds before giving a new file its ID, because Git writes a file and its `.solmeta` one after the other: a look in between would give the file a new ID and lose the one it has on other computers. **Fix:** nothing; models brought in with **Import model** show at once.
- **Problem:** I renamed a model in Explorer *and* changed it before Solis was open, and it lost its ID. **Why:** Solis finds a renamed file by its content; with the content changed too, it can't tell. **Fix:** rename and change in separate steps, or rename the `.solmeta` along with it.
- **Problem:** models placed before opening a project disappeared. **Why:** opening a project starts a new area. **Fix:** Solis asks to save first; [save the area](saving-areas.md) inside the project to keep it.
- Imported models open much faster the second time: the cache remembers them, even after a rename.
- **Problem:** a file shows an icon instead of a thumbnail. **Why:** thumbnails are made for models and pictures only, a few at a time, and a damaged file gets none. **Fix:** wait a moment; for a damaged file, the Console says what's wrong when you place it.
- **Problem:** Solis says there's already a file with that name. **Why:** renaming and moving never replace another file. **Fix:** choose another name, or delete the other file first.

## Related
- [Importing models](importing-models.md)
- [Entities and components](entities.md)
- Concepts: [project, asset, sidecar, cache](concepts.md#projects)
