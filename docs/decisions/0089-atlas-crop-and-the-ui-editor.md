# 0089. Atlas Crop, the UI editor's layout, and how screens get looks and pictures

- **Status:** Accepted (the project lead, 2026-11-06: "Could we call it Atlas Crop instead? Otherwise, go ahead and make it, looks good", and "Yes it did cover everything"; builds on 0083, 0087 and 0088)
- **Date:** 2026-11-06

## Problem

Decision 0083 settled cutting named pictures out of a sheet (Godot's AtlasTexture) as **Cut out pictures…**, kept in `tools.pictures.lst`. Its mock-up (`ui5`) was lost, so the feature was described again from the decision. The UI editor (the game UI plan's M4 and M5) needed its layout approved from a mock-up drawn on a real screenshot (`m4`), and three choices were open: glow and outlines on single pieces (Lust has no words for them), pictures on screens (Lust's `image` takes a file as text), and text size for players.

## Decision

- **Atlas Crop** is the feature's name: right-click a picture, **Atlas Crop…**, which opens the **Atlas Crop** tab. A picture's crops are kept in `<picture>.atlas.lst` beside it (a Lust data file, decision 0081, kind `atlas`, version 1): the picture's file name, the grid, and each crop's name and box. The picture never changes. Crops snap to the grid; **Crop every tile** cuts every cell with something drawn in it. Game code names a crop `Picture.Sprites.Tools.Axe` (a project state in `solis/project.lst`).
- **The UI editor's layout is approved as drawn (`m4`):** a UI workspace in the top bar, a screen opening as a document tab; the Elements list in the Area panel's place; the canvas in the viewport's, with preview sizes, the safe area, snapping and preview values, and a tool bar like the viewport's (its keys 1 to 7 and the radial menu on E, as the plan's answers chose); Properties in Lust's own terms (width and height, a 3 × 3 alignment grid, padding, colour, corners, hover look, disabled), and for a button, **When clicked: sends … to …**, with anything more in code.
- **The recommended answers to the three choices:**
  - Glow, outline and pixel size are a **whole screen's look**, set on its Screen component; per piece, Lust's own `shadow` is drawn as a soft glow in the piece's colour. Lust doesn't change.
  - Pictures on screens use Lust's own `image` with the file as text, checked by Solis (a missing one in Problems); a crop is `image "sprites/tools.png#Axe"`.
  - Text size and UI size are player settings game code sets and reads: `Screens:setTextSize(1.5)`, `Screens:setUiSize(1.2)`, `Screens:textSize()`, `Screens:uiSize()`.

## Consequences

- Atlas Crop comes first (this decision's first pull request); pictures drawn on screens follow, then the UI editor (M4, M5), then styles and text size (M6) and the UI sample (M7).
- `image "file#Crop"` is a string name for something Solis makes, an exception to the API guidelines' rule, because `image` is Lust's own piece. Solis checks every such name, so a typo is still a problem shown before the game runs.
