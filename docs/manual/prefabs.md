# Prefabs

> Build something once (a chicken, a lamp post, a whole market stall) and place copies of it everywhere. Change the prefab and every copy follows.

## When to use it
Use a [prefab](concepts.md#the-level) for anything that appears more than once: props, buildings, animals, groups of enemies, even whole districts (prefabs can hold other prefabs). Every thing in a level is an [entity](concepts.md#the-level); a prefab is an entity, with its parts and settings, saved to be placed many times, and each placed **copy** is an entity linked to it.

A copy follows its prefab unless the copy changes a setting: then that one setting is the copy's own. One chicken can walk faster than the rest; one lamp can be brighter. A copy keeps only what it changed, so when you change the prefab, every copy follows in everything it didn't change itself.

An imported model works much the same way: placing it makes one entity, and **Unpack** splits it into its parts when you need them.

## How to use it

**Make one**
1. In the [Level panel](interface-tour.md), pick the entities that make up the thing (a click, `Ctrl + click` or `Shift + click`).
2. Right-click one of them and choose **Make prefab…** (also on Home's ribbon, under **Create › More**).
3. In the dialog, give it a **Name**, and choose where to **Save in** (it starts in the folder the [Library](projects-and-the-library.md) shows; **Browse…** picks another folder of the project).
4. Leave **Swap these for a linked copy** on to turn what you picked into the prefab's first copy. Turn it off to keep the entities as they are.
5. Click **Make prefab**. The dialog says how many entities become one.

The prefab is saved as a file, `chicken.prefab.lst`, and shows in the Library with its picture, a blue prefab mark, and a **New** badge. The swap is one undo step: `Ctrl + Z` puts your entities back (the file stays; delete it in the Library if you don't want it).

> **Good to know:** one picked entity becomes the prefab's top, with everything under it. Several become the parts of a new top entity named as you named the prefab, each keeping its place in the world.

**Make an empty one:** right-click in the Library and choose **New prefab**. It opens on its own page, ready to build.

**In the Library:** pick a prefab to see it in **Details**: its picture, its folder, how many times it's placed and in how many levels ("Placed 6 times, in 2 levels"), and its parts. **Open** opens its page; **Place** puts a copy where the camera looks. Or drag it into the view.

**Place copies:** drag the prefab from the Library into the view, as often as you like. Each copy shows a link mark in the Level panel, and its parts are listed inside it.

**Change one copy:** pick a copy (or one of its parts) and change it in [Properties](entities.md) as usual. At the top, **Prefab: Chicken** says which prefab it follows. Values in white come from the prefab; a setting this copy changes has an orange dot beside it, and after its field **↶** with the prefab's value faint beside it: click **↶** to put that setting back, so the copy follows the prefab there again. **↑** makes the copy's value the prefab's, for every copy.

**Change every copy:** click **Open** (in Properties, or in Details), or double-click the prefab's picture in the Library. The prefab opens on its own page, in a tab beside the level's: its entities in the Level panel, the view and Properties, as a level has them, with a save and undo of its own. Change it, then save it (`Ctrl + S`). Every copy in the open level follows at once, and each keeps its own changes. Close the page with the × on its tab; Solis asks first if it has changes that aren't saved.

**Move a part of one copy:** drag it in the Level panel, under another entity or onto empty space to bring it out to the top level. It keeps its place in the world and is still part of its copy: the move is one of that copy's own changes, saved with the level, and the part still follows the prefab in everything else. If you delete the whole copy, a part you moved out of it stays, as an ordinary entity that no longer follows the prefab (undo puts both back).

**Unlink:** click **Unlink** beside **Prefab: Chicken** in Properties. The copy becomes ordinary entities that no longer follow the prefab, as one undo step. For a model, **Unpack** (Properties' **⋯**) splits it into one entity per part of the file, so each part can be moved, hidden or given components on its own.

## Settings
| What you see | What it means |
|---|---|
| A blue mark on a Library tile | The file is a prefab. **New** marks one made since the project opened. |
| A link mark in the Level panel | This entity is a placed copy, linked to its prefab. |
| An orange dot in the Level panel | This copy changes some settings (on a part: this part does). |
| No mark | A plain entity, not from a prefab. |
| **Prefab: Chicken** · **Open** · **Unlink** | Properties of a copy: which prefab it follows, its page, and making it plain. **Part of Chicken** on one of its parts. |
| An orange dot and **↶ 0.8** after a setting | This copy's own value; the prefab's is 0.8. **↶** puts it back. |
| *This part is gone from …* | The prefab no longer has this part, but this copy had changes to it. They're kept in case it comes back; **Remove** if you don't need them. |

## Shortcuts
| Action | Shortcut |
|---|---|
| Save the prefab's page (while it's in front) | `Ctrl + S` |
| Rename, duplicate or delete a copy, as any entity | `F2`, `Ctrl + D`, `Delete` |
| Open a prefab's page | Double-click its picture in the Library |
| Place a copy | Drag it from the Library into the view |

## Tips and common problems
- **Problem:** "prefabs place each other in a loop". **Why:** prefab A contains B, which contains A. **Fix:** remove one of the copies from the other.
- **Problem:** "placed more than 16 deep". **Why:** prefabs inside prefabs, 17 levels down. **Fix:** unlink some of the middle levels.
- **Problem:** I undid *Make prefab* and the file is still in the Library. **Why:** undo changes the level, not files. **Fix:** delete the prefab in the Library if you don't want it.
- **Problem:** copies in another level didn't change. **Why:** saving a prefab updates the copies in the open level at once; other levels show the change when they're opened. Nothing to fix: their copies follow the prefab as soon as they're read.
- A level file stores each copy as one block naming its prefab, plus only the settings that differ, so copies keep files small and Git merges them easily. The prefab's own file is a Lust data file you can read: a part per entity, a line per setting.
- **Assemblies:** Solis called prefabs *assemblies* until this version. Opening an older project renames each `.solassembly` file a `.prefab.lst` beside it, keeping its ID, so every copy and every spawn in game code still finds it; the older file is kept in the backups ([Backups and upgrades](backups-and-upgrades.md)). In game code, `Assembly.Ships.Fighter` is now `Prefab.Ships.Fighter` ([Game code](game-code.md#making-and-removing-entities)).
- A model's parts keep their links through a new export from Blender, as long as the parts keep their names.

## Related
- [Entities and components](entities.md)
- [Saving levels](saving-levels.md)
- [Projects and the Library](projects-and-the-library.md)
- [Game code: making and removing entities](game-code.md#making-and-removing-entities)
- Concepts: [prefab, copy, assembly](concepts.md#the-level)
