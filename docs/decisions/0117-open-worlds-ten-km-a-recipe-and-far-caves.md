# 0117: Open worlds: 10 × 10 km by default, a recipe for the ground, and caves seen far

- **Status:** Accepted (the project lead, 2026-11-14, after [the open-worlds study](../reports/open-worlds-study.md); builds on [0116](0116-the-ground-voxels-regions-and-what-games-may-change.md))
- **Date:** 2026-11-14

## Problem

Decision 0116 built the ground for 1 × 1 km in Tools 1b and set a following "Open worlds" phase for vast worlds. The open-worlds study measured how to get there:
- streaming chunks from the region files under a memory budget;
- drawing far ground coarser, in rings of fixed-size chunks with seams closed;
- a far field drawn as a height map;
- colliders only near characters;
- ways to make a starting ground.

It left five questions: the largest world promised, whether far caves may disappear, how a starting ground is made, the memory budget, and how tall a big world may be.

## Options

These are in the [study](../reports/open-worlds-study.md#questions-for-the-project-lead):
- the size: 10, 20 or 41 km;
- the far ground: a height map past 2 km, voxels to 4 km, or voxels to the horizon;
- the starting ground: baked from noise or a picture, a picture only, or the recipe kept;
- memory: 256 MB, 512 MB or 1 GB;
- the height: 128, 256, or 512 m or more.

## Decision

- **Size: 10 × 10 km (100 km²) is the default maximum.** Solis promises it and tests it at full speed. Settings and tools allow going further: 20 km, with 41 km tested as the stress case. The project lead first chose 41 km, then settled on 100 km² as the default, since 400 km² was more than needed.
- **The far ground: voxels to 4 km by default, and voxels to the horizon as a setting.**
  - Caves, arches and overhangs are drawn as voxels out to about 4 km, and past that the cheap height map takes over.
  - "Voxels to the horizon" is an option per world, and **it must be made as well as the default**:
    - its own scale cases and targets;
    - no seams, cracks or popping;
    - streaming that keeps up.
    
    Where a typical PC can't meet the frame target with it, the setting says so plainly rather than quietly drawing less.
- **The starting ground is a recipe.** A world keeps the settings of the noise that makes it (mountains, valleys, caves) and stores only the regions that have been sculpted.
  - Anything unsculpted is made from the recipe when it's needed, on workers.
  - Files stay small however big the world is.
  - The study measured making a chunk at about 40 times slower than reading one. So the phase must make that fast enough to keep up with the camera: made on workers ahead of the camera, coarse levels made first, and what's been made kept in a cache on disk that can be thrown away.
  - Importing a height-map picture can be added as one more recipe.
- **Memory: 1 GB** for a big world's ground on a typical PC (the study's view needed 48 MB). The rest goes to reading ahead, to units spread far apart, and to the horizon setting.
- **Height: 512 m or more**, so a big world can have real mountains. The phase plans chunks stacked in height, and only the columns that hold ground are kept.

## Consequences

- The phase's milestones in [ROADMAP.md](../../ROADMAP.md) (W1 to W5, proposed by the study) change:
  - **W1** streams from a recipe and sculpted regions, not from baked files.
  - **W2** draws voxels to 4 km, with the horizon setting.
  - **W4** makes the recipe's tools: the world map, noise settings, and how a picture is imported.
  - Its targets are 10 km at 1 GB and 512 m high.
  
  The phase's builder revises them from this decision when it starts.
- Tools 1b itself is unchanged: 1 × 1 km at 128 m (decision 0116). Its file format must allow taller grounds and recipes later, by a version raise with an upgrader (rule 23).
- Disk use falls: only sculpted regions are stored. The cost moves to making ground while playing, which the phase measures on a typical PC.
- Past about 40 km, numbers lose precision. Going further than 41 km needs a moving world origin, which W5 already plans.
