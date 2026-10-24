# The benchmark: how much can this computer play?

> Two fleets of ships flying loops and firing lasers, in three sizes, so you can see what your computer keeps at 60 frames a second.

## When to use it
- **Before you plan a big game:** see how many units and projectiles your computer (and your players' computers) can take.
- **After changing your graphics card, driver or settings:** compare the numbers.
- **When something feels slow:** if the benchmark is fine, the slowness is in your own game, and the [Profiler](profiler.md) shows where.

## How to use it
1. **Help > Make the benchmark…** and choose an empty folder. Solis makes a new project there and opens it on `target.solarea`.
2. Press **Play**. The ships appear over the first few frames (500 a tick, so no frame stalls), then fly and fire.
3. Open the **Profiler** (beside the Console) to see the frame's time, and where it goes.
4. Press **Stop**, open another area from the Library, and play that.

| Area | Ships | Lasers in flight | What it shows |
|---|---|---|---|
| `target.solarea` | 2,000 | about 20,000 | Solis's target: should hold 60 frames a second. |
| `stress.solarea` | 10,000 | about 100,000 | Far past it: should stay playable (30 frames a second or better) without stutter. |
| `ceiling.solarea` | 1,000, then more | ten per ship | Adds 1,000 ships every few seconds while it holds 60 frames a second, then says in the Console where it ended: "This computer keeps 60 frames a second up to about 14,000 ships and 140,000 lasers." |

The ceiling run measures for two seconds after each step settles, so it takes a minute or two. Keep the window in front while it runs; the frame rate of a window in the background says little.

## What's in it
Everything is ordinary Solis, so you can read and change it:
- **`ships/fighter.glb`**: a ship model Solis made (about 2,000 triangles, with [levels of detail](levels-of-detail.md)), and **`ships/fighter.solassembly`**, an [assembly](assemblies.md) of it with the `Ship` component.
- **`main.lst`**: the [game code](game-code.md). `Ship` flies its loop and fires a laser six times a second; `Fleets` makes the ships with `spawn`; `Ceiling` is the ceiling run, reading `frameTime()`.
- The three areas: the sun and sky, a camera, and one entity whose component makes the fleets. Change **Ships** on it in Properties to try your own number.

## Tips and common problems
- **Problem:** the frame rate is stuck at 60 even with few ships. **Why:** your screen shows 60 pictures a second, and Solis waits for it. **Fix:** nothing to fix: 60 is the goal. Ships past that are what the ceiling run finds.
- **Problem:** the ceiling run's number changes between runs. **Why:** other programs share the computer. **Fix:** close what you can and run it twice; the higher number is closer to what the computer can do.
- **Problem:** the first frames after Play are slow. **Why:** the ships are being made, 500 a tick. **Fix:** watch the numbers once all are made.

## Related
- [The Profiler](profiler.md)
- [Game code](game-code.md): `spawn`, `fire` and `frameTime`
- [Levels of detail](levels-of-detail.md)
- Concepts: [frame, frames a second, CPU and GPU](concepts.md#playing)
