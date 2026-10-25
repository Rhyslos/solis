# Your first game: the turret sample

> A small game to read, play and change: a turret shoots at a drone, and the drone's health, armour and sparks react. It's written in all three of Solis's languages, which talk to each other through events.

## When to use it
Right after reading [Game code](game-code.md), or instead of it if you learn best from a working example. Everything in it is ordinary game code you can change while it runs.

## How to use it
1. Choose **Help > Make the sample game…** and pick an empty folder. Solis makes a project there called **Turret sample**, opens it, and opens its area.
2. Press **Play**. The sample has no models yet, so its story is told in the Console (later phases give it models). Watch the Console: every half second the turret fires. The drone's armour dents three times ("Sparks fly from Drone: 2 armour plates left"), and after eight hits the drone is destroyed and drops to the ground.
3. Press **Stop**, then open the code from the Library's **Code** tab.

## What's in it

**The turret** (Lust, in `main.lst`) has a target (the drone, set in Properties), ammo, damage and a cooldown. Each tick, `update` counts the cooldown down. When it reaches zero, it fires: `send Hit(me, damage) to target;`.

```
component Turret {
    has {
        entity target = nothing;
        int ammo = 12;
        float damage = 2.5;
        float cooldown = 0.5;
        float reload = 0;
    }

    method update(float dt) {
        reload = reload - dt;
        is reload <= 0 && ammo > 0? then {
            send Hit(me, damage) to target;
            ammo = ammo - 1;
            reload = cooldown;
        }
    }
}
```

**The drone** carries three components, one in each language. Each reacts to the events it cares about:

| Component | Language | Reacts to | Does |
|---|---|---|---|
| **Health** | Lust (`main.lst`) | `Hit` | Takes the damage; at 0 it sends `Destroyed` to the drone. |
| **Armour** | Rust (`armour.rs`) | `Hit` | Dents a plate, and sends `Dented` with the plates left. |
| **Sparks** | Lua (`sparks.lua`) | `Dented`, `Destroyed` | Says so in the Console; when destroyed, drops the drone to the ground. |

The Lua side, for example:

```lua
function Sparks:on_Dented(platesLeft)
    solis.say("Sparks fly from " .. solis.nameOf(self.me) .. ": " .. platesLeft .. " armour plates left.")
end
```

And the Rust side:

```rust
fn handle(&mut self, cx: &mut Context<'_>, event: &Event) -> bool {
    if event.name != "Hit" || self.plates == 0 {
        return false;
    }
    self.plates -= 1;
    let me = cx.me();
    cx.send(Some(me), "Dented", vec![Value::Int(self.plates)]);
    true
}
```

The events are declared once, in Lust (`event Hit(entity by, float damage);`). Lust checks its own `send`s before the game runs, so a wrongly-sent event in Lust is a mistake in Problems. When Lua or Rust sends one with the wrong values, it isn't sent, and the Console names the component that sent it.

## Things to try
- **While it plays,** change the turret's `damage` to `10` in Properties: the drone goes down in two hits.
- **While it plays,** change `reload = cooldown;` in `main.lst` to `reload = cooldown / 5;` and save. The turret fires five times faster at once, keeping the ammo it has left. (Changing a starting value instead, such as `cooldown = 0.5`, doesn't change a turret that's already running: running components keep their values. Set it in Properties for that.)
- In `sparks.lua`, change what it says and save while it plays: the next dent says the new words.
- Give the drone more armour: select it and set **Plates** to 10.

## Tips and common problems
- **Problem:** I changed `armour.rs` and nothing changed. **Why:** Solis can't build a game's own Rust yet (that comes with building games, in Phase 10). Until then, the Armour built into Solis runs, and the Console says your changes don't. **Fix:** try changes in the Lust and Lua instead.
- **Problem:** "Couldn't make the sample game". **Why:** the folder is already a project, or already has a file where the sample's would go (such as `assets/main.lst`); Solis never replaces your files. **Fix:** choose an empty folder.

## Related
- [Game code](game-code.md): components, events and changing code while it runs, in detail
- [Play mode](play-mode.md)
- Concepts: [component, event, tick](concepts.md#game-code)
