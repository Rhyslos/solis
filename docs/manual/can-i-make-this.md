# Can I make this?

> What the three kinds of game Solis aims at need, and whether each need works today, works with a recipe of game code, or isn't there yet and when it comes.

## When to use it
Before you start a game, or when you wonder whether Solis can do something yet. Solis is judged by the games it makes well, and these three are the yardstick: **a platformer**, **a 2D action game** (a platformer with sword fighting, or a side-view shooter) and **a farming and life game** like Stardew Valley. Most 2D games need a mix of the same things.

Each need is marked:
- **Works:** it's in Solis now; the link shows how.
- **Recipe:** a few lines of game code do it, ready to copy from the recipe pages. A recipe is not a stopgap: what game code does well stays game code, so it's yours to change.
- **Not yet:** it isn't there. It says when it comes, and what to do until then where there's a way.

The games come in order of cost: the platformer first, then the action game (everything both need is here now), then the farming game. This page changes with every update, so it always says what you can do in the version you have.

> **Good to know:** checked against Solis itself on 2026-11-26, not guessed. If something here is wrong, it's a mistake in the manual: please report it.

## How to use it
Find your kind of game below and read down its list. Anything marked **Recipe** links to the code to copy. Anything marked **Not yet** you can leave out of your first version, or work round as it says.

### A platformer
Start with [your first platformer](first-platformer.md), then the [platformer recipes](platformer-recipes.md).

| Need | | How |
|---|---|---|
| Running and jumping, landing on floors, bumping heads | Works | `walk` and `grounded()` on a character ([your first platformer](first-platformer.md)) |
| Levels painted from tiles, with solid ground | Works | [Tile maps](tile-maps.md), solid terrains ([Walls and walking](walls-and-walking.md)) |
| A camera that follows, kept inside the level | Works | `FollowCamera2D` from the kit ([your first platformer](first-platformer.md#6-make-it-play)) |
| A jump set by its height and time | Recipe | [A jump that feels right](platformer-recipes.md#a-jump-that-feels-right) |
| Coyote time and jump buffering | Recipe | [A jump that feels right](platformer-recipes.md#a-jump-that-feels-right) |
| A short hop when Jump is let go early | Recipe | [A jump that feels right](platformer-recipes.md#a-jump-that-feels-right) |
| Wall jumps | Recipe | [A jump that feels right](platformer-recipes.md#a-jump-that-feels-right) |
| One-way platforms, jumped up through and stood on | Works | **One way** on a `Collider2D` ([One-way platforms](walls-and-walking.md#one-way-platforms)); dropping down through one is game code (`setSolid(false)` for a moment) |
| Moving platforms | Recipe | A solid `Collider2D` moved along a path ([Following a path](platformer-recipes.md#following-a-path-moving-platforms-patrolling-enemies)) |
| Being carried along by a moving platform | Works | A solid moved with `moveTo` carries who stands on it ([Moving platforms](walls-and-walking.md#moving-platforms)) |
| Slopes | Not yet | Later, when a game asks: 2D colliders and tiles are boxes and circles |
| Enemies and platforms following a path | Recipe | Empty entities as stops ([Following a path](platformer-recipes.md#following-a-path-moving-platforms-patrolling-enemies)) |
| Paths drawn as lines in the editor | Not yet | Later, when a game asks; empty entities as stops do it now |
| Components that draw in the editor (an enemy's reach) | Not yet | Later, when a game asks |
| Coins, keys and things to collect | Works | [Trigger zones](trigger-zones.md), [tags](tags.md) ([your first platformer](first-platformer.md#7-add-coins-to-collect)) |
| Animations: run, jump, idle, facing left and right | Works | [Animating sprites](sprite-animation.md), the `Flip flip` field |
| Tweens (things gliding to a new value) | Works | [Tweens](game-code.md#tweens) |
| Tweens kept in save games | Not yet | Later, when a game asks; glide the value yourself in `update` where it must survive a save |
| Screen shake | Works | Send `ShakeCamera` to the kit's `FollowCamera2D` ([Characters and cameras](characters-and-cameras.md)) |
| A flash when hurt | Recipe | [A hurt flash](platformer-recipes.md#a-hurt-flash) (a blink; a white flash later) |
| Hit-stop | Recipe | `World.gameSpeed` ([Hit-stop](platformer-recipes.md#hit-stop-the-game-stops-for-a-blink-on-a-big-hit)) |
| Squash and stretch | Not yet | Later, when a game asks: game code can't size a sprite yet |
| Controller rumble | Not yet | Later, when a game asks |
| Sounds and music | Works | [Sound events](sound-events.md), [Sounds](sounds.md) |
| A HUD, a pause menu, a title screen | Works | [Game screens](screens.md) |
| Saving and loading | Works | [Save games](save-games.md) |
| Sharing the game with players | Works | [Exporting for playtesting](playtest-export.md) |

### A 2D action game
Everything the platformer needs, and the [combat recipes](combat-recipes.md).

| Need | | How |
|---|---|---|
| A sword whose hit lands on one frame of its swing | Recipe | Frame events, `World:within` and tags ([A sword](combat-recipes.md#a-sword-that-hits-on-one-frame-of-its-swing)) |
| Health, getting hurt and knockback | Recipe | [Getting hurt](combat-recipes.md#getting-hurt-health-and-knockback) |
| Hit-stop | Recipe | [Hit-stop](platformer-recipes.md#hit-stop-the-game-stops-for-a-blink-on-a-big-hit) |
| Aiming with a stick | Works | `Input:axis` ([Bullets that hit](combat-recipes.md#bullets-that-hit)) |
| Aiming with the mouse, in the world | Works | `Input:pointer()` and `Input.pointerShown` ([Input](input.md)) |
| Bullets that hit, as game state | Recipe | A small module made with `new` ([Bullets that hit](combat-recipes.md#bullets-that-hit)) |
| Hundreds of bullets at once (pools) | Not yet | Only if a game finds `new` too slow; dozens at once are no work |
| Lasers and tracers by the thousand, drawn only | Works | [Projectiles](game-code.md#projectiles) |
| Sparks and muzzle flashes | Recipe | Bursts of short projectiles ([Sparks](combat-recipes.md#sparks), [the gun](combat-recipes.md#bullets-that-hit)) |
| Trails | Recipe | Short projectiles along the path, as sparks are |
| Smoke and dust (particles that fall and fade) | Not yet | With rain and snow, in the farming game's pieces |
| "Is there ground ahead?" and "is there a wall?" for enemies | Works | `Physics:castRay` in 2D ([Rays in 2D](walls-and-walking.md#rays-in-2d)) |
| Enemies that patrol and turn at edges | Recipe | [Enemies that patrol](combat-recipes.md#enemies-that-patrol-and-turn-at-edges), or [paths](platformer-recipes.md#following-a-path-moving-platforms-patrolling-enemies) |
| Bots that find their way round a level, jumping and dropping | Not yet | Later, when a game asks |
| Music that changes when a fight starts | Recipe | [Music that follows the fight](combat-recipes.md#music-that-follows-the-fight) |

### A farming and life game, like Stardew Valley
The engine's part is listed here; crops and seasons, shops, crafting, fishing, friendship, quests and mail are the game's own code.

| Need | | How |
|---|---|---|
| A farm of tiles with values on each cell (watered, grown) | Works | [Tile maps](tile-maps.md) |
| "Which cells hold a crop?" | Recipe | Go over the map's cells in a `loop` with its own file's methods ([Tile maps](tile-maps.md)); fine for a farm, slow for a huge map |
| The day's clock and its colours, things at an hour | Works | [Sky, fog and exposure](sky-fog-and-exposure.md), [Timers](timers.md) |
| Night lighting, glowing windows | Works | [Lights](lights.md) |
| Walking behind trees and buildings | Works | Sprites lower on the screen are drawn in front ([Sprites](sprites.md)) |
| Doors and places that notice who walks in | Works | [Trigger zones](trigger-zones.md) |
| Houses, the town and the mines as separate levels | Not yet | In the farming game's pieces. Until then, put rooms side by side in one level and move the player with `moveTo` at a door |
| A fade between rooms | Not yet | With rooms: a screen that fades |
| Villagers walking round solid tiles to a place | Not yet | In the farming game's pieces. Their days are game code on [timers](timers.md) |
| Talking to people, conversations, a dialogue box | Not yet | Dialogue, built in full, before the farming game's pieces |
| Data: items, prices, crops | Works | [Data tables](data-tables.md) |
| An inventory and a hotbar shown on screen | Works | [Game screens](screens.md); the inventory itself is game code |
| Clicking a slot in a screen | Works | A screen's `button` sends an event ([Game screens](screens.md)) |
| Tips when pointing at a slot | Not yet | In the farming game's pieces |
| Dragging items between slots | Not yet | Later, when a game asks; click to pick up and click to put down works now |
| Save slots | Works | `Saves:saveTo(slot)` with a slot per name (`slot1`, `slot2`); the menu is the game's own screen ([Save games](save-games.md)) |
| Music that follows the season and the hour | Recipe | Swap tracks as [Music that follows the fight](combat-recipes.md#music-that-follows-the-fight) does, on the hour |
| A smooth crossfade from one track to the next | Not yet | In the farming game's pieces; `stopSounds()` ends a track with a short fade now |
| Rain and snow | Not yet | In the farming game's pieces, on a particle emitter |
| Festival cutscenes | Recipe | [Timers](timers.md), `walk` and `moveTo`, and screens for the words |
| Text in several languages | Works | [Translations](translations.md) |

### What comes when
In order, each a small update:
1. **Rust and Lua stop being game languages:** Lust is the language games are written in.
2. **Dialogue,** in full: conversations, the dialogue box and the conversation editor.
3. **The farming game's pieces:** separate levels and moving between them, walking round solid tiles, tips on screens, a music crossfade, and rain and snow.

What isn't on the list waits until a game asks for it: puzzle and turn-based tools, 3D games, multiplayer.

## Settings
This page has no settings.

## Tips and common problems
- **Problem:** something here says *Works* but you can't find it. **Why:** the manual describes the version on the main line, which may be newer than yours. **Fix:** update Solis ([Installing and updating](installing-and-updating.md)).
- **Problem:** your game needs something no list mentions. **Why:** the lists cover what these three games share, not every game. **Fix:** ask for it; a need from a real game moves up.

## Related
- [Your first platformer](first-platformer.md), [Platformer recipes](platformer-recipes.md), [Combat recipes](combat-recipes.md).
- [Concepts](concepts.md): the words used here.
