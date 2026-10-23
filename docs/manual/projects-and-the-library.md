# Projects and the Library

> A project is a folder holding your game's models (and later its areas, code and sounds). The Library shows what's in it.

## When to use it
Make a project as soon as you start something you want to keep. Without one, Solis works as a viewer: models you import are shown but not kept. With one:

- models you import are copied into the project's **assets** folder, so the project has everything it needs, on any computer;
- each [asset](concepts.md#projects) gets an ID that never changes, so renaming or moving a file (in Solis, in Explorer, or through Git) breaks nothing;
- when you save a model again in Blender or Aventail, Solis notices and shows the new version straight away;
- the **Library** lists every model, ready to place.

## How to use it

**Make a project**
1. **File > New project…** and choose an empty folder (make one in the window if you need to). The project is named after the folder.
2. Solis makes `project.solis`, an `assets` folder, and a hidden `.solis` folder for its cache. If you use Git, it also adds `.gitignore` and `.gitattributes` files set up for games.

**Open a project:** **File > Open project…** and choose the project's folder, or pick it from **File > Recent projects**. Solis opens the last project again when it starts.

**Add models:** **File > Import model…**, or drop a `.glb` or `.gltf` file onto the window: it's copied into the project's assets and placed in the area. You can also put files straight into the assets folder (or a folder inside it) from Explorer: they appear in the Library within a moment.

**Place a model:** in the **Library**, double-click it, or click the **+** on its right. It's placed in the area like an import.

**Change a model:** export it again over the same file in Blender. Everything using it shows the new version a moment later. If the new file is broken, the old version stays, and the Console says why.

## Settings

| File or folder | What it is |
|---|---|
| `project.solis` | Marks the folder as a project and holds its name. |
| `assets/` | Everything you made or brought in. Watched for changes. |
| `<file>.solmeta` | A small text file beside each asset holding its ID. **Commit it with the asset**, and move or rename them together (Solis copes if you forget, as long as the content didn't change). |
| `.solis/` | Solis's cache: models it has already read, so they open quickly. Safe to delete; it's rebuilt. Never committed. |

## Tips and common problems
- **Problem:** the Console says a sidecar couldn't be read. **Why:** the `.solmeta` file was damaged, perhaps by a bad Git merge. **Fix:** fix it in a text editor, or delete it to give the model a new ID (anything using the old ID will then say the model is missing).
- **Problem:** the Console says a model "is a copy, so it got a new ID". **Why:** a file was copied together with its `.solmeta`; two files can't share an ID. Nothing is wrong.
- **Problem:** I renamed a model in Explorer *and* changed it before Solis was open, and it lost its ID. **Why:** Solis finds a renamed file by its content; with the content changed too, it can't tell. **Fix:** rename and change in separate steps, or rename the `.solmeta` along with it.
- **Problem:** models placed before opening a project disappeared. **Why:** opening a project starts a new area. **Fix:** Solis asks to save first; [save the area](saving-areas.md) inside the project to keep it.
- Imported models open much faster the second time: the cache remembers them, even after a rename.

## Related
- [Importing models](importing-models.md)
- [Entities and components](entities.md)
- Concepts: [project, asset, sidecar, cache](concepts.md#projects)
