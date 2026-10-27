# 0055. Lua's speed: measured, sped up where it's safe, and explained honestly

- **Status:** Accepted
- **Date:** 2026-10-27

## Problem

Lua runs slower than Lust or Rust: it's interpreted, and its values can be anything. The project lead asked whether Solis could turn Lua into Rust when building. If not, a warning that Lua performs worse would do.

## Options

1. **Translate Lua into Rust when building.** Lua's values can change kind while a game runs, and its tables change behaviour through metatables. A faithful translation has to wrap every value in an "it could be anything" box, and often ends up barely faster than Luau already is. It is large and fragile, and every build would wait on Rust's slow compiling.
2. **Luau's own native code generation.** Luau can compile scripts to machine code, and the `mlua` library Solis uses exposes it (its `luau-jit` feature). It helps number-heavy code called often, at some cost in memory and start-up time, within size limits.
3. **Guidance:** tell people, with measured numbers, when Lua is the right choice and when Lust is.

## Decision

Chosen by the project lead: **no translation (option 1). Lust already is the typed language that compiles to Rust.** A Lua component that turns out slow moves to Lust.

**Option 2 as an experiment.** Native code generation is switched on only if both hold:
- Solis's own scale cases show it is faster;
- the determinism checks (state checksums, recordings replayed) give exactly the same results with it as without it.

If either fails, it stays off, and the findings are written up.

**Option 3, always:**
- **The manual** gets a "which language when" page, with numbers from Solis's own tests.
- **The profiler** marks a Lua component when it is among a frame's biggest costs, with a hint that Lust runs that kind of code faster. It only speaks when it matters, so it never nags.
- **Lua stays a fine choice** for most gameplay logic. In effects, it's for per-effect decisions, not per-particle work.

## Consequences

- The experiment and the guidance are a small milestone, built alongside compatibility (Phase 8), since both are about game code staying trustworthy.
- If native code generation is kept, the sandbox tests (decision 0034's time limits, and the hostile-code tests) run with it on too.
