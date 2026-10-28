# 2D: the pixel-perfect view

> Make a 2D game whose pixel art stays crisp: the game is drawn small, at its own resolution, and scaled up to the screen in whole steps, and the editor's 2D view shows the world in pixels.

## When to use it
For a 2D game, especially one in pixel art: a farming game, a platformer, a top-down adventure. Pixel art is drawn one pixel at a time, and it only looks right when every one of its pixels becomes the same number of screen pixels. Draw it straight at the screen's size and pixels end up different widths, edges blur when the camera moves by less than a pixel, and sprites shimmer as you walk. Solis avoids all three by drawing the game small, at its [base resolution](concepts.md#2d), then scaling the finished picture up.

2D in Solis lives in the same world as 3D: flat things lie on the ground, seen from straight above, so lights, the sun, sound, saving and game code all work on them as they do in 3D. A project can mix both, with a 3D camera in one area and a 2D camera in another.

For now you can set a project up for 2D, look at it in pixels, and play it through a 2D camera. Sprites from Aseprite, frame animation and tile maps come next.

## How to use it
1. **Set the project's 2D settings:** **Edit > Settings…**, then **2D** under *This project*. The standard ones suit most pixel-art games: 480 × 270, 16 pixels to a metre and 16-pixel tiles. They're saved in the project (`pixels.solpix`), so everyone working on it shares them.
2. **Look in pixels:** in the viewport, the **Perspective ▴** button at the bottom left, then **2D**. The view looks straight down at the ground. The grid is the tile size, and the corner shows the zoom.
   - **Hold the right or middle button and drag** to slide the view.
   - **Turn the wheel** to zoom in or out one whole step at a time (100%, 200%, 300%…), towards the pointer. At whole steps every pixel of your game is the same size on screen, so nothing blurs.
3. **Add a 2D camera:** with the 2D view in use, **Entity > Add camera**. It goes over the middle of the view, on a whole pixel, as a 2D camera. A dashed frame shows what it sees, labelled **Game screen** for the camera the game looks through (the first camera in the area). To turn an existing camera into one, set its **View** to **2D** in Properties.
4. **Move things on whole pixels:** in the 2D view, moves land on whole pixels of your game by themselves, so nothing rests between two pixels and looks smeared. **Snap ▴** in the bottom-right corner changes that to whole tiles, or off. Hold **Ctrl** while dragging to flip it for one move.
5. **Press Play.** The Game panel draws the game at its base resolution and scales it up to fill the panel in whole steps, with black bars round the edge if the panel doesn't divide evenly. Move the 2D camera (game code moves it like any entity) and the picture slides smoothly, without shimmering.

## Settings
| Setting | What it does |
|---|---|
| Base resolution | The size the game is drawn at, in pixels, before it's scaled up. 480 × 270 scales to a 1080p screen four times and to 4K eight times exactly; 320 × 180 gives chunkier pixels and shows less of the world; 640 × 360 shows more. The menu offers common sizes, or type your own. |
| Pixels per unit | How many pixels make one metre of the world. Usually the same as the tile size, so a tile is one metre: physics, sound distances and game code then all count in tiles. |
| Tile size | The 2D view's grid, in pixels, and the size tile maps will use when they arrive. |
| Scaling to the screen | **Whole steps** (the standard): every pixel becomes the same whole number of screen pixels, with bars at the edges when the screen doesn't divide evenly: always crisp. **Fill the screen**: no bars; it scales up as far as whole steps go, then smooths only the last bit at each pixel's edge, so edges stay sharp. |
| Camera > View | **Perspective** (as the eye sees) or **2D** (flat, straight down, at the base resolution). A 2D camera follows its entity's place but never turns. |
| 2D view > Snap | What moves land on: **whole pixels**, **tiles** or **off**. |
| 2D view > Zoom | Screen pixels for each pixel of the game, from 25% (to see a big map) to 2400%. |

## Shortcuts
| Action | Shortcut |
|---|---|
| Slide the 2D view | Hold right or middle mouse, drag |
| Zoom the 2D view a step | Mouse wheel |
| Flip snapping for one move | Hold `Ctrl` while dragging |

## Tips and common problems
- **Problem:** black bars round the game. **Why:** the panel or screen isn't a whole number of times the base resolution, and Whole steps keeps every pixel the same size. **Fix:** that's expected; choose **Fill the screen** if you'd rather have no bars, or a base resolution that divides your screen (480 × 270 for 1080p and 4K).
- **Problem:** the Game panel looks blurry or the pixels are uneven. **Why:** the area's first camera is a 3D one. **Fix:** set its **View** to **2D**, or move the 2D camera first in the Area panel.
- **Problem:** something isn't in the game's picture though it's under the camera. **Why:** a 2D camera sees what's below it, between its **Near** and **Far**. **Fix:** keep the camera above everything (Add camera puts it 100 m up), or raise its **Far**.
- The 2D view's frame and the Game panel show the same thing, so you can line up a scene in the editor and know what the player will see.

## Related
- [Settings](settings.md): the Settings window, and the This project pages.
- [Moving around the viewport](navigating-the-viewport.md): the other views.
- [Play mode](play-mode.md): cameras and the Game panel.
- [Concepts](concepts.md#2d): base resolution, pixels per unit, pixel-perfect, whole steps.
