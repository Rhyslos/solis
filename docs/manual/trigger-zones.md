# Trigger zones

> A place in a level that tells your game code when something walks in or out: a door, a shop counter, a pit.

## When to use it
Whenever something should happen because of **where** a character is: a door that opens when the player reaches it, a shop that greets them at the counter, a pit that sends them back to the start, a room that starts its music. A trigger zone is invisible in the game. It watches a box, a circle (a sphere in 3D) or, in 2D, a shape you draw point by point, and each time a character or a moving body crosses its edge, it tells game code.

Why a zone and not a check in your own code ("is the player near the door?") every tick? The zone does the asking for you, the same way every time, and only says something when it **changes**: one `Entered` as the player steps in, one `Exited` as they leave, never sixty a second while they stand there. It also remembers who's inside in [save games](save-games.md), so loading a save never says "entered" twice.

Trigger zones work the same in 2D and 3D levels: the same component, the same settings and the same events.

> **Good to know:** a zone only tells. What happens (a door opening, a fade, a teleport) is your game code's. Ready-made doors come later in stage 1, as a **Door** in the kit, built on trigger zones.

## How to use it
1. On **Home**'s ribbon, in **Create**, click **Trigger zone** (in a 3D project it's in **Create › More**). A new zone appears in the view, in the middle of where you're looking, and is picked.
2. **Size it:** drag the small squares on its teal outline (its handles). Dragging a side moves that side and keeps the far one where it is; a circle's handle sets its radius. Sizes snap as moves do (whole pixels or tiles in 2D, the grid in 3D), and each drag is one step to undo. Or type its **Size** in Properties' **TriggerZone** card.
3. Pick its **Shape**: **Box**, or **Circle** (2D) or **Sphere** (3D). In 2D there's **Freeform** too (below).
4. Pick when it **Fires**: **Every time** something walks in or out, or **Once**, the first time something walks in and never again.
5. Give it game code: in the **What game code hears** card, click **New code for this zone…**. Solis makes a code file named after the zone, with empty `Entered` and `Exited` ready to fill in, adds its component to the zone, and opens the file.
6. Press **Play** and walk a character in. While playing, the card's **Inside now** names who is inside.

In the view, a zone is a **teal outline**: solid with handles when it's picked, dashed when it isn't, with its name and size beside it. Click its outline to pick it (a click inside it is left for whatever is in the zone); the Level panel lists it with a dashed square and **Trigger zone**. It turns and grows with its entity, like any shape: move it with the move gizmo (`W`), turn it with `R`.

### A freeform shape (2D)
A room that isn't a rectangle, a winding path, a pond's bank: pick **Freeform** in **Shape**, and drawing starts in the view.
1. Click to place each point, going round the shape.
2. Click the first point again, or press `Enter`, to close it. `Esc` while drawing gives up and keeps the shape it had.
3. Later, press `Tab` with the zone picked (or click **Edit points** in its card) to change its points: drag a point to move it, `Ctrl + click` on an edge to add a point there, and `Delete` to remove the picked point. `Tab` or `Esc` finishes. Each drag or change is one step to undo.

A freeform shape may bend inward (an L, a horseshoe), but its outline may never cross itself: a change that would make it cross is refused, saying why, and the last good shape is kept. It has at most 256 points and at least 3.

> **Good to know:** a freeform shape is split into simple pieces once, when you change it, so even a big one costs the game nothing more each tick.

## Settings
| Setting | What it does |
|---|---|
| Shape | **Box**, **Circle** in 2D or **Sphere** in 3D, and in 2D **Freeform**. |
| Size | A box's width and height (and depth in 3D), in metres; a circle's or sphere's radius. For a freeform shape, its number of points and **Edit points**. |
| Notices | What the zone watches for: **Characters and bodies**, meaning characters (a CharacterBody, or a Collider2D ticked Character) and bodies that move (a RigidBody that's dynamic or kinematic). Walls, floors and other things that never move are never "inside". |
| Fires | **Every time**: `Entered` whenever something walks in, `Exited` whenever it walks out. **Once**: only the first `Entered` (and the walker's `EnteredZone`), then nothing ever again, even after loading a save. |
| Only tags | Greyed for now: it comes with tags, to notice only some kinds of thing ("only the player"). |
| Inside now | While playing, who is inside the zone. While editing it says "Shown while playing". |

## What game code hears
Each tick, after physics, Solis looks at who is inside each zone and compares it with the tick before:
- **The zone** hears `Entered(entity who)` when something walks in, and `Exited(entity who)` when it walks out.
- **Whatever walked** hears `EnteredZone(entity zone)` and `LeftZone(entity zone)`.
- A zone can be asked who's inside it now: `inside()`, a list of entities.

```lust
-- door.lst: a door zone that greets whoever comes in.
fetch "solis/world":World;
fetch "solis/zones":[Entered, Exited];

component Door {
    -- Someone walked in: say who, and how many are inside now.
    on Entered(entity who) {
        int count = #inside();
        World:say(who:name() + " came in; " + count.text() + " inside");
    }

    -- Someone walked out.
    on Exited(entity who) {
        World:say(who:name() + " left");
    }
}
pass Door;
end
```

```rust
use reflect::Value;
use script::{Context, Event, RustComponent};

pub struct Door;

impl RustComponent for Door {
    fn handle(&mut self, cx: &mut Context<'_>, event: &Event) -> bool {
        // The zone's events carry who walked in or out.
        let Some(Value::Entity(Some(who))) = event.values.first() else {
            return false;
        };
        let name = cx.host().name(*who).unwrap_or_default();
        match event.name.as_str() {
            // Someone walked in: say who, and how many are inside now.
            "Entered" => {
                let me = cx.me();
                let count = cx.host().inside(me).len();
                cx.host().say(&format!("{name} came in; {count} inside"));
            }
            // Someone walked out.
            "Exited" => cx.host().say(&format!("{name} left")),
            _ => return false,
        }
        true
    }

    // values() and set(): as Mover's in Game code, under Components.
}
```

```lua
local Door = component("Door", {})

-- Someone walked in: say who, and how many are inside now.
function Door:on_Entered(who)
    local count = #solis.entity.inside(self.me)
    solis.World:say(solis.entity.name(who) .. " came in; " .. count .. " inside")
end

-- Someone walked out.
function Door:on_Exited(who)
    solis.World:say(solis.entity.name(who) .. " left")
end
```

The one who walked in hears its own pair, which is handy for the player's own code (a "you found the shop" sign):

```lust
-- player.lst: the player notices the zones it walks through.
fetch "solis/world":World;
fetch "solis/zones":[EnteredZone, LeftZone];

component Player {
    -- Walked into a zone: which one.
    on EnteredZone(entity zone) {
        World:say("Now in " + zone:name());
    }

    -- Walked out of it again.
    on LeftZone(entity zone) {
        World:say("Left " + zone:name());
    }
}
pass Player;
end
```

```rust
use reflect::Value;
use script::{Context, Event, RustComponent};

pub struct Player;

impl RustComponent for Player {
    fn handle(&mut self, cx: &mut Context<'_>, event: &Event) -> bool {
        // These carry the zone.
        let Some(Value::Entity(Some(zone))) = event.values.first() else {
            return false;
        };
        let name = cx.host().name(*zone).unwrap_or_default();
        match event.name.as_str() {
            // Walked into a zone: which one.
            "EnteredZone" => cx.host().say(&format!("Now in {name}")),
            // Walked out of it again.
            "LeftZone" => cx.host().say(&format!("Left {name}")),
            _ => return false,
        }
        true
    }

    // values() and set(): as Mover's in Game code, under Components.
}
```

```lua
local Player = component("Player", {})

-- Walked into a zone: which one.
function Player:on_EnteredZone(zone)
    solis.World:say("Now in " .. solis.entity.name(zone))
end

-- Walked out of it again.
function Player:on_LeftZone(zone)
    solis.World:say("Left " .. solis.entity.name(zone))
end
```

**The rules, exactly:**
- **Only where things are at each tick counts.** Something that goes in and out within one tick (a teleport straight through, a very fast bullet) fires nothing.
- **Several at once come in order:** two walking in during the same tick arrive in the order of their IDs, the same on any computer and any number of cores.
- **Taken out while inside:** something despawned inside a zone counts as leaving, so the zone hears `Exited`. A zone that's despawned says nothing.
- **Saved:** who is inside, and whether a **Once** zone has fired, are kept in save games and in the game's checksum, so a loaded game carries on exactly.
- A zone never counts itself, even if its entity is a character.

## Custom code on a zone
A zone is an entity like any other, so it can carry your own components (**+ Component** in Properties), in Lust, Lua or Rust. **New code for this zone…** is the quick way: it makes the file and adds the component in one step (one step to undo, too). See [Game code](game-code.md) for components and events.

## Making one a prefab
A zone can be made into a [prefab](prefabs.md) like any entity: right-click it in the Level panel and make a prefab, and every copy has the same shape, settings and code. A "spike pit" prefab with its own `Entered` code, placed ten times, is ten pits that each know who fell in.

## Shortcuts
| Action | Shortcut |
|---|---|
| Freeform: place a point while drawing | click |
| Freeform: close the shape | click the first point, or `Enter` |
| Freeform: give up drawing | `Esc` |
| Edit a freeform zone's points | `Tab` (with the zone picked) |
| Move a point | drag it |
| Add a point on an edge | `Ctrl + click` |
| Remove the picked point | `Delete` |
| Finish editing points | `Tab` or `Esc` |

## Tips and common problems
- **Problem:** nothing is heard when the player walks in. **Why:** the player isn't a character or a moving body (no CharacterBody, Collider2D ticked Character, or dynamic RigidBody), or the zone has no game code. **Fix:** check the player's components, and use **New code for this zone…**.
- **Problem:** `Entered` comes once and never again. **Why:** **Fires** is **Once**. **Fix:** set it to **Every time**.
- **Problem:** a fast teleport through a zone isn't heard. **Why:** only where things are at each tick counts. **Fix:** make the zone bigger, or check in the teleport's own code.
- **Problem:** a freeform change is refused. **Why:** it would make the outline cross itself, or leave fewer than 3 or more than 256 points. **Fix:** move the point somewhere the outline stays one loop.

## Related
- [Concepts](concepts.md#game-code): trigger zone, event.
- [Game code](game-code.md): components and events.
- [Physics](physics.md) and [Walls and walking in 2D](walls-and-walking.md): characters and bodies.
- [Save games](save-games.md): what's kept.
- [Prefabs](prefabs.md).
