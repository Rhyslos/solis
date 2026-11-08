# Parallax layers

> Make a sprite a background or foreground layer that moves with the camera at its own speed, so far hills drift slowly, leaves in front rush past, and a flat 2D level feels deep.

## When to use it
In side-on 2D games (platformers above all) and anywhere the camera travels a long way: a sky that stays put, mountains that crawl, trees that pass a little slower than the ground, and bushes or posts close to the camera that sweep past faster. It's the oldest trick for depth in 2D: things far away seem to move less when you move.

A **[parallax](concepts.md#2d) layer** is an ordinary [sprite](sprites.md) with a **Parallax** component, so it keeps everything sprites do: clips, tint, flash, lamps and day and night.

Top-down games rarely need it: everything there is about the same distance from the camera.

## How to use it
**A background**
1. Draw the layer in Aseprite: a line of hills, say, as wide as you like. For a layer that repeats, make its left and right edges meet.
2. Drag it from the Library into the viewport, so it's a sprite. Put it where it should be in the game's first picture: around where your game's camera starts.
3. In **Properties**, choose **Add component > Parallax**.
4. Set **Scroll across** and **Scroll up**: 0.2 for faraway mountains, 0.5 for hills, 0.8 for trees just behind the action. Leave **Repeat across** ticked so the hills never run out.
5. Press **Play** and move your character. The hills move slower than the ground.

Layers behind are drawn behind everything, tile maps included: the slowest first, so a 0.2 layer is behind a 0.5 one, whatever their places. The sky is usually a layer of its own with a scroll of 0, so it never moves.

**A foreground**
1. Make a sprite of something close to the camera: leaves, a post, fog.
2. Add a **Parallax**, tick **In front**, and give it a scroll above 1, such as 1.5.
3. Press Play: it sweeps past faster than the world, over everything.

**Where it's drawn.** A layer is exactly where you put it while the game's [camera](concepts.md#playing) is where it starts. As the camera moves, the layer moves at its own speed: with 0.5, half as far as the world. So the editor shows your game's first picture, layers and all. A layer can be several sprites (a row of trees with the same scrolls): they stay as far apart as you put them.

In the editor, layers stay where you put them (the editor's camera isn't the game's), repeated across the view, behind or in front as in the game. Clicking one behind the world selects it only where nothing else is in the way; the Area panel always finds it.

## Settings
**Parallax**

| Setting | What it does |
|---|---|
| Scroll across | How far it moves sideways with the camera. 0 stays still on the screen (the sky); 0.5 moves half as far (far hills); 1 moves with the world; 2 twice as far (close in front). From 0 to 4. |
| Scroll up | The same, up and down. Platformers often use the same number as across; a smaller one keeps the horizon steadier when the player jumps. |
| Repeat across | Drawn again and again side by side, at its sheet's width, so it never runs out. On to start. |
| Repeat up | Drawn again and again above and below. For skies or walls that should fill the screen whatever the height. |
| In front | Drawn over everything instead of behind it. Use it with a scroll above 1. |

## Tips and common problems
- **Problem:** a repeated layer shows a seam where the copies meet. **Why:** its left and right edges don't match, or the picture has empty columns at its sides. **Fix:** draw it so the edges continue into each other, using the whole canvas. Copies are one canvas width apart.
- **Problem:** a slow layer moves in little steps during a slow pan. **Why:** layers are drawn on the game's whole pixels, and a slow layer reaches the next pixel only now and then. Its own smooth slide comes in a later update.
- **Problem:** a layer far along the level isn't where I put it. **Why:** layers are where you put them only while the camera is at its start; by the time the camera gets there, a slow layer has gone along with it. **Fix:** place layers around the camera's start, and use **Repeat across** for ones that must last the whole level.
- **Problem:** the player bumps into nothing in front of a layer. **Why:** a Parallax only changes how the sprite looks; a [Collider2D](walls-and-walking.md) on the same entity stays where the entity is. **Fix:** keep layers as pictures only, and put colliders on entities of their own.
- Tile maps can't be layers yet: draw backgrounds as pictures.

## Related
- [Sprites from Aseprite](sprites.md): making the pictures layers use.
- [2D: the pixel-perfect view](2d-pixel-perfect.md): 2D cameras and the game's pixels.
- [Concepts](concepts.md#2d): parallax, sprite.
