# Luau's native code: tried, and left off for now

Phase 8, C6, [decision 0055](../decisions/0055-lua-speed-measured-and-explained.md). Date: 2026-10-28.

## The question

Luau can turn game code into the computer's own machine code as it loads (its "native code generation", `mlua`'s `luau-jit` feature). Decision 0055 allows it only if Solis's own scale cases show it's faster **and** the determinism checks give exactly the same results with it. Lust runs on Luau too (Lust is compiled to Luau), so it would affect both Lust and Lua.

## What was measured

The scale case `scale_check_the_same_work_in_rust_lust_and_lua` (crates/editor/src/game_code_tests.rs): 2,000 entities, each with a component doing the same number work every tick (a running sum of sines), written once in each language. Release build, one Linux machine, milliseconds for one tick of all 2,000, two runs each.

**Light work** (40 steps a tick, as in the scale case):

| | Rust | Lust | Lua |
|---|---|---|---|
| Native code off (as shipped) | 0.36 | 2.74 | 6.03 |
| Native code on | 0.33 | 2.18 | 6.30 |
| Native code on, Lua file marked `--!native` | 0.46 | 1.92 | 6.08 |

**Heavy work** (400 steps a tick, ten times as much):

| | Rust | Lust | Lua |
|---|---|---|---|
| Native code off | 4.0 to 4.5 | 26.1 to 28.6 | 35.4 to 36.2 |
| Native code on | 4.9 to 5.2 | 17.8 to 19.3 | 42.8 to 46.1 |

**Determinism:** the three languages gave the same result to the last bit, with native code off and on (`the_same_work_comes_out_the_same_in_each_language`).

## What it shows

- **Lust's heavy number work got about a third faster.** Its light work hardly changed: there, the cost is mostly calling into each component, not the sums.
- **Lua's got slower** (about a fifth, on heavy work), and its light work didn't change either. A Lua component's fields go through Solis's checks on every read and write, and native code doesn't speed that path up.
- So it's **not faster across the board**, which decision 0055 requires. It would also lengthen every build of Solis (Luau's code generator is a large C++ library), use more memory per game, and widen what the sandbox's hostile-code tests must cover.

## Decision

**Left off.** Rust and Lust stay the answer for heavy per-tick work, and the manual's [Which language when](../manual/which-language.md) page says so with these numbers. It's worth trying again when:
- Lua's field access is reworked (where Lua's time goes), or
- a game's Lust is compiled to Rust in the full export (Phase 18), which makes this moot for Lust.

The scale case stays in `tools/scale/scale-check.sh`, so the gap between the languages is measured at the end of every phase.
