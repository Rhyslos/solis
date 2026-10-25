# 0042. A hardening milestone before Phase 7

- **Status:** Accepted
- **Date:** 2026-10-25

## Problem

Phase 6 had three review rounds (decision 0027), and each found a good number of problems. The deep ones became rarer each round:

- **Round one:** a design flaw. Saves kept the physics library's own bytes.
- **Round two:** the first security audit. It found holes such as game code being able to reach `require`.
- **Round three:** mostly hardening against hostile files, plus edge cases.

But three of round three's bugs were caused by round two's fixes: a character no longer rode a moving platform, replays with saves stopped matching, and bodies stayed asleep after an edit. A round-three fix also made a crowd of characters 30% slower. The suite passed each time. **The tests weren't strong enough to catch a fix that broke something else.**

Two more things make this riskier than it looks:

- **59 pull requests are open, stacked on each other back to #26.** Nothing has been merged since Phase 3. Every fix builds on unmerged fixes, and a mistake low in the stack reaches everything above it.
- **Fuzzing is shallow.** File readers are fuzzed with short loops of damaged copies inside the tests (a few thousand cases). The audits still found inputs those loops never tried.

## Options

1. **More review rounds.** They find problems after they're built, and each round's fixes need reviewing in turn.
2. **Carry on to Phase 7 and harden as we go.** Phase 7 (sound, animation) builds on the game loop, saves and replays. Gaps there grow.
3. **A milestone of its own, before Phase 7, that makes the tests stronger** (chosen). The aim is that a fix which breaks something is caught by a test before review, and that the tests themselves are measured.

## Decision

Chosen by the project lead (2026-10-25: "Stricter testing and more testing is only beneficial!"). **Phase 6 hardening** comes between Phase 6 and Phase 7 (ROADMAP.md). It is built one milestone per pull request like any phase ([plan](../plans/phase-6-hardening.md)). The project lead asked for it to be built straight away, before they test the open pull requests, so that what they test has been through the stronger tests too (2026-10-25: "it would ease my mind knowing future work is way more robust and less error prone"). Steps 2 to 5 are built on top of the stack, and merging comes last:

1. **Merge the stack** (last). The open pull requests, these included, are merged in order once tested.
2. **Whole-game invariant tests** (property tests, with `proptest`). Random games with walking, spawning, pushing, saving, loading, recording and edits during play, checked against rules that must always hold:
   - a saved and loaded game carries on exactly like one that never stopped;
   - a replay matches its recording;
   - batched and one-by-one moves agree;
   - Stop puts the area back exactly.

   A failure is shrunk to the smallest game that shows it, and is kept as a regression test.
3. **Real fuzzing** with `cargo-fuzz` (libFuzzer). Every reader of a file a project can ship gets a fuzz target: saves, recordings, input actions, areas and assemblies, models, the import cache, sidecars and the project file. Each runs for hours locally, and for a time box in a weekly CI job. Crashes become regression tests.
4. **Measured tests.** A coverage report (`cargo-llvm-cov`) per crate shows code no test reaches. Mutation testing (`cargo-mutants`) on the core crates (`world`, `physics`, `script`, `area`, `editor`'s play and save code) deliberately changes the code and checks that a test notices. Each surviving change either gets a test or is explained.
5. **Checks that stay.** Invariant tests run in CI with a fixed budget. Fuzzing runs weekly. Coverage and surviving mutations go in each phase's review. CONTRIBUTING.md gains the rule that a fix comes with a test that fails without it.

**Gate:** the project lead reads the hardening report: coverage per crate, mutation results, hours fuzzed and what was found, and the invariants that hold. They then try the playground once more.

## Consequences

- Phase 7 starts later, by about one phase's worth of work.
- New tools: `proptest` (MIT or Apache-2.0) as a development dependency. `cargo-fuzz`, `cargo-llvm-cov` and `cargo-mutants` are installed tools, not dependencies of Solis. Fuzzing needs the nightly compiler, so it runs only in its own weekly job and locally; everything else stays on the pinned version.
- Decision 0027's reviews carry on at each phase's end, now with coverage and mutation results in hand.
- Bugs this milestone finds are fixed in it, in pull requests grouped by kind, each with the test that found it.
