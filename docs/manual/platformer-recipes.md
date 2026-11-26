# Platformer recipes

> Ready-to-copy Lust for what a platformer needs that is game code rather than an engine feature: a jump that feels right, wall jumps, hit-stop, a hurt flash, and things that follow a path.

## When to use it
When your hero from [your first platformer](first-platformer.md) runs and jumps, and you want it to feel like a real platformer. Each recipe is a whole file: copy it into your project, fetch it from `main.lst`, and add its component to an entity, as you did with `Hero`.

Why recipes and not settings? Every platformer wants its jump a little differently. A few lines you can read and change give you more say than a box of settings, and they cost the engine nothing. [Can I make this?](can-i-make-this.md) lists what's a recipe and what's built in.

## How to use it

### A jump that feels right
This hero replaces the one from [your first platformer](first-platformer.md). It adds the four things players feel without knowing their names:
- **A jump set by height and time.** You say how high a full jump goes (`jumpHeight`, in metres) and how long it takes to get there (`jumpTime`, in seconds); the code works out gravity and the jump's speed. Changing one never spoils the other.
- **Coyote time.** For a moment after running off a ledge (`coyoteTime`), Jump still works, so a jump pressed a hair late isn't lost.
- **Jump buffering.** Jump pressed a moment before landing (`bufferTime`) jumps as soon as the hero lands.
- **A shorter jump when Jump is let go early,** so a tap hops and a hold leaps.
- **Wall jumps.** In the air, pushed against a wall, Jump kicks the hero up and away from it.

```lust
-- hero.lst: a platformer hero with a jump set by height and time, coyote time, jump buffering, short hops and wall jumps.
fetch "solis/input":Input;
fetch "solis/project":Action;

component Hero {
    has {
        float speed = 6; -- [unit "m/s"]
        float jumpHeight = 3; -- [unit "m", tip "How high a full jump goes."]
        float jumpTime = 0.4; -- [unit "s", tip "How long a full jump takes to reach the top."]
        float coyoteTime = 0.1; -- [unit "s", tip "How long after leaving a ledge Jump still works."]
        float bufferTime = 0.12; -- [unit "s", tip "How long before landing a Jump is remembered."]
        float wallKick = 7; -- [unit "m/s", tip "How hard a wall jump pushes away from the wall."]
        float fall = 0; -- [hidden]
        float kick = 0; -- [hidden]
        float sinceGround = 99; -- [hidden]
        float sinceJump = 99; -- [hidden]
    }

    method update(float dt) {
        -- From height and time: gravity, and the speed that reaches the top.
        float gravity = 2 * jumpHeight / (jumpTime * jumpTime);
        float jumpSpeed = gravity * jumpTime;
        float push = Input:axis(Action.Move).x;

        sinceGround = sinceGround + dt;
        sinceJump = sinceJump + dt;
        is grounded()? then {
            sinceGround = 0;
            fall = -1;
            kick = 0;
        }
        is Input:pressed(Action.Jump)? then {
            sinceJump = 0;
        }
        -- Bumped its head: start coming down.
        is fall > 0 && velocity().y <= 0? then {
            fall = 0;
        }
        fall = fall - gravity * dt;

        -- Pushing sideways but not moving sideways, in the air: against a wall.
        bool onWall = not grounded() && push != 0 && Maths:abs(velocity().x) < 0.1;
        bool wanted = sinceJump <= bufferTime;
        is wanted && sinceGround <= coyoteTime? then {
            fall = jumpSpeed;
            sinceJump = 99;
            sinceGround = 99;
        } or is onWall && wanted? then {
            fall = jumpSpeed;
            kick = -Maths:sign(push) * wallKick;
            sinceJump = 99;
        }
        -- Let go early: the rest of the way up is cut short.
        is fall > 0 && Input:released(Action.Jump)? then {
            fall = fall * 0.5;
        }
        -- The wall's kick fades over about a quarter of a second.
        kick = Maths:moveTowards(kick, 0, Maths:abs(kick) * dt * 4 + dt);

        walk([(push * speed + kick) * dt, fall * dt, 0]);
    }
}

pass Hero;
end
```

**Tuning it:** with `jumpHeight` 3 and `jumpTime` 0.4 the hero jumps three tiles in under half a second, snappy and not floaty. A longer `jumpTime` with the same height floats more. `coyoteTime` and `bufferTime` around a tenth of a second feel fair without being noticed; at 0 they're off.

### Hit-stop: the game stops for a blink on a big hit
A hit lands harder when everything freezes for a few frames. `World.gameSpeed = 0` freezes time, physics and animation, but components still run, so a component can count the frozen frames and start the game again. Send `Freeze` to the entity that has it, from whatever lands the hit.

```lust
-- hitstop.lst: freezes the game for a few frames when a big hit lands.
fetch "solis/world":World;

event Freeze(int frames);

component HitStop {
    has {
        int left = 0; -- [hidden]
    }

    on Freeze(int frames) {
        left = frames;
        World.gameSpeed = 0;
    }

    method update(float dt) {
        -- While frozen, dt is 0 but update still runs every frame.
        is left > 0? then {
            left = left - 1;
            is left == 0? then {
                World.gameSpeed = 1;
            }
        }
    }
}

pass HitStop;
end
```

Four to six frames is a solid hit; more than ten feels like the game stuck. Put `HitStop` on one entity (the camera is a good home) and `send Freeze(5) to camera;` from the attack.

### A hurt flash
A hero that blinks after being hit tells the player they're safe for a moment. The component's `bool visible` field shows and hides its entity ([Hiding entities](game-code.md#hiding-entities-and-turning-colliders-off)).

```lust
-- blink.lst: blinks its entity for a moment after it's hurt.
event Hurt(float amount, vec3 from);

component Blink {
    has {
        bool visible = true; -- [hidden]
        float seconds = 1; -- [unit "s", tip "How long it blinks."]
        int frames = 0; -- [hidden]
    }

    on Hurt(float amount, vec3 from) {
        frames = Maths:round(seconds * 60);
    }

    method update(float dt) {
        is frames > 0? then {
            frames = frames - 1;
            -- Shown four frames, hidden four.
            visible = frames == 0 || (frames / 4) % 2 == 0;
        }
    }
}

pass Blink;
end
```

### Following a path: moving platforms, patrolling enemies
Mark the stops with empty entities (**Home › Empty**), named so you can find them, and point the follower's fields at them in Properties. A follower with a solid `Collider2D` (not a character) is a moving platform: characters can stand on it.

```lust
-- patrol.lst: goes back and forth between two places.
component Patrol {
    has {
        entity from = nothing; -- [tip "Where it starts: an empty entity."]
        entity to = nothing; -- [tip "Where it turns back: an empty entity."]
        float speed = 2; -- [unit "m/s"]
        bool back = false; -- [hidden]
    }

    method update(float dt) {
        is from == nothing || to == nothing? then {
            return;
        }
        vec3 goal = to:position();
        is back? then {
            goal = from:position();
        }
        vec3 way = goal - position();
        float step = speed * dt;
        is way:length() <= step? then {
            moveTo(goal);
            back = not back;
        } or {
            moveTo(position() + way:normalized() * step);
        }
    }
}

pass Patrol;
end
```

For a path of more stops, give it more `entity` fields (`stop1`, `stop2`, `stop3`) and a number saying which it heads for. For an enemy, use `walk` instead of `moveTo` so it stops at walls.

## Settings
The recipes' settings are their fields, shown in Properties with their tips once the component is on an entity.

## Tips and common problems
- **Problem:** the hero can't wall jump. **Why:** it only counts as against a wall while you push towards it and it isn't moving sideways. **Fix:** hold the direction of the wall as you press Jump.
- **Problem:** after a hit-stop the game stays frozen. **Why:** the entity with `HitStop` was removed while frozen, so nothing counted down. **Fix:** keep `HitStop` on something that stays, such as the camera.
- **Problem:** the hero slides off a moving platform. **Why:** standing on a moving platform doesn't carry a character along yet. See [Can I make this?](can-i-make-this.md).

## Related
- [Your first platformer](first-platformer.md), which this builds on.
- [Can I make this?](can-i-make-this.md): what each kind of game needs, and what's ready.
- [Combat recipes](combat-recipes.md): sword hits, knockback, bullets and sparks.
- [Game code](game-code.md), [Events](game-code.md#events) and [field hints](game-code.md#field-hints-and-the-projects-names).
