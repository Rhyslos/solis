# ui

**Layer 1 · Engine**

Game UI (Sungraph UI, [docs/plans/game-ui.md](../../docs/plans/game-ui.md); [decision 0083](../../docs/decisions/0083-game-ui-on-any-screen.md)): screens as trees of elements, laid out on whatever screen the game is on and turned into shapes ready to draw.

## Purpose

- **Screens** (`screen.rs`): a screen has a design size, a scale rule and elements. An element has a name, what it draws (a group, a box with rounded corners, an ellipse, a line, a bar filled to a value, text), an anchor (one of left, middle, right or stretch across, and top, middle, bottom or stretch down), an offset and size in design pixels, a fill (one colour, or two across or down), a border, an opacity and children.
- **Layout** (`layout.rs`): each element placed in its parent by its anchor; sizes times the scale rule's scale (grow: the screen's height over the design's; keep: 1; whole steps: the game picture's whole step, every edge snapped to the game's pixels); out come `Shape`s in drawing order, in screen pixels; an element that `clips` (a Lust canvas) cuts off what's inside it at its edges (`Shape::clip`). `layout_at` lays out inside part of the screen (a 2D game's picture between bars). Hostile numbers (NaN, infinite, huge) never make a shape that isn't finite; past `MAX_ELEMENTS` or `MAX_DEPTH` the rest is left out.
- **Text** (`text.rs`, decision 0084): `Fonts` reads font files with `ab_glyph` (a broken or oversized one is left out with a note), lays words out left to right with kerning and a line per `\n`, and makes each letter once into a single-channel distance image in one `Atlas`. The atlas changes only in `begin_frame`, called once a frame before any text is laid out: when a letter didn't fit, the next frame lets go of the letters the last one didn't show and moves the rest together (copied, not made again); a frame that needs more letters than fit leaves the rest out without making anything again. `layout_with` lays text elements out with the fonts (without them, text is left out); crisp screens land each letter on the game's pixels.
- **Views from Lust** (`view.rs`, decision 0087): a screen's `view` tree laid out with Lust's own rules. Buttons (decision 0088): `lay_out_with` also gives each clickable piece's box (`Clickable`), drawing the one in `Looks::hover` darker (or with its `hover:[…]` styles) and the one in `Looks::focus` with a ring; `hit` and `layout::clickable_at` find the one at a point; `has_actions` says whether a view has any; `layout::clickables` gives a screen's, and `next_focus` moves the keys' focus to the nearest one a way. `layout::picture_area` is where a 2D game's picture (and so its screens) goes on the screen, which `render` uses too.
- **Pictures** (`pictures.rs`, decision 0089): `Pictures` packs every picture a game's screens show (`image "sprites/coin.png"`, or a crop, `#Axe`) into one 2048 × 2048 atlas, each with a pixel of its edge copied round it and its colours premultiplied; a `Screen` carries the game's (`Screen::pictures`). A `Picture` element fits its picture inside its box, or covers it (`cover`), and becomes a `Picture` shape from its part of the atlas, blocky when shown bigger than it is; one not there is Lust's grey box with its name. The view measures an `image` by its picture's size (`Measure::picture`, `view::WithPictures`).
- **Preview sizes** (`presets.rs`): the screens the UI editor's Screen list offers.
- **A sample HUD** (`sample.rs`): the mock-up's, for the Console's `ui test` and the tests.

## Must never

- Know about the GPU, a window or the editor: `render` draws the shapes, the editor shows them.
- Give back a shape whose numbers aren't finite, or one inside out.

## Depends on

`ab_glyph` for fonts (proptest for its tests).

## Status

Usable first, step 7 (game UI), M1a (layout and shapes), M1b (text), M2 (screens in Lust), M3 (buttons, menus, the game's speed) and pictures on screens built. The UI editor is next.
