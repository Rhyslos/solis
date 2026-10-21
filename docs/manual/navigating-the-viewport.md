# Moving around the viewport

> Fly the camera through the 3D world, and use the controls in the viewport's corners.

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

**The corner controls:**
- **Top right:** frames per second, the time each frame takes, how many entities are in the scene, and how many triangles the imported models have.
- **Bottom left:** **≡** chooses which corner controls show. The look button (**Standard ▴**) chooses how models are shaded; see [Looks](looks.md). **Perspective** and **Show** list views that arrive in later phases. These menus open upwards.
- **Bottom centre:** the tools, Select (**Q**), Move (**W**), Rotate (**R**), Scale (**S**) and Transform (**T**). They work on things once scenes have them (Phase 2).
- **Bottom right:** Grid, Angle and Camera. **Camera** is the camera's speed step.

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

## Tips and common problems
- **Problem:** pressing W picked the Move tool instead of flying. **Why:** the right mouse button wasn't held. The tool keys work only when it isn't. **Fix:** hold the right button first, then press W.
- **Problem:** you're lost. **Fix:** fly up with E, look down, and find where the red and green lines cross: that's the origin.
- **Problem:** flying feels too slow or too fast. **Fix:** change the speed with the wheel while holding the right button.

## Related
- [Keyboard shortcuts](keyboard-shortcuts.md)
- [Concepts](concepts.md): Z up, axes, metres, grid.
