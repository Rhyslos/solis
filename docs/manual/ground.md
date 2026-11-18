# The ground

> Rolling land under your level, made of one-metre voxels, which characters stand and walk on.

## When to use it

Most 3D levels stand on land: fields, hills, a valley to walk through. The **ground** is that land. A level has one, up to 2 × 2 km, and everything else (models, [brushes](brushes.md), characters) stands on it.

Why voxels and not a stretched sheet (a height map)? A height map can only say how high the land is at each spot, so it can never have a cave, an arch or a cliff that leans out. The ground is kept as **[voxels](concepts.md#the-ground)**: one-metre cubes, each knowing how far it is from the surface and what it's made of. That costs more memory (about 118 MB a square kilometre), but caves and overhangs come with it. Sculpting them comes next (see "What's coming").

For a small floor or a wall, a [brush](brushes.md) is quicker; for the land under a whole level, use the ground. It's 3D only: a 2D project hides it.

## How to use it

1. Open a 3D project and a level.
2. On **Home**, open **Create › More** and choose **Ground…** (or right-click in the [Library](projects-and-the-library.md) and choose **More › New ground…**).
3. In the dialog, choose its **Size** (1 × 1 km or 2 × 2 km), its **Height** (64, 128 or 256 m), its **Shape** (**Gentle hills** or **Flat**) and **Surface at**: how far above the ground's floor its surface is.
4. Press **Create**. The ground is made on the other cores of your computer, saved in the Library's folder, and an entity called **Ground** with a Ground component is added to the level, picked (one [undo](undo.md) step).
5. Its **[chunks](concepts.md#the-ground)** appear around the camera first, then farther out, without the editor ever stopping. Fly around it as you would anywhere ([Moving around the viewport](navigating-the-viewport.md)).
6. Press **Play**: characters stand and walk on it. **Stop** puts everything back.

> **Good to know:** a level has one ground. **Ground…** says so and does nothing when the level has one already, and so does adding a second Ground component. To use a ground saved earlier, add a Ground component to an entity and choose its file in **Ground**.

## What it's saved as

A ground is two kinds of file, which the Library shows as one:

- **`Ground.ground.lst`**: its settings, as text a person can read: its look, its size and height, and its list of ground kinds (Grass, Rock, Dirt and Sand to start with, each a name and a colour).
- **The folder beside it** (`Ground.ground/`): its voxels, one binary **[region file](concepts.md#the-ground)** for each 256 × 256 m square (`0_0.region`, `1_0.region`, …). The Library doesn't list the folder: rename, move, copy or delete the settings file and the folder goes with it.

Why binary? Voxels are like a picture's pixels: millions of numbers no one reads by hand. Each region is its own file so that, with Git, people working on different parts of a big world change different files. New projects keep region files in Git LFS and let them be locked, since two changes to one region can't be merged.

Each region file is written beside the old one and swapped in, so a crash while saving never leaves half a file.

## Settings

The **Ground** component, in Properties:

| Setting | What it does |
|---|---|
| Ground | The ground's settings file (`.ground.lst`). Its voxels are in the folder beside it. |
| Solid | On: characters and bodies stand on it and bump into it. Off: they fall through, for a ground that's only scenery. |

The ground's own settings, chosen when it's made:

| Setting | What it does |
|---|---|
| Size | 1 × 1 km (the target: drawn and solid at full speed) or 2 × 2 km (tested, but heavier). |
| Height | How tall it can be, floor to sky: 64, 128 or 256 m. Taller costs no memory where there's only sky. |
| Shape | **Gentle hills**: rolling land about 10 m up and down; **Flat**: level ground. |
| Surface at | How high its surface is above its floor. Leave room under it for caves to come. |

## Tips and common problems

- **Problem:** the ground appears a piece at a time. **Why:** each 32-metre chunk is made on another core, nearest the camera first, so the editor never stops while a big ground arrives. **Fix:** nothing to fix: it fills in within a few seconds.
- **Problem:** a character stops at an invisible line when Play starts. **Why:** the chunk ahead isn't solid yet: its collider is still being built, and the character is held at its edge rather than dropped through. **Fix:** it walks on once the chunk is ready, a moment later.
- **Problem:** the ground stops at its edge. **Why:** that's how it's made: the surface just ends there. **Fix:** keep players inside it, or put scenery along the edges.
- **Problem:** "A level has one ground". **Why:** a level holds one ground, up to 2 × 2 km. **Fix:** pick the Ground entity in the Level list to change it.

## What's coming

The ground is built in steps (Tools 1b):

- **Sculpting:** add, dig, smooth and flatten with a round brush (left-drag, `Ctrl` digs, `Shift` smooths), making hills, caves and overhangs.
- **Blocky and Sharp:** two more looks besides Smooth, chosen per ground.
- **Painting:** ground kinds (grass, rock, sand) painted on with the same brush, blended where they meet.
- **Game code:** asking the ground its height at a place, what it's made of, or where a ray meets it, and changing it while the game plays.
- **Open worlds:** worlds of 10 × 10 km and more, streamed in as you go.

## Related

- [Brushes](brushes.md): shapes for blocking out a level, standing on the ground.
- [Physics](physics.md): what's solid, and characters.
- [Play mode](play-mode.md): Play and Stop.
- [Concepts](concepts.md#the-ground): ground, voxel, chunk, region file.
