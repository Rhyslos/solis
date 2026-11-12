# Getting started: your first 2D platformer

> Build a small platformer from an empty project, step by step: a level of solid ground, a hero who runs and jumps, a camera that follows, and coins to collect. Along the way you learn the editor and your first Lust code.

## When to use it
Start here if you're new to Solis, to Lust, or to making games at all. It takes about an hour, and needs no art skills: the level is drawn in plain colours, and the hero and coins are two tiny pictures you can make in any paint program.

You'll learn:
- the editor's parts: the [Library](projects-and-the-library.md), the viewport, [Properties](concepts.md#the-window) and the Area panel;
- [entities and components](entities.md): how everything in a game is built;
- [tile maps](tile-maps.md) for the level, and [2D settings](2d-pixel-perfect.md) that keep pixel art crisp;
- writing [components](concepts.md#game-code) in Lust: fields, `update`, `dt`, input, and fetching one file from another.

Each step says *why* as well as *what*, so you can change things and make the game your own. Where a step uses a feature, it links to that feature's own page if you want more.

## How to use it

### 1. Make a project and an area
A [project](concepts.md#projects) is a folder holding everything your game is made of. An [area](concepts.md#the-area) is one level (or menu, or room) in it.

1. **File > New project** (or **+ New project** on the launcher), make an empty folder called `Platformer`, and choose it.
2. Save the empty area: **Ctrl + S**, name it `level`. Solis suggests the `assets/areas` folder; that's fine.
3. Look in pixels: click the **Perspective ▾** pill at the top left of the viewport and choose **2D**. The view now looks straight at the world as the game will. **Hold the right or middle mouse button and drag** to slide it; **turn the wheel** to zoom.

**Why 2D?** A platformer is seen from the side. In Solis, a 2D game lies flat and is seen from straight above, so across the screen is *x* and up the screen is *y*. The project's 2D settings (**File > Project settings**) say 16 pixels make one metre, so one 16-pixel tile is one metre. Every number in this guide is in metres, so think of a metre as a tile.

### 2. Build the level from tiles
A [tile map](tile-maps.md) is a grid of cells, each painted with a [terrain](concepts.md#2d). Terrains come from a **tileset**. We need two: *sky* (empty air) and *ground* (solid).

1. **Make the tileset:** right-click an empty spot in the Library and choose **New > Tileset**. Solis makes one with three terrains (grass, tilled, water) and opens it in the **Tileset** tab.
2. **Turn grass into sky:** choose *grass*, and in **Properties** rename it `sky` and make its colour light blue. The first terrain fills every new map, which is why the sky goes first.
3. **Turn tilled into ground:** rename it `ground` and make it brown. Leave water as it is.
4. **Make the map:** **New > Tile map** in the Library. It's filled with sky. Click it in the Library and, in Properties, set its **Size** to 60 across and 17 up: 17 tiles is about one screen high.
5. **Place it:** double-click the map in the Library. It appears in the viewport, where you're looking.
6. **Select it** in the Area panel. The **Map** tab opens beside Properties, and the painting tools appear at the bottom of the viewport.
7. **Make ground solid:** in the Map tab, under **Terrains**, **right-click** ground's swatch and tick **Solid**. Solid cells are walls and floors: nothing with a body passes into them.
8. **Paint the level:** left-click ground's swatch, then paint with the brush (**B**) or the rectangle (**U**). Make a floor along the bottom two rows, a wall up both ends (so the hero can't run off), and a few floating platforms two or three tiles above the floor and each other. Leave a gap in the floor somewhere if you like a challenge.
9. Press **Walls** at the top of the viewport: green boxes show what's solid. Check the floor and walls have them.

**Why tiles?** Painting a level is much quicker than placing every block by hand, and Solis merges neighbouring solid cells into a few big walls, so the hero never catches on the seams between tiles.

### 3. Make the hero and a coin
Any paint program will do (Aseprite, Paint, Krita, Photoshop).

1. Draw a **16 × 16** picture for the hero: a simple figure, or just a coloured block with eyes. Make the space around it see-through. Save it as `hero.png` in the project's `assets` folder.
2. Draw an **8 × 8** yellow circle and save it as `coin.png` beside it.

Both appear in the Library. A picture becomes a [sprite](sprites.md) when you place it: an entity that shows the picture, standing on the picture's bottom middle.

### 4. Place the hero and give it a body
1. **Drag `hero.png` into the viewport**, into the sky just above the floor. An entity called *hero* appears, with a **Sprite** component showing the picture.
2. With it selected, in **Properties** choose **Add component > Collider2D**. This gives it a shape for [2D physics](walls-and-walking.md), drawn as a green outline.
3. Set its **Shape** to **Box**, **Width** 12 and **Height** 16 (in pixels), and **Up** 8 so the box sits on the hero's feet instead of being cut in half by them.
4. Tick **Character**. A character is moved by game code and stops at walls and floors instead of passing through them.

**Why a box?** A box stands flat on the edge of a platform, which is what a platformer needs. (Top-down games use a circle, which slips round corners.)

If you press **Play** now, nothing happens yet: the hero has a body, but nothing tells it to move. That's the code's job.

### 5. Write the hero's code
Game code in Solis is written in [Lust](game-code.md). A **component** you write in Lust works like Solis's own: you add it to an entity, set its fields in Properties, and its `update` runs every [tick](concepts.md#game-code) while the game plays (60 times a second).

1. Right-click in the Library and choose **New > Lust code file**. The first one is always `main.lst`, at the top of `assets`: your game's code starts there. It opens in your code editor (Ledger).
2. Make a second one the same way. It's called `component.lst`; right-click it, choose **Rename…** and call it `hero.lst`.
3. Replace everything in `hero.lst` with this, and save:

```lust
-- hero.lst: the player. Runs left and right, falls, and jumps.
fetch "solis/input":Input;
fetch "solis/project":Action;

component Hero {
    has {
        float speed = 6;
        float jumpSpeed = 13;
        float gravity = 35;
        float fall = 0;
    }

    method update(float dt) {
        -- Which way the player pushes: -1 is left, 1 is right, 0 is neither.
        float across = Input:axis(Action.Move).x;

        -- On the floor: stop falling, but keep a small push down, so the
        -- floor still stops it next tick and grounded() stays true.
        is grounded()? then {
            fall = -1;
        }
        -- Going up but stopped: it bumped its head, so start coming down.
        is fall > 0 && velocity().y <= 0? then {
            fall = 0;
        }
        -- Gravity: every tick, it falls a little faster.
        fall = fall - gravity * dt;
        -- Jump, but only from the floor.
        is grounded() && Input:pressed(Action.Jump)? then {
            fall = jumpSpeed;
        }

        walk([across * speed * dt, fall * dt, 0]);
    }
}

pass Hero;
end
```

4. Open `main.lst`, replace everything in it with this, and save:

```lust
-- main.lst: where the game's code starts. It fetches the other files.
fetch "hero":[Hero];
fetch "solis/kit/cameras":[FollowCamera2D];
end
```

5. Solis checks your code a moment after each save. If you made a typing mistake, the [Problems](problems.md) panel says where (click it to jump to the line). An empty Problems panel means the code is sound.

**What the code says, line by line:**
- Lines starting with `--` are comments: notes for people, which Lust skips.
- `fetch` brings in names from another file. `Input` reads the player's controls, and `Action` holds their names: `Action.Move` is WASD or the arrow keys (or a gamepad's stick), `Action.Jump` is Space (or the gamepad's A). Players can change these keys in **File > Input** without touching your code.
- `component Hero` is a new kind of component. Everything in `has` is a **field**: a value each hero keeps, shown in Properties so you can change it there. `fall` is how fast the hero is going up (more than 0) or down (less than 0), in metres a second.
- `update(float dt)` runs every tick. `dt` is how long a tick is, in seconds (about 1/60). Multiplying a speed by `dt` turns "metres a second" into "metres this tick", so the game runs the same speed on every computer.
- `is …? then { … }` is Lust's *if*: the code in the braces runs only when the question is true.
- `grounded()` is true when a floor stopped the hero's last move down. `velocity()` is how fast its last move really went.
- `walk([across, up, 0])` moves the hero this far, this tick, stopping at walls and floors and sliding along them. The three numbers are across, up and height; a 2D game ignores the height.
- `pass Hero;` lets other files fetch it, and `end` ends the file.

**Why `main.lst` fetches `hero.lst`:** Lust reads `main.lst` first, then each file it fetches, and each file *those* fetch. A file nothing reaches isn't part of the game. For a big game, `main.lst` stays short and fetches a few files that each fetch their own; this is how code stays tidy as it grows.

### 6. Make it play
1. Select the hero. In Properties, choose **Add component > Hero**. Its four fields appear.
2. Add a camera: **Home > Camera**. In the 2D view it's a 2D camera, and a dashed frame labelled **Game screen** shows what it sees.
3. With the camera selected, **Add component > FollowCamera2D**. This is a ready-made component from Solis's kit. Set its **target** to the hero and its **bounds** to the map's entity, so it follows the hero and never shows past the map's edges.
4. Save (**Ctrl + S**) and press **Play**. Click the Game panel so it gets the keys.
5. Run with **A** and **D** (or the arrows), jump with **Space**. Press **Stop** when you're done: everything goes back to how it was before Play.

**Make it feel right.** While stopped, select the hero and change its fields in Properties, then play again:
- **speed**: how fast it runs. 6 is brisk; 4 is a stroll.
- **jumpSpeed** and **gravity** together set the jump. Higher gravity feels snappier and less floaty; a higher jumpSpeed jumps higher. With 13 and 35 it jumps about 2.4 tiles. Try 10 and 20 for a moon jump.

Changing the numbers in Properties changes only this hero. Changing the `= 6` in the code changes the starting value of every hero made from now on.

### 7. Add coins to collect
1. Make another code file, rename it `coin.lst`, and replace everything in it with this:

```lust
-- coin.lst: a coin that disappears when the player touches it.
fetch "solis/world":World;

component Coin {
    has {
        entity player = nothing;
        float reach = 0.8;
    }

    method update(float dt) {
        is player == nothing? then {
            return;
        }
        -- How far the player is, across and up.
        vec3 gap = player:position() - position();
        is gap.x * gap.x + gap.y * gap.y < reach * reach? then {
            World:say("Got a coin!");
            despawn();
        }
    }
}

pass Coin;
end
```

2. In `main.lst`, add a line under the hero's, and save:

```lust
fetch "coin":[Coin];
```

3. Drag `coin.png` into the viewport, somewhere the hero can reach (on a platform, say). **Add component > Coin**, and set its **player** to the hero.
4. **Make more coins:** select the coin and press **Ctrl + D** to duplicate it, then drag the copy somewhere else. Copies keep their Coin and its player.
5. Play. Touch a coin: it vanishes, and the [Console](console.md) says *Got a coin!*

**What's new here:**
- `entity player = nothing;` is a field holding another entity: the one you pick in Properties. `nothing` means none is picked yet, and `return` stops `update` early, so a coin without a player does nothing instead of going wrong.
- `player:position()` asks *another* entity where it is; `position()` on its own asks this one.
- `gap.x * gap.x + gap.y * gap.y < reach * reach` checks the distance without a square root (Pythagoras, both sides squared): the player is within `reach` metres.
- `despawn()` removes the coin from the running game. When you Stop, it comes back, since Play never changes your area.

### 8. Where to go next
You've built a game and written two components. Some ideas, each with its page:
- **Animate the hero** with frames for running and jumping: draw them in Aseprite as tags, then see [Sprites from Aseprite](sprites.md), [Animating sprites](sprite-animation.md) and [Animators](animators.md).
- **Real tile art** instead of colours: give the terrains pictures in the Tileset tab ([Tilesets and autotiling](tilesets.md)).
- **A sky and hills that move slower than the ground**, for depth: [Parallax layers](parallax.md).
- **A coin counter on screen**: [Game screens](screens.md) and [the UI editor](ui-editor.md).
- **Sounds** for jumping and coins: [Sounds](sounds.md) and [Sound events](sound-events.md).
- **Send it to a friend**: [Exporting for playtesting](playtest-export.md).
- **Learn more Lust**: [Game code](game-code.md) covers events, spawning and more. This guide teaches Lust; the same game can be written in Lua or Rust (see [Which language when](which-language.md)). In Lua, the hero's calls are `solis.Input:axis("Move")`, `solis.Input:pressed("Jump")`, `solis.entity.grounded(self.me)`, `solis.entity.velocity(self.me)` and `solis.entity.walk(self.me, {x, y, 0})`.

## Settings
The fields of the two components you wrote:

| Component | Field | What it does |
|---|---|---|
| Hero | speed | How fast it runs, in metres (tiles) a second. |
| Hero | jumpSpeed | How fast it leaves the ground when it jumps. Higher jumps higher. |
| Hero | gravity | How quickly it falls faster. Higher feels snappier; lower, floatier. |
| Hero | fall | How fast it's going up or down right now. The code sets it; leave it at 0. |
| Coin | player | Who can collect it. Pick the hero. |
| Coin | reach | How close, in metres, the player must come to collect it. |

## Shortcuts
| Action | Shortcut |
|---|---|
| Run | `A` `D`, the arrow keys, or a gamepad's left stick |
| Jump | `Space`, or a gamepad's A |
| Slide the 2D view | Hold right or middle mouse, drag |
| Zoom the 2D view | Mouse wheel |
| Paint with the brush or the rectangle (map selected) | `B`, `U` |
| Duplicate | `Ctrl + D` |
| Save | `Ctrl + S` |

## Tips and common problems
- **Problem:** the hero falls through the floor. **Why:** the ground terrain isn't solid, or the hero's Collider2D isn't ticked Character. **Fix:** right-click ground's swatch in the Map tab and tick Solid (press **Walls** to check), and check the Collider2D.
- **Problem:** nothing happens when I press keys. **Why:** the Game panel doesn't have the keys, or the hero has no Hero component. **Fix:** click inside the Game panel while playing; check Properties for Hero; check [Problems](problems.md) for mistakes in the code.
- **Problem:** Hero or Coin isn't in the Add component menu. **Why:** the code has a mistake, or `main.lst` doesn't fetch the file (a file nothing fetches isn't part of the game). **Fix:** look in Problems, and check the `fetch` lines in `main.lst`.
- **Problem:** the Game panel shows only sky. **Why:** the camera isn't following, or is looking somewhere else. **Fix:** check the camera's FollowCamera2D **target** is the hero.
- **Problem:** the hero sometimes won't jump. **Why:** it only jumps from the floor, and you pressed Space while it was still in the air. **Fix:** that's how it's meant to work; for a more forgiving jump, see the idea below.
- **Problem:** the hero's feet sink into the floor or float above it. **Why:** the box isn't lined up with the picture. **Fix:** set the Collider2D's **Up** to half its **Height**.
- **Idea:** many platformers let you jump a moment after running off a ledge ("coyote time"). Try it yourself: keep a field counting the time since the hero was last grounded, and allow a jump while it's under 0.1 seconds.

## Related
- [Concepts](concepts.md): the words used here.
- [Walls and walking in 2D](walls-and-walking.md): solid terrains, Collider2D and `walk`.
- [2D: the pixel-perfect view](2d-pixel-perfect.md) and [Characters and cameras](characters-and-cameras.md) (FollowCamera2D).
- [Game code](game-code.md) and [Input](input.md).
- [Your first game: the turret sample](first-game.md): a 3D example, in all three languages.
