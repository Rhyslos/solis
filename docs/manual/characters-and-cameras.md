# Characters and cameras

> A character that walks, sprints, jumps and climbs, and three cameras (third person, first person and RTS), ready-made, so you can walk around a level before writing any code.

## When to use it
Almost every game has someone to move around and a camera to see them with. Getting these right is fiddly: a character has to climb steps, slide along walls instead of sticking to them, not float off the top of a slope, and ride a moving platform; a camera behind it mustn't end up inside a wall. Solis has them ready:

- **CharacterBody** (a built-in component) does the hard part: it moves a standing capsule as far as it can go, the way game characters move. It's what `moveCharacter` moves.
- **CharacterController** (a ready-made component in Lust) reads the player's input and tells the CharacterBody where to go.
- **ThirdPersonCamera**, **FirstPersonCamera** and **RtsCamera** (ready-made, in Lust) put the [camera](concepts.md#playing) behind the character, in its eyes, or high above a map. **FollowCamera2D** follows the player in a 2D game.

They're written in Lust and their files are in your project's `solis/kit/` folder, so you can read exactly what they do and copy them to make your own. Use them as they are to get going; change a copy when your game needs something different.

For a thing that's pushed around by physics (a ball, a crate) use a [RigidBody](physics.md) instead: a character isn't moved by physics, it moves itself and pushes physics things out of its way.

## How to use it

**Try the playground first.** **Make the playground…** (press **Space** and type it, or use **File > Learn**) makes a new project with a level to walk around in: steps, a gentle ramp and one too steep to climb, crates to push, a moving platform to ride, and a character with the third-person camera. Press **Play**, click the **Game panel**, and walk with **W A S D**, look with the **mouse**, jump with **Space**, sprint with **Shift**. **F6** saves the game and **F7** loads it. **Esc** gives the editor the mouse back.

It has sounds too ([Sound events](sound-events.md)): footsteps as the character walks (faster when it runs, made by the playground's `Footsteps` code), music, a radio on a crate whose chatter makes the music dip as you come near (the [Mixer](mixer.md)'s Voice ducking Music), and a cave with a drip, where every step echoes (a [reverb zone](reverb-zones.md)). The sounds are Solis's own, free to use for anything.

**A character in your own area**
1. Add an entity for the character (**Home > Empty**, which adds an empty one, or a model of a person) with its feet where it should stand.
2. In **Properties**, choose **Add component > CharacterBody**. A green capsule shows its shape when it's selected. Its feet are at the entity's origin.
3. Make sure there's ground under it: an entity with a **Collider**, see [Physics](physics.md).
4. In `main.lst`, fetch the ready-made components:
   ```
   fetch "solis/kit/character":[CharacterController];
   fetch "solis/kit/cameras":[ThirdPersonCamera];
   end
   ```
   (For a 2D game, fetch `FollowCamera2D` and the kit's `TopDownController` instead: see [Tile maps](tile-maps.md).)
5. Add **CharacterController** to the character. Set its **camera** to the camera entity (next step).
6. On the entity with the **Camera** component, add **ThirdPersonCamera** and set its **target** to the character.
7. Press **Play** and click the Game panel.

**The other cameras**
- **FirstPersonCamera**: set its **body** to the character. It sits at the eyes and turns the character with it, so untick the CharacterController's **facesWhereItWalks** (it would turn the other way).
- **RtsCamera**: needs no character. It looks down on the map from above, as strategy games do.
- **FollowCamera2D**, for a 2D game (a camera whose **View** is 2D: [2D: pixel-perfect](2d-pixel-perfect.md)): set its **target** to the player, and its **bounds** to the entity showing your [tile map](tile-maps.md). It has no keys: it just follows.
  - The player can walk a little way from the middle (the **deadZone**, in metres across and up) before the camera moves, so small steps don't shake the picture.
  - Then it catches up smoothly in about **smoothing** seconds, easing in and stopping without going past (0 follows at once). **Why:** a camera glued to the player jerks with every step; a little slack and easing feels calm, as in farming and adventure games.
  - With **bounds** set, it never shows past the map's edges: near the edge the player walks towards the side of the screen instead. A map smaller than the view stays in the middle.
  - The picture stays crisp: Solis keeps the camera on whole pixels as it draws, whatever the camera does.

| RTS camera | Keyboard and mouse | Gamepad |
|---|---|---|
| Move over the map | arrow keys | left stick |
| Drag the map | hold the middle mouse button and move the mouse | |
| Turn | Alt + left or right arrow | right stick |
| Zoom | mouse wheel, Page Up and Page Down | right and left triggers |

These are the ready-made [actions](input.md) **Pan**, **Drag**, **Turn** and **Zoom**, so players can rebind them in **File > Input**. A project made before these cameras existed doesn't have them yet: the Input actions window offers to add them.

**Moving a character from your own code**

`walk(wanted)` moves the component's character by `wanted` metres (`target:walk(wanted)` moves another), as far as it can: it climbs steps and slopes, slides along walls, pushes crates and rides moving platforms, and stops where it can't go. It happens at the end of the [tick](concepts.md#game-code), after physics has had game code's other changes. Call it once a tick with speed times `dt`, plus your own falling. Always multiply by `dt`: it's the tick's length at the game's speed, so the character slows down and stops with `World:setGameSpeed` like everything else. A `walk` without `dt` ignores the game's speed.

```
fetch "solis/input":Input;
fetch "solis/project":Action;

component Hopper {
    has {
        float fall = 0;
    }

    method update(float dt) {
        vec3 move = Input:axis(Action.Move);
        vec3 step = [move.x, move.y, 0];
        is grounded()? then {
            fall = 0;
            is Input:pressed(Action.Jump)? then {
                fall = 5;
            }
        } or {
            fall = fall - 20 * dt;
        }
        walk(step * 4 * dt + [0, 0, fall * dt]);
    }
}

pass Hopper;
end
```

Why your own falling? Because how a character falls and jumps is part of how a game feels (floaty or snappy), so it's yours to choose. On the ground it stops falling (`fall = 0`): the CharacterBody keeps it to the ground going down steps and slopes (its **Keeps to the ground** setting).

In Lua it's `solis.entity.walk(self.me, {x, y, z})` and `solis.entity.grounded(self.me)`.

## Settings

**CharacterBody** (built-in)

| Setting | What it does |
|---|---|
| Height | Feet to head, metres. 1.8 suits a person. |
| Radius | Half its width. 0.35 m suits a person; wider characters can't fit through narrow gaps. |
| Step height | The tallest step it walks up without jumping. 0.35 m climbs stairs but not a table. |
| Steepest slope | The steepest slope it walks up, in degrees; it slides down steeper ones. 45° is usual. |
| Keeps to the ground | How far down it stays on the ground walking down steps and slopes, instead of flying off them. 0 never. |
| Mass | How hard it pushes crates it walks into. |

**CharacterController** (ready-made)

| Setting | What it does |
|---|---|
| camera | The camera following it: forward is the way that camera looks. Nothing: forward is the way the character faces. |
| walkSpeed, sprintSpeed | Metres a second, walking and with Sprint held. |
| jumpHeight | How high a jump goes, metres. |
| gravity | How fast it falls. 20 feels snappier than real life's 9.81, as most games do. |
| facesWhereItWalks | It turns to face where it's walking (third person). Untick for first person. |
| turnSpeed | How fast it turns to face where it walks, degrees a second. |

**ThirdPersonCamera** (ready-made)

| Setting | What it does |
|---|---|
| target | The character it follows. |
| distance | How far behind it stays, metres. A wall in between pulls it closer. |
| height | How high above the character's feet it looks: about the head. |
| wheelZooms | Tick it to let the mouse wheel (Zoom) change the distance, between closest and farthest. Off, the distance stays fixed. |
| lowest, highest | How far down and up the player can look, degrees. |

**FirstPersonCamera** (ready-made): **body** (the character), **eyes** (how high its eyes are), **turnsBody**, **lowest** and **highest**.

**RtsCamera** (ready-made): **focus** (the point on the map it looks at), **height**, **lowest** and **highest** (how far it zooms), **tilt** (degrees it looks down), **panSpeed** and **turnSpeed**.

**FollowCamera2D** (ready-made): **target** (who it follows), **bounds** (an entity with a tile map it stays inside; empty: no edges), **deadZone** (how far, in metres across and up, the target can move from the middle before it follows; 1 and 0.75 to start), **smoothing** (about how many seconds it takes to catch up; 0.2 to start, 0 for at once), and **speed** (how fast it's moving now; you don't need to set it).

The fields at the bottom of each (`fall`, `yaw`, `pitch`) are how it's moving right now: how fast it's falling, and which way it's looking. While playing, Properties shows them changing. They're fields so a [save game](save-games.md) keeps them; you don't need to set them.

## Shortcuts
| Action | Shortcut |
|---|---|
| Walk | W A S D (left stick) |
| Look | Mouse (right stick) |
| Jump | Space (A) |
| Sprint | Shift (left stick click) |
| Save and load, in the playground | F6 and F7 |

All of these are the game's only while it has the mouse and keys (click the Game panel); see [Input](input.md). The kit's actions (Move, Look, Jump, Sprint, Zoom, Pan, Turn, Drag) are always named in game code, so the kit works in a project that hasn't made them all; one your project hasn't made in **File > Input** is never pressed.

## Tips and common problems
- **Problem:** the character doesn't move. **Why:** the game doesn't have the keys, or there's no ground under it (it fell). **Fix:** click the Game panel; give the floor a Collider.
- **Problem:** W walks the wrong way. **Why:** the CharacterController's **camera** isn't set, so forward is the way the character faces. **Fix:** set it to the camera.
- **Problem:** the character can't get up a step. **Why:** the step is taller than its **Step height**. **Fix:** raise Step height, or make the step lower.
- **Problem:** it slides down a ramp it should walk up. **Why:** the ramp is steeper than **Steepest slope**. **Fix:** raise it, or make the ramp gentler.
- **Problem:** my changes to `solis/kit/character.lst` disappeared. **Why:** Solis writes the `solis/` folder again whenever the project opens, so it always matches this version of Solis. **Fix:** copy the file outside `solis/` (say, `player.lst`), rename the component inside (`component MyController`), and fetch that instead.
- **Problem:** the game slows down with a big crowd of characters. **Why:** each character's move looks around it for walls, steps and slopes, about 10 to 15 microseconds each. Solis works them out on every core at once, so a few hundred cost little, but thousands add up. **Fix:** for a huge horde, move the far-away ones yourself with `moveTo` and give a CharacterBody only to those near the player.
- **Problem:** the first-person view spins when I walk. **Why:** both the camera and the controller are turning the character. **Fix:** untick the CharacterController's **facesWhereItWalks**.

## Related
- [Physics](physics.md): colliders, rigid bodies, pushing things
- [Input](input.md): the actions, and rebinding them
- [Save games](save-games.md)
- [Game code](game-code.md)
- Concepts: [character controller, ready-made components](concepts.md#simulation)
