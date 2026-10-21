# 0003. Games can be written in Lust, Rust or Lua

- **Status:** Accepted (extends [0002](0002-engine-in-rust-games-in-lust.md); replaces "LustScript/Lua only for editor scripting, never game code")
- **Date:** 2026-10-21

## Problem

Decision 0002 made Lust the game language, with Rust possible on the side, and the first design kept Lua for editor scripting only. The project lead doesn't want to force anyone to learn Lust to make a game.

## Decision

Game code can be written in **Lust, Rust or Lua**, and all three are supported, not tolerated:

- **Lust:** the language the docs teach first, and the one the engine's ready-made gameplay components are written in.
- **Rust:** for people who already know it, or need the last bit of speed. Rust components are the engine's own kind, so they work from the start.
- **Lua** (Luau, through `mlua`, the same runtime as editor scripting): for the many people who already know Lua from Roblox, Garry's Mod and modding. LustScript, which turns into Lua, works here too.

All three see the same game API: components with fields in the inspector, typed events, and the same entities. A game can mix them, one component in Lust and another in Lua.

## Consequences

- **Lua is quick to change and slower to run.** No compile step, so changes show instantly in play mode, but it's slower than compiled code and has a garbage collector. Fine for most gameplay; heavy, per-unit work at the scale in DESIGN.md §2 is better in Lust or Rust. The docs say so.
- **Lua and deterministic netcode need care.** Lua code must avoid things that can differ between machines, such as the order `pairs` walks a table in, and use the engine's seeded randomness. The engine checks for what it can and warns, and the docs list the rules.
- **The game API needs a Lua face** as well as a Lust and Rust one. Designing it once and generating the bindings keeps the three alike.
- **Three languages in the docs:** every page about game code shows Lust first, with Rust and Lua versions alongside.
