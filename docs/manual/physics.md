# Physics

> Make things fall, collide, stack and get pushed, the same way on every computer.

## When to use it
Whenever something in your game should move like a real object without you writing how: a crate that tips off a ledge, a ball that bounces, a door that swings shut and knocks things out of the way, or a floor and walls that characters and objects can't pass through.

Not everything needs it. [Projectiles](game-code.md#projectiles) (lasers, bullets) fly in straight lines without physics, which is why a million of them are cheap. And a unit that only slides across a map is simpler to move yourself with `moveTo`.

Solis's physics is **strict**: the same game, with the same inputs, comes out exactly the same on every computer, to the last decimal ([determinism](concepts.md#simulation)). Recordings play back the same anywhere, and online strategy games can keep every player's game in step. It costs physics a little speed, which you won't notice at the sizes Solis aims for.

## How to use it

**A floor**
1. Select the entity that should be the floor (an imported model, or **Entity > Add empty** placed where the floor goes).
2. In **Properties**, choose **Add component > Collider**. A green outline shows its shape in the viewport while it's selected.
3. Set its **Shape** and **Size** to cover the floor. A box 20 × 20 × 1 m is a good start; its middle sits at the entity, so lower the entity half a metre to put the top at its feet, or set **Centre** to `0 0 -0.5`.

An entity with a collider and nothing else never moves: floors, walls, rocks.

**A crate that falls**
1. Select the crate and add a **Collider** sized like it (a 1 m crate: Box, size `1 1 1`).
2. Add a **RigidBody** too. Its **Kind** is **Dynamic**: physics moves it.
3. Press **Play**. The crate falls onto the floor and settles. Press **Stop** and it's back where it was.

**Moving things yourself**
- A **Kinematic** body is moved only by your game code (`moveTo`, `turnTo`), and pushes dynamic things out of its way without being pushed back: lifts, moving platforms, doors. A character standing on one is carried along.
- A player's character is better with a **CharacterBody**, which climbs steps, slides along walls and walks up slopes: see [Characters and cameras](characters-and-cameras.md).
- A **Fixed** body never moves, like a collider on its own.
- You can still `moveTo` a dynamic body to put it somewhere new (say, back on its shelf); physics carries on from there.

**From game code** (Lust; in Lua they're `solis.push`, `solis.velocityOf` and so on):

```
get "solis/world":[find, say];
get "solis/entity":[positionOf, nameOf];
get "solis/physics":[push, castRay, castRayHit];

component Kicker {
    has {
        int kicked = 0;
    }

    method update(float dt) {
        entity ball = find("Ball");
        is kicked == 0? then {
            push(ball, [5, 0, 2]);
            kicked = 1;
        }
        // Is anything below me, within 2 metres?
        is castRay(positionOf(me), [0, 0, -1], 2) >= 0? then {
            say(nameOf(castRayHit(positionOf(me), [0, 0, -1], 2)) + " is below me");
        }
    }
}

pass Kicker;
end
```

| Method | What it does |
|---|---|
| `push(entity which, vec3 impulse)` | A push, like a kick: a heavy body moves less than a light one. `[5, 0, 0]` gives a 1 kg crate 5 m/s along X. |
| `vec3 velocityOf(entity which)` | How fast and which way it's moving, metres a second. |
| `setVelocity(entity which, vec3 velocity)` | Makes it move this fast, this way, whatever its weight. |
| `float castRay(vec3 from, vec3 direction, float reach)` | Looks along a line: how far away the first collider is, or -1 if there's nothing within reach. |
| `entity castRayHit(vec3 from, vec3 direction, float reach)` | The entity it meets first, or nothing. |
| `vec3 castRayNormal(vec3 from, vec3 direction, float reach)` | Which way that surface faces: `[0, 0, 1]` for a floor. |

Like moves, pushes and velocities land when every `update` in the tick is done, then physics moves everything on once ([what game code sees](game-code.md#components)). A ray sees the world as the tick started. A ray that starts inside a collider (your own character's, say) passes out of it and finds what's beyond.

## Settings
**RigidBody**

| Setting | What it does |
|---|---|
| Kind | **Dynamic**: moved by physics. **Kinematic**: moved by game code, pushing dynamic things. **Fixed**: never moves. |
| Mass | How heavy, in kilograms. 0 works it out from the collider's size: a kilogram for each cubic metre (a 1 m box weighs 1 kg). Give real things their real weight: a crate 20 kg, a person 75 kg. |
| Damping | How quickly it slows down by itself, like air resistance. 0 never; 1 or 2 feels like moving through water. |
| Turn damping | How quickly it stops spinning by itself. |
| Gravity | How much gravity pulls it: 1 as normal, 0 floats, below 0 rises. |
| Stays upright | It never tips over or spins: for characters and things that should stand. |
| Fast | Checked between ticks too, so it can't pass through thin walls when it moves fast (thrown things). Costs a little more. |

**Collider**

| Setting | What it does |
|---|---|
| Shape | **Box**, **Sphere**, or **Capsule** (a cylinder with rounded ends, standing up: the usual shape for a character). |
| Size | A box's size along X, Y and Z, in metres. |
| Radius | A sphere's or capsule's radius. |
| Height | A capsule's height, end to end. |
| Centre | Where the shape's middle is, from the entity's origin. Raise a character's capsule by half its height so its feet are at the origin. |
| Friction | How much it grips: 0 ice, 0.5 most things, 1 rubber. |
| Bounciness | How much it bounces back: 0 not at all, 1 keeps all its speed. |

Shapes grow with the entity's **Scale**.

**CharacterBody** is described in [Characters and cameras](characters-and-cameras.md). It takes the place of a RigidBody and Collider on the same entity, and doesn't grow with Scale.

## Shortcuts
Physics adds no keys. In the [Console](console.md), `seed` and `checksum` help check that two plays come out the same.

## Tips and common problems
- **Problem:** my crate falls through the floor. **Why:** the floor has no Collider, or it's smaller than it looks (check the green outline with the floor selected). **Fix:** add a Collider to the floor and size it.
- **Problem:** something small and fast passes through a thin wall. **Why:** between two ticks it went from one side to the other. **Fix:** tick **Fast** on its RigidBody, or make the wall thicker.
- **Problem:** a dynamic body doesn't move when I push it. **Why:** it's heavy for the push (a push of 5 moves a 1 kg box at 5 m/s, but a 100 kg one at only 0.05 m/s), it's resting on something with high friction, or it's Kinematic or Fixed. **Fix:** push harder, or make it Dynamic.
- **Problem:** my character tips over. **Why:** physics turns it when it bumps into things. **Fix:** give it a **CharacterBody** instead of a RigidBody ([Characters and cameras](characters-and-cameras.md)), or tick **Stays upright**.
- **Problem:** the Console says the game has the most physics bodies it can. **Why:** at most 100,000 entities take part in physics at once. **Fix:** use fewer bodies; things that never move can share one big collider.
- **Problem:** the game paused, saying physics took too long. **Why:** one tick of physics took more than a quarter of a second. **Fix:** fewer bodies, or simpler shapes.

## Related
- [Game code](game-code.md)
- [Play mode](play-mode.md)
- [Entities and components](entities.md)
- [Characters and cameras](characters-and-cameras.md)
- Concepts: [physics, rigid body, collider, determinism](concepts.md#simulation)
