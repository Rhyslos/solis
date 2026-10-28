# 0068. Tile art and autotiling: a picture, kinds of terrain, slots and pieces, the Tileset tab

- **Status:** Accepted (builds on 0067)
- **Date:** 2026-10-28

## Problem

D4's maps store what each cell means and draw each terrain in a flat colour. D5 gives terrains art: each cell's picture worked out from its neighbours (autotiling), so edges come out smooth and a map never stores art. It had to settle:
- where the art comes from;
- which kinds of autotiling there are, and how the art is marked up;
- what happens when a needed tile isn't drawn;
- how the editor shows it;
- how big a piece of work it is.

## Options

- **Size:** two pull requests (art, 16-tile and 47-tile first, then edges, rules and the wizard), three, or one. The project lead chose one.
- **Art:** PNG only; Aseprite only; or either. Either was chosen: an Aseprite picture's frames also give animated tiles for free.
- **The 16-tile kind (dual grid)** draws art half a tile off the cells, each piece chosen by the four cells at its corners. It needs the fewest drawings, but a drawn tile doesn't sit on one cell. The alternative was 47-tile only, which needs more drawings. Shifted art was chosen.
- **The editor:** three mock-ups drawn on a real screenshot.
  - **A:** a Tileset tab like the Animator, with terrains, the picture and what's needed side by side.
  - **B:** the template as the canvas, with the picture as a strip along the bottom.
  - **C:** a wizard window, then Properties.

  The project lead chose A.

## Decision

- **A tileset's art comes from one picture**, a PNG, JPEG or Aseprite file in the project, cut into tiles of one size. Pictures become assets with IDs (sidecars like any other), so a tileset keeps its picture when the picture is moved or renamed. The app reads the picture whole, never scaled (4,096 pixels a side at most, 64 frames, fuzzed), again whenever it's saved.
- **Each terrain has a kind**, and each kind has numbered slots, one for each way the neighbours can be:
  - **plain:** one slot;
  - **16-tile:** pieces sit where four cells meet, chosen by which of them are the terrain;
  - **47-tile:** chosen by the eight neighbours, corners counting only where both sides match;
  - **edges:** chosen by the four sides that join (fences, walls, roads);
  - **rules:** the tileset's own patterns, from 1 × 1 to 9 × 9, tried in order. Each cell of a pattern is anything, the same, not the same, nothing, or a named terrain. The first pattern that matches gives the slot.

  Past a map's edge counts as the same terrain, so a map shows no border.
- **A piece is one tile of the picture filling a slot.** A second piece in the same slot is a variation, picked by weight from a hash of the cell's place: the same every time, never a random number. A piece can animate, either through tiles to its right (each frame lasting so many milliseconds) or through an Aseprite picture's frames (each with its own time).
- **No silent guessing.** A slot with no piece draws pink on the map. The Tileset tab dashes it red. Problems lists each map whose tileset still needs tiles. A terrain with no art at all is drawn in its colour.
- **The tileset file is version 2** (`picture`, `tile`, a `kind` per terrain, `piece` and `rule` blocks by stable random IDs). Version 1 reads as plain terrains with no art. A pattern's own characters (`? = ! _ *`) can no longer be terrain codes.
- **Drawing:** each visible 32 × 32 chunk's tiles are worked out once and kept until that chunk, or one beside it, changes. They are drawn as sprites from one atlas per picture: the frames stacked, plus a white patch for colours and pink. Later terrains in the tileset are drawn over earlier ones.
  - Seen from far away (more than 256 chunks in view), a map is drawn as D4 drew it, one coloured quad per layer.
  - A screen of 8 × 8 chunks autotiles in 3.4 ms.
- **The Tileset tab** (mock-up A) opens first beside the viewport when a tileset is double-clicked, or when one is made, and the open tileset shows as a second document tab in the top bar, as an open animator does.
  - **Left:** the terrains, with the picture and tile size. The grid is found from where the art changes, and offered when it differs from the size set.
  - **Middle:** a strip of steps (picture, grid, kind, mark up so many of so many) and the picture with its grid, numbers marking the chosen terrain's tiles. While the terrain is plain with no art, a guess of its kind is offered, worked out from the block of tiles at the tile picked.
  - **Right:** the chosen terrain's kind and what it needs: its template, each slot showing its art or a picture of its neighbours, dashed red when missing. A rules terrain shows its rules instead. Below that, a patch to try the edges on.
  - **Filling slots:** click a tile, then a slot, or drag the tile onto the slot. **Mark up from the template** fills every slot from a picture drawn over the template, starting at the tile picked.
  - **Templates:** **Save … template** writes the template as a PNG beside the tileset, at the tileset's tile size, to draw over.
  - **Properties** shows the picked slot's tiles, each with its weight, how often it's picked and its animation. With no slot picked, it shows the chosen terrain's name, code and colour.
  - **Saving and undo:** every change is one undo step (Ctrl+Z over the tab) and is saved at once, showing on every map straight away.

## Consequences

- **Changing a tileset's kind or art redraws every map**, because maps store meaning.
- **Painting comes with the tile painter (D6),** which will use clicks on a map. Until then a map's cells come from its file or the game.
- **Game code still can't reach maps;** that waits for the API milestone (decisions 0065 and 0067).
- **Pictures having IDs adds a sidecar beside each PNG and JPEG** in a project when it's next opened.
