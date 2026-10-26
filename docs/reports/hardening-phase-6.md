# The Phase 6 hardening report

What the hardening milestone ([decision 0042](../decisions/0042-a-hardening-milestone-before-phase-7.md), [plan](../plans/phase-6-hardening.md)) did, what it found, and how much the tests now catch. Written for the project lead; the numbers come from the runs on 25 and 26 October 2026, on a 2-core Linux machine.

## In short

- **Eight real bugs found and fixed:**
  - four by the whole-game tests, all in saving and loading;
  - two by fuzzing: one could freeze the code check for good, and one was a leaked thread;
  - two by mutation testing: an ordering mistake in areas, and a game code mistake that was let through.
- **Mutation testing also found NEWLY_CAUGHT mistakes no test would have noticed, in behaviour that worked**, now tested so they stay that way.
- **Fuzzing ran about 12 hours in all**, across ten file readers, about 230 million tries. It found nothing beyond those first two.
- **The tests cover 86% of the code.** The engine's core crates are between 87% and 98%.
- **Mutation testing**, which changes the code on purpose and checks a test notices, went from catching about 76% of changes to about RESULT_ALL. What's left is listed below, with why it doesn't matter.
- **The new checks stay:**
  - every pull request runs the whole-game tests;
  - a weekly job runs every fuzzer and a much larger whole-game run, and opens an issue if anything fails;
  - CONTRIBUTING has two new rules: a fix comes with a test that fails without it, and each phase's review looks at coverage and mutation results.

## What each kind of testing does

- **Whole-game tests (invariants):** the computer writes hundreds of random little games: walk, push crates, spawn and despawn, save, load, edit while playing, swap code, record. After each, it checks rules that must always hold. For example, a loaded game plays on exactly like the one that never stopped.
- **Fuzzing:** a fuzzer feeds each file reader millions of damaged and made-up files, mutating the ones that reach new code. Anything that crashes, hangs or leaks is a bug, because Solis must refuse a bad file with a message.
- **Coverage:** which lines of code the tests run at all.
- **Mutation testing:** a tool makes thousands of small deliberate mistakes in the code, one at a time: `<` becomes `<=`, `+` becomes `*`, a function returns nothing. Then it runs the tests. A mistake the tests don't notice ("missed") shows a gap in the tests.

## What was found and fixed

**By the whole-game tests** (all in saving and loading; saves are now version 3):

1. A game saved after game code spawned anything, or with a copy of an assembly, couldn't be loaded.
2. Rotations came back a hair different after loading (they were made unit length again on reading), so a loaded game slowly went its own way.
3. A turned entity was placed with a -0 where the running game had a 0, with the same effect.
4. After a load, moving something in the editor while playing started from the game's place instead of the editor's.

**By fuzzing:**

5. Two components (or classes, or objects) with the same name made the Lust compiler loop for ever. The editor's code check never finished, and a core stayed busy. It was found from a 26-byte input: `component h{}component h{}`. Solis now finds same-named declarations first and says where the other one is, and gives up on any check that takes more than 30 seconds, with a message. The real fix belongs in Lust's class ordering; it's worth raising there.
6. The code-check thread was left running after it answered, leaking a little memory each time. It's joined now.

**By mutation testing:**

7. A hand-edited entity order with leading zeros made the next new entity sort before it instead of after. A property test now checks that any order works.
8. In game code, a state field with no states listed was silently accepted; it's now a mistake in Problems.

**Behaviours that worked but nothing tested** (mutation testing showed a mistake there would go unnoticed). Each now has a test that fails if it breaks:

- **Copies:** a copy of an assembly whose fields point at its own parts (in a field, and in a list).
- **Physics:** a ray's hit point; which bodies take part and which physics moves; rays that can't go anywhere.
- **Input:** the mouse wheel turned down used as a button; a combination tapped between ticks; every limit at its exact edge; a held pad button pressed only once; the stick past its dead zone; the one-hour recording limit.
- **The world:** settling under a turned parent; frame times that aren't numbers; the seed and clock as set; the entity list; pushes and character moves; the camera between ticks; projectile lifetimes.
- **Components:** removing a component type; finding fields by their exact name; rotations as four numbers.

Fuzzing also stopped once on a panic inside a library Solis uses (gltf-json, a mesh naming a part that isn't there). Solis already catches that one and refuses the file, so the fuzzers now count only panics that get past Solis.

## Fuzzing

| Reader | Tries | Found |
|---|---|---|
| Save games | 3.6 M | — |
| Recordings | 20 M | — |
| Input actions | 20 M | — |
| Areas and assemblies | 14.6 M | — |
| Models | 43 M | a library panic Solis already catches |
| The import cache | 87.7 M | — |
| Sidecars | 18.7 M | — |
| Project files | 10 M | — |
| Game code (Lust and Lua) | 0.1 M | the same-name hang (bug 5) and the leaked thread (bug 6) |
| Update answers from GitHub (new) | 11.9 M | — |

Game code is slow to fuzz (each try compiles and runs a game), so it has far fewer tries; the weekly job keeps adding to it. One later report from the game code fuzzer was a check still running when the fuzzer's time ran out. Run alone, the same input takes 0.1 s and leaks nothing.

## Coverage

| Crate | Lines run by the tests |
|---|---|
| render | 97.7% |
| world | 96.4% |
| input | 95.1% |
| assets | 93.4% |
| physics | 92.8% |
| area | 92.1% |
| asset_db | 90.4% |
| editor | 88.8% |
| reflect | 88.7% |
| script | 86.5% |
| app | 39.5% |
| **All** | **85.8%** |

The app is low because most of it opens the window and talks to the graphics card. That part is checked by the window and GPU tests on Linux and Windows, which don't count towards these numbers. What isn't run elsewhere is mostly error messages for things that can't be made to fail in a test (a graphics card disappearing, say).

## Mutation testing

"Caught" is the share of mutations (that compile) some test notices. **Before** is the code as it was when the milestone started. **After** is main with the hardening tests, plus the tests added in this report's pull request.

| Crate | Mutations | Before | After |
|---|---|---|---|
| physics | 226 | 78% | 96% |
| input | 270 | 81% | 92% |
| world | 245 | 81% | 91% |
| reflect | 152 | 80% | 91% |
| area | 584 | 67% | AREA_AFTER |
| script | 773 | SCRIPT_BEFORE | SCRIPT_AFTER |
| editor's play files | 340 | — | EDITOR_AFTER |

**What's still missed, and why it doesn't matter:**

- **Exact edges of sanity limits** (`<` versus `<=` on "is this number sensible"): one more or less in a million makes no difference to anything a game does.
- **Arithmetic that only has to over-estimate:** the recording size estimate, for example. Changing a `+` to a `*` makes it bigger, which is still safe.
- **Bookkeeping that only saves time:** "work out only what's under the moved entity". Doing more work gives the same answer, more slowly. The scale check, not a test, watches speed.
- **How evenly IDs spread in a hash table:** bit-mixing that only affects speed.
- **Text for programmers:** debug printing of internal structures.
- **Equivalent changes:** `dt > 0` versus `dt >= 0` when `dt = 0` does nothing either way.

LEFT_NOTE

## The checks that stay

- **Every pull request** runs the whole-game tests (12 games per rule, in about a minute).
- **Every week** (`.github/workflows/deep-checks.yml`), each fuzzer runs for 20 minutes from the corpus it kept, and the whole-game tests run 1,000 games per rule. A failure uploads the input that caused it and opens an issue.
- **CONTRIBUTING.md:**
  - rule 9: every file reader has a fuzz target;
  - rule 14: a fix's test must fail without the fix;
  - rule 19: each phase's review includes coverage and mutation results.
- **ARCHITECTURE.md, Checks,** says how to run each locally.

## What it means for the next phases

- Saving, loading and replays now have tests that would have caught every bug the Phase 6 reviews found in them.
- File readers are fuzzed as they're written; the update check's answers already are.
- The review rounds should now find fewer deep problems; when they do, the rule that a fix comes with a failing test means the same problem stays fixed.
