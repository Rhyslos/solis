# Sprites from Aseprite

> Draw your pixel art in Aseprite, save it into the project, and place it in the world as sprites: Solis reads the file as it is, packs its frames, and shows every save at once.

## When to use it
For the characters, crops, trees, furniture and anything else in a 2D game that's a picture rather than a tile of the ground. A [sprite](concepts.md#2d) is one frame of a [sprite sheet](concepts.md#2d), lying flat on the ground at its entity, standing on its [pivot](concepts.md#2d).

Solis reads Aseprite's own `.aseprite` files: there's nothing to export. Its [tags](concepts.md#2d) (runs of frames such as *walk* or *swing*) and [slices](concepts.md#2d) (named rectangles such as *feet*) come with it, so the picture and how it's used stay in one file. Keep the files in the project's assets folder, as you'd keep models and sounds.

Sprites are drawn with their colours exactly as you painted them: never lit, shaded or tone mapped. (Lamps and day and night on sprites come later in Phase 9.)

For the ground itself, tile maps come next (D4). Playing a tag as an animation comes with D3; until then a sprite shows the one frame you choose.

## How to use it
1. **Save your Aseprite file into the project's assets**, for example `assets/sprites/farmer.aseprite`. It appears in the **Library** with its first frame and how many frames it has. Saving it again in Aseprite updates it in Solis straight away.
2. **Look it over:** click it in the Library. **Properties** shows the frame big, a strip of every frame (click one to see it), and what Solis read from the file: its size, layers, tags (with their direction and repeats), slices and palette. Anything it couldn't use is listed in yellow.
3. **Give it a pivot:** in Aseprite, draw a slice called **feet** (or **pivot**) round where the character stands, and set the slice's pivot point on the ground between its feet. Without one, a sprite stands on the bottom middle of its frame.
4. **Place it:** drag it from the Library into the viewport, double-click it, or right-click it and choose **Place**. It lands under the pointer (or in the middle of the view) on a whole pixel, standing on its *feet* or *pivot* slice, as drawn. The new entity is named after the file and has a **Sprite** component.
5. **Choose the frame and the rest in Properties:** the Sprite component's **Sheet** offers every sprite sheet in the project, **Pivot** offers the sheet's slices, and **Frame** picks which frame it shows.
6. **Front and behind look after themselves:** the lower on the screen a sprite stands, the nearer it's drawn, so a farmer walking down past a tree goes in front of it, and walking up goes behind it. Things a whole metre higher (a roof, a bird) are always on top.

## Settings
The **Sprite** component, in Properties:

| Setting | What it does |
|---|---|
| Sheet | The sprite sheet (an Aseprite file in the project). Empty draws nothing. |
| Frame | Which frame it shows, counting from 1 as Aseprite does. A frame past the end shows the first. |
| Pivot | The slice it stands on, such as *feet*: the slice's pivot point (or its middle, if it has none) sits on the entity's place. Empty: the bottom middle of the frame. |
| Flip across | Mirrored left to right, round its pivot: one drawing faces both ways. |
| Flip up | Mirrored top to bottom. |
| Tint | Multiplied into its colours. White leaves it as drawn; a blue tint for night, red for a hurt enemy. |
| Flash | How white it flashes, 0 to 1. Turn it up for a moment when something's hit. |
| Outline | A one-pixel outline round it, to show what's selected or can be picked up. |
| Outline colour | The outline's colour. |
| See-through | For glass, smoke, ghosts and shadows painted half see-through. Otherwise each pixel is drawn or not (half see-through pixels are treated as clear), which needs no sorting and is faster. |

What Solis takes from the Aseprite file:

| In Aseprite | In Solis |
|---|---|
| Frames and their durations | The frames, and how long each shows (for D3's clips). |
| Tags | Clips: a run of frames with its direction (forward, reverse, ping-pong) and repeats. |
| Slices | Pivots (where a sprite stands), and later 9-slices, hitboxes and attach points. |
| User data on a cel | Events on that frame, such as *footstep* (for D3). |
| Visible layers | Flattened into one picture, with each layer's and cel's opacity. Hidden layers and reference layers are left out. Blend modes other than Normal are drawn as Normal (Properties notes it). |
| Palette | Listed in Properties. |

## Shortcuts
Sprites add no keys or mouse actions of their own: placing them uses the Library's, and moving them uses the usual move tools.

## Tips and common problems
- **Problem:** a sprite looks slightly smeared, or **Problems** says it *rests between pixels*. **Why:** it was moved or typed in at a place that isn't a whole pixel (perhaps with snapping off, or by game code). The game draws it on the nearest pixel, so it may seem to jump a pixel as it moves. **Fix:** move it with snapping to pixels on (the 2D view's standard), or type a place that's a whole number of pixels (with 16 pixels a metre, a multiple of 0.0625).
- **Problem:** the sprite floats above or sinks below where you put it. **Why:** it stands on its pivot, and the sheet has none, or the slice's pivot point isn't where the feet are. **Fix:** add a *feet* slice in Aseprite with its pivot between the feet, then pick it in **Pivot**.
- **Problem:** Properties says the file couldn't be read. **Why:** it isn't an Aseprite file, it's damaged, or it's bigger than Solis takes (64 MB, 4096 × 4096 pixels, 2048 frames, 256 layers). **Fix:** save it again from Aseprite. While you're saving, a half-written file keeps the last good picture, so sprites never vanish.
- **Problem:** Properties says the frames don't fit in a 4096 × 4096 picture. **Why:** all the frames of one file, packed together, must fit in a 4096 × 4096 [atlas](concepts.md#2d). **Fix:** split the file into several (one per character, say).
- **Problem:** **Problems** says a Sprite has no sprite sheet, or that its sheet isn't in the project. **Why:** the Sheet field is empty, or the file was deleted or never committed. **Fix:** choose a sheet in **Sheet**, or bring the file back.
- **Tip:** identical frames (a held pose repeated to last longer) are packed once, so long animations cost little.

## Related
- [2D: the pixel-perfect view](2d-pixel-perfect.md): base resolution, the 2D view, snapping to pixels.
- [Projects and the Library](projects-and-the-library.md): where asset files live.
- [Entities and components](entities.md).
- [Concepts](concepts.md): [sprite](concepts.md#2d), [sprite sheet](concepts.md#2d), [tag](concepts.md#2d), [slice](concepts.md#2d), [pivot](concepts.md#2d), [atlas](concepts.md#2d).
