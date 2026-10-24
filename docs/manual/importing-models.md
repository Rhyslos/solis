# Importing models

> Bring a 3D model from Blender, Aventail or any other program into the viewport.

## When to use it
Whenever you want to see a model in Solis. Solis reads **glTF** files ([concepts](concepts.md#models)), the standard format for game models: Blender, Aventail and most 3D programs export it.

Until areas and entities arrive (Phase 2), an imported model sits in the viewport where its file puts it. It isn't saved: next time Solis starts, import it again.

## How to use it

**From the menu:**
1. Choose **File > Import model…**.
2. Pick a `.glb` or `.gltf` file and press **Open**.

**By dragging:** drag a `.glb` or `.gltf` file from your file explorer onto the Solis window. While you drag, the window shows where it will go; let go anywhere.

Then:
- The model is read in the background, so the editor keeps running while it loads. The Console says **Importing…**, then **Imported** with the model's triangle count and size in metres.
- The camera moves to show the whole model, and picks a [camera speed](navigating-the-viewport.md) that suits its size.
- With a [project](projects-and-the-library.md) open, the file is first copied into the project's assets folder (with the `.bin` and images a `.gltf` reads), so the project keeps it; it then also appears in the **Library**.
- The model appears in the **Area** panel as one [entity](entities.md), however many parts the file has (they keep their places inside it). **Unpack** in Properties splits it into one entity per part when you need them ([Assemblies](assemblies.md)). A mesh used many times in the file is stored once, with its triangle count. The **×** at the end of its row deletes it from the area. Select it to see its Transform and Model in Properties.

**Exporting from Blender:** File > Export > glTF 2.0, choose **glTF Binary (.glb)**, and keep **+Y Up** ticked (the standard). Solis turns glTF's "Y is up" into its own "Z is up" for you, so the model stands upright.

**Exporting from Aventail:** export as glTF (Aventail's manual, "Export and import", explains how). Kits exported from Aventail import the same way.

## Settings
| What Solis reads | Notes |
|---|---|
| Meshes | Placed where the file puts them. Triangles only: points and lines are left out, with a warning in the Console. |
| Material | Colour, metal, roughness, normal, occlusion and glow, with their textures (PNG or JPEG, inside the `.glb` or as files beside a `.gltf`). See [Looks](looks.md). |
| See-through parts | Cut-out (leaves, fences) and blended (glass). |
| Double-sided surfaces | Drawn from both sides when the file says so; otherwise the back of a surface is hidden. |
| Units | Metres, as glTF uses. The Console shows the model's size, so you can spot one exported 100 times too big. |

**Limits:** files up to 1 GB, models up to 30 million triangles, and textures up to 16,384 pixels across. Textures bigger than your graphics card holds are scaled down to fit.

## Tips and common problems
- **Problem:** the model is lying on its back or facing the wrong way. **Why:** it was exported with a different "up" setting. **Fix:** in Blender, keep **+Y Up** ticked when exporting glTF.
- **Problem:** the model is tiny or enormous. **Why:** it was made in other units (centimetres, inches). **Fix:** check the size in the Console; scale it in your modelling program, or set the export's scale.
- **Problem:** the Console says a texture was left out. **Why:** the image file isn't next to the `.gltf`, or it isn't PNG or JPEG. **Fix:** export as **glTF Binary (.glb)**, which keeps everything in one file.
- **Problem:** "Couldn't import … the file is damaged". **Why:** the file is incomplete or broken. **Fix:** export it again.
- **Problem:** the model looks flat and grey-blue in the shadows. **Why:** where the sun doesn't reach, only the sky's soft light does. **Fix:** move the sun or add lamps; see [Lights and shadows](lights.md).
- **Problem:** "it needs a glTF extension Solis doesn't read yet". **Why:** the file was saved with compression (Draco or meshopt, common from Sketchfab or gltfpack). **Fix:** export it again without compression; in Blender, untick **Compression** in the glTF export.
- **Very large models** are split into pieces the graphics card can hold; you won't see the seams, but the Area panel's triangle count stays the same.
- **Lights in the file:** lamps exported from Blender (point lights and spotlights) come in with the model as entities under it in the Area panel, and are deleted with it; a sun in the file sets the area's sun. Turn on **Punctual Lights** under Include in Blender's glTF export to export them.
- **Want to see how it should look?** **Help > Add the sample area** brings in a model made for checking the [looks](looks.md). With a project open, it's added to the project's assets as `Sample area.glb`, like any import, so areas using it open again.

## Related
- [Moving around the viewport](navigating-the-viewport.md)
- [Concepts](concepts.md): models, meshes, triangles, materials, textures, glTF.
