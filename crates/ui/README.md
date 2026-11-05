# ui

**Layer 1 · Engine**

Game UI (Sungraph UI, [docs/plans/game-ui.md](../../docs/plans/game-ui.md); [decision 0083](../../docs/decisions/0083-game-ui-on-any-screen.md)): screens as trees of elements, laid out on whatever screen the game is on and turned into shapes ready to draw.

## Purpose

- **Screens** (`screen.rs`): a screen has a design size, a scale rule and elements. An element has a name, what it draws (a group, a box with rounded corners, an ellipse, a line, a bar filled to a value), an anchor (one of left, middle, right or stretch across, and top, middle, bottom or stretch down), an offset and size in design pixels, a fill (one colour, or two across or down), a border, an opacity and children.
- **Layout** (`layout.rs`): each element placed in its parent by its anchor; sizes times the scale rule's scale (grow: the screen's height over the design's; keep: 1; whole steps: the game picture's whole step, every edge snapped to the game's pixels); out come `Shape`s in drawing order, in screen pixels. `layout_at` lays out inside part of the screen (a 2D game's picture between bars). Hostile numbers (NaN, infinite, huge) never make a shape that isn't finite; past `MAX_ELEMENTS` or `MAX_DEPTH` the rest is left out.
- **Preview sizes** (`presets.rs`): the screens the UI editor's Screen list offers.
- **A sample HUD** (`sample.rs`): the mock-up's, for the Console's `ui test` and the tests.

## Must never

- Know about the GPU, a window or the editor: `render` draws the shapes, the editor shows them.
- Give back a shape whose numbers aren't finite, or one inside out.

## Depends on

Nothing (proptest for its tests).

## Status

Usable first, step 7 (game UI), M1a: layout and shapes. Text is next.
