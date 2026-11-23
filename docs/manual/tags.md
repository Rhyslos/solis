# Tags

> Labels you give entities, such as Enemy or Collectible, so game code can find every one of a kind: "all enemies within 3 m", "the nearest collectible".

## When to use it
Whenever game code needs to find things by **what they are** rather than by name: a guard looking for the nearest enemy, a magnet pulling in every coin within reach, a door that opens only for the player, a turret that ignores anything flying. A [tag](concepts.md) is a label an entity carries; an entity can carry several (a bat can be both Enemy and Flying), and many entities can carry the same one.

Why tags and not names? A name belongs to one entity, and `World:find("Bat 12")` finds that one. A tag belongs to a **kind**, so code written once works for every enemy you'll ever place, copy or spawn. And because the project keeps its own list of tags, game code names each as `Tag.Enemy`: a misspelt tag (`Tag.Enmey`) is a mistake in Problems before the game runs, not a question that quietly finds nothing.

> **Good to know:** to ask "which of these have a Health?", you can already ask by component. Tags are for kinds you decide yourself, whatever components the entities have.

## How to use it

### Make the project's tags
1. Click the **File** tab, then **Project settings**, then the **Tags** sub-tab.
2. Click **+ New tag**, type its name, and press `Enter` to keep it (`Esc` gives it up). A tag's name is what game code calls it, so it starts with a capital letter and holds only letters, digits and `_`: **Enemy**, **Collectible**, **BigDoor**. A name that can't be one says why beside it, and suggests one that can.
3. Each tag's row says what uses it: "Used by 14 entities and 2 modules" (entities in all the project's levels, copies of modules among them).

**Renaming:** double-click a tag's name (or ⋯ › **Rename**), type the new one and press `Enter`. It's renamed **everywhere** at once: every level and module keeps its tags by an ID that never changes, so they all follow, and game code's name changes with it. Code still saying the old name (`Tag.Enemy` after renaming it **Monster**) becomes a mistake in [Problems](problems.md), pointing at each line to change.

**Deleting:** ⋯ › **Delete**. A tag nothing uses goes at once. One that's used asks first, saying what carries it (which levels' entities, which modules): deleting it takes it off all of them.

The tags are kept in the project's `tags.lst`, a small text file beside `project.solis` that Git can merge. If it changes on disk (a pull, an edit by hand), Solis reads it again. In the [Library](projects-and-the-library.md) it's shown as **Project's tags**, with a gear: a double-click opens Project settings › Tags, never a code editor.

### Tag entities
1. Pick an entity. Its tags are under its name in **Properties**, as chips.
2. Click **+ Tag ▾**: a list of the project's tags opens, with a box at the top to search it. Click one to give it, or type a name and click **New tag "…"** (or press `Enter`) to make a new tag and give it in one go.
3. A chip's **×** takes that tag off.

Each change is one step to [undo](undo.md). With several entities picked, Properties shows the tags they all carry, and a change applies to each of them.

The [Level panel](interface-tour.md) shows each entity's tags faintly after its name ("Bat  Enemy · Flying"), and its search finds entities by tag as well as by name: search *enemy* to see every enemy in the level.

### Tags on modules
Tags set on a [module](modules.md) are on **every copy** of it: tag the Bat module Enemy, and every bat placed or spawned is an enemy. A copy may add a tag of its own or take one away: its Tags row then has the orange dot (it differs from its module) and ↶, which puts the module's tags back. As with any setting, the copy's own change wins: if the module's tags change later, a copy that changed its own keeps them, and the others follow.

### Ask the world in game code
Three questions, and two changes:

| Call | What it gives |
|---|---|
| `hasTag(Tag.Enemy)` | Whether the entity carries the tag (`is target:hasTag(Tag.Enemy)?`). |
| `World:within(Tag.Enemy, place, 3)` | Every entity carrying the tag within 3 m of `place`, **nearest first**; two at the same distance come in the order of their IDs. An empty list when there are none. |
| `World:nearest(Tag.Enemy, place)` | The nearest entity carrying the tag, or `nothing` when none does. |
| `addTag(Tag.Enemy)`, `removeTag(Tag.Enemy)` | Give the entity the tag, or take it off. |

They're answered from where everything was **as the tick started**, the same on every core, so a game comes out the same however many cores run it. For the same reason, a tag added or taken off lands at the **end of the tick**, as moves do: `hasTag` sees it from the next tick. Calling `addTag` for a tag already carried changes nothing, so it's safe every tick.

```lust
-- guard.lst: a guard that watches for enemies.
fetch "solis/world":World;
fetch "solis/project":Tag;

component Guard {
    has {
        float reach = 3;
    }

    method update(float dt) {
        -- The nearest enemy, wherever it is.
        entity closest = World:nearest(Tag.Enemy, position());
        is closest != nothing? then {
            World:say(name() + " watches " + closest:name());
        }

        -- Every enemy within reach, nearest first.
        entity array near = World:within(Tag.Enemy, position(), reach);
        int count = #near;
        is count > 0? then {
            World:say(count.text() + " enemies within reach");
        }

        -- Only those that don't fly can be caught: they stop being enemies.
        loop (entity enemy in near) {
            is not enemy:hasTag(Tag.Flying)? then {
                enemy:removeTag(Tag.Enemy);
            }
        }
    }
}
pass Guard;
end
```

```rust
use script::{Context, RustComponent};

pub struct Guard {
    reach: f32,
}

impl RustComponent for Guard {
    fn update(&mut self, cx: &mut Context<'_>, _dt: f64) {
        let me = cx.me();
        let place = cx.host().position(me).unwrap_or_default();

        // The nearest enemy, wherever it is.
        if let Some(closest) = cx.host().nearest("Enemy", place) {
            let name = cx.host().name(me).unwrap_or_default();
            let theirs = cx.host().name(closest).unwrap_or_default();
            cx.host().say(&format!("{name} watches {theirs}"));
        }

        // Every enemy within reach, nearest first.
        let near = cx.host().within("Enemy", place, self.reach);
        if !near.is_empty() {
            cx.host().say(&format!("{} enemies within reach", near.len()));
        }

        // Only those that don't fly can be caught: they stop being enemies.
        for enemy in near {
            if !cx.host().has_tag(enemy, "Flying") {
                cx.host().remove_tag(enemy, "Enemy");
            }
        }
    }

    // values() and set(): as Mover's in Game code, under Components.
}
```

```lua
local Guard = component("Guard", {
    { "reach", "float", 3 },
})

function Guard:update(dt)
    local place = solis.entity.position(self.me)

    -- The nearest enemy, wherever it is.
    local closest = solis.World:nearest("Enemy", place)
    if closest then
        solis.World:say(solis.entity.name(self.me) .. " watches " .. solis.entity.name(closest))
    end

    -- Every enemy within reach, nearest first.
    local near = solis.World:within("Enemy", place, self.reach)
    if #near > 0 then
        solis.World:say(#near .. " enemies within reach")
    end

    -- Only those that don't fly can be caught: they stop being enemies.
    for _, enemy in ipairs(near) do
        if not solis.entity.hasTag(enemy, "Flying") then
            solis.entity.removeTag(enemy, "Enemy")
        end
    end
end
```

In Lust, `Tag` comes from the project's own names (`fetch "solis/project":Tag;`), written again whenever its tags change. In Lua and Rust a tag is its name as text (`"Enemy"`); a name the project hasn't is a tag nothing carries.

### Distance in 2D and 3D
In a **2D project** (Project settings › General › Kind), distance is measured **across the ground**, whatever the layers: a bat drawn on a layer above the slug is still "within 3 m" if it's 3 m away on the map. In a **3D** project, or one of **Both**, distance is measured in 3D: a bird 50 m up isn't within 3 m of a guard standing under it.

> **Good to know:** the questions are quick even with thousands of tagged entities: each tick, Solis sorts each tag's entities into a grid over the ground once, and a question looks only in the cells its distance reaches. 2,000 units each asking every tick cost under a millisecond.

### Zones that notice only some
A [trigger zone](trigger-zones.md)'s **Only tags** limits it to entities carrying at least one of the tags chosen there: a door that opens only for the Player, a pit that catches only Enemies.

## Settings
| Setting | What it does |
|---|---|
| Project settings › Tags | The project's tags: make, rename everywhere, delete. Kept in `tags.lst`. |
| Tags (under an entity's name) | The tags it carries: chips with ×, and **+ Tag ▾**. On a module's copy, the orange dot and ↶ when they differ from the module's. |
| Only tags with several zones picked | The tags they share, by name; nothing when they differ. Pick one zone to change them. |
| Only tags (a trigger zone's card) | The tags a zone notices; none chosen, it notices everything it can. |

## Shortcuts
| Action | Shortcut |
|---|---|
| Rename a tag everywhere | Double-click its name in Project settings › Tags (`Enter` keeps it, `Esc` leaves it) |
| Keep a new tag's name | `Enter` (`Esc` gives it up) |
| In **+ Tag**'s box: give the first tag found, or make the tag typed | `Enter` |

## Tips and common problems
- **Problem:** Problems says `Tag.Enmey` isn't a Tag. **Why:** the project has no tag of that name: a typo, or a tag renamed since. **Fix:** correct the line, or make the tag in Project settings › Tags.
- **Problem:** `hasTag` says no just after `addTag`. **Why:** changes land at the end of the tick, so every core sees the same world during it. **Fix:** nothing to fix: from the next tick it says yes. Keep what you just did in a variable if the same tick needs it.
- **Problem:** a name is refused. **Why:** game code calls a tag `Tag.<name>`, so it must be one Lust can name: a capital letter first, then letters, digits or `_`, not all capitals, and none of Lust's own words. **Fix:** take the suggestion beside it (*big door* becomes **BigDoor**).
- **Problem:** a copy of a module doesn't get a tag the module was just given. **Why:** that copy changed its own tags (it has the orange dot), and a copy's own change wins. **Fix:** click ↶ on its Tags row to follow the module again.

## Related
- [Trigger zones](trigger-zones.md): Only tags.
- [Modules](modules.md): copies and their own changes.
- [Game code](game-code.md): components, the tick, and the project's names.
- [Settings](settings.md): Project settings.
- [Concepts](concepts.md): tag, tick, module.
