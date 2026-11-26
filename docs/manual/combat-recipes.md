# Combat recipes

> Ready-to-copy Lust for a 2D action game: a sword whose hit lands on one frame of its swing, knockback, bullets that hit, sparks and muzzle flashes, and music that changes when a fight starts.

## When to use it
When your platformer gets enemies to fight. Each recipe is a whole file to copy into your project and fetch from `main.lst`. They build on [Platformer recipes](platformer-recipes.md) and on Solis's own parts: [frame events](sprite-animation.md), [tags](tags.md), [events](game-code.md#events), [projectiles](game-code.md#projectiles) and [modules](modules.md).

Why recipes? How a sword hits and what a hit does is the heart of a game, and different in each. Written in your own code, it's yours to change. [Can I make this?](can-i-make-this.md) lists what's a recipe and what's built in.

## How to use it

### A sword that hits on one frame of its swing
A swing hurts only on the frame where the blade is out, not for the whole animation. Mark that frame with a **frame event** on the attack clip (in the Frames tab, see [Animating sprites](sprite-animation.md)); here it's called `hoeHits`. When the clip reaches it, Solis sends `CueReached`, and the component looks for enemies in front of the hero with `World:within` and the **Enemy** [tag](tags.md), and sends each a `Hurt`.

```lust
-- sword.lst: swings on Fire, and hurts enemies in front on the swing's hit frame.
fetch "solis/world":World;
fetch "solis/input":Input;
fetch "solis/project":[Action, Clip, Cue, Tag];
fetch "solis/animation":[CueReached, ClipFinished, Flip];

event Hurt(float amount, vec3 from);

component Sword {
    has {
        Clip animation = Clip.HeroIdle; -- [hidden]
        Flip flip = Flip.None; -- [hidden]
        float reach = 0.9; -- [unit "m", tip "How far in front the blade reaches."]
        float damage = 1;
        bool swinging = false; -- [hidden]
    }

    method update(float dt) {
        is Input:pressed(Action.Fire) && not swinging? then {
            swinging = true;
            animation = Clip.FarmerHoe;
            restartAnimation();
        }
    }

    on {
        CueReached(Cue cue) {
            is cue == Cue.HoeHits? then {
                -- The middle of the blade's reach, on the side the hero faces.
                float side = 1;
                is flip == Flip.Across? then {
                    side = -1;
                }
                vec3 blade = position() + [side * reach * 0.5, 0.5, 0];
                loop (entity enemy in World:within(Tag.Enemy, blade, reach * 0.6)) {
                    send Hurt(damage, position()) to enemy;
                }
            }
        }

        ClipFinished(Clip clip) {
            is clip == Clip.FarmerHoe? then {
                swinging = false;
                animation = Clip.HeroIdle;
            }
        }
    }
}

pass Sword;
end
```

**Why a frame event?** The animation and the hit stay in step: speed the clip up and the hit comes sooner, by itself. A box that's switched on for a few frames would need its timing kept by hand.

### Getting hurt: health and knockback
An enemy that's hit loses health, is knocked away from whoever hit it, and is removed when its health is gone. The knockback is a speed that fades, added to its own walking.

```lust
-- enemy.lst: an enemy with health that's knocked back when hurt.
event Hurt(float amount, vec3 from);

component Enemy {
    has {
        float health = 3;
        float knockback = 8; -- [unit "m/s", tip "How hard a hit knocks it away."]
        vec3 knock = [0, 0, 0]; -- [hidden]
        float fall = 0; -- [hidden]
    }

    on {
        Hurt(float amount, vec3 from) {
            health = health - amount;
            is health <= 0? then {
                despawn();
                return;
            }
            -- Away from the hit, and a little up.
            float side = Maths:sign(position().x - from.x);
            knock = [side * knockback, 0, 0];
            fall = knockback * 0.5;
        }
    }

    method update(float dt) {
        is grounded() && fall < 0? then {
            fall = -1;
        }
        fall = fall - 35 * dt;
        -- The knock fades in about a fifth of a second.
        knock = knock * Maths:max(0, 1 - 5 * dt);
        walk([knock.x * dt, fall * dt, 0]);
    }
}

pass Enemy;
end
```

Give the enemy's entity the **Enemy** tag (the chips under its name in Properties) so the sword finds it, and a `Collider2D` with **Character** ticked so `walk` works. For a hit that lands harder, add the hit-stop from [Platformer recipes](platformer-recipes.md#hit-stop-the-game-stops-for-a-blink-on-a-big-hit).

### Bullets that hit
[Projectiles](game-code.md#projectiles) fly in their thousands but hit nothing, so a bullet that hurts is a small [module](modules.md): an entity with a sprite and this component, made with `new` when the gun fires. It flies straight, hurts the first enemy it reaches, and is gone after a second.

```lust
-- bullet.lst: flies straight and hurts the first enemy it reaches.
fetch "solis/world":World;
fetch "solis/project":Tag;

event Launch(vec3 way);
event Hurt(float amount, vec3 from);

component Bullet {
    has {
        float speed = 18; -- [unit "m/s"]
        float damage = 1;
        float life = 1; -- [unit "s"]
        vec3 way = [0, 0, 0]; -- [hidden]
    }

    on {
        Launch(vec3 to) {
            way = to:normalized();
        }
    }

    method update(float dt) {
        life = life - dt;
        is life <= 0? then {
            despawn();
            return;
        }
        moveTo(position() + way * speed * dt);
        entity hit = World:nearest(Tag.Enemy, position());
        is hit != nothing && hit:position():distance(position()) < 0.4? then {
            send Hurt(damage, position()) to hit;
            despawn();
        }
    }
}

pass Bullet;
end
```

The gun aims with the right stick when it's pushed, and at the mouse otherwise ([Aiming with the mouse](input.md)). Save an entity with the bullet's sprite and `Bullet` as a module called `bolt` (right-click it in the Level panel, **Make module…**). The gun makes one and tells it which way to fly:

```lust
-- gun.lst: fires a bullet on Fire, the way the player aims, with a muzzle flash.
fetch "solis/world":World;
fetch "solis/input":Input;
fetch "solis/project":Action;
fetch "solis/made":[Bolt, Shot];

event Launch(vec3 way);

component Gun {
    has {
        int cooldown = 0; -- [hidden]
    }

    method update(float dt) {
        -- Show the pointer to aim with.
        is not Input.pointerShown? then {
            Input.pointerShown = true;
        }
        cooldown = cooldown - 1;
        -- Aim with the right stick, or else at the mouse.
        vec3 aim = Input:axis(Action.Look);
        is aim:length() < 0.2? then {
            aim = Input:pointer() - position();
        }
        is aim:length() < 0.01? then {
            aim = [1, 0, 0];
        }
        is Input:held(Action.Fire) && cooldown <= 0? then {
            vec3 muzzle = position() + aim:normalized() * 0.6;
            entity shot = new Bolt { position = muzzle };
            send Launch(aim) to shot;
            -- A muzzle flash: three short bright streaks.
            loop 3 {
                vec3 spread = aim:normalized() + [World:randomRange(-0.3, 0.3), World:randomRange(-0.3, 0.3), 0];
                new Shot { from = muzzle, direction = spread, speed = 12, life = 0.05, colour = [4, 2.5, 0.8] };
            }
            cooldown = 8;
        }
    }
}

pass Gun;
end
```

A dozen bullets at once is no work at all. Hundreds at once would want a pool of bullets kept and reused; tell us if your game gets there.

### Sparks
Sparks where a sword or bullet hits are a burst of short [projectiles](game-code.md#projectiles), flying out every way. Put this on anything that can be hurt, beside its health.

```lust
-- sparks.lst: a burst of sparks where it's hurt.
fetch "solis/world":World;
fetch "solis/made":Shot;

event Hurt(float amount, vec3 from);

component Sparks {
    has {
        int count = 10;
        float speed = 6; -- [unit "m/s"]
        vec3 colour = [4, 2, 0.5]; -- [tip "Red, green and blue; above 1 glows."]
    }

    on {
        Hurt(float amount, vec3 from) {
            vec3 at = position() + [0, 0.5, 0];
            loop count {
                float angle = World:randomRange(0, 6.2832);
                vec3 way = [Maths:cos(angle), Maths:sin(angle), 0];
                new Shot { from = at, direction = way, speed = speed * World:randomRange(0.5, 1), life = World:randomRange(0.08, 0.2), colour = colour };
            }
        }
    }
}

pass Sparks;
end
```

Sparks made this way fly straight and vanish; they don't fall or fade. Smoke, dust, rain and snow need a particle emitter, which comes later ([Can I make this?](can-i-make-this.md)).

### Enemies that patrol and turn at edges
An enemy walks one way until there's a wall in front of it or no ground ahead, then turns round. Two short rays answer both: one along the way it walks, one down from just in front of its feet. Give it a `Collider2D` ticked **Character**, about the size of its sprite.

```lust
-- walker.lst: walks back and forth, turning at walls and ledges.
fetch "solis/physics":[Physics, RayHit];
fetch "solis/animation":Flip;

component Walker {
    has {
        float speed = 2; -- [unit "m/s"]
        float side = 1; -- [hidden]
        float fall = 0; -- [hidden]
        Flip flip = Flip.None; -- [hidden]
    }

    method update(float dt) {
        is grounded()? then {
            fall = -1;
            vec3 middle = position() + [0, 0.5, 0];
            vec3 way = [side, 0, 0];
            -- A wall within half a metre, or no ground just ahead: turn.
            RayHit wall = Physics:castRay(middle, way, 0.5);
            RayHit ground = Physics:castRay(middle + way * 0.5, [0, -1, 0], 1);
            is wall.found || not ground.found? then {
                side = -side;
            }
        }
        fall = fall - 35 * dt;
        is side < 0? then {
            flip = Flip.Across;
        } or {
            flip = Flip.None;
        }
        walk([side * speed * dt, fall * dt, 0]);
    }
}

pass Walker;
end
```

The rays start half a metre up, from the enemy's middle, so they pass out of its own collider and don't meet the floor it stands on. For a bigger enemy, look further ahead and further down.

### Music that follows the fight
Play the calm music on an entity, and when an enemy comes near the player, swap it for the battle music. `stopSounds()` stops the old with a short fade. Tick **Loop** on both sound events.

```lust
-- music.lst: calm music, and battle music while an enemy is near the player.
fetch "solis/world":World;
fetch "solis/project":Tag;
fetch "solis/made":[Theme, Battle];

component Music {
    has {
        entity player = nothing;
        float near = 8; -- [unit "m", tip "How close an enemy is for the battle music."]
        bool started = false; -- [hidden]
        bool fighting = false; -- [hidden]
    }

    method update(float dt) {
        is player == nothing? then {
            return;
        }
        bool danger = #World:within(Tag.Enemy, player:position(), near) > 0;
        is not started || danger != fighting? then {
            started = true;
            fighting = danger;
            stopSounds();
            is fighting? then {
                new Battle {};
            } or {
                new Theme {};
            }
        }
    }
}

pass Music;
end
```

## Settings
The recipes' settings are their fields, shown in Properties with their tips once the component is on an entity.

## Tips and common problems
- **Problem:** the sword never hits. **Why:** no frame event on the attack clip, or the enemy lacks the Enemy tag. **Fix:** add the frame event in the Frames tab, and the tag under the enemy's name.
- **Problem:** bullets fly through walls. **Why:** the bullet only looks for enemies. **Fix:** look ahead with a ray each tick and stop at what it meets: `RayHit hit = Physics:castRay(position(), way, speed * dt);` then `is hit.found? then { despawn(); }` ([Rays in 2D](walls-and-walking.md#rays-in-2d)).
- **Problem:** the music restarts again and again. **Why:** an enemy is right on the edge of `near`. **Fix:** make the calm music wait until no enemy is within a little more than `near`.

## Related
- [Platformer recipes](platformer-recipes.md): the jump, hit-stop, the hurt flash and paths.
- [Can I make this?](can-i-make-this.md): what each kind of game needs, and what's ready.
- [Tags](tags.md), [Animating sprites](sprite-animation.md), [Modules](modules.md), [Sound events](sound-events.md).
