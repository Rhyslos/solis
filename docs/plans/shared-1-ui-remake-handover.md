# Shared 1, the editor's UI remade: where it stands and how to pick it up

Written 2026-11-11, when the project lead paused the work to save usage ("record all your progress and stop here"). Everything needed to continue is in the repository; nothing lives only in a session.

## Where it stands

| Milestone | State |
| --- | --- |
| **M1 The look** (tokens, fonts, one theme, five accents, picture tests) | Merged (pull request 236) |
| **M2 The kit** (icons, the design's controls, unfinished parts and the setting that hides them) | Merged (pull request 237) |
| **M3 The top** (tab bar, ribbon, command search, the File page's frame; the menu bar and workspace tabs gone) | Built and merged with the pull request that adds this file |
| **M4 The middle** (editor groups, the shelf, the status bar; the dock removed) | Built on the branch `ui/part-a-the-frame`, the first of four pull requests (answer 36); its "As built" notes are in the plan |
| M5 to M16 | Not started. Planned in [`shared-1-ui-remake.md`](shared-1-ui-remake.md); all their mock-ups are approved. Built as three more pull requests: Part B (M5 to M9), Part C (M10 to M12), Part D (M13 to M16) |

So, once Part A's pull request is merged, the editor has the design's look, its controls and its whole frame (tab bar, ribbon, editor groups, shelf, status bar), with today's panels inside it. Shared 1a stands at 4 of 4, and its gate is the project lead's to try.

**Also paused: the full code review** of November 2026, with 72 of 119 findings fixed and merged (one more half fixed, 45 open, one left for the UI remake on purpose). Its state is in [`docs/reviews/2026-11-full-review-status.md`](../reviews/2026-11-full-review-status.md). The project lead's instruction (answer 34 in the plan): it does not resume by itself after the UI.

## How it continues (answered)

**Four pull requests, one a part, with the leaner process** (the project lead, 2026-11-12, answer 36 in the plan): each commit builds and passes the editor's tests, the editor needs to be fully usable only at the pull request's end, and the part's manual pages may be written once, at the end. The first, `ui/part-a-the-frame`, finishes Part A with M4. A builder of a later part works on a branch named `ui/part-b-...` and so on, and takes the brief below with that in place of "one milestone".

## What M4 left for later (its builder's own list)

- "**Shelf opens on**" waits for its Preferences setting (M9); the unsaved dot on a document tab (spec 124) isn't drawn yet (M5's Home is the natural place).
- Animate's **Frames** and **Clips** shelf tabs carry a × that closes the clips file, which the design doesn't draw: M11 gives a clips file its own document.
- Properties is short in Home's Entity shelf tab (Hints under it, the Area and the Library beside it) until M5 rebuilds it as cards; several tests give the shelf more room for that (`test_support::roomy_shelf`).
- The window smoke test drags the shelf's grip and the Debug tab's divider now; its positions are for the new frame and haven't been run on the GitHub runners (the Linux jobs are paused, decision 0102).

## What M3 left for M4 and later (its builder's own list)

- The ribbon's collapsed state is not saved between runs (`ShellState` arrives with M4).
- The Library, Animate and Audio ribbons hold only Import and unfinished controls; UI's has only Play; contextual tabs have no ribbon yet (M7, M10 to M13 fill them, to the approved round-2 and round-3 pictures).
- `InterfaceLook::ribbon_labels` is saved but has no effect yet. The unfinished part `build.targets` is registered but not drawn (M15).
- `playing_border` in `panels.rs` stays until M4 removes the dock.
- No test yet samples the top of the window against the design's own pictures (01, 06); today's sampling is against the tokens.
- **Three small looks for the project lead to see when work resumes** (ask as questions, with pictures): the small list arrow on the Model and Light buttons; "Trigger zone" not fitting its 50 px button where the mock-up says Volume; the temporary **Input** item in the File page's column.
- **For the next builder:** the test harness draws a frame when it is built, so a hand-built harness needs `fonts_ready`; the test window has an 8 px margin; a double-click needs both clicks pushed through `input_mut()`; ribbon controls' accessible names are command names ("Add entity") and tabs are "Home tab"; `switch_workspace` now moves the ribbon tab; `KeyOwner::Page` rests the area's keys on File and unbuilt screens; Debug is the old Code workspace.

## How the work is run

One builder per milestone, one after another (CLAUDE.md, "One line of work"), each given the brief below and an assignment, working on a branch named `ui/mN-short-name`. A lead reviewer reads the commits, runs the editor and looks at it beside the design, merges `origin/main` in, runs the full checks on Linux by hand (CLAUDE.md, Builds: the Linux jobs on GitHub are paused, decision 0102), opens the pull request, merges it when GitHub's checks pass, adds its "How to test" steps to the project lead's testing document, and tells the project lead "Pull request N is merged and checks passed. Next up: X". Every choice goes to the project lead as a question to click.

**The numbers so far:** M1 took one builder session and a short follow-up, M2 one, M3 one (each about an hour and a half to two hours of building), plus roughly an hour and a half of checks and merging each. The plan estimated two to three sessions each; M4 is the largest and least certain.

## The builder brief

What every builder is given first. Paths are the repository's.

You are building **one milestone** of roadmap phase "Shared 1: The editor's UI, remade" in
**Solis**, a game engine in Rust (the editor is `crates/editor`, the window `crates/app`, drawn
with egui on wgpu, Windows only). The project lead (not a programmer) designed the new interface
themselves ("the G layout") and wants Solis to look exactly like it. A lead reviewer reads your
work afterwards and opens the pull request; you do not.

#### The sources of truth, in this order
1. The lead's answers: `the plan's section "What the project lead has decided" (`docs/plans/shared-1-ui-remake.md`)`
2. The lead's design:
   - spec (1,192 lines): `docs/design/g-layout/specification.md`
   - pictures of the 13 screens: `docs/design/g-layout/ (the thirteen PNGs)`
   - the mock-up's HTML, unpacked per screen (exact CSS, icon paths): `docs/design/g-layout/markup/` (one file a screen)
   - approved mock-ups of parts the design lacks: `docs/design/g-layout/additions/` (rounds 1 to 3, all approved)
3. The plan and the inventory (your milestone's section is your task list):
   ``docs/plans/shared-1-ui-remake.md`` and `docs/plans/shared-1-ui-remake-inventory.md`.
   Where the plan and DECISIONS.md disagree, DECISIONS.md wins; where the plan and the code disagree
   (the plan was written from reading, not building), the code wins: say so in your report.
4. The project's own rules: `CLAUDE.md`, `CONTRIBUTING.md` (25 ground rules), `ARCHITECTURE.md`,
   and the README of every folder you touch.

#### Where you work
- Worktree `/home/claude/solis-review`; your branch is already checked out. Stay on it. Do **not**
  push, open pull requests, switch or create branches, or touch any other folder or repository.
- Every cargo command: `export RUSTUP_TOOLCHAIN=stable CARGO_TARGET_DIR=/home/claude/solis/target CARGO_INCREMENTAL=0`
  and `--offline`. 2 cores, little disk: one `-p` at a time, never another target directory,
  never `cargo clean`, never delete under target yourself. No new downloads: if something you
  need isn't in the offline registry, stop and report BLOCKED with its name and version.
- GPU and window tests: `WGPU_BACKEND=gl REQUIRE_GPU=1 xvfb-run -a cargo test ...`. To look at the
  editor yourself, run it under `xvfb-run` and capture the window (ARCHITECTURE.md, Checks, says how;
  `tools/smoke/window-smoke.sh` is an example), then open the picture with the Read tool. **Look at
  what you build, beside the lead's picture of the same screen, before you call anything done.**

#### How to work
- **Commits:** small ones that each build and pass the tests of the crates they touch, in an order a
  reviewer can follow (the plan's milestone section often names the order). Plain-language subjects
  starting with the milestone id (`M1: ...`), each ending with exactly:
  ```
  Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01Q7DM1s4Tfj99cNHqFUFqmU
  ```
  Commit as you go: a session can end without warning, and whatever is committed is kept.
  Leave the worktree clean when you stop.
- **Nothing becomes unreachable.** After your work every feature that existed before can still be
  reached. If your milestone moves something, a test proves it's still there.
- **Exactly the design.** Colours, sizes, radii, fonts and spacing come from the tokens (spec lines
  31-125, Inventory part d), never typed as numbers at the place of use. Where the spec and the HTML
  differ, DECISIONS.md / PLAN D18 says which wins.
- **Keys and mouse actions:** today's keys stay (Q select, W move, R rotate, S scale, T all three,
  Space opens the search). Add no key or mouse action that isn't listed as approved in DECISIONS.md;
  if your milestone needs one, build without it and report it under "needs the lead".
- **Unbuilt things** are drawn only through the placeholder mechanism (`unbuilt.rs`, from M2 on) and
  vanish when "Hide unfinished parts" is on.
- **UI the lead hasn't designed** (anything marked NEEDS MOCK-UP that DECISIONS.md doesn't list as
  approved): don't invent it. Keep today's version of it working inside the new frame and report it.
- **Words on screen** follow DECISIONS.md (Solis's own words unless it says otherwise). Never the
  program's name typed out (use `crates/app/src/branding.rs` / `AppInfo`), never another product's
  name anywhere in code, comments, tests or docs (rule 20). "Ledger" is the lead's own code editor.
- **Readable code (rule 3):** new files under about 500 lines, functions under about 60, names that
  say what a thing is; comments say why, not what. `Workbench` must not grow: new state gets its own type.
- **The frame loop never waits (rule 10):** no file reading, no heavy work on the window's thread.
- **Saved settings are a format (rules 9, 23):** new keys with defaults, old keys read and left alone,
  fixtures for old files, a fuzz test for every new reader.
- **Tests with every batch:** normal, edge, failure, regression, round trip, fuzz where files or
  settings are read; the picture checks the plan names. Tests must not depend on chance, timing,
  thread scheduling or map order (IDs are random): run each new test 20 times from its built binary.
  Tests run on Windows too: treat `\r\n` as `\n` when reading text files, never write `/` in a
  compared path.
- **Docs in the same commits:** code comments; folder READMEs when files are added, removed or
  renamed; `ARCHITECTURE.md`; `CHANGELOG.md` (plain words; `### Breaking` when something is taken
  away); the user manual `docs/manual/` for everything a user would notice (follow
  `docs/manual/_template.md`, write for someone new to game engines, link terms to `concepts.md`);
  decision records your milestone's plan section names (a file in `docs/decisions/` numbered in
  order, a line in `DESIGN.md` §17, a row in the decisions README); `ROADMAP.md` and the README
  tracker only when the assignment says so.
- **Checks before you report** (paste the result lines):
  `cargo fmt --all -- --check`; `cargo clippy --offline -p <crate> --all-targets -- -D warnings` for
  each crate touched and its dependents; `cargo test --offline -p <crate>` likewise;
  `RUSTDOCFLAGS="-D warnings" cargo doc --offline --no-deps -p <crate>`. The lead reviewer runs the
  whole workspace's checks afterwards.

#### Not allowed (stop and report BLOCKED if the milestone seems to need one)
`unsafe`; a new dependency (a feature of one already used is allowed if it's in the offline
registry: say so); a second way of doing something that already has a way; weakening or deleting a
test to make it pass (a test of removed behaviour is removed with a line in your report saying
which and why); a project file format change.

#### If you run out of room
A milestone may be larger than one session. Stop at a clean commit and report exactly what is done
and what is left, as a list the next builder can continue from. Never leave half a change uncommitted.

#### Your final message (read by the lead reviewer; under 400 words; nothing else)
```
RESULT: DONE | PARTIAL (continue from …) | BLOCKED
commits: <first..last, how many>
built: <what now exists, 5-10 lines, with file names>
moved or removed: <what changed place or went, and the test that proves it's still reachable>
looked at: <which screens you captured and compared with which of the lead's pictures; what still differs>
tests: <added, changed, removed (with why)>
checks: <each command and its result line>
not done: <anything in the milestone's plan section you did not build, and why>
needs the lead: <questions, keys, mock-ups>
noticed: <problems seen and left alone>
```

## The assignment for M4

Read `the builder brief above` and follow
it exactly. You work in `/home/claude/solis-review` on the branch `ui/m4-the-middle` (already checked out; it
starts from M3's finished branch).

The plan and inventory are in the repository: `docs/plans/shared-1-ui-remake.md` (your section: "M4"; also
sections 2, 3 and 4.2, 4.3) and `docs/plans/shared-1-ui-remake-inventory.md` (part (a): where every panel
goes; part (b) for each screen's shelf tabs and columns and which are unbuilt; part (e) on replacing the
dock). The design: `docs/design/g-layout/` (specification.md lines 126-165 for the five bands, 148-158 for
groups and the shelf, 160-164 for the status bar, 1131-1146 for what each workspace remembers and how it
reflows), and the approved additions in `docs/design/g-layout/additions/round-1/`. Read
`the plan's section "What the project lead has decided" (`docs/plans/shared-1-ui-remake.md`)` whole first.

#### What M1 to M3 left you
Tokens, fonts, theme, `InterfaceLook`; `icons/`; `widgets/`; `unbuilt.rs` and its table; `shell/` with
`Shell`, `RibbonTab`, `ShellCommand`, the tab bar, the ribbon and its tables, the Play group, the File
page's frame; the test helpers in `test_support.rs` (`ribbon_tab`, `run`, `file_item`);
`shell_reach_tests.rs`; the picture tests and approved pictures. Read M3's "As built" notes in the plan.

#### Settled for this milestone (the lead's answers)
- **The running game is its own Game tab** beside the area's tab in the same group, brought to the front on
  Play (`06-game-tab.png`). **No play tint** anywhere.
- **The document tab's "+" and right-click menus** as approved (`08-doc-tab-*.png`).
- **The Console on the shelf** as approved (`09-console-shelf.png`): its filters, search, Clear and Copy on the
  shelf bar's right, the command line at the bottom.
- **The shared dialog card** as approved (`10-dialog-*.png`).
- **Ctrl + Space folds and unfolds the shelf** (the design's key, new, approved by the lead); the Library
  drawer that used it today goes. It works while typing, as today's key did.
- **Shelf tab names use Solis's words:** Entity (not Object); the Area column (not Scene); Properties.
- **Saved panel arrangements are not carried over** (plan 4.2): every ribbon tab starts from the design's
  standard, the Console says so once, the changelog has a `### Breaking` line, and the old settings keys are
  read no more but never deleted or rewritten.
- **Hints** will be a card under Properties (round 2; M5 builds it): in M4 the Hints panel sits under
  Properties in the Entity shelf tab as it is drawn today.
- Dragging a document tab to split or merge groups is **M15**, not now: groups have fixed splits with a
  draggable divider.

#### What to build (plan section 5, M4)
`shell/groups.rs`, `shell/shelf.rs`, `shell/workspaces.rs`, `shell/status.rs`, `shell/saved.rs`
(`ShellState`, restore, the one-time note), and every `Panel` given its home in the new frame **as it is
drawn today** (the plan's table says where each goes; the panels' contents are not touched in this
milestone, only where they sit). The dock is removed: `layout.rs`, `workspace.rs`, `library_drawer.rs`,
`egui_dock` from the workspace's dependencies (with `docs/dependencies.md`, `deny.toml` and the notices
tooling following), and a **decision record "Docking is Solis's own"** (numbered on from the highest; its
DESIGN.md §17 line and README row).

Hold to these especially:
- **The work's order inside the pull request** (the plan's third guard): groups and shelf first with the dock
  still compiled; the switch second; the removal last; each a commit that builds and passes.
- **Nothing becomes unreachable:** `shell_reach_tests.rs` grows to every `Panel`; `key_owner_tests.rs` passes
  unchanged in meaning (the shell keeps filling `pointer_over`: who owns a key is still the panel under the
  pointer).
- **`Workbench::show` gets shorter** (the frame moves out of it; the shell's state is out of `Workbench`).
- **The frame loop never waits**; the Area column and the status bar read counts cached by the document's
  revision, not by walking the area every frame (review finding BUG-28: fix it here, where the code moves).
  Leave the tile caches alone (BUG-19): the Game tab must not add a third user of them.
- **Unbuilt shelf tabs and status items** go through `unbuilt.rs` (the plan lists them); the status bar's
  right-hand text is where an unfinished part's click message shows.
- **Saved state:** `ShellState` under its own key, every field with a default, a fuzz test, `restore`
  repairing what it can; the three settings fixtures from M1 start with the standard shell and say so once.
- The window smoke test drags a panel today: it becomes dragging the shelf's grip and the split divider.
- **Manual:** `interface-tour.md` (panels, docking, the Library drawer, the status bar: rewritten),
  `keyboard-shortcuts.md` (Ctrl + Space; the Panels table), `projects-and-the-library.md` (the drawer),
  `concepts.md` (docking, layout, workspace), `settings.md` (Reset layout), `play-mode.md` (the Game tab).
- Tick M4 in `README.md`'s tracker (Shared 1a: 4 of 4, the bar, the overall count) and mark it built in
  `ROADMAP.md`. Shared 1a's gate is the lead's to try; say in the changelog what to look for.

Before you report: run the editor under `xvfb-run` at 1440 × 900 and at 1100 × 700 with the sample project
open, capture Home, Library, Animate, UI, Audio and Debug, and compare each with the design's screen of that
tab: the five bands, their heights, colours and lines must match; the panels inside still look like today's.
List what differs and why.

This is planned as about four sessions and is the riskiest milestone of the phase: stop at a clean commit and
report PARTIAL with what is left whenever you run short.
