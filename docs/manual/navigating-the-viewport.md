# Moving around the viewport

> Fly the camera through the 3D world, split the viewport into several views (flat ones too), and use the controls in the viewport's corners.

## When to use it
Whenever you want to look at the world from somewhere else. Solis has one camera style, the [fly camera](concepts.md#the-3d-view): you steer it like a drone, as in Unreal Engine.

## How to use it

**Looking and flying:**
1. Point at the viewport and **hold the right mouse button**. The pointer disappears.
2. Move the mouse to look around.
3. While still holding the button, use the keys:
   - **W** forward, **S** back, **A** left, **D** right (forward follows where you look, so looking down and pressing W flies down);
   - **E** straight up, **Q** straight down;
   - **Shift** to go three times faster while held.
4. Let go of the right button. The pointer comes back where it was.

**Without holding the right button:**
- **Mouse wheel:** moves forward or back in steps.
- **Hold the middle button (the wheel) and drag:** slides the view sideways and up or down. The ground under the pointer moves with it.

**Several views** (the **≡** menu, under *Views*): **One view**, **Two side by side**, **Two, one above the other**, or **Four views**. Each view has its own camera and its own look; with four, they start as Perspective, Top, Front and Right, the classic way to line things up exactly.
- **Click in a view** (or turn the wheel over it) to use it: it gets a thin coloured outline and the tool bar and snapping controls. **F**, Entity > Add, and anything else that goes "where the camera looks" use that view.
- Selecting, the gizmos, and dragging things in from the Library work in every view.
- **Maximise** (bottom left of each view) shows that view alone, filling the viewport; **All views** brings the others back.
- Solis remembers the layout and every view's camera next time.

**Flat views** ([concepts](concepts.md#the-3d-view)): the **Perspective ▴** button at the bottom left of each view chooses what it shows: **Perspective**, **Top**, **Front** or **Right**. A flat view shows things at their true size whatever their distance, with the grid standing in the view's own plane, so lining up edges and heights is exact.
- **Hold the right or middle button and drag** to slide the view.
- **Turn the wheel** to zoom in or out, towards the pointer.
- Flat views don't fly: switch back to Perspective for that.
- Flat views have no [fog](concepts.md#light), so faraway parts are as clear as near ones.
- A camera or spotlight added from a flat view (and **Move here**) goes just in front of where the view looks from, as if it were a camera above or beside the area looking in.

These are the flat-view controls Unreal Engine uses.

**The corner controls:**
- **Top right:** frames per second, the time each frame takes, how many entities are in the area, and how many triangles the imported models have.
- **Bottom left:** **≡** chooses the views and which corner controls show. **Perspective ▴** chooses the view's kind (flat views above). The look button (**Standard ▴**) chooses how models are shaded; see [Looks](looks.md). **Show** lists things to hide, arriving in later phases. These menus open upwards.
- **Bottom centre:** the tools, Select (**Q**), Move (**W**), Rotate (**R**), Scale (**S**) and Transform (**T**). See [Selecting and moving things](selecting-and-moving.md).
- **Bottom right:** the axis locks (**X**, **Y**, **Z**), **World** or **Object** axes, snapping (**Grid**, **Angle**, **Scale**), and **Camera**, the camera's speed step. See [Selecting and moving things](selecting-and-moving.md).

With several views, only the view in use shows the tools (top left), snapping (top right) and frame rate (bottom right), so the small views stay clear.

## Settings
| Setting | What it does |
|---|---|
| Camera speed (1 to 8) | How fast W, A, S, D, Q and E move: 1, 2, 4, 8, 16, 32, 64 or 128 metres a second. Change it by turning the mouse wheel **while holding the right button**. 4 (8 metres a second) is a good start; use 6 to 8 to cross a big map. Shown in the Camera box. |

The camera's position, direction and speed are remembered next time.

## Shortcuts
| Action | Shortcut |
|---|---|
| Look around | Hold right mouse, move the mouse |
| Fly | Hold right mouse + W A S D |
| Down, up | Hold right mouse + Q, E |
| Faster for a moment | Hold Shift while flying |
| Change speed | Hold right mouse + mouse wheel |
| Forward, back in steps | Mouse wheel |
| Slide | Hold middle mouse, drag |
| Slide a flat view | Hold right or middle mouse, drag |
| Zoom a flat view | Mouse wheel (towards the pointer) |
| Use a view | Click in it |

## Tips and common problems
- **Problem:** pressing W picked the Move tool instead of flying. **Why:** the right mouse button wasn't held. The tool keys work only when it isn't. **Fix:** hold the right button first, then press W.
- **Problem:** you're lost. **Fix:** fly up with E, look down, and find where the red and green lines cross: that's the origin.
- **Problem:** holding the right button doesn't fly. **Why:** the view is a flat one (Top, Front or Right). **Fix:** drag to slide it, or switch it to Perspective.
- **Problem:** F or Add entity went to the wrong view. **Why:** they follow the view in use, the one with the outline. **Fix:** click in the view you want first.
- **Problem:** flying feels too slow or too fast. **Fix:** change the speed with the wheel while holding the right button.

## Related
- [Keyboard shortcuts](keyboard-shortcuts.md)
- [Concepts](concepts.md): Z up, axes, metres, grid.
