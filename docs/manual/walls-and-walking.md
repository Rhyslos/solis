# Walls and walking in 2D

> Make terrains solid so nothing walks through fences and water, give things a 2D collider, and walk a character around a top-down map in 8 directions or 4.

## When to use it
In any top-down 2D game: a farmer who walks the farm but not through its fences, a shop counter you can't walk through, villagers who stop at walls. Solis gives you the pieces; your game decides what they mean. Solis never decides that water is solid or that NPCs walk in 4 directions: you do.

Three parts work together:
- **[Solid](concepts.md#2d) terrains:** cells nothing with a body can walk into (fences, water, rock). Set per terrain, in its tileset, so every map using the tileset agrees.
- **[Collider2D](concepts.md#2d):** a shape on the ground for 2D physics, in pixels. Solid and still on its own (a house, a bin); ticked **Character**, game code walks it.
- **The TopDownController:** a ready-made Lust component in the kit that walks a character with the Move action (WASD, the arrow keys, a controller's left stick).

2D physics is its own world, separate from 3D [physics](physics.md), and gives the same result on every PC. For 3D characters, see [Characters and cameras](characters-and-cameras.md).

## How to use it
**Make terrains solid**
1. Click your tile map in the Library (or select its Tile Map entity) so the **Map** tab shows it.
2. Under **Terrains**, right-click a terrain's swatch (say, fence) and tick **Solid**. A small wall mark appears on its swatch. Left-click still picks it for painting.
3. Do the same for every terrain that should block: water, rock, walls.
4. Press **Walls** at the top of the viewport (beside Chunks) to see what's solid: green rectangles over the map. Neighbouring solid cells merge into a few big rectangles, so a character sliding along a fence never catches on the seams between tiles.

**Walk a character**
1. Select your player's entity (one with a [Sprite](sprites.md), say).
2. In **Properties**, choose **Add component > Collider2D**. Leave its shape a **Circle** (circles slide round corners best) and set its **Radius** to about a third of the sprite's width. Set **Up** to put the circle at its feet. A green outline shows it in the viewport.
3. Tick **Character**.
4. Fetch the kit's walker in your game code (`main.lst`):

   ```lust
   fetch "solis/kit/topdown":[TopDownController];
   end
   ```

5. Save, then in **Properties** choose **Add component > TopDownController**.
6. Press **Play** and walk with WASD or the arrow keys. It stops at solid tiles and other colliders and slides along them.

**Walk an NPC from your own code**
Add a TopDownController, untick **Uses input**, set **Directions** to 4 if you like, and set its `wanted` field from your code to where it should head (`[1, 0, 0]` walks right, `[0, 0, 0]` stops). Or call `walk` yourself with how far to move this tick:

```lust
component Wanderer {
    method update(float dt) {
        walk([1.5 * dt, 0, 0]);
    }
}
```

In Lua it's `solis.entity.walk(self.me, {1.5 * dt, 0, 0})`. In Rust, through the component's host: `cx.host().walk(cx.me(), Vec3::new(1.5 * dt, 0.0, 0.0))`.

A 2D character moves on the ground: the move's third number (height) is ignored, and it stays on its layer.

**Side-on games (platformers).** The second number is up and down, so walk a hero down a little every tick for gravity, and ask how its last move went:

- `grounded()` is true when a floor under it stopped it going down: jump only then. A wall, or a slope steeper than about 45°, isn't ground.
- `velocity()` is how fast its last tick's move really went, in metres a second: less than you asked for when something stopped it. A head bump shows as `velocity().y` dropping to 0 while you asked to go up.

```
component Hero {
    has { float fall = 0; }
    method update(float dt) {
        fall = fall - 30 * dt;
        is grounded()? then {
            fall = 0;
            is Input:pressed(Action.Jump)? then { fall = 12; }
        }
        walk([4 * dt, fall * dt, 0]);
    }
}
```

When an entity with a Collider2D is despawned, its collider goes with it at once, so a squashed enemy never leaves an invisible wall.

## Settings
**Collider2D**

| Setting | What it does |
|---|---|
| Shape | **Circle** (best for characters: it slips round corners) or **Box** (buildings, counters). |
| Radius | A circle's size, in pixels. A third of a 16-pixel sprite's width (about 5) is a good start. |
| Width, Height | A box's size, in pixels. |
| Across, Up | Where the shape's middle is from the entity's place, in pixels. Raise a character's circle to its feet so its head can overlap things above it, as top-down games do. |
| Character | Ticked: game code walks it (`walk`, or the TopDownController) and it slides along what's in its way. Unticked: solid and still. |

Pixels become metres with the project's 2D settings (**File > Project settings**, pixels per unit).

**TopDownController**

| Setting | What it does |
|---|---|
| Speed | Metres a second at full push. 4 is a brisk walk at 16 pixels a metre. |
| Acceleration | How quickly it gets up to speed, in metres a second each second. High feels snappy; low feels heavy. |
| Deceleration | How quickly it stops when let go. |
| Directions | 8, or 4 (only the bigger of across and up counts: no diagonals, like older games). |
| Uses input | Ticked: walks with the Move action. Unticked: walks towards `wanted`, set by your code. |
| Facing | The way it last walked (read it to choose a sprite's facing). |

**Terrains** (right-click a swatch in the Map tab)

| Setting | What it does |
|---|---|
| Solid | Nothing with a Collider2D passes into its cells, on any layer. Saved in the tileset (`solid = true;`). |

## Shortcuts
| Action | Shortcut |
|---|---|
| Walk (with the TopDownController) | `W A S D`, the arrow keys, or a controller's left stick |

Change them in **File > Input** (the Move action). A project with its own input actions file from before keeps its keys: add the arrows to Move there if you want them.

## Tips and common problems
- **Problem:** the character walks through a fence. **Why:** the fence's terrain isn't solid, or the character has no Collider2D ticked Character. **Fix:** right-click the fence's swatch and tick Solid; press Walls to check it shows.
- **Problem:** nothing moves when I press keys. **Why:** the entity has no TopDownController, or its Collider2D isn't ticked Character (walk only moves characters). **Fix:** check both in Properties, and look in the Console for game code mistakes.
- **Problem:** the character stops at a doorway it should fit through. **Why:** its circle is wider than the gap. **Fix:** make the Radius smaller, or the door wider.
- **Problem:** it catches on corners. **Why:** a Box collider meets corners square on. **Fix:** use a Circle for characters.
- **Problem:** the [Console](console.md) says 2D physics stopped and was started again. **Why:** 2D physics met something it couldn't work out, usually a Collider2D or a tile map whose size or place is far out of the ordinary. Rather than leave every character standing still, Solis builds 2D physics again from the entities and tile maps as they are, and the game carries on; the message names the character it was moving when it knows. If it keeps happening, it's built again at most once a second (so the game doesn't slow to a crawl), and 2D characters stand still in between. It's said once each time you press Play. **Fix:** look at the entity named, or at anything very big or very far away, and bring it back into range.
- Walking into a door or a shop counter to make something happen (a trigger) comes with a later update.

## Related
- [Tile maps](tile-maps.md), [Tilesets](tilesets.md) and [Painting tiles](painting-tiles.md)
- [Input actions](input.md)
- [Game code](game-code.md)
- [Physics](physics.md) (3D) and [Characters and cameras](characters-and-cameras.md) (3D characters)
- [Concepts: 2D](concepts.md#2d)
