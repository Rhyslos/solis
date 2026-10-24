# Levels of detail: simpler models far away

> Far from the camera, Solis draws simpler versions of your models, so thousands of them cost far less without looking any different.

## When to use it
You don't have to do anything: every model you import gets [levels of detail](concepts.md#models) (LODs) on its own. A ship a few pixels tall on screen doesn't need its 20,000 triangles; drawing 2,000 of them instead lets a battle of thousands of ships keep its frame rate.

Make your own when the automatic ones don't suit a model: a character whose face should stay recognisable, a building whose outline matters, or a low-poly model that can't be simplified (Solis leaves those in full).

## How it works
1. **When you import a model,** Solis makes up to three simpler versions of each of its meshes: about a half, a quarter and an eighth of the triangles. It does this once, and keeps them with the imported model, so later imports and Play are just as quick.
2. **While drawing,** Solis works out how tall each copy of the model is on screen. It draws the simplest version that differs from the full one by less than a pixel, so you shouldn't be able to see the switch.
3. **A copy near the point where it would switch** keeps the version it had until it's clearly past that point, so it doesn't flicker between two.

To see it at work, open the **Profiler** while looking out over many copies: its last line says how many are drawn in full and at each level. **Properties** shows a model's triangles, then its simpler levels after "far away": `Ship · 20,000 tris · far away 10,000 → 5,000 → 2,500`.

## Making your own
1. In your modelling program, make the simpler version of the object as a separate object, in the same place.
2. Name it after the full one with `_LOD1` on the end: for an object called `Ship`, name it `Ship_LOD1`. Make even simpler ones `Ship_LOD2`, `Ship_LOD3` and `Ship_LOD4` if you like. Upper or lower case both work, and the full one may also be called `Ship_LOD0`.
3. Export the model as glTF with both objects, and import it (or save over the file Solis already has).

Solis then uses your versions for that object instead of making its own, and doesn't draw them as parts of their own. In Blender, the names are the ones in the Outliner. If an object has several meshes (several materials), they're matched in order.

Your own levels are used from about a quarter of the screen tall (level 1), each next level at about half that size.

## Tips and common problems
- **Problem:** a model pops between versions as the camera moves. **Why:** the simpler version differs more than Solis's estimate says, often with your own levels, which don't say how much they differ. **Fix:** make the level closer to the full model, or tell us which model it is.
- **Problem:** the Console says `Ship_LOD1` looks like a level of detail, but there's no part called `Ship`. **Why:** the full object has another name (`ship`, `Ship.001`). **Fix:** rename one so the names match exactly, apart from `_LOD1`. Until then it's drawn as a part of its own.
- **Problem:** a model has no simpler levels in Properties. **Why:** it has fewer than 64 triangles in each mesh (already cheap), or its shape couldn't be simplified without changing it too much. **Fix:** nothing; it's drawn in full.
- **Problem:** the first import of a very detailed model takes longer. **Why:** making the levels takes time, about a second for a few million triangles. It's done once; meshes with more than 4 million triangles are left in full.

## Related
- [Importing models](importing-models.md)
- [The Profiler](profiler.md)
- Concepts: [mesh, triangle, level of detail](concepts.md#models)
