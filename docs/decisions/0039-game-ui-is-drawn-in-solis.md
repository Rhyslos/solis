# 0039. Game UI is drawn in Solis

- **Status:** Accepted
- **Date:** 2026-10-25

## Problem

Most engines build game UI from images: draw a button in another program, export it, slice it, import it, and do it all again for every change. The images blur at sizes they weren't made for, and effects like a glow or a pixel-art look have to be painted into every one. How should Solis's game UI be made?

## Decision

Chosen by the project lead: **Solis has a visual UI editor where game UI is drawn, canvas style, inside Solis.**

- **Drawn, not imported:** lines, shapes, curves and text, drawn by Solis for whatever screen it's on, so the UI is sharp at any size and needs no outside images (images can still be placed where wanted).
- **Live values:** any text, size, colour or visibility can be tied to game code (a variable or a method), so a health bar's width or a score's text follows the game by itself, in real time.
- **Code behind every element:** a UI file is ordinary Lust (a `view`), made by the editor; each element can be reached from code, and drawing and code are two ways of working on the same file. Files merge in Git like areas (decision 0020).
- **Styles over the whole UI:** glow, pixelation (for 3D pixel-art games), stylized outlines and more, applied by Solis because Solis draws everything.

DESIGN.md §12's "(to decide)" is settled by this: the visual UI editor and Lust's `view { }` are the game UI.

## Consequences

- **When** is decided when Phase 9 (effects and game UI) is planned: built then if it fits, or after Solis 0.1 on top of Phase 9's basics. Either way, Phase 9's UI basics are built so the drawn UI can grow from them (shapes drawn by Solis, values tied to code), not as an image-only system to be replaced.
- Other visual editors Solis plans follow the same rule, a graph saved as readable text: state machines (Phase 7), behaviour trees, and shader and material graphs. General visual scripting (a Blueprint-style replacement for code) isn't planned; Lust is the language.
