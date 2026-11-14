# 0116: The ground: voxel chunks, three looks, region files, and what games may change

- **Status:** Accepted (the project lead, 2026-11-14, after [the ground study](../reports/ground-study.md); builds on [0111](0111-tools-1-brushes-and-ground.md))
- **Date:** 2026-11-14

## Problem

Tools 1b's ground has caves and overhangs, so a height map alone can't hold it. Its look is a setting per ground (Smooth, Blocky, Sharp), and it's sculpted and painted. Rule 22 asked for competing ways to store and draw it, built and measured against each other. That is G0, the ground study. The plan's "Still to settle" left four questions for the project lead: the file, the size and the edges, what game code may change, and what painting uses until materials exist.

## Options

The study measured these:

- **Storage:** fixed chunks, sparse chunks and an octree.
- **Smooth:** marching cubes and surface nets.
- **Sharp:** dual contouring and sharpened surface nets.
- **Blocky:** plain cube faces and merged cube faces.

It also gave 2 or 3 options for each question; [the report](../reports/ground-study.md) has the numbers and the costs.

## Decision

- **Storage:** fixed chunks of 32 × 32 × 32 one-metre voxels, 128 m high. Each voxel takes two bytes: its distance from the surface, and a ground-kind number. A chunk that is the same all through, such as sky or deep rock, is kept as one voxel. Storage sits behind one small interface, so sparse chunks can replace it when grounds grow.
- **The looks:** Smooth uses surface nets. Sharp uses sharpened surface nets, with hard edges where faces meet at more than 40°. Blocky uses cube faces merged into rectangles. Chunks are made on the job system's workers, never on the frame. Colliders are built when a stroke ends, off the frame.
- **The file:** a text file per ground holds its settings and palette. Its voxels go in binary files, one per 256 × 256 m region (runs of voxels, deflated), each one a file Git stores and locks on its own. The format has a version, upgraders and a fuzz target (rules 9 and 23). Decision 0006's text-first rule holds for everything a person reads; voxels are treated like a texture.
- **The size:** Tools 1b is built for 1 × 1 km at the scale target, with 2 × 2 km tested as the stress case. **Vast open worlds (20 × 20 km or more) come in a following "Open worlds" phase**, planned by its own study, which adds streaming and drawing far ground coarser. The region files are already shaped for streaming.
- **The edges:** the ground just stops. The surface ends at its edge, with nothing past it.
- **Game code may change everything the editor can:**
  - ask the ground (its height at a place, a ray against it, the kind at a place);
  - dig, add, smooth and paint at a place with the same round brush;
  - also change the look, the size and the palette.

  The calls are on the ground (decision 0065), with a limit per tick. Changed chunks are kept in save games and put back on Stop. Changing the look or the size remakes every chunk, which takes about a second for 1 km²; the call says so in its documentation.
- **Painting until materials exist:** each ground has a palette of up to 256 ground kinds, each a name and a colour now, or a picture laid on from three sides. That costs one byte a voxel. When Shared 7 brings materials, a kind can name one, and the voxels don't change.

## Consequences

- About 118 MB of voxels per km² in memory; edits take under 0.4 ms per dab; Blocky draws a quarter of the triangles of the smooth looks.
- The physics crate gains collider shapes built on a worker.
- G3 settles the step height on Blocky grounds and the hairlines between merged rectangles.
- G5's scale cases are the 1 km² ground (the target) and the 4 km² ground (stress). The 2,000 units follow the ground by reading its voxels (0.25 ms a tick), not as physics characters on it (43 ms a tick).
- The game-code interface grows with the ground's calls (rule 24), including the slow ones (look and size), which games should use rarely.
- A ground seen from outside its edge is a thin skin. Games keep players inside, or put scenery at the edges.
- ROADMAP.md gains the Open worlds phase after Tools 1b, once its study reports.
