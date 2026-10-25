# Assemblies

> Build something once (a lamp post, a market stall, a whole street) and place copies of it everywhere. Change the original and every copy follows.

## When to use it
Use an [assembly](concepts.md#the-area) for anything that appears more than once: props, buildings, groups of enemies, even whole districts (assemblies can hold other assemblies). Each copy can still have its own changes: one lamp brighter, one stall without its awning. Those changes are kept when the assembly changes.

Other engines call this a *prefab*. An imported model works the same way: placing it makes one entity, and **Unpack** splits it into its parts when you need them.

## How to use it

**Make one**
1. Build it in an area: a parent entity with everything under it (drag entities onto the parent in the Area panel).
2. Select the parent and choose **Entity > Create assembly from selection**.
3. It's saved in your [project](projects-and-the-library.md)'s `assets/assemblies` folder, appears in the **Library**, and what you selected becomes the first copy of it.

**Place copies:** double-click the assembly in the Library (or click its **+**). The copy goes where the camera looks.

**Change one copy:** select any part of it and change it as usual. Settings that differ from the assembly show in **bold**, with two small buttons:
- **↺** puts that setting back to the assembly's value;
- **↑** makes it the assembly's value, for every copy (**Apply**).

**Change every copy:** use ↑, or open the `.solassembly` file and edit it (in Solis or a text editor). Every copy updates, and each keeps its own bold changes.

**Unpack:** select a copy's top entity and click **Unpack** in Properties. It becomes ordinary entities that no longer follow the assembly. For a model, **Unpack** splits it into one entity per part of the file (as in Blender's outliner), so each part can be moved, hidden or given components on its own.

## Settings
| What you see | What it means |
|---|---|
| *A copy of …* | This entity is the top of a placed assembly. |
| *Part of …* | This entity is inside a placed assembly. |
| **Bold** setting | This copy's own change. |
| *This part is gone from …* | The assembly no longer has this part, but this copy had changes to it. They're kept in case it comes back; **Remove** if you don't need them. |

## Tips and common problems
- **Problem:** "assemblies place each other in a loop". **Why:** assembly A contains B, which contains A. **Fix:** remove one of the copies from the other.
- **Problem:** "placed more than 16 deep". **Why:** assemblies inside assemblies, 17 levels down. **Fix:** unpack some of the middle levels.
- **Problem:** I undid *Create assembly* and the file is still in the Library. **Why:** undo changes the area, not files; delete the `.solassembly` file in Explorer if you don't want it.
- An area file stores each copy as one block naming its assembly, plus only the settings that differ, so copies keep files small and Git merges them easily.
- A model's parts keep their links through a new export from Blender, as long as the parts keep their names.

## Related
- [Entities and components](entities.md)
- [Saving areas](saving-areas.md)
- [Projects and the Library](projects-and-the-library.md)
- Concepts: [assembly, copy](concepts.md#the-area)
