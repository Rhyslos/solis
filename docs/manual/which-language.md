# Which language when

> Lust, Lua and Rust can all write game code, and they work together on one entity. This page says which to use for what, with Solis's own measurements.

## When to use it
When you're starting a component and wondering which language to write it in, or when the [Profiler](profiler.md) says game code takes much of a frame.

## The short answer
- **Lust** for most of your game: it's made for Solis, Problems checks it as you type, and it's fast enough for almost everything. Start here.
- **Lua** for logic you already know how to write in Lua, or code copied from elsewhere: deciding things, reacting to events, menus, quests. It's fine anywhere the work is small per tick.
- **Rust** for the heaviest per-tick work on many entities (crowds, simulations), once it's clear that's where the time goes. For now, Rust components need a Solis built from source.

All three can sit on the same entity and send each other events (see [Game code](game-code.md#events)), so you can start a component in Lust and move just the slow part later.

## The numbers
The same work, written once in each language: 2,000 entities each doing a running sum of 40 sines every tick. Milliseconds for one tick of all 2,000 (a frame at 60 frames a second has about 16 ms in all), from Solis's scale check:

| | Rust | Lust | Lua |
|---|---|---|---|
| 40 steps of number work each | 0.36 | 2.7 | 6.0 |
| 400 steps each (ten times as much) | 4.3 | 27 | 36 |

What that means:
- **Rust is about 7 times quicker than Lust** at number work, and Lust about twice as quick as Lua when the work is light.
- **Most of Lua's time is reaching its own fields** (`self.value`), which Solis checks every time. A Lua component that keeps its sums in local variables and writes its fields once at the end runs closer to Lust.
- All three give **exactly the same answer**, to the last bit, so a game plays the same whichever you choose, and [recordings](input.md) replay the same.

Solis tried making Lust and Lua faster with Luau's "native code". Lust's heavy work got about a third quicker but Lua's got slower, so it's off for now ([the report](../reports/lua-native-code.md)).

## Tips and common problems
- **Problem:** game code takes most of a frame. **Why:** many entities do a lot each tick. **Fix:** see which component it is, then: do the work every few ticks instead of every tick; keep sums in local variables; or move that component to Lust (from Lua) or Rust (from Lust).
- **Problem:** "is Lua bad, then?" **Why:** no: 2,000 Lua components doing light work each tick take about 6 ms, which fits easily in a frame. **Fix:** nothing, unless the Profiler says otherwise.

## Related
- [Game code](game-code.md): writing components in all three languages.
- [The Profiler](profiler.md): where each frame's time goes.
- [Concepts](concepts.md#game-code): game code, components.
