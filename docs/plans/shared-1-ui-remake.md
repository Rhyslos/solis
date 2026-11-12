# Shared 1: The editor's UI, remade. The build plan

Written 2026-11-11 from reading the code and the design, before anything was built; brought into the repository with milestone M1. Where the code turns out to differ from what this plan says of it, the code wins and the plan is corrected in the pull request that finds out. Its companion is [`shared-1-ui-remake-inventory.md`](shared-1-ui-remake-inventory.md): every piece of today's interface and where it goes, every element of the design and what stands behind it, the keys, the tokens, and what egui can and can't do. This plan refers to it as "Inventory a.3", "Inventory b.6" and so on.

**The design** is the project lead's own, already approved by them, and lives in [`docs/design/g-layout/`](../design/g-layout/README.md): the specification (`specification.md`, 1,192 lines; "spec 140-146" means those lines) and thirteen screens as pictures (`01-home-level-editor.png` to `13-project-settings-input.png`). The mock-up's HTML is not in the repository; the exact values read from it (paddings, icon paths, colours the specification leaves out) are written down in Inventory part (d) and marked "from the HTML".

**What was asked** (the project lead, 2026-11-11): remake all of Solis's interface to this design; where the engine lacks a feature the design shows, put a placeholder page or element; and one setting hides everything that doesn't work yet.

## What the project lead has decided

Every answer so far, with its date. These override the plan's own recommendations: where a recommendation was overruled, the milestone sections below already say what was decided, and section 7 marks each decision as settled or still open. The decisions are recorded in [0099](../decisions/0099-the-g-layout.md) (the G layout), [0100](../decisions/0100-one-theme-five-accents.md) (one theme, five accents) and [0101](../decisions/0101-unfinished-parts-as-placeholders.md) (placeholders, and the setting that hides them).

**2026-11-11, with the request:**

1. **Order:** "UI now, review after". The code review's group in flight was finished and merged, the review paused, and this phase is the one line of work; the review resumes afterwards on the new code. The plan starts from `main` as it was after the review's sixth part (the `script` and `world` crates only, so nothing here had moved).
2. **Keys:** "Keep today's keys". Q select, W move, R rotate, S scale, T all three; **Space opens the search**. The design's keys are used only for new things (Ctrl + Space folds the shelf). The search box shows today's key. Any other new key still goes through rule 17.
3. **Themes:** "Replace both". Standard and Graphite are removed; only the design's Neutral dark and its five accents remain; decision 0096 is replaced.
4. **Unfinished parts** are shown as placeholders by default, and one setting, **Hide unfinished parts**, hides them all.
5. **The 2D editors** (tile map, tileset, picture crops, clips, frames, animator, the game-screen canvas) have no screen in the design: each gets a place that follows the design's patterns, mocked up and signed off before it is built.
6. **Code** opens in the user's code editor (Ledger, the project lead's own) as today; the design's built-in code editor is a later feature.

**2026-11-11, on the first round of mock-ups:**

7. **A placeholder's look (D2): look A.** The label in the lowest readable grey (#8C8C8C), its icon fainter (#5A5A5A), a 5 px hollow square mark, no hover highlight. A whole unbuilt screen is its ribbon and shelf tabs as placeholders with one centred page card.
8. **Words (D4): Solis's own** on screen: Area, Entity, Properties, Model, Picture, assembly, component, game code. The design's names where Solis has none: Preferences, Project settings, Recent projects, Exit, "Search or run a command", "+ Component". The search box's hint reads "Space".
9. **The play tint (D13): none, as designed.** While the game runs, Play becomes **Stop**, with **Pause** and **Step** beside it, and nothing else changes colour. This overrules the plan's recommendation to keep a tint, and changes decision 0085's "playing is unmistakable" colouring (recorded in decision 0099). The accent line round the viewport and the tinted status bar are not built.
10. **The running game (D14): its own Game tab** beside the area's tab, brought to the front on Play.

Shown to the project lead in the same pictures and not objected to, so treated as approved unless they say otherwise:

11. **A UI ribbon tab (D6)** between Animate and VFX, for the game-screen editor.
12. **The logo tile (D5):** Solis's own logo in the design's 20 × 20 light tile.
13. **The ribbon collapses to its tabs (D10);** a click on a tab drops the ribbon over the center.
14. **Home's ribbon groups:** Edit (Undo, Redo) · Create (Empty, Model, Sprite, Light, Camera, Volume unbuilt, Tile map, More) · Snapping (Grid, Angle, Scale: three rows, no group label) · Lighting (unbuilt) · Project (Save, Build) · Play (Play or Stop, Pause, Step, Simulate unbuilt). Three 20 px rows and a group label don't fit a 66 px ribbon, so three-row groups are drawn at 19 px rows with no label.
15. **The icon sheets:** the 69 new icons, redrawn in the design's style (square ends, as the design's own 58).

**2026-11-11, later the same day:**

16. **The logo:** "I think we can remake the logo later by the way. I was never happy with it." The remake uses today's logo in the tile, kept as one swappable asset (no logo shapes drawn in code). A new logo is on the ideas list ([`ideas.md`](ideas.md)), to be designed with the project lead, mock-ups first; it is not part of Shared 1.

**Taken by the lead reviewer without asking** (the project lead signs off mock-ups and architecture; they were told): **D1** four phases, Shared 1a to 1d; **D3** the accent mapping as recommended, and saved panel arrangements are not carried over (from M4, when the dock goes; M1 leaves them alone); **D18** every token the specification leaves out, as recommended.

**2026-11-11, 18:50, on the whole first round:** "yeah the mockups look good".

17. **All of mock-up round 1 is approved as drawn,** including what had not been shown before: the Debug ribbon with its Session group, the document tab's "+" and right-click menus, the Console's tools on the shelf bar, and the shared dialog card. The approved pictures are in the repository: [`docs/design/g-layout/additions/round-1/`](../design/g-layout/additions/round-1/README.md).


**2026-11-11, 19:59, two mouse actions for the ribbon (rule 17):**

18. **The mouse wheel over the ribbon scrolls it sideways** when it doesn't fit. **A Create button's list of kinds** opens from a small arrow on the button or a right-click; a plain click makes the usual kind; no long press. (Decision 0103.)

**2026-11-11, about 20:40, on the second round of mock-ups** ([`additions/round-2`](../design/g-layout/additions/round-2/README.md)), each asked as a question:

19. **Hints: a card under Properties** (`01-hints-card-a`), chosen over a strip pinned to the bottom of the shelf. Its known cost: it is below the fold when component cards are tall.
20. **Placing (D16): the Place shelf tab being in front is the mode** (`06-placing-a`). No ribbon button.
21. **Colours for the kinds of file (D17): option A** (mapped onto the design's colours, `07-library-kinds-a`) **but with option B's colour** (today's `theme::kind`) **for three kinds: sound events, code files and areas.** The Library's milestone (M7) makes one final sheet of every kind's colour for the project lead to check.
22. **Templates (D8): `09-recent-projects-a`:** the blank project and Solis's four examples (real), then the design's templates as unfinished parts, without VR / XR.
23. **How Solis starts (D7): in the full editor window,** on the Recent projects page. Decision 0090's small launcher window goes; its two promises stay (always the projects page first; nothing reopened by itself).
24. **2D settings (D9): a tab of their own** in Project settings, right after General.
25. **New keys and mouse actions (D15): every recommended answer approved** (`14-keys-and-mouse`): F2 and Delete over the Library or the Assets column act on the picked file, on the entity everywhere else · dragging a field's label changes its value (dragging the value stays) · Shift while dropping a file keeps it loaded for more copies · placing without snapping for one drop is **Ctrl** · R while dragging a file in turns the ghost by the ribbon's Angle step · click-then-click placing and a key that drops under the pointer: **not now** (unfinished part) · a double-click in the Library tab only selects (Place places; the shelf's Assets and Place columns still place) · dragging a tile onto the Home tab: **not now** · all four arrow keys walk the project cards · Backspace clears a picked binding (× stays; while listening, Backspace is a key to bind) · over the Frames tab Left and Right step one frame, Up and Down jump to the next event · a fine fader move is **Shift** · a click on a meter resets its peak (the two drags later) · a click on a bar of the frame-time graph picks that frame · a dragged document tab lands on the left or right half of the center or on another group's tab strip.
26. **The rest of round 2 is approved as drawn.**

**2026-11-11, 22:57 and 23:15, on the third round of mock-ups** ([`additions/round-3`](../design/g-layout/additions/round-3/README.md)): "All of these look fine as templates, can't wait to see them in engine!", then each choice asked as a question:

27. **A tile map's paint tools are in the ribbon** of a contextual tab, HOME TOOLS · Tile map (`01-tile-map-painting-a`).
28. **A clip's settings are their own Clip shelf tab** (`04-sprite-clips-a`).
29. **The game-screen editor's drawing tools are a toolbox** (`07-ui-editor-a`).
30. **The mixer's Master strip is last,** on the right; the Bus ribbon group stays.
31. **Two new actions approved:** clicking a layer's row paints on that layer; Esc puts a dragged document tab back.
32. **The rest of round 3 is approved as drawn.** All three mock-up rounds are now approved: nothing more is needed from the project lead for M4 to M16 except questions that come up while building.

**2026-11-11, how the work is run:**

33. **Choices come as questions.** "Give me them in question form next time so i pay better attention": every choice goes to the project lead as a question to click, one per choice, with the pictures named in it, never as a list in a message.
34. **Stop after the UI** (21:03): "after the UI update is done, i would like for you to stop working on the next updates and fixes." When Shared 1 is merged, nothing else starts: the paused code review does not resume and the next phase does not begin until the project lead says so.
35. **Paused to save usage** (23:35): "i will need you to record all your progress and stop here." The work stopped with M3 built. **Where it stands and how to pick it up: [`shared-1-ui-remake-handover.md`](shared-1-ui-remake-handover.md).** Before resuming, ask the project lead how to continue: milestone by milestone as planned, four pull requests (one a phase), or one; the last two with a leaner process (no working in-between states, the manual once at the end), which bends the rules "one milestone per pull request" and "documentation in every pull request" and is the project lead's call.

**2026-11-12, how to continue** (the answer to 35's question):

36. **Four pull requests, one a part, with the leaner process.** The rest of the remake is built as four pull requests, one for each part (A to D); the first finishes Part A with M4. Inside a pull request each commit builds and passes the editor's tests, but the editor needs to be fully usable only at its end, and the part's user manual pages may be written once, at the end, in one commit (still in the same pull request). CHANGELOG, READMEs, decision records and the tracker are kept up as usual. This bends "one milestone per pull request" and "documentation in every pull request", by the project lead's choice.

**What the mock-up rounds found out** (to be folded into the milestones as they are built): the Model component has one setting today (the model), so the Model card's Material and Shadows are unfinished parts and need table lines (M5) · Solis is Z up · "Used by" is unbuilt for every kind but code · a screen is a Lust file shown as Code today · the ready-made input actions share keys on purpose, so a "same input" warning must not mark them · there is no eraser or stamp tool (a right-drag empties; the Eyedropper's copy is the stamp) and no align or distribute · draw calls are measured today (the inventory says unbuilt: wrong) · Solo and ducking exist · the toolbox with nine tools exactly fills the design's viewport height, so it must scroll or wrap when the shelf is taller · six small icons are new in round 3 (minus, add state, transition, start here, wall, take a frame out).

**In numbers** (Inventory b.15): the design has 332 elements. 124 exist in Solis today (40 of them pure shell that this phase builds), 44 partly, and 164 are placeholders, 62 of those being three whole unbuilt screens. In the other direction, 29 pieces of today's interface have no place in the design (Inventory part (a)); each has a proposed place that follows the design's own patterns and needs a mock-up, as do the pages the design names but leaves undrawn (Build, Learn, and several sub-tabs of Preferences and Project settings).

---

## 1. The shape of the phase

**Sixteen milestones, one pull request each, in four parts.** After every one the editor works and every feature that exists today can still be reached; from M4 on, everything is inside the new frame, and the later milestones each make one area match its screen.

| Part | Milestones | What the lead can try at its end |
| --- | --- | --- |
| **A. The look and the frame** | M1 The look · M2 The kit · M3 The top · M4 The middle | The editor opens in the new colours, with the tab bar, ribbon, editor groups, shelf and status bar; every old panel lives inside them; unfinished parts are marked and can be hidden |
| **B. The everyday screens** | M5 Home · M6 Placing · M7 Library · M8 File: projects · M9 File: settings | Building an area, placing things, managing files, opening projects and changing settings all look like their screens |
| **C. 2D and its editors** | M10 Tile maps, tilesets, pictures · M11 Animate · M12 Game screens | Solis's 2D editors, which the design has no screens for, re-housed to approved mock-ups |
| **D. The rest, finished** | M13 Audio · M14 Debug · M15 The unbuilt screens · M16 Finish | Mixer and profiler on their screens; Shading, VFX, Build and the model viewer as placeholders; the manual rewritten; the old code gone; the review; the gate |

Why this order improves on the first sketch (tokens and widgets; shell; Home; Library; File; 2D editors; Animate; Audio; Debug; placeholders; finish):

- **The first step is split in two** (M1, M2). M1 changes how the *present* editor looks and settles the saved settings; M2 adds the parts nothing uses yet (icons, widgets, the placeholder mechanism). Each is reviewable; together they were not.
- **The shell is split in two** (M3, M4). M3 replaces the top (menus and workspace tabs become the tab bar and ribbon) while the dock stays in the middle; M4 replaces the dock. The second is the riskiest change of the phase and deserves to be alone.
- **The placeholder mechanism comes in M2 and the placeholder tabs in M3**, not near the end: the lead's switch has to exist from the first moment something unfinished is on screen.
- **File pages are two milestones**: projects (the launcher's replacement, which changes how Solis starts) apart from settings and input.
- **2D gets its own part**, right after the everyday screens, because 2D is what is built next (decision 0097) and its editors need mock-ups the lead hasn't seen yet. That time is used by parts A and B.

**The rule on phase size.** CLAUDE.md says phases stay at two to six milestones (decision 0097). Sixteen is not that. Recommended (decision D1): the four parts become four phases in `ROADMAP.md`, **Shared 1a to 1d**, each with its own gate from the table above, the last carrying the roadmap's gate ("the project lead works in the new editor for an evening and wouldn't go back"). The review of decision 0027 runs once, at the end of 1d, as the lead's answer 3 implies.

---

## 2. What every milestone keeps to

1. **Nothing becomes unreachable.** Each pull request lists, from Inventory part (a), the pieces it moves and where each now is; a test walks that list (`shell_reach_tests.rs`, added in M3: every command the old menus had can be run from the ribbon, a File page or the search, by name).
2. **`Panel` stays the unit of content** until a milestone rebuilds that panel. `PanelViewer::ui(ui, panel)` (`panels.rs:267`) draws any panel into any space, so the shell only decides *where*. Who a key belongs to stays "the panel under the pointer" (`key_owner.rs`; review fix BUG-06): the shell keeps filling `FramesOut::pointer_over`.
3. **Unfinished parts go through the one mechanism** (section 3), from M2 on. No milestone draws a "coming later" note any other way.
4. **Mock-ups before building what the lead hasn't designed** (CLAUDE.md). Every "NEEDS MOCK-UP" in the Inventory is drawn in the design's own language (the mock-up's HTML and CSS are at hand) and approved before its milestone starts. They are batched into three rounds (section 6) so the lead is asked three times, not thirty.
5. **Keys and mouse actions:** nothing new without the lead's answer (rule 17); the ones this phase raises are decision D15.
6. **Tests with every batch**, including the three kinds of picture check (Inventory e.4): measured layout against the spec, sampled colours against the mock-up picture, and Solis's own approved pictures. Every new reader of saved settings gets a fuzz test (rule 9).
7. **The manual in the same pull request** (rule 15), describing only what is in `main`: each milestone rewrites the pages for what it changed; M16 does the whole-manual pass.
8. **Readable code** (rule 3): new files under about 500 lines, functions under about 60. Where a milestone rebuilds one of the files the review found too large (`library.rs`, `launcher.rs`, `search.rs`, `properties.rs`, `viewport.rs`: findings DEBT-08 and DEBT-09), it splits it; no milestone may leave such a file larger than it found it.
9. **`Workbench` doesn't grow** (finding DEBT-05: 108 fields, a 430-line `show`). The shell's state is its own type (`shell::Shell`), its drawing is free functions that take what they need, and M4 shortens `Workbench::show` by moving the frame out of it.
10. **No name written out** (rule 16): the logo, About, the version in the status bar and the engine-version row read `AppInfo`. **No other product named** anywhere (rule 20): the spec's own comparisons are not copied into code, manual or decisions.
11. **The frame loop never waits** (rule 10): nothing in the new Library or File pages reads a file on the window's thread (findings BUG-31, BUG-32 and BUG-53 are open there and must not get worse).

---

## 3. The placeholder mechanism

One design, built in M2, used by every later milestone.

### 3.1 The registry: `crates/editor/src/unbuilt.rs`

Every unfinished element is one line in one table. The id is a Rust name, so a typo doesn't compile and deleting a line (when the feature is built) makes the compiler point at every place that still draws it.

```rust
unbuilt! {
    // id                    name               filled in          what it will do (one plain sentence)
    HomeLightingBake      => ("Bake",           Phase::ThreeD5,    "Works out bounced light for the area"),
    ShellTabShading       => ("Shading",        Phase::Shared7,    "Materials and shaders, drawn as a graph"),
    PlaceRandomYaw        => ("Random yaw",     Phase::NotPlanned, "Turns each placed thing a random way"),
    …
}
```

- `UnbuiltId` (the enum the macro makes), `Unbuilt { id, key, name, phase, what }`, `unbuilt::ALL`, `unbuilt::get(id)`.
- `key` is the dotted id of Inventory part (b) (`"home.lighting.bake"`), kept so the plan, the manual and the code name the same thing.
- `Phase` is an enum of the roadmap's phases still to come (`Shared2` … `Shared13`, `TwoD1`, `TwoD2`, `ThreeD1` … `ThreeD5`) plus `NotPlanned`. `Phase::title()` gives the words shown ("Shared 7: Materials and shaders"). A test reads `ROADMAP.md` and fails if a title isn't a heading there, so a hover text can't name a phase that was renamed (review finding DOC-04 was exactly that).
- The first table is Inventory part (b)'s 164 rows.

### 3.2 One way to draw: the functions in `unbuilt.rs`

Nothing else may draw an unfinished element. One function per shape the shell has, each taking the id and nothing about looks:

`ribbon_big`, `ribbon_row`, `tool`, `tab` (ribbon and contextual tabs), `shelf_tab`, `menu_item` (File column), `sub_tab`, `setting_row`, `field_row`, `chip`, `icon_button`, `status_item`, `link`, `card`, `column`, and `page` (a whole center or File page).

**The proposed look** (decision D2; needs one mock-up, which shows a ribbon with placeholders among real buttons, a placeholder shelf tab, a placeholder setting row and a placeholder page):

- **In place and in shape.** A placeholder sits exactly where the design puts the real thing, at its real size, so the layout the lead sees is the layout they designed.
- **Three signs, always together:** the label in the lowest readable grey (#8C8C8C, where a real one is #C8C8C8) with its icon fainter still (#5A5A5A); a small **hollow square mark** (5 px, 1 px #6E6E6E outline) at the top right corner of a button or tile, or after the label of a tab or row; and **no hover highlight**, with an ordinary arrow pointer. It reads as "not here yet", never as "broken" or "disabled for now": nothing is struck through, red or greyed to nothing.
- **A whole page** (`unbuilt::page`): a centered card on the group's own surface: the feature's icon, its name ("Shading"), the one sentence from the table, the line "Not built yet · planned for Shared 7: Materials and shaders" (or "Not built yet · not planned yet"), and a link **Hide unfinished parts** that turns the setting on.
- **Hover text**, everywhere: "Not built yet: Bake. Works out bounced light for the area. Planned for 3D 5: Bounced light."
- **Clicking does nothing except say so:** the status bar's right-hand text shows "Bake isn't built yet (3D 5: Bounced light)" for three seconds. No window, no sound, no Console line.
- **For screen readers and tests:** the element's label is its name and its description is "Not built yet", so `egui_kittest` can find every placeholder on screen.

### 3.3 Hidden entirely

**Hide unfinished parts** (off at first: placeholders show) lives in Preferences › General; until M9 it is a row on today's Settings › Appearance page; it is also an action in the command search ("Hide unfinished parts" / "Show unfinished parts").

Hiding is done **before layout, on data**, never by drawing something invisible:

- The ribbon, the shelf's tabs and columns, the File menu column, sub-tabs and setting lists are all described as tables (`shell/ribbon.rs` and friends) whose entries are either `Built(…)` or `Unbuilt(UnbuiltId)`.
- With the setting on, `unbuilt::filter` removes the `Unbuilt` entries first. A ribbon group left with nothing is removed; a ribbon tab left with no group and no real center is removed (Shading, VFX); a shelf tab with no real column is removed; a File page left empty is removed with its menu item. The layout code only ever sees what is left, so there are no holes to close.
- A PARTLY element keeps its real part; only its unbuilt piece goes.
- The command search never lists unbuilt things, with the setting on or off (today's rule: what can't be done isn't listed).

### 3.4 The tests that keep it true (`unbuilt_tests.rs`, `source_rules_tests.rs`)

1. **Drawn no other way.** A source rule, in the manner of `source_rules_tests.rs`: outside `unbuilt.rs`, no file may contain the words "Not built yet", "isn't built", "not built", "arrives in", "coming in" or "coming later" in a string; `UnbuiltId::` may appear only as an argument to an `unbuilt::` function or inside an `Unbuilt(…)` table entry; and `add_enabled(false, …)` must have a hover reason on the next call (so a dead button can't stand in for a placeholder). Today's `panels::placeholder` (a quiet note such as "No problems found") is renamed `empty_note` in M2 so the rule has one meaning to guard.
2. **Every registered id is drawn somewhere, and only registered ones are.** `unbuilt.rs` notes each id it draws in a per-frame set. A whole-window test visits every ribbon tab, shelf tab, File page and sub-tab with the setting off and checks the union of those sets equals `unbuilt::ALL` minus the few that are behaviours with nothing to draw (listed in the test, each with why), and that every node on screen described "Not built yet" came from `unbuilt.rs`.
3. **With the setting on, nothing registered is on screen.** The same walk with the setting on: the per-frame set stays empty, no node is described "Not built yet", no ribbon group, shelf tab or page is empty, and each ribbon group's left edge touches the one before it (no gaps).
4. **The table is sound:** ids and keys are unique, every name and sentence is non-empty, every phase is a heading in `ROADMAP.md`, and the table's keys are exactly the PLACEHOLDER rows and "unbuilt piece" ids of the plan's inventory table (kept as `docs/plans/shared-1-ui-remake-inventory.md`), so the document and the code can't drift.
5. **A picture test** of the placeholder mock-up's four cases.

### 3.5 What this replaces

Decision 0070 said "nothing that isn't built is shown", and decision 0085 repeated it. The lead's request reverses that for the new interface: unfinished parts are shown, marked, and can all be hidden at once. That needs its own decision record (section 8).

---

## 4. Themes, saved layouts and settings

### 4.1 The two themes go (the lead's answer 2)

| Where | What changes |
| --- | --- |
| `crates/editor/src/palette.rs` (304 lines) | Removed. `Theme`, `Appearance { theme, rounded }`, `STANDARD`, `GRAPHITE` and the thread-local choice go. The colour functions it gave (`panel()`, `text_weak()`, `line()` …) move to the new `tokens.rs` and read one palette. They stay functions, not constants, on purpose: the spec leaves a high-contrast option open (spec 1187), and a second palette later then touches no drawing code. |
| `crates/editor/src/theme.rs` (859 lines) | `Accent` goes from ten to the design's five (`Blue`, `Grey`, `Orange`, `Teal`, `Purple`); `AccentColors` is worked out for dark text on the fill. `GO` (Play's blue), `UPDATE`, `FOLDER`, `WARNING`, `ERROR`, `kind::*` and `axis::*` take the design's values (Inventory d.3). `dock_style`, `CARD_GAP`, `CARD_RADIUS`, `corner()` and `rounded()` go with the dock in M4. `fonts()` gains the three new weights. |
| Tests that pin the old values | `theme.rs`: `the_standard_accent_is_blue` (kept, new value), `the_accents_are_aventail_s` (removed), `a_saved_accent_still_reads_after_more_were_added` (becomes the mapping test below), `white_text_reads_on_every_accent` (becomes dark text), `graphite_is_darker_and_rounded_panels_are_cards` (removed), `the_launcher_s_colours_are_the_standard_theme_s_as_they_were` (removed), `text_reads_on_every_surface` and `_now`, `the_front_tab_stands_out` and `_now`, `colours_that_mean_something_keep_theirs` (rewritten for the tokens). `palette.rs`'s three tests (removed with it). `settings_window.rs`: `graphite_and_rounded_panels_restyle_the_editor_and_reset_puts_them_back` (removed), `choosing_an_accent_changes_the_editor_s_colour` (clicks "Plum": changed). `source_rules_tests.rs`: the allowed files become `tokens.rs` and `theme.rs`, and its `KEPT` list of 16 exceptions is gone through one by one. `launcher_tests.rs` and `workbench_tests.rs`: the places that set Graphite. |
| `crates/app/src/window.rs` | `APPEARANCE_KEY` and `ACCENT_KEY` are read once more and never written (4.3). `accent_from_before` grows into the mapping. |
| `crates/app/src/thumbnails.rs`, `gpu.rs` | Thumbnails no longer redraw "when the theme changes" (there is one). The viewport's backdrop colours come from the tokens. |
| The manual | `settings.md` (Theme, Rounded panels, the ten accents), `interface-tour.md`, `launcher.md` ("the launcher wears the theme too"), `concepts.md` (accent colour). |
| Decisions | A new decision replaces 0096 whole and changes 0060 (Play is no longer blue: it is the one light button; the accent stays blue by default). |

### 4.2 The saved panel layouts can't be carried over

Today's layouts are `egui_dock` trees (`panel_layout2`, `workspaces`). The new interface has no docking: a workspace remembers its shelf tab, the shelf's height and whether it is folded, the split between its groups, and whether the ribbon is collapsed (spec 1131-1138). Nothing in a dock tree maps onto that honestly, so **every workspace starts from the design's standard**, and the Console says so once: "The interface is new: your panel arrangement from before wasn't carried over. Each tab now remembers its own shelf and split." The changelog gets a line under `### Breaking` saying the same (rule 23's check asks for it when something is taken away).

### 4.3 The settings file (a saved format: rule 23, rule 9)

The editor's settings are key-and-value pairs in eframe's storage, each key read on its own with a fallback (`app/src/window.rs:170-260`). That makes the change safe if three rules are kept:

1. **New keys, never new meanings for old keys.** `look` holds `Look { accent, scale, density, ribbon_labels, hide_unfinished }`; `shell` holds `ShellState { tab, ribbon_collapsed, shelf_opens_on, workspaces: BTreeMap<RibbonTab, WorkspaceState { shelf_tab, shelf_height, shelf_folded, split }> }`. Every field has a default (`#[serde(default)]`), so a file written by a later build with more fields still reads.
2. **Old keys are read once and left alone.** With no `look` key: the accent comes from `accent2` (or the older `accent`) through the mapping; `appearance` is read only to be dropped (any theme becomes Neutral dark; `rounded` has no meaning); `hints` carries over; `library_pinned`, `panel_layout2` and `workspaces` are not read. The old keys are **not deleted or rewritten**, so a person who goes back to an older build finds their old look and layout untouched.
3. **A value that can't be read is the default, never a crash** (as today), proved by a fuzz test over `Look` and `ShellState` in the manner of `layout.rs`'s `random_damage_to_a_saved_layout_never_loses_panels`, and `ShellState::restore` repairs what it can (an unknown shelf tab becomes the workspace's first; a height outside the window becomes the standard; a split outside 0.15 to 0.85 becomes the standard).

**The accent mapping** (decision D3): Blue → Blue. Petrol blue, Green (marine green), Olive → Teal. Red, Copper, Orange, Amber, Gold → Orange. Plum → Purple. Nothing maps to Grey; it is new.

**Proof that old files keep opening:** M1 adds `tests/fixtures/editor-settings/before-g-layout/` with three real settings files written by today's build (Standard with the default accent; Graphite, rounded, Plum and a rearranged layout; one from before `accent2`), made by a test at the start of the branch before any type changes, and a test in `crates/app` that starts the new build from each: no error, Neutral dark, the mapped accent, and the kept things kept (known projects, camera, snapping, look, views, console history, auto-update, skipped build, window size). The fixtures are never changed afterwards, as rule 23 says of all fixtures.

**Project files are not touched** by this phase. `pixels.solpix`, `input.solinput` and `mixer.solmix` keep their formats; the pages that edit them change, the files don't. If a milestone finds it wants to store something new in a project (the design's "editor start level", say), that is a placeholder, not a format change.

---
## 5. The milestones

Sizes are new or rewritten lines of code, then lines of tests, then lines removed; "sessions" is how many builder-agent sessions the milestone is likely to take, each followed by its review. File names are in `crates/editor/src/` unless they start with another folder. "Findings" are the still-open findings of the code review that was paused for this phase, named by their ids (BUG-28, DEBT-05 and so on); the review's report in `docs/reviews/` lists them when it is finished.

### Part A: the look and the frame

#### M1. The look: tokens, fonts, one theme

**As built (2026-11-11), where the code differs from the plan below** (the code wins; the rest was built as written):

- The saved look is `InterfaceLook`, under the settings key `interface_look`: the editor already had a `Look` (how models are shaded in the viewport, saved as `viewport_look`), so the plan's name and key were taken. It lives in `interface_look.rs`; `tokens.rs` is the colours, with sizes and type styles in `tokens/`; the font table is `fonts.rs`.
- The saved panel arrangements (`panel_layout2`, `workspaces`, `library_pinned`) are still read and written: the dock is still there until M4.
- **Fonts:** all three new weights are bundled (Inter Medium and Bold from Inter's own release, the same one as the Regular and SemiBold already there; JetBrains Mono Bold from the family's own package). The font table still has one place where a weight can stand in for one whose file is missing. Nothing in today's editor is drawn in Medium or Bold yet: the type styles that use them (`tokens/text.rs`) are taken up as the frame and the screens are built.
- **The scene list's selected row is the accent at 22%**, not 16%: the design's first screen and the mock-up both say so, and a test samples it. 16% is for selected tiles, table rows and cards (Inventory d.4, corrected).
- **The kinds' colours (`theme::kind`) are unchanged**: taking only the design's seven would make Solis's sixteen hard to tell apart (a picture and game code, a sound and a sprite sheet would nearly match). They wait for decision D17, which is still open and blocks M7 (the Library), not M2.
- **Picture tests** are `picture_tests.rs` with `picture_support.rs` (the harness and Solis's own comparer), drawing through `egui_kittest`'s `wgpu` feature; approved pictures are compared on Linux, sampled colours everywhere. The side-by-side sheet is written by an ignored test. Both it and the editor's approved picture open the sample project `tests/fixtures/ui-sample/` (a small harbour named after the design's own sample content: a dozen entities, one component of game code, six files of different kinds), so later milestones' changes show on real content.
- egui's own controls can't take every shape of the design; what is left for the kit (M2) is listed in the pull request: the accent border of a focused field, number fields and drop-downs as dark fields, the checkbox's and toggle's own shapes, tooltips apart from menus.

**Built.** `tokens.rs`: every colour, size, radius and type style of Inventory part (d), `mix`, the hairline rule, the density table and the scale steps. The three new font files and their families. `theme.rs` re-made on the tokens: egui's own widgets (buttons, fields, checkboxes, menus, tooltips, scroll bars, windows) take the design's fills, borders, radii and text sizes, so today's whole editor (still docked, still with menus) turns Neutral dark in one step. The five accents. `Look` and its settings key, the old keys' one-time read, the fixtures (section 4). **Interface scale** and **Density** as two rows on today's Settings › Appearance page (they move in M9). The picture-test harness: `egui_kittest`'s `wgpu` feature, Solis's own small comparer (Inventory e.4), `picture_tests.rs` with the three kinds of check, `crates/editor/tests/pictures/` for approved frames, the sample project `tests/fixtures/ui-sample/`, and the side-by-side sheet.

**Replaces or changes.** `palette.rs` (removed), `theme.rs`, `settings_window.rs` (Theme and Rounded panels rows go; Scale and Density arrive), `source_rules_tests.rs`, `lib.rs`, `assets/fonts/` (+3 files) and `assets/README.md`, `crates/app/src/window.rs` (keys, mapping), `thumbnails.rs`, `gpu.rs`; `licenses/README.md`; `Cargo.toml` (the test feature) and `docs/dependencies.md` (its note). The settings fixtures go in `tests/fixtures/editor-settings/`; if `crates/app/src/fixtures_tests.rs` objects to a folder that isn't a project format, they go under `crates/app/tests/data/` instead. The 638 uses of `theme::…` colour functions in 49 files are **not** touched: `theme` keeps re-exporting the same names from `tokens`, and each later milestone moves its own files to the tokens' names.

**Placeholders.** None yet.

**Tests.** Contrast for every text-on-surface pair in the tokens (with the three listed exceptions, Inventory d.2); dark text on every accent; the accent mapping; the three settings fixtures; fuzz of `Look`; `symbols_have_glyphs` over the new families; hairlines land on whole pixels at 100, 125 and 150%; the first approved pictures (today's layout in the new look, so later milestones show as differences); the token sheet (every colour as a swatch) against the spec's tables.

**Manual.** `settings.md` (theme and accent rows, scale, density), `concepts.md` (accent colour), `launcher.md` and `interface-tour.md` (the sentences about themes).

**Risks.** egui's own widgets can be pushed only so far: some screens will look half-way until their milestone. Saved accents change colour for nine of ten old choices. Picture tests need a GPU or the software driver in CI on both systems: if Windows' runners can't, the colour and picture checks run on Linux only and skip with a note elsewhere, as the render tests do.

**Ask or mock up first.** Nothing left: D1, D3 and D18 are settled as recommended (section 7). The saved panel arrangements are still read and written in M1, since the dock is still there; they stop being carried over in M4.

**Findings.** DEP-01 (the game's font library): don't add the editor's new weights to the game's font set in `gpu.rs`. DOC-05: `src/README.md` rows for `palette.rs`, `theme.rs`, `tokens.rs`.

**Size.** 900 + 600 − 700; about 25 files; 2 sessions.

#### M2. The kit: icons, widgets, unfinished parts

**As built (2026-11-11), where the code differs from the plan below** (the code wins; the rest was built as written):

- **Icons: 127, not about 105.** The design's screens draw 58 different icons (two more are the same drawings with other attributes) and the approved sheets add 69. `icons/paths.rs` is one table made by one macro (the `Icon` enum, each icon's name, weight and parts); `tool_icon`, `paint_icon` and the canvas's `paint_tool` are gone, and each tool says which icon it has (`Tool::icon`, `PaintTool::icon`, `CanvasTool::icon`). The new icons are drawn in the design's style: square ends, sharp corners, the design's weights by size. A line that turns straight back on itself is cut at the turn (a corner that sharp would shoot out past the icon; the design's own drawings were made under a limit that does the same).
- **The kit** is as listed, plus `swatch.rs` (the accent's swatches). Each control is a function (or, for buttons with options, a small builder) that returns egui's `Response`. The focus ring, Space and Enter, and the arrow keys are in every one; `field.rs`'s editing is egui's own inside the kit's frame.
- **The mechanism is `unbuilt.rs` with a folder** (`unbuilt/table.rs`, `draw.rs`, and the test-only `sheet.rs`), so no file is long. A function that draws an icon takes the icon as an argument (`ribbon_big(ui, id, icon)`, `tool`, `icon_button`, `menu_item`, `page`): the table has no icons. `ribbon_row` also takes the kind of row (plain, a box to tick, a list). `page` reports whether its **Hide unfinished parts** link was clicked, for its caller to turn the setting on. What was drawn this frame, and the click the status bar reports, are kept with egui's context, so `Workbench` gained no field.
- **The first table has 173 lines:** the inventory's 164 PLACEHOLDER rows and 20 unbuilt pieces of PARTLY rows, less eleven that a test lists with why: the four of D12 (below), the code editor group and the three rows that belong to it (D11), and three pieces this phase builds itself or has nothing to show for (`shell.search.settings`, `shell.groups.second-area`, `debug.graph.select`). The PARTLY rows of Animate that are "a placeholder with no clips file open" have no id of their own yet: M11 gives them one.
- **D12 is taken as recommended for the four parts decided against** (the lead reviewer's call): a VR / XR template, Default game mode, Units and axes, and Players can rebind are not registered. The rest of D12 (parts with no phase are shown like any other, reading "not planned yet") is how the mechanism works; which of them become milestones is still for the rethink.
- **`Phase::Shared13`'s title is "Shared 13"**, not the roadmap's whole heading, which holds the program's name (rule 16). The test that checks titles against `ROADMAP.md` accepts a phase's number alone.
- **One unfinished part is on screen already:** the design's **Code editor split**, as a row of Settings › Appearance, so the whole path (table, drawing, hover, click, hiding) is proved in the real editor a milestone early. The whole-editor walk of section 3.4 checks against a list of the parts drawn so far (`DRAWN_SO_FAR` in `unbuilt_tests.rs`), which each milestone extends until it is the whole table less the parts with nothing to draw.
- **Hide unfinished parts** is reset by **Reset this page**, since it is on that page until M9.
- **The Settings window keeps a size of its own** (its page scrolls): a setting that changes the page's length would otherwise move the window from under the pointer.
- **A layout saved with the Layers panel** can't be read once `Panel::Layers` is gone, so the editor opens in the standard arrangement (such a layout could only come from a build before decision 0070). The dock goes in M4.
- **egui's own checkbox and focused field could not be brought closer through `theme.rs`.** egui gives buttons, fields and checkboxes one corner radius between them, and uses one colour for a selected row's words and a focused field's border; changing either for the checkbox or the field changes the others away from the design. They stay as M1 left them inside today's panels, and each screen gets the kit's own checkbox and field in its milestone.
- **Tests:** the whole-window test harness loads the editor's fonts (the kit asks for the design's weights by name, as the app has them before its first frame), and the picture harness applies the look one frame before it draws.

**Built.** `icons/` (`mod.rs` the `Icon` enum and `icon()`, `paths.rs` the table, `reader.rs` the path reader): the mock-up's 60 icons copied as path text, today's 45 redrawn in the same grid (Inventory e.3). `widgets/`: `field.rs` (text, number with an axis letter, dropdown), `toggle.rs`, `checkbox.rs`, `segmented.rs`, `chip.rs`, `pill.rs`, `button.rs` (plain, primary, light, dashed, icon, link), `card.rs`, `keycap.rs`, `slider.rs`, `tile.rs`, `rows.rs` (setting row, key-value row, section caps, column header), `dashed.rs`, each with its `WidgetInfo`. `unbuilt.rs` with the full first table, the drawing functions, the filter, and **Hide unfinished parts** as a row on Settings › Appearance and an action in the search. A **widget sheet** (a test-only page drawing every widget in every state) for the picture tests. `panels::placeholder` renamed `empty_note`.

**Replaces or changes.** `icons.rs` (489 lines, becomes the folder; `tool_icon`, `paint_icon` and `canvas_tools.rs`'s icons become table entries), the 19 files that call `icons::`, `panels.rs` and the 12 call sites of `placeholder`, `settings_window.rs`, `search.rs` (the new action), `panel.rs` (`Panel::Layers` and `NOT_BUILT` removed; a saved layout naming it still reads until M4 removes the dock).

**Placeholders.** The mechanism itself. Nothing is drawn with it yet except on the widget sheet.

**Tests.** Section 3.4's five; every icon at 12, 15 and 18 px stays in its square and isn't empty; the reader fuzzed; each widget: its measured size against the tokens, its states on the sheet, its label reachable by `egui_kittest`; the widget sheet against sampled colours from the mock-ups (the toggle from Screen 8, the keycap from Screen 13, a card from Screen 1).

**Manual.** A new page, `unfinished-parts.md` (what the mark means, the setting); `settings.md`; `concepts.md` (placeholder, ribbon, shelf: terms the next milestones use).

**Risks.** The 45 redrawn icons are a matter of taste: the sheet goes to the lead first. Custom widgets lose egui's keyboard handling unless each is given focus and key support (Tab, Space, arrows): budget for it.

**Ask or mock up first.** D2 and the placeholder mock-up; the icon sheet (mock-up round 1): both settled. (D17, the colours for Solis's kinds of file, was first listed here; it is still open and waits for M7, where the kinds are shown as chips.)

**Findings.** DEBT-03 (six copies of one drop-down in `inspector.rs`): `widgets::field::dropdown` is written to be the one; the copies go in M5. DOC-04: placeholder texts name phases only through `Phase::title()`.

**Size.** 2,100 + 900 − 500; about 30 files (25 new); 3 sessions.

#### M3. The top: tab bar, ribbon, command search, the File page's frame

**As built (2026-11-11), where the code differs from the plan below** (the code wins; the rest was built as written):

- **Files.** `shell/` has `mod.rs`, `commands.rs` with `command_words.rs` (the table split in two so neither is long), `tab_bar.rs`, `ribbon.rs` (the table types and the layout) with `controls.rs` (the drawing of working controls), `ribbons/` (also `shading.rs`, `vfx.rs`; `mod.rs` hands out a tab's table), `play_group.rs`, `file_page.rs` and `create.rs`. `search.rs` became `search.rs` and `search/` (`list.rs`, `score.rs`, `draw.rs`).
- **Tabs and workspaces.** Home is Level, Animate is Animate, UI is UI, **Debug is the Code workspace** with the Profiler brought to the front; Library and Audio show Level with their panel in front; Build keeps whatever was shown. The dock's arrangements are still saved as before; the shell's own saved state (`ShellState`) comes with M4, so **whether the ribbon is collapsed is not remembered between runs yet**.
- **A ribbon control is called, for screen readers and tests, by its command's name** ("Add entity" for the button that says Empty; "Save area" for Save), and a tab is called "Home tab". The words drawn are the design's.
- **Create.** Empty, Model (runs Import model…), Sprite and Reverb zone (two new one-step makers, `shell/create.rs`), Light (point light; its list has the spotlight), Camera, Tile map (a new tile map file in the Library's folder), More (Reverb zone, Assembly from selection). The unfinished button in Volume's place is the registered **Trigger zone**, whose name is too long for the button and is cut off; pointing at it says it whole. A button with a list carries a small arrow, which the approved picture (drawn before the project lead's answer on it) doesn't show.
- **Library's, Animate's and Audio's ribbons** hold only Import and the design's unfinished controls. Their working controls (the Library's + New, view and kind filter; a clip's speed and direction; Add bus) stay on their panels until M7, M11 and M13 rebuild those screens: moving them now would have meant designing those ribbons without their mock-ups. **UI's ribbon** has only the Play group, for the same reason (mock-up round 3).
- **The Play group is on every tab** (the plan says "editing tabs only"): until M4 it is the only Play button there is.
- **The File page** shows the launcher's three pages in its page area (the launcher's own window still stands at start-up until M8, with its own sidebar). The column has one item the design lacks: **Input**, under Project settings, which opens today's Input actions window until M9 makes it a sub-tab. Updates and About are the two added items approved with mock-up round 2.
- **Contextual tabs** appear for an open tileset, a picture's crops and a selected tile map, and bring that panel to the front. They have no ribbon of their own yet (mock-up round 3). An open animator has none: Animate is its home.
- **The document tabs** of an open animator or tileset left the top with the old bar and come back in M4's tab strips; until then each is its own docked panel's tab.
- **The play tint** went whole in this milestone: the bars' tint and the window's coloured edge. The accent border round the viewport and the Game panel while playing (`panels.rs`) goes in M4, as the plan says.
- **Restart to update** is a link at the tab bar's right, left of the project's name, until the status bar's slot exists (M4).
- **Show ribbon labels** (a field of the saved look) has no effect yet: its switch arrives with Preferences (M9).
- **Tests.** The test window draws nothing in the frame it is built in (the editor's fonts arrive with the next, `test_support::fonts_ready`). `shell_reach_tests.rs` also holds the shortcuts-table test: each key in `ShellCommand::keys` is pressed and held against the command of that name.

**Built.** `shell/` begins: `mod.rs` (`Shell`, `RibbonTab`), `commands.rs` (`ShellCommand`: every action the old menus had, each with its name, shortcut, whether it can run now and why not, and what it does; `search.rs` lists from it, so there is still one table of shortcuts, rule 21), `tab_bar.rs` (logo, tabs, contextual tabs, the search box with its "Space" hint, the project / area path, the collapse chevron), `ribbon.rs` (the drawing) and `ribbons/` (one small table per tab: `home.rs`, `library.rs`, `animate.rs`, `audio.rs`, `debug.rs`, `build.rs`, and the unbuilt `shading.rs` and `vfx.rs`), the **Play** group (Play turning into Stop; Pause and Step while running; nothing else changes colour while the game runs: there is no play tint, the project lead's answer 9), `file_page.rs` (the menu column and an empty page area). The File menu column's items work from the first day by opening what exists: Recent projects shows today's launcher page, Preferences today's Settings window, Project settings its 2D page, Input today's Input actions window, Learn and Updates the launcher's pages, Exit quits.

The ribbon tabs switch what today's workspaces switch: Home is Level, Animate is Animate, and Library, Audio and Debug bring the Library, Mixer and Profiler panels to the front of the dock. The dock is still in the middle.

**Replaces or changes.** `top_bar.rs` (671 lines: the menu bar, document tabs and Play buttons go, and with them the bar's blue and amber tint while playing; what's left moves to `shell/`), `menu_items.rs` (its commands move into `ShellCommand`; the Area panel's right-click menus keep using them), `workspace.rs` (`workspace_tabs` goes; the Code and UI workspaces become Debug and a temporary UI tab), `search.rs` (854 lines: split into the list, the scoring and the drawing while it is open), `workbench.rs:880` (`Panel::top("top_bar")` becomes the tab bar and ribbon), `updates.rs` (`restart_button` moves to the status bar in M4; until then it sits at the right of the tab bar), `test_support.rs` (helpers `ribbon_tab`, `run`, `file_item`, so tests stop clicking through menus), about 60 tests in `workbench_tests.rs`, `search_tests.rs`, `settings_window.rs`, `workspace_tests.rs`, `key_owner_tests.rs`; `tools/smoke/window-smoke.sh` (File > Quit becomes File › Exit; positions).

**Placeholders.** `shell.tab.shading`, `shell.tab.vfx` (tab, ribbon, a placeholder page), `shell.play.simulate`, and every unbuilt ribbon control of Inventory b.1 to b.6 and b.10 to b.11 (about 75 ids).

**Tests.** `shell_reach_tests.rs` (every old menu item's command runs from its new place); the ribbon's measured layout (66 px, groups, 50 × 44 buttons, 20 px rows); it scrolls instead of wrapping at 900 px wide; collapse and expand; contextual tab appears and disappears with an open tileset; the unbuilt walk (3.4) now has real ground to cover; sampled colours for the tab bar and ribbon from Screens 1, 3 and 6; the shortcuts table test.

**Manual.** `interface-tour.md` (the top bar, the menus, workspaces: rewritten for tabs and ribbon), `keyboard-shortcuts.md` (search), `play-mode.md`, and every page that says "File >", "Edit >", "Help >" for the items moved (about 20 pages; one sentence each).

**Risks.** Tests: this is where most of the 467 label look-ups meet changed labels. Mitigation: the `test_support` helpers land first, in the same pull request, and tests are moved onto them before the menus are removed. The dock in the middle with the ribbon above will look unfinished for one milestone.

**Ask or mock up first.** Settled: D4 (Solis's words), D5 (today's logo in the tile), D6 (a UI tab), D10 (the ribbon collapses to its tabs), D13 (no play tint). Still to ask: D15 rows 13 and 24 (long press, wheel over the ribbon). Mock-up round 1 is approved for the Play group with Pause and Step, the Edit group (Undo, Redo) and the Create group for Solis's kinds; Debug's Session group was approved with the rest of the round (2026-11-11).

**Findings.** DEBT-05: `Shell` is its own type; no new fields on `Workbench` beyond it. DEBT-08: `search.rs` is split, not grown. TEST-03 (the smoke test's fixed sleeps): it must be edited anyway; replace the sleeps with waits for a window title or pixel while there. BUG-28 (`search.rs:702` walks the whole area each frame while open): don't carry it into the new list; cache by the document's revision.

**Size.** 2,200 + 1,500 − 1,100; about 45 files; 3 sessions.

#### M4. The middle: editor groups, the shelf, the status bar

**As built (2026-11-12, in the pull request for Part A, answer 36), where the code differs from the plan below** (the code wins; the rest was built as written):

- **Files.** `shell/workspaces.rs` (the table: `groups`, `shelf_tabs`, `ShelfTab` and its `columns`, `OpenNow`, `Arrangement`, `home_of`, `tab_for`), `shell/groups.rs`, `shell/shelf.rs`, `shell/status.rs` (moved from `status_bar.rs`), `shell/saved.rs` (`ShellState` under the settings key `shell`). `layout.rs`, `workspace.rs` and `library_drawer.rs` are gone with their tests; `code_uses` moved to `workbench_library.rs`. Decision record **0104, "Docking is Solis's own"**.
- **The shelf tabs, in Solis's words:** Home: Entity (Area · Properties with Hints under it · Library), Animation (unfinished), Console, Problems, and Map while a tile map is selected. Library: Details (Properties, which shows the file picked, or a tileset's terrains while the Tileset is in front), Code, Import queue and History (unfinished). Animate: Sequencer (unfinished), **Frames** and **Clips** while a clips file is open (each with a × after it that closes the file: the design has no way to close one until M11 gives it a document), **Properties** (the animator's state picked, or the entity), Key (unfinished), Console. UI: Screen (Elements · Properties), Console. Audio: Bus (Properties), Snapshot (unfinished), Console. Debug: Console, Problems, Hottest functions and Memory snapshot (unfinished). Build: as Home. There is no separate shelf tab for a tileset's or crops' Properties: Details shows them.
- **Groups.** Home: the area (its tab named by the area, as the design's is), and the Game beside it while playing, brought to the front on Play. Library: the Library, and a Tileset or Atlas Crop as a second group while open. Animate: the viewport, and an Animator as a second group. UI: the Canvas, always (a × while a screen is open). Audio: the Mixer. Debug: Game and the viewport, beside the Profiler. Opening a file's document brings its tab to the front (`Workbench::show_panel`). The area's tab has the **+** and right-click menu of mock-up round 1; Close on it is greyed, with why on hover (one area is open at a time).
- **Not built here:** "**Shelf opens on**" (selecting opens a chosen shelf tab): on its own it took the Console away from a command that selects, so it waits for its setting in Preferences (M9); a tile map selected still brings Home's Map tab to the front, as the Map tab came to the front before. The unsaved dot on a document tab (spec 124) is still only in the tab bar's path.
- **The status bar:** a dot and Ready (red, with the count, while the code has mistakes), the branch (unfinished), the counts as two dots (red: the code's mistakes; amber: the area's problems), then the context (Playing or Paused, saved or not, the area and its 2D size), the version in mono and Restart to update (moved from the tab bar). It is exactly 24 tall: egui's panel line is off, and its one line is drawn along its top.
- **Sizes.** `SHELF_H` (300) is the shelf's content under its bar, not "bar and all" as M1's comment said: that leaves the center 442 tall at 1440 × 900, inside the design's 400 to 470, and the content inside its 290 to 360. New tokens: `SHELF_MIN_H`, `CENTER_MIN_H`, `SCENE_COLUMN_W` (240), `ASSETS_COLUMN_W` (300), `COLUMN_MIN_W`, `SPLIT`, `SPLIT_LIBRARY`, `GROUP_MIN_W`, `DIVIDER_REACH`.
- **Commands.** Pin and Unpin the Library are gone; **Fold the shelf** (Ctrl + Space) joined the one table of shortcuts, so the shortcuts test presses it too.
- **Tests.** `shell/middle_tests.rs` (the five bands at both sizes, the grip and its limits, Ctrl + Space while typing, the divider, stacking and wrapping, the area tab's menus, the Game tab, each tab's state across a restart), `shell/saved_tests.rs` (round trip, repair, a fuzz of 3,000 damaged files), `shell/middle_picture_tests.rs` (the shelf's and status bar's colours against the tokens and Screen 1, folding, approved pictures of the Library, Animate, UI and Audio tabs; Home is `editor.png`; Debug's isn't kept, its Profiler shows this machine's times). Tests of the dock, the drawer and the workspaces went with them; tests of things that stayed were moved onto the frame (`test_support`: `rest_over`, `body_of`, `click_the_x`, `has_panel`, `in_view`, `roomy_shelf` for tests that click far down Properties, which is short in Home's shelf).
- **Review finding BUG-28** is fixed where the code moved: which entities use each code file is cached by the area's revision, the sources' count and the build of the game's code.

**Built.** `shell/groups.rs` (one or two groups, document tabs or a header, the divider and its ratio, stacking when narrow, the tab strip's right-end slot), `shell/shelf.rs` (bar, grip, tabs, right-hand tools, wrapping columns, fold with Ctrl + Space, "opens on"), `shell/workspaces.rs` (for each ribbon tab: which documents its groups hold and which shelf tabs and columns it has), `shell/status.rs` (from `status_bar.rs`: the Ready dot, counts, context text, the version, the update slot), `shell/saved.rs` (`ShellState`, restore, the Console's one-time note). Every `Panel` is given a home in the new frame **as it is drawn today**:

| Ribbon tab | Center | Shelf tabs → columns |
| --- | --- | --- |
| Home | Viewport (and Game while playing) | Object → Area · Properties (with Hints under it) · Library (thumbnails); Console; Problems; **Map** while a tile map is selected |
| Library | Library (full) | Details → Properties showing the picked file; Code |
| Animate | Viewport, and Animator when one is open | Frames; Clips; Console |
| UI (temporary until M12) | Canvas | Screen → Elements · Properties; Console |
| Audio | Mixer | Bus → Properties showing the picked bus; Console |
| Debug | Game or Viewport, Profiler | Console; Problems |
| Tileset and Atlas Crop | Open as a second group beside the Library's center | Their Properties as a shelf tab |

**Replaces or changes.** `layout.rs` (443 lines) and `workspace.rs` (406) and `library_drawer.rs` (154), removed with their tests; `panels.rs` (`TabViewer` goes; `PanelViewer::ui` becomes a plain `draw_panel`; the tab titles and icons move to `shell/workspaces.rs`); `theme.rs` (`dock_style`, `dock_frame` and the card constants go); `status_bar.rs` (moves, without its tint while playing); `panels.rs` `playing_border` (the coloured border round the viewport while playing: removed, the project lead's answer 9); `workbench.rs` (`dock`, `other_layouts`, `in_front`, `library_pinned`, `drawer_open` leave `Workbench`; `show` loses the dock section, about 300 lines); the 34 files that mention the dock (`workbench_animator.rs`, `workbench_atlas.rs`, `workbench_frames.rs`, `workbench_screens.rs`, `workbench_tiles.rs`, `workbench_tileset.rs`, `play.rs`, `project_state.rs`, `mixer_view.rs`, `search.rs` and their tests): `layout::open_beside`, `bring_to_front` and `close` become `Shell::open_document`, `show_shelf_tab`, `close_document`; `key_owner.rs` (unchanged in logic; its tests' set-up changes); `Cargo.toml`, `crates/editor/Cargo.toml`, `docs/dependencies.md` (egui_dock removed) and `deny.toml` (the advisory exception kept only for a helper egui_dock used goes with it); `crates/app/src/window.rs` (`LAYOUT_KEY`, `WORKSPACES_KEY`, `LIBRARY_PINNED_KEY` no longer written; `SHELL_KEY`); `tools/smoke/window-smoke.sh` (it drags a panel and checks it comes back: becomes dragging the shelf's grip and the split).

**Placeholders.** `shell.status.branch`; the unbuilt shelf tabs (`home.shelf.animation`, `lib.shelf.import-queue`, `lib.source.history`, `anim.shelf.sequencer`, `anim.shelf.key`, `debug.shelf.hottest`, `debug.shelf.memory`, `audio.mixer.snapshot`'s tab) and `home.shelf.transport`, `home.shelf.frame-counter`.

**Tests.** Measured layout of the five bands at 1440 × 900 and 1100 × 700; columns wrap and groups stack at narrow widths; the grip resizes within limits; Ctrl + Space folds (and works while typing, as today's key did); each workspace remembers its tab, height and split across a restart; fuzz of `ShellState`; the three old settings fixtures start with the standard shell and say so once; every `Panel` is reachable (`shell_reach_tests.rs` grows); `key_owner_tests.rs` passes unchanged in meaning (each owner, each key); files-that-come-with-a-panel open and close as before (`workbench_*_tests.rs`); the smoke test; sampled colours for the shelf bar, columns, status bar from Screens 1 and 3; a first whole-screen approved picture per ribbon tab.

**Manual.** `interface-tour.md` (panels, docking, the Library drawer, the status bar: rewritten), `keyboard-shortcuts.md` (Ctrl + Space; the Panels table), `projects-and-the-library.md` (the drawer), `concepts.md` (docking, layout, workspace change meaning), `settings.md` (Reset layout).

**Risks.** The largest single change: 34 files, the dock's removal, saved layouts dropped. Three guards: (1) the reach test; (2) the panels' *contents* are not touched in this milestone, only where they sit; (3) the split of work inside the pull request: groups and shelf first with the dock still compiled, the switch second, the removal last, each a commit that passes. Wide panels in narrow columns (Properties at 260 px beside a tree) will look cramped until M5.

**Ask or mock up first.** Settled: D14 (the Game is its own document tab). Still to ask: D12 (what to do with placeholders that have no phase: it decides what the unbuilt shelf tabs say). Mock-up round 1's pictures of the document tab's "+" and right-click menu, the Console's search, Clear and Copy beside the filters, and the shared dialog card were approved with the rest of the round (2026-11-11).

**Findings.** DEBT-05: `show` gets shorter, the shell's state is out of `Workbench`. BUG-28 (`workbench.rs:1257` walks the whole area every frame for the panels): the Scene column and status bar read counts cached by revision; fix it here, where the code is being moved anyway. BUG-19 (tile caches shared by every view and the Game panel evict each other): the Game becoming a tab beside the viewport must not add a third user; leave the caches alone. DOC-05 and DOC-02: `src/README.md` and `ARCHITECTURE.md` lose the dock's rows and statements. TEST-03 again.

**Size.** 1,800 + 1,400 − 2,000; about 60 files; 4 sessions.

### Part B: the everyday screens

#### M5. Home: the level editor (Screen 1, spec 166-234)

**As built so far (2026-11-12, in the pull request for Part B; partly built, the rest listed below)** (the code wins; the rest was built as written):

- **Files.** `toolbox.rs` (the group's column of tools; it scrolls, so a tall shelf never cuts it), `viewport_pills.rs` (a child module of `viewport.rs`: the pills), `properties_cards.rs` (the card row, a card's title row and menu, the Hints card), `assets_column.rs` (Home's Assets column), `widgets/menu.rs` (the menus' rows), `home_screen_tests.rs`. `widgets::field::framed` puts the kit's field box round any of egui's editing, which is how Properties' fields became the kit's without losing `settled.rs`'s "counts once let go".
- **The toolbox** holds the five transform tools, a selected map's painting tools (until M10 gives them a contextual ribbon) and the four unfinished tools. A map's Exact, Art or Meaning and its layer, which sat on the old tool bar, are a small strip at the view's bottom left while a map is selected (also M10's to move).
- **The pills:** each view has its own **Perspective ▾** (with how many views, and Maximise or All views while there are several), **Standard ▾** and **Show ▾**; the view in use has the frame time (the counts on hover: the design's pill has room for the time alone) and the bottom-right **Lock X Y Z**, **World ▾** (World or Object, key L) and the camera's speed. The ≡ menu is gone. Bands of light in the look's list step to the next number on a click.
- **Snapping's steps are on the ribbon:** a click on the value after "·", or a right-click on the row, opens the steps (spec 172: "clicking the value edits the step"; the right-click follows answer 18's lists). `Picked` carries a step back from the ribbon beside a command. The status bar says "Snap 0.25 · World" (or "Snap off", "Snap whole pixels" in 2D).
- **The Area column:** the header's search, Fold everything and ⋯ (put in the header by the shelf: `PanelViewer::column_tools`); rows `row_h()` tall, the selected row the accent at 22% across the column; hide and lock while pointed at; a row's right-click menu (answer: round 2's `05-area-column-menu`) selects the row first. The panel's menu row and a model row's × are gone (`MenuState` went with them).
- **Properties:** the header is drawn in the column header's place (`Column::own_header`); the components are cards, as many across as fit at 180 px (`properties_cards::across`), each row as tall as its tallest; the card's ⋯ menu is also its right-click menu; a game code card's second line is its file and **Open code**. The Model card's **Material** and **Shadows** are unfinished parts (`home.object.model-material`, `home.object.model-shadows`, two new table lines and inventory rows). **Hints are the last card** (`Panel::Hints` is gone); **Turn hints off** says for three seconds in the status bar where they come back (`shell::status::tell`).
- **Assets** is a new `Panel::Assets` on Home's Entity tab, in the Library's place; the whole Library is on the Library tab. Its folder is the Library's (one folder, not one per place: simpler, and the Library tab opens folders); choosing a placeable file in the search shows it there.
- **The Console's rows** are the design's (time, level word, message; an error's and a warning's row tinted).
- **Finished in the same pull request:** Problems' rows are the design's (spec 228-230: the level's icon, what's wrong, and in grey where: the file and line, or the area); the count sentence over them went (the shelf tab's name has the count). **F2 and Delete over the Library or the Assets column** act on the file picked there (answer 25; `Workbench::file_keys_over_the_library`); everywhere else on the entity, as before. **Dragging a number's name** changes it as dragging its value does (answer 25; `field_rows.rs`, split from `inspector.rs` with the rows' marks), one undo step per drag. **A field changed while playing** keeps the accent border and "↺ value" to put back its value from before Play (`Playing::yours`, noted when Properties sets a field); the note says how many changes so far. `properties.rs` is split three ways (`properties_header.rs`: the header, its menus and the notes under it; the cards' extras in `properties_cards.rs`), each under 500 lines.
- **Not done (not needed for the screen to match; left on purpose):** the 10,000-entity frame-time case; dropping a tile on a card's field to assign it (`home.object.assign-drop`); the "just edited" border that stays after the edit (`home.object.edited-border`); BUG-27 (`text_view.rs` parses Markdown every frame). Also left: the Game tab's pills (round 2), the measured layout of the pills and cards against spec 183-216, and Screen 1's sampled colours and side-by-side sheet.

**Built.** The toolbox column (Select, Move, Rotate, Scale, Transform, then the four unbuilt tools). The viewport's pills (view, look, Show, stats; camera speed and 2D zoom at the bottom right) in place of the corner controls and the ≡ menu. The **Scene** column (count in the header, search and fold as header icons, hide and lock on hover, 24 px rows, the accent-mixed selected row). The **Inspector** column: the header (kind icon, name, kind · parent, dashed **+ Component**), components as wrapping cards with the kit's fields (axis letters, the accent border on a focused or just-edited field), the card's ⋯ and right-click menu, Hints as the last card. The **Assets** column (four-column tiles of the current folder, the Import tile, drag to place, drop on a field to assign). Console and Problems in the design's rows (time, level word, message; row tints). The **Game** document tab with its pills. The status bar's "Snap 0.25 · World".

**Replaces or changes.** `viewport.rs` (1,798 lines: the corner controls and menu, `viewport.rs:1345-1560`, move to a new `viewport_pills.rs`), `viewport_tools.rs` (the tool bar and snapping row leave it), `workbench_tools.rs`, `area_panel.rs` (760), `properties.rs` (858) and `inspector.rs` (933), `hints.rs`, `console.rs` (551), `problems.rs` (813: the drawing only), `game_panel.rs`, `day_colours.rs`, `sound_view.rs`, `text_view.rs` (restyled where they show in the Inspector), `shell/ribbons/home.rs` (Snapping reads and sets the steps), `shell/workspaces.rs`.

**Placeholders.** `home.tool.pose`, `.motion-path`, `.add-key`, `.measure`; `home.lighting.*`; `home.shelf.animation`; `home.shelf.transport`; `home.code` (not drawn unless D11 says otherwise); `home.create.shapes`, `home.create.trigger`; `home.console.line-link`.

**Tests.** `properties_tests.rs`, `viewport_tools_tests.rs`, `transforming_tests.rs`, `workbench_tools_tests.rs`, `workbench_panels_tests.rs`, `two_d_tests.rs`, `day_colours_tests.rs`, `play_tests.rs` (set-up changes; meanings kept); measured layout of the toolbox, pills, columns and cards against spec 183-216; cards wrap at 180 px; a 10,000-entity area keeps its frame time (the scale check's editor case); Screen 1's sampled colours and approved picture; the side-by-side sheet for Screen 1.

**Manual.** `selecting-and-moving.md`, `navigating-the-viewport.md`, `entities.md`, `console.md`, `problems.md`, `play-mode.md`, `undo.md`, `2d-pixel-perfect.md`, `looks.md`, `interface-tour.md` (Home).

**Risks.** The Inspector is the most-used surface and is all custom fields after this; every kind of field (`inspector::fields`) must keep its "counts once let go" behaviour (`settled.rs`, review fix BUG-20). Several views (two, four) inside a group with a toolbox need room: the toolbox belongs to the group, not to each view.

**Ask or mock up first.** D15 rows 10, 15, 16 (F2 and Delete by where the pointer is; label scrubbing). Mock-up round 2: the Hints card, the third snapping row and the bottom-right pill, the Game tab's pills.

**Findings.** DEBT-03 fixed here (one dropdown). DEBT-09: `viewport.rs` gets smaller, not larger. DEBT-08: `properties.rs` split into header, cards and menus if it would pass 500 lines. BUG-28 (`inspector.rs:168`): the Inspector reads the selection's components, not the area. BUG-27 (`text_view.rs` parses Markdown every frame): the text view is moved here; cache the parsed blocks by the file's text while moving it.

**Size.** 1,500 + 800 − 900; about 20 files; 3 sessions.

#### M6. Home: placing (Screen 2, spec 236-287)

**As built (2026-11-12, in the pull request for Part B)** (the code wins; the rest was built as written, except where "not built" says):

- **Files.** `placement.rs` (the Placement column; where a drop lands; the label, footprint and hint strip while a file is dragged over a view), `placing_tests.rs`, and the approved picture `placing.png`. `ShelfTab::Place` and `Panel::Placement` are new; the Area and Assets columns are on Place too, but their home stays Entity (`home_of` skips Place), so showing either never starts placing, and Home still opens on Entity.
- **The mode** is `Workbench::placing()`: Home's (or Build's) Place shelf tab in front. The ribbon's build form is Edit, **Place** (Create's buttons), Snapping with **Snap to surface** unfinished beside it, **Paint** (Brush, Scatter: unfinished) and **Transform** (Select, Move, Rotate, Scale, All three: `ShellCommand::UseTool`, with the tools' keys in the one table); the Play group follows as on every tab. The Edit group stays, as round 2 drew it. The viewport has no toolbox while placing.
- **Where a drop lands:** on the surface under the pointer, as before, then on the grid's step in X and Y when the ribbon's Grid is on and **Align to** is **Grid**; **Free** (`Snapping::place_free`, saved with the snapping, defaulting to Grid) and **Ctrl** while dropping (answer 25) never snap. Shift (keep placing) and R (turn the ghost) are unfinished chips on the hint strip (`place.keep-placing`, `place.rotate-ghost`).
- **While a file is dragged over a view:** the footprint (the grid cell, or a metre square, in the accent at 16% with its outline at 80%), the label beside the pointer ("crate", "x 2.75  y -13.25 · grid") and the hint strip; the status bar says "Placing crate · Snap 0.25 · Grid". The Assets column's tile being dragged is dimmed with the dashed accent border and the "dragging" badge.
- **The Place tab's Assets column** is the Assets column in its placing form: Favourites and Recent (unfinished), All, Models, Sprites, Maps (today's chips that hold placeable kinds, with their kinds' dots), the search ("Find something to place": the Library's own search, so it finds files in every folder) and **Open Library**; only placeable files, tiles of at least 92 px, no Import tile.
- **Not built:** the ghost's bounds box (a picture of the model's box needs its size, which the editor has only once it is placed); the snap guides and drag trail; the Area column's "(placing)" row and its "New entities go into" footer (`place.scene.target`, unfinished: not drawn); Lights as tiles to place.

**Built.** The **Place** shelf tab (Scene with the "(placing)" row; Placement; Assets with chips, search, Open Library, 92 px tiles and the "dragging" badge). The Home ribbon's build form while Place is in front (Place, Snapping, Paint, Transform, Play) and no toolbox. Over the viewport while a file is dragged: the bounds-box ghost, the footprint, the snap guides, the label with live coordinates and what it snaps to, the hint strip. The status bar's "Placing … · Snap …". Library › Place and the Assets column's double-click both end here.

**Replaces or changes.** `workbench_library.rs` (placing what's dropped), `viewport.rs` ("Drop to place"), `viewport_tools.rs` (overlay painting), `screen.rs` (projecting the bounds), `shell/ribbons/home.rs`, `shell/workspaces.rs`, `library.rs` (the tiles' drag).

**Placeholders.** `place.snap.surface`, `place.paint.brush`, `place.paint.scatter`, `place.scene.target`, `place.align.surface`, `place.rotation`, `place.random-yaw`, `place.scale`, `place.physics`, `place.chip.favorites`, `place.chip.recent`, and the hint strip's chips for keys not yet agreed.

**Tests.** `workbench_library_tests.rs` (placing still places where pointed, snapped as the ribbon says, selected afterwards, one undo step); Esc and dropping back on the shelf make nothing; the ghost's footprint is at the snapped cell; the ribbon swaps with the shelf tab and back; with **Hide unfinished parts** on, the Placement column shows only Align (Grid, Free) and the Paint group is gone; Screen 2's layout and colours.

**Manual.** `projects-and-the-library.md` (placing), `selecting-and-moving.md`, `assemblies.md`, `sprites.md`, `tile-maps.md` (placing a map), `importing-models.md`.

**Risks.** Most of this screen's options are placeholders (12 of 27): with them shown, the Placement column is mostly marks. That is the design's doing; D12 may trim it.

**Ask or mock up first.** D16 (how the mode is entered), D15 rows 6, 7, 8, 17, 18 (Shift, Alt or Ctrl, R, click-to-place, what double-click does).

**Findings.** None open in this code. BUG-19: the ghost is painted over the picture, never as a new view.

**Size.** 900 + 500 − 200; about 12 files; 2 sessions.

#### M7. Library (Screen 3, spec 289-333)

**Built.** The Library ribbon (Add, View, Filter, Source control, Use). The browser: Back and Up, the breadcrumb with its dropdown (which is where the folder tree lives now), kind chips with Solis's kinds, search, the tile grid (108 px, folders as tiles, the selected tile's look), list view. The shelf: **Details** (a preview box and the facts for each kind: what Properties showed for a picked file), **Used by** (code files' uses; "shared by"), the count at the right. In-place rename. The first real contextual-tab users are M10's; here the mechanism gets its document tabs and **‹ Back to Library**.

**Replaces or changes.** `library.rs` (1,665 lines) becomes `library/`: `mod.rs` (state and requests), `browser.rs`, `grid.rs`, `list.rs`, `details.rs`, `menus.rs`; `workbench_library.rs` (710); `sound_view.rs`, `sound_event_view.rs`, `sheet_view.rs`, `text_view.rs` (they draw into Details); `workspace.rs`'s `code_uses` moves to `library/details.rs`; `shell/ribbons/library.rs`.

**Placeholders.** `lib.filter.tags`, `lib.filter.missing`, `lib.source.check-in`, `lib.source.history`, `lib.use.show-in-level`, `lib.shelf.import-queue`, `lib.import.any-kind`, `lib.preview.modes`, `lib.used-by.all-kinds`.

**Tests.** `workbench_library_tests.rs` (601 lines) and the Library's own tests, moved onto the new drawing with meanings kept (new files, moves, renames, deletes, right-click menus, what may open in another program: `open_externally_tests.rs`); the grid reflows; a folder of 5,000 files keeps its frame time (only rows in view are drawn, as today); Screen 3's layout, colours and picture.

**Manual.** `projects-and-the-library.md` (rewritten), `sounds.md`, `sound-events.md`, `sprites.md`, `importing-models.md`, `game-code.md` (code files and their uses).

**Risks.** Two Libraries now exist (the tab and the shelf's Assets column) over one state: decide once which parts are shared (the folder and the search are per place; the selection is one).

**Ask or mock up first.** **D17** (the colours for Solis's kinds of file: still open, and this milestone can't start without it); D15 rows 10, 18, 19, 20. Mock-up round 2: Solis's kinds as chips (in D17's proposed colours, so the project lead decides on the picture); Details for a sound, a sprite sheet and a text file.

**Findings.** DEBT-08 (`library.rs`) is answered by the split. BUG-31, BUG-32, BUG-53 (file work on the window's thread when assets change): the new browser asks for nothing new from the app per frame; the facts it shows come from what the app already holds.

**Size.** 1,600 + 900 − 1,300; about 20 files; 3 sessions.

#### M8. File: projects, Learn, Updates (Screen 7, spec 531-616)

**Built.** `file_pages/recent.rs` (header with search, sort, Open… and New project; project cards with pin, build mark, missing-folder state and the right-click menu; the template strip), `learn.rs`, `updates.rs`, `about.rs`. Starting up: Solis opens on File › Recent projects in the editor's own window; the other ribbon tabs are dimmed until a project is open; **Back to editor** and Esc return. The shared dialog card for Unsaved changes, Changed on disk, Keep or Undo all, and the project rename. The status bar's update slot.

**Replaces or changes.** `launcher.rs` (1,551 lines, removed; its state `Launcher`, `KnownProject` and helpers `shown`, `ago` move to `file_pages/`), `launcher_tests.rs` (634), `splash.rs` (restyled; the dead update question removed), `updates.rs`, `workbench.rs` (`launcher()`, `prompts`, `about`, the three `egui::Window` dialogs at 1643, 1691 and 1762), `play_changes.rs` (its `Modal`), `crates/app/src/window.rs` (the small launcher window, `LAUNCHER_SIZE`, and growing to the editor's size on open: gone if D7 says so), `crates/app/src/startup.rs`.

**Placeholders.** `file.plugins`, `recent.card.thumbnail`, `recent.card.badge`, `recent.template.first-person`, `.top-down`, `.platformer`, `.vr`, `.all`.

**Tests.** Everything `launcher_tests.rs` checks today, on cards (open by Enter or double-click, pin, sort, search by words in any order, rename, copy, remove, locate, scan, the build marks, a folder that's gone, the update notice, "always choose the project" at start, closing the editor's window returns to the page with the project still open); the questions over the File page (`play_changes_leaving_tests.rs`); Screen 7's layout, colours and picture; fuzz of nothing new (the known-projects list keeps its format).

**Manual.** `launcher.md` (becomes "Projects": rewritten), `getting-started.md`, `installing-and-updating.md`, `first-game.md`, `first-platformer.md`, `ui-sample.md`, `benchmark.md` (where the examples are made from), `backups-and-upgrades.md`.

**Risks.** Decision 0090 ("always through the launcher") is the lead's own strong wish: the page must keep its two promises (a choice at every start; nothing reopened by itself) while losing its separate window. The update flow (download, restart to update) crosses the app and the installer: test it on an installed copy by hand ("How to test").

**Ask or mock up first.** D7, D8. Mock-up round 2: Learn, Updates and About; the card's extras (pin, missing folder, sort, Scan a folder…). D15: Left and Right on the card grid.

**Findings.** DEBT-08 (`launcher.rs`) and DEBT-01 (the splash's dead update question) end here. DOC-06 (`RENAMING.md` misses places): check that nothing in the new pages writes the name; About and the engine-version row read `AppInfo`.

**Size.** 1,700 + 900 − 1,600; about 18 files; 3 sessions.

#### M9. File: Preferences, Project settings, Input (Screens 8, 12 and 13, spec 618-674 and 909-1062)

**Built.** `file_pages/settings.rs` (the shared page: header, search across sub-tabs with matched words marked, sub-tabs, setting rows, section caps, the changed dot with Reset and the "changed from defaults" pill and card), `preferences.rs` (General: hints, updates at start, **Hide unfinished parts**; Appearance: base theme, accent, scale, density, ribbon labels, shelf opens on, the live miniature, reset; Viewport: overlays, snapping steps, camera speed, look; Shortcuts: the read-only list), `project_settings.rs` (General: name, start area, engine version; 2D or Rendering: the four pixel settings; Build: what the export remembers), `input.rs` (the table with keycaps and pad chips, device filter, search, Add action, listening, conflicts, the details card, the test card).

**Replaces or changes.** `settings_window.rs` (762 lines, removed; `Setting` and its search move), `input_window.rs` (449, removed; `Workbench::change_bindings` and the saving rule stay), `export_dialog.rs` (its remembered values are shown on Build), `shell/commands.rs` (settings become search results: `shell.search.settings`).

**Placeholders.** `prefs.tab.input`, `prefs.tab.source-control`, `prefs.code-split`, `prefs.shortcuts.edit`; `proj.tab.physics`, `.audio`, `.platforms`, `.plugins`, `proj.company`, `.version`, `.icon`, `.editor-start`, `.game-mode`, `.frame-rate`, `.units`, `.version-control`; `input.contexts`, `.trigger`, `.groups`, `.buffer`, `.dead-zone`, `.invert-y`, `.players-rebind`.

**Tests.** `settings_window.rs`'s tests and `input_window_tests.rs`, moved (every setting on a listed page; search by name, hint or page; a dragged number saves once when let go; binding by the next key, Esc cancels and binds nothing, the key bound doesn't also act, the 256-action limit; `SaveBindings` and `SavePixels` sent once); every change applies at once; reset by scope; conflicts found; the settings-file fuzz again with the new fields; Screens 8, 12 and 13: layout, colours, pictures.

**Manual.** `settings.md` (rewritten as Preferences and Project settings), `input.md`, `2d-pixel-perfect.md`, `playtest-export.md`, `keyboard-shortcuts.md` (the binding rules), `unfinished-parts.md` (where the setting now is).

**Risks.** With placeholders shown, Project settings › General is half marks (7 of its 11 rows) and four sub-tabs are empty pages. D12 matters most here.

**Ask or mock up first.** D9, D11, D12; D15 row 32 (Backspace). Mock-up round 2: Preferences › General, Viewport and Shortcuts; Project settings › 2D and Build.

**Findings.** None open in these files. The settings fuzz target in `fuzz/` (CI-01 lists targets that don't run) gains nothing new: `Look` and `ShellState` are fuzzed in the editor's own tests, since they are not project files.

**Size.** 1,800 + 1,000 − 1,200; about 15 files; 3 sessions.

### Part C: 2D and its editors

All three need mock-up round 3 approved before they start (Inventory a.4). Until then each of these editors works inside the new frame as M4 left it, in its old look.

#### M10. Tile maps, tilesets and pictures' crops

**Built.** The contextual tab **HOME TOOLS · Tile map** while a tile map is selected: Paint (Brush, Rectangle, Line, Fill, Eyedropper), Show (Art or Meaning, Values, Chunks, Walls), Copy (flip, flip, turn); the **Map** shelf tab (Layers, Terrains as tiles, Cell values, Cell). **LIBRARY TOOLS · Tileset**: picture group and slots-or-rules group, ribbon, shelf tabs (Terrains, Slot, Rules, Try it). **LIBRARY TOOLS · Picture**: the picture with its crops, ribbon, the Crops shelf tab.

**Replaces or changes.** `map_tab.rs` (830), `tile_paint.rs` (323, the tool bar leaves the viewport), `tile_view.rs` (the overlay's menu), `workbench_paint.rs`, `workbench_tiles.rs`, `tileset_view.rs` (1,247: split by group and shelf tab), `workbench_tileset.rs`, `atlas_view.rs` (295), `workbench_atlas.rs`, `key_owner.rs` (owners stay; the panels they name are now groups and columns), `shell/ribbons/` (three new tables), `shell/workspaces.rs`.

**Placeholders.** None new (2D 1's scatter brushes, patterns and stamps are not in the design; they arrive with their own mock-ups).

**Tests.** `map_tab_tests.rs`, `tile_view_tests.rs`, `workbench_paint_tests.rs`, `workbench_tiles_tests.rs`, `tileset_view_tests.rs`, `tileset_editor_tests.rs`, `workbench_tileset_tests.rs`, `workbench_atlas_tests.rs`, `atlas_editor_tests.rs`: meanings kept (each change one undo step and one save; keys B, U, N, G, I, Tab, X, Y, Z only while their owner has them); the contextual tabs come and go; approved pictures from the approved mock-ups.

**Manual.** `tile-maps.md`, `painting-tiles.md`, `tilesets.md`, `atlas-crop.md`, `keyboard-shortcuts.md`.

**Risks.** These editors are the lead's daily tools for their own game; a regression here costs more than anywhere else. The undo and save paths (`own_files.rs`) are not touched.

**Findings.** BUG-22 (the tile-map interface scans the whole map every frame: `map_tab.rs:296`, `tile_view.rs:764`): the Terrains column's counts are being redrawn anyway; count once per change of the map. DEBT-02 (four copies of snapshot undo): not this phase's work; don't add a fifth. TEST-04 (a wall-clock limit in `workbench_paint_tests.rs:77`): leave as found.

**Size.** 1,400 + 700 − 900; about 20 files; 3 sessions.

#### M11. Animate: sprite clips, animators, and the sequencer's place (Screen 5, spec 388-451)

**Built.** Animate with a clips file as its document: the sprite playing and the sheet's frames in the center; the tall **Frames** shelf tab in the Sequencer's shapes (the clips list where Tracks are, the frame ruler and an events lane where the dope sheet is, the accent playhead with its flag, transport in the shelf bar), a **Clip** tab. Animate with an animator as its document: the graph and preview groups, the ribbon, **Parameters** and **State · name** shelf tabs. With neither open: the sequencer's placeholders.

**Replaces or changes.** `frames_view.rs` (904), `workbench_frames.rs`, `animator_view.rs` (1,091: split into graph side, preview, shelf tabs), `graph_canvas.rs` (635: recoloured to the node graph's tokens, spec 351-364, since it is the one graph editor), `workbench_animator.rs`, `sheet_view.rs`, `shell/ribbons/animate.rs`.

**Placeholders.** `anim.keys.*`, `anim.interp.*`, `anim.tracks.*`, `anim.camera`, `anim.curves`, `anim.shelf.sequencer`, `anim.shelf.key`.

**Tests.** `frames_editor_tests.rs`, `workbench_frames_tests.rs`, `animator_editor_tests.rs`, `animator_view_tests.rs`, `workbench_animator_tests.rs`, `graph_canvas_tests.rs`: meanings kept (Space plays only over the Frames tab; Delete, Ctrl + D and undo are the tab's own); the ruler draws only the marks in view at any zoom; pictures from the approved mock-ups.

**Manual.** `sprite-animation.md`, `animators.md`, `sprites.md`, `keyboard-shortcuts.md`.

**Risks.** Two different documents share one ribbon tab with different ribbons; the ribbon table is per document kind. The shared graph editor's look changes for the Animator now and must suit Shading later.

**Findings.** BUG-21 (the Frames ruler's work has no bound: `frames_view.rs:750`) is fixed here by drawing only what is in view. DEBT-02 as above.

**Size.** 1,300 + 700 − 800; about 15 files; 3 sessions.

#### M12. Game screens: the UI editor

**Built.** The **UI** ribbon tab (or the contextual tab, by D6): the canvas with a toolbox column (Select, Box, Ellipse, Line, Pen, Text, Image, Button); the ribbon (Screen size, Safe area, Snap, Text %; Preview values; Open as code); the **Screen** shelf tab (Elements; the piece's cards; Pictures).

**Replaces or changes.** `canvas_view.rs` (1,162: the tool bar and ring move out; split), `canvas_tools.rs`, `elements_view.rs`, `screen_props.rs` (718, onto cards and the kit's fields), `workbench_screens.rs`, `shell/ribbons/ui.rs`.

**Placeholders.** None.

**Tests.** `canvas_view_tests.rs`, `canvas_tools_tests.rs`, `elements_view_tests.rs`, `screen_props_tests.rs`, `screen_doc_tests.rs`, `workbench_screens_tests.rs`, and `crates/app/src/ui_sample_tests.rs`: meanings kept (keys 1 to 7 and E over the canvas; the canvas and Elements close together); pictures from the approved mock-up.

**Manual.** `ui-editor.md`, `screens.md`, `ui-sample.md`, `keyboard-shortcuts.md`.

**Risks.** The tool ring (E) is drawn over the canvas today; it stays as it is unless the mock-up says otherwise.

**Findings.** None open in these files (BUG-45 is in the `ui` crate).

**Size.** 1,000 + 600 − 600; about 12 files; 2 sessions.

### Part D: the rest, finished

#### M13. Audio (Screen 10, spec 745-826)

**Built.** The mixer console to the design: strips of 104 px with header and routing, fader with its dB scale and cap, the meter pair, readout, M and S, the selected and muted looks. The right group: **Bus** with the Ducking / Effects switch (Ducking real, Effects a placeholder page). The **Sound events** shelf tab: the table (event, bus, volume, where heard, playing) and the detail column with Preview, waveform, takes and fields. Monitor's "Clear solo" and "Mute the game".

**Replaces or changes.** `mixer_view.rs` (912: split into strip, console, bus), `sound_event_view.rs` (467), `sound_view.rs` (368), `game_sound.rs` (the playing counts the table reads), `shell/ribbons/audio.rs`.

**Placeholders.** `audio.mixer.send`, `.snapshot`, `audio.monitor.meters`, `audio.listen.choose`, `audio.fx.*`, `audio.strip.inserts`, `.pan`, `audio.meters.stereo`, `audio.effect`, `audio.event.used-by`.

**Tests.** The Mixer's and sound events' tests (`game_sound_tests.rs`, the mixer and event tests in their files): a fader saves once when let go; double-click resets; mute, solo and ducking as before; the meters fall back; Screen 10's layout and colours for the strips.

**Manual.** `mixer.md`, `sound-events.md`, `sounds.md`, `listeners.md`, `reverb-zones.md`.

**Risks.** 10 of the screen's 18 elements are placeholders; the console will look sparse beside its mock-up.

**Ask or mock up first.** D15 rows 30 and 31. Mock-up round 3: the Bus group with ducking; the takes block.

**Findings.** BUG-35 (a streamed sound's file is scanned on the frame loop) is in the `sound` crate: the Preview button mustn't add a second such call. DEP-02 untouched.

**Size.** 1,000 + 600 − 500; about 10 files; 2 sessions.

#### M14. Debug (Screen 6, spec 453-529)

**Built.** The frame-time graph (bars, budget lines, the picked frame), the frame timeline (the CPU's stages and the GPU's as two lanes of blocks on a time ruler), Statistics rows with their bars, the Console as this tab's first shelf tab with the completions shown at the right of the command line, Show › CPU and GPU, and the Session group (Load a save…, Play and record inputs, Replay a recording…).

**Replaces or changes.** `profiler.rs` (451: the drawing, `profiler.rs:305` on), `console.rs` and `console_commands.rs` (the hint), `shell/ribbons/debug.rs`, `play_input.rs` (only where its menu items were).

**Placeholders.** `debug.capture.*`, `debug.target.source`, `.deep`, `debug.show.memory`, `debug.tools.*`, `debug.timeline.threads`, `debug.stats.draw-calls`, `.memory`, `debug.shelf.hottest`, `.memory`.

**Tests.** The Profiler's tests (the kept frames, averages) unchanged; picking a frame shows that frame's stages; `console_commands_tests.rs` unchanged; recording and replaying still start from their new buttons (`play_input_tests.rs`); Screen 6's layout and colours.

**Manual.** `profiler.md`, `console.md`, `save-games.md`, `input.md` (recordings), `play-mode.md`.

**Risks.** Small. The timeline promises less than the mock-up shows (two lanes, not six).

**Ask or mock up first.** D15 row 35. (Session group: round 1.)

**Findings.** BUG-33 (a crash in the game-code worker is silent) is the app's; the Debug tab is where a note for it would show once fixed. Nothing here makes it worse.

**Size.** 900 + 500 − 300; about 8 files; 2 sessions.

#### M15. The unbuilt screens, the Build tab, and dragging tabs

**Built.** Shading (Screen 4), VFX (Screen 11) and the model viewer (Screen 9) as the design's ribbons and shelf tabs drawn through `unbuilt`, each with one placeholder page; the model viewer's contextual tab opened from a model's right-click. The **Build** tab with Export for playtesting as its real page and a placeholder for targets. Dragging a document tab to split or merge groups.

**Replaces or changes.** `export_dialog.rs` (555: its window becomes a page; `Playtest`, `ExportRequest` and the rule that it never looks at the disk stay), `shell/groups.rs`, `shell/ribbons/shading.rs`, `vfx.rs`, `mesh.rs`, `build.rs`.

**Placeholders.** Inventory b.4, b.9, b.11 (62 ids) and `build.targets`.

**Tests.** `export_dialog_tests.rs` (466 lines) with meanings kept, including the check that the file has no call that looks at the disk; the unbuilt walk covers the three screens, and with the setting on Shading and VFX are not in the tab bar and no model shows "Open"; a tab dragged to the other half splits, dragged onto a strip merges, the ratio is kept.

**Manual.** `playtest-export.md`, `unfinished-parts.md` (the whole screens), `interface-tour.md`.

**Risks.** How much of an unbuilt screen to draw is D2's second question; this plan draws its ribbon and shelf tabs but not a copy of the mock-up's sample content.

**Ask or mock up first.** D2 (second half), D15 row 23. Mock-up round 3: the Build tab.

**Findings.** None.

**Size.** 900 + 600 − 300; about 12 files; 2 sessions.

#### M16. Finish: the manual, the old code out, the review, the gate

**Built.** The whole-manual pass: `interface-tour.md` and `keyboard-shortcuts.md` rewritten from nothing; every page read against the editor (41 of the 55 pages mention menus or panels); every screenshot retaken; `SUMMARY.md`; `concepts.md`. The leftovers removed: `theme`'s re-exported old colour names (every file now uses `tokens`), unused icons, `Panel` variants nothing shows, the old settings keys' readers that no fixture needs. Documents: `README.md` (the progress tracker's Shared 1 rows), `ROADMAP.md`, `ARCHITECTURE.md` (the frame, the shell, how a frame runs), `DESIGN.md` §13 and §17, `crates/editor/README.md` and `src/README.md`, `CONTRIBUTING.md` (the window test's steps), `RENAMING.md` (checked), `CHANGELOG.md`. Then the phase's review (decision 0027: three reviewers over everything since M1, fixes grouped by kind), the scale check with a new case for the shell (a full ribbon, a full shelf, 10,000 entities), coverage and mutation testing of `shell/`, `widgets/` and `unbuilt.rs`, and the gate.

**Tests.** Every screen's approved picture renewed and signed off; the side-by-side sheets for all thirteen screens in one document for the gate; the whole suite, the GPU tests, the smoke test, `cargo deny`.

**Risks.** The manual is 5,000 lines; rewriting it honestly is a third of this milestone. The review will find things: its fixes are their own pull requests, as always.

**Findings.** DOC-01, DOC-02, DOC-05, DOC-07 for the editor: answered by the rewritten READMEs and `ARCHITECTURE.md`. DOC-04: no retired phase numbers in anything this phase wrote. The paused review (G6 to G10 and its second pass) resumes on this code.

**Size.** 300 + 300 − 600 of code, about 2,500 lines of documents; about 70 files; 3 sessions, plus the review's fixes.

---
## 6. Mock-ups, in three rounds

Every piece below is something the lead hasn't designed. Each is drawn in the design's own language (the mock-up's HTML and CSS are at hand, so additions are made on copies of the real screens) and approved before the milestone that builds it. Three rounds, so the lead is asked three times.

| Round | Needed before | What is drawn |
| --- | --- | --- |
| **1** | M2 to M4 | The placeholder look (four cases, section 3.2) · the icon sheet (the 45 icons the mock-up lacks) · the Play group while playing (Stop, Pause, Step) · Home's Edit group (Undo, Redo) · Home's Create group for Solis's kinds · Debug's Session group · the document tab's "+" and right-click menu · the Console's search, Clear and Copy on the shelf bar · the shared dialog card. *Drawn and **approved, all of it as drawn**, by the project lead on 2026-11-11. The approved pictures are in [`docs/design/g-layout/additions/round-1/`](../design/g-layout/additions/round-1/README.md), with a README naming each and the decision it settles.* |
| **2** | M5 to M9 | The Hints card · the third Snapping row and the viewport's bottom-right pill · the Game tab's pills · the Library's chips for Solis's kinds, and Details for a sound, a sprite sheet and a text file · File › Learn, Updates and About · a project card's extras (pin, a missing folder, sort, Scan a folder…) · Preferences › General, Viewport and Shortcuts · Project settings › 2D (or Rendering) and Build |
| **3** | M10 to M15 | HOME TOOLS · Tile map, with the Map shelf tab · LIBRARY TOOLS · Tileset · LIBRARY TOOLS · Picture (crops) · Animate with a clips file (Frames and Clips) · Animate with an animator · the UI tab · Audio's Bus group with ducking, and a sound event's takes · the Build tab |

Round 3 is the largest and is not needed until part B is under way, so it can be drawn while M5 to M9 are built, by a helper that only draws (one line of work is kept: nothing is built in parallel).

---

## 7. Decisions needed

Each is a short question with a recommended answer and the alternatives, sorted by the first milestone it blocks. **Each now starts with where it stands:** *Settled* (how, and when) or *Still open*. The answers are gathered in "What the project lead has decided" at the top.

### Before M1

**D1. Is Shared 1 one phase of sixteen milestones, or four phases?** *Settled 2026-11-11 (the lead reviewer's call; the project lead was told): four phases, Shared 1a to 1d.*
Recommended: four, **Shared 1a to 1d** (section 1), each with a small gate, the last with the roadmap's gate. CLAUDE.md's rule is two to six milestones a phase. Alternatives: one phase with the rule set aside for it; or two phases (A + B, then C + D).

**D3. Old saved choices: is this mapping right, and is it all right that panel arrangements aren't carried over?** *Settled 2026-11-11 (the lead reviewer's call; the project lead was told): yes to both. The mapping is in force from M1; panel arrangements stop being carried over in M4.*
Recommended: yes to both. Accents: Blue stays Blue; Petrol blue, Marine green and Olive become Teal; Red, Copper, Orange, Amber and Gold become Orange; Plum becomes Purple. Every tab starts from the design's standard, said once in the Console. Alternative for accents: everyone starts on the design's blue.

**D18. The tokens the spec leaves out, and where the HTML and the spec differ.** *Settled 2026-11-11 (the lead reviewer's call; the project lead was told): all as recommended.* Recommended, all at once:

- Density: Compact rows 22, fields 22, shelf tabs 24; Default 24, 24, 26; Comfortable 28, 28, 30. Ribbon and tab bar don't change.
- Interface scale steps: 100, 125, 150, 175, 200%.
- A disabled control (real, but not usable right now): text #6E6E6E, no hover, a hover text saying why (as today).
- The front document tab's text: #E6E6E6 (the HTML), not white (spec 150). Icon buttons' hover: #262626 (the HTML), not #242424 (spec 118).
- Menus and dropdown lists: fill #181818, border #333333, radius 7, rows 24 px, hover #242424.
- Tooltips: fill #121212, border #2E2E2E, radius 5, 11 px text. Scroll bars: 8 px, #333333 on nothing, #3A3A3A when hovered.
- Keyboard focus: a 1 px accent ring outside the control.
- The shelf's standard height: 300 px; 360 on Animate.
- The three faint greys below 4.5:1 (Inventory d.2) are allowed for the marks listed there only.

### Before M2

**D2. What does a placeholder look like, and how much of an unbuilt screen is drawn?** *Settled 2026-11-11 (the project lead, on the mock-ups): look A, which is the recommended look below; a whole unbuilt screen is its ribbon and shelf tabs as placeholders and one centred page card.*
Recommended: section 3.2's look (in place, lowest readable grey, a small hollow square mark, no hover, a hover text naming the phase, a click that says so in the status bar), and for a whole unbuilt screen its ribbon and shelf tabs as placeholders with one placeholder page in the center. Alternatives for the look: a dashed outline (but dashes already mean "add" in the design); a small "soon" tag (no room on a 50 px button). Alternative for whole screens: a still copy of the mock-up with its sample content, dimmed (pretty, but it shows a crane Solis doesn't have and takes a session per screen), or only the tab and a page with no ribbon (less to look at, loses the design's layout).

### Before M3

**D4. Which words does the interface show?** *Settled 2026-11-11 (the project lead): Solis's words, and the design's names where Solis has none: the "Recommended on screen" column below. "Check in" waits for Shared 11.* The spec's samples use other words than Solis's. Recommended: Solis's own words everywhere, since the manual, the game code's calls and every earlier decision use them. The lead may prefer the design's for some; each is listed:

| The design says | Solis says | Recommended on screen |
| --- | --- | --- |
| level, scene; the "Scene" column | area; the Area panel | **Area**, for the file, the path and the shelf column, so one word means one thing |
| object; the "Object" shelf tab | entity | **Entity**, in text and on the shelf tab |
| prefab | assembly | **assembly** |
| inspector; the "Inspector" column | Properties | **Properties** (what every manual page calls it) |
| mesh, "Mesh" button | model | **Model** |
| texture | picture | **Picture** |
| material | (not built) | **Material** |
| script, component "CraneController" | game code, component | **game code**, **component** |
| "+ Component" | "+ Add component…" | **+ Component** (the design's, shorter) |
| Lit / Unlit | Standard / Stylized / Unlit (looks) | Solis's three |
| Light, Camera, Volume | Light, Camera, (reverb zone) | the design's |
| Simulate | (not built) | the design's |
| Check in | (not built; Git says commit) | decide with Shared 11 |
| "Search or run a command" | "Search or do anything…" | the design's |
| "Ctrl K" in the search box | Space | **Space** (settled) |
| Preferences / Project settings | Settings (Editor / This project) | the design's two |
| Recent projects | Projects (the launcher) | the design's |
| Exit | Quit | the design's |
| "Open in Ledger" | Open in Ledger ↗ (today's Properties) | keep |

**D5. The logo tile.** *Settled 2026-11-11: today's logo in the tile for now, as one swappable asset; a new logo is designed with the project lead later (the ideas list), outside this phase.* Recommended: Solis's own logo (`branding/logo.svg`) in the design's 20 × 20 light tile. Alternative: the mock-up's cube mark, which would be a new logo and a new entry in `RENAMING.md`.

**D6. Where does the game-screen (UI) editor live?** *Settled 2026-11-11 (shown to the project lead, not objected to): a UI ribbon tab between Animate and VFX.* Recommended: a ribbon tab of its own, **UI**, between Animate and VFX: it is an area of the engine like the others, and a screen is opened the way a material opens Shading. Alternative: a contextual tab **LIBRARY TOOLS · Screen**, like the tileset's.

**D10. How does the ribbon collapse** (the spec's own open question, spec 1184)? *Settled 2026-11-11 (shown to the project lead, not objected to): tabs only, as recommended.* Recommended: tabs only; clicking a tab while collapsed drops the ribbon down over the center until something is chosen or the pointer leaves. The chevron and a double-click on a tab switch between the two. Alternatives: icons only (the "Show ribbon labels" setting already does that); show on hover (jumpy).

**D13. Does the play tint stay?** *Settled 2026-11-11 (the project lead), against the recommendation below: **no tint, as designed.** Play becomes Stop, with Pause and Step beside it; no accent line, no tinted status bar. Decision 0099 records the change to decision 0085.* While the game runs, today's bars turn blue and amber when paused (decision 0085: "playing is unmistakable"). The design shows none. Recommended: keep it, as a 2 px accent line round the viewport and an accent-mixed status bar (amber when paused), since Play becoming Stop is easy to miss. Alternative: the design as drawn.

### Before M4

**D12. Placeholders with no phase behind them.** *Settled in part 2026-11-11 (the lead reviewer's call, with M2): the four parts decided against below are not registered, and a part with no phase is shown like any other, reading "not planned yet". Which of the unplanned parts become milestones is still open, for the rethink.* 83 of the 164 are for things no roadmap phase builds (Inventory b.15): the model viewer, most of placing's options, the mixer's effects, input contexts, frame rate and units, and more. Recommended: show them like any other, with "not planned yet" in place of a phase, and decide at the rethink which become milestones; but leave out the four the engine has decided against or has another answer for: **VR / XR** template (Solis is for Windows desktops), **Default game mode** (game-wide objects do that job), **Units and axes** as a setting (fixed: metres, Z up), and **Players can rebind** (a game's own menu does it). Alternatives: hide every unplanned one always; or give each a milestone now.

**D14. The running game: its own document tab, or inside the viewport?** *Settled 2026-11-11 (the project lead): its own Game tab beside the area's tab, to the front on Play.* The spec says Play runs the game "in the viewport" (spec 178). Solis keeps the editor's view and the game's view apart, so the area can be watched and changed while the game plays. Recommended: a **Game** tab beside the area's tab in the same group, brought to the front on Play. Alternative: one picture that switches, as the spec reads.

### Before M5 to M9

**D17. Colours for the kinds of file the design doesn't have.** *Still open: it waits for the project lead to see mock-up round 2 (the Library's chips in the proposed colours), and blocks M7, the Library. It was first sorted under M2, which doesn't need it: until it is answered `theme::kind` keeps today's sixteen colours.*
The design gives seven (folder, mesh, material, texture, animation, audio, prefab). Recommended: model = its mesh cyan #5FC4D6; picture and sprite sheet = texture #E57F6A; clips and animator = animation #B48CFF; sound and sound event = audio #E8B04A; assembly = prefab #7FB2FF; area = #C8C8C8 (the design draws areas with a grey ring, Screen 3's Used by); code and text = #8FB8F0; tileset and tile map = material green #6CC28A; screen = #C38CFF. Alternative: keep today's sixteen kind colours (`theme::kind`), which were chosen to differ more.

**D15. New keys and mouse actions** (rule 17). *Still open, row by row; asked before the milestone that builds each. (The clashes with today's keys are settled: today's keys stay.)* These are new; the numbers are Inventory part (c)'s rows:

| Row | Question | Recommended |
| --- | --- | --- |
| 13 | A list of kinds from a Create button: long press? | Click makes the usual kind; a small ▾ on the button, or a right-click, opens the list |
| 24 | The wheel over the ribbon scrolls it sideways? | Yes |
| 10 | F2 and Delete over the Library act on the picked file, elsewhere on the entity? | Yes (the pointer decides, as for every other key) |
| 15 | Drag a field's label to change its value? | Yes, and dragging the value stays |
| 6 | Shift while dropping keeps the file "loaded" for more copies? | As designed |
| 7 | Place without snapping: Alt (the design) or Ctrl (Solis's key for "flip snapping")? | Ctrl |
| 8 | R turns the ghost by the Angle step? | As designed |
| 17 | Click a tile, then click in the scene; a key to drop under the pointer? | Not now (placeholder) |
| 18 | Double-click in the Library tab: place (today) or only select? | Only select there; it still places from the shelf's Assets and Place columns |
| 19 | Drag a tile onto the Home tab? | Not now |
| 32 | Backspace clears a picked binding? | Yes, and × stays |
| 30 | Fine fader moves: Alt (the design) or Shift (Solis's key for "finer")? | Shift |
| 31 | Click a meter to reset its peak; drag routing between strips? | The click now; the drags later |
| 33 | Arrow keys step a frame in the Frames tab? | Yes: Left and Right step, Up and Down jump to the next event |
| 35 | Click a bar in the frame-time graph to pick that frame? | Yes; ranges and zoom later |
| 23 | Where may a dragged document tab land? | The left or right half of the center, or another group's tab strip |
| – | Left and Right, with Up and Down, on the project cards? | Yes |

**D16. How is Level design (placing) entered and left?** *Still open.* The spec shows two forms of Home and not the switch. Recommended: the **Place** shelf tab being in front *is* the mode (the ribbon takes its build form, the toolbox goes); choosing **Object** or selecting something leaves it; Library › Place enters it. Alternative: a button in the ribbon.

**D7. Does Solis still start in a small window of its own?** *Still open.* Decision 0090 chose a small launcher window (1100 × 700) that grows when a project opens. The design draws Recent projects as a page of the editor's window. Recommended: the design's page at the editor's size, keeping 0090's two promises (the page at every start; nothing reopened by itself). Alternative: keep the small size for the page until a project opens.

**D8. Templates, the 2D / 3D badge and Learn.** *Still open.* Solis has one kind of empty project and four examples (Playground, Turret sample, UI sample, Benchmark). Recommended: the strip shows **Blank project** (one tile, real) and the four examples (real: they make a project in a folder you choose), then the design's four as placeholders (less VR / XR, by D12); no 2D / 3D badge until projects have a kind; Learn keeps the first steps and the manual. Alternative: the design's six tiles exactly, two real and four placeholders, with the examples only under Learn.

**D9. Where do the project's 2D settings go?** *Still open.* Recommended: a **2D** sub-tab of Project settings, right after General (2D is built first, and these are the settings a 2D project changes most). Alternative: inside Rendering.

**D11. The code editor and "Scripting".** *Still open; the project lead has said that code opens in their code editor as today and the built-in code editor is a later feature, which is what the recommendation assumes.* Recommended: the code group is not drawn at all for now, with or without placeholders, exactly as the spec says of the first release (spec 29, 190); **Code editor split** in Preferences is a placeholder row; Project settings › **Scripting** shows "Your code editor (Ledger)" as plain words with nothing to choose; every link to code opens the file in the user's code editor as today. The spec's own open question (how a file and line are handed to Ledger, and whether changes reload while playing, spec 1191) can wait: saving a Lust file already reloads the running game within two seconds. Alternative: a placeholder page where the code group would be, so the split is visible.

### The spec's own open questions (spec 1180-1192)

Settled where a D above is settled (the ribbon's collapse, the light theme, the scale's steps and density); the rest still open.

| Question | Blocks | Recommended |
| --- | --- | --- |
| The scene list also as a slide-out panel? | Nothing | No, for now |
| The Build tab undesigned | M15 | Mock-up round 3; Export for playtesting is its real content |
| How the ribbon collapses | M3 | D10 |
| Editor groups and the shelf in separate windows? | Nothing | No, for now (today's floating panels go with the dock) |
| Light theme | – | Settled: dark only |
| Accessibility: scale range, keyboard-only use, high contrast | M1 for the scale (D18); the rest nothing | Scale to 200%; every kit widget takes keyboard focus; high contrast later, as a second palette |
| Density and click targets; touch or pen? | M1 (D18) | Mouse only; Comfortable is the larger choice |
| When the code editor hides | Nothing | D11 |
| The hand-off to Ledger | Nothing | D11 |
| The mixer's bus list and the VFX module stages | Nothing now | Solis's buses are the project's own (`mixer.solmix`); stages wait for Shared 8 |

---

## 8. Decision records and documents to write

New decision records, each also a line in `DESIGN.md` §17. The first three were written with M1 as 0099, 0100 and 0101; the other two take the next numbers when their milestones come:

1. **The G layout** (written with M1, in force from M3 and M4): the editor's frame is a tab bar, ribbon, editor groups, shelf and status bar. Replaces decision 0004's layout (the one-row top bar, every panel dockable) and decision 0061's settings window (its "no layout presets" stands); changes 0085 (workspaces become ribbon tabs; the Library drawer goes; its search key and status bar stand) and 0086 and 0090 (the launcher becomes a page of the editor's window, by D7).
2. **One theme, five accents** (M1): replaces 0096; changes 0060 (Play is the one light button; the meaning colours take the design's values).
3. **Unfinished parts are shown as placeholders, and one setting hides them** (M2): changes 0070 and 0085 ("nothing that isn't built is shown").
4. **Today's keys stay** (recorded in 0099; M3 adds D15's answers as they come, in a record of their own if they amount to one).
5. **Docking is Solis's own** (M4): egui_dock leaves `docs/dependencies.md`, by its own exit plan.

Documents that change besides the manual: `ROADMAP.md` (Shared 1's milestone list, below), `README.md` (the tracker), `DESIGN.md` §13 (the editor) and §17, `ARCHITECTURE.md`, `CLAUDE.md` (its mock-up rule now points at the design in `docs/design/g-layout/` and at "drawn on the design's screens": done with M1), `CONTRIBUTING.md` (the window test), `docs/dependencies.md`, `licenses/README.md`, `docs/plans/README.md` (this plan and its inventory).

**For `ROADMAP.md`** (D1 is settled; written into `ROADMAP.md` with M1):

- **Shared 1a: The look and the frame.** The look: one theme, five accents, the design's sizes and type · The kit: icons, the design's controls, and unfinished parts marked, with a setting that hides them · The top: tab bar, ribbon and command search in place of the menus · The middle: editor groups, the shelf and the status bar in place of the dock. *Gate:* the editor opens in the new frame, everything that worked before can be found, and "Hide unfinished parts" leaves no gaps.
- **Shared 1b: The everyday screens.** Home: the level editor · Home: placing · The Library · File: projects, Learn and updates · File: Preferences, Project settings and Input. *Gate:* build a small area, place things from the shelf, change a setting and a key, and open another project, all on screens that match their pictures.
- **Shared 1c: 2D and its editors.** Tile maps, tilesets and pictures' crops · Animate: sprite clips and animators · Game screens. *Gate:* paint a map, edit a tileset, change a clip and an animator, and lay out a screen, each as its approved mock-up.
- **Shared 1d: The rest, finished.** Audio · Debug · The unbuilt screens, the Build tab and dragging tabs · The manual, the old code out, the review. *Gate:* the project lead works in the new editor for an evening and wouldn't go back.

---

## 9. Size, and what to cut

### 9.1 An honest estimate

| Milestone | New or rewritten code | Tests | Removed | Files | Sessions |
| --- | --- | --- | --- | --- | --- |
| M1 The look | 900 | 600 | 700 | 25 | 2 |
| M2 The kit | 2,100 | 900 | 500 | 30 | 3 |
| M3 The top | 2,200 | 1,500 | 1,100 | 45 | 3 |
| M4 The middle | 1,800 | 1,400 | 2,000 | 60 | 4 |
| M5 Home | 1,500 | 800 | 900 | 20 | 3 |
| M6 Placing | 900 | 500 | 200 | 12 | 2 |
| M7 Library | 1,600 | 900 | 1,300 | 20 | 3 |
| M8 File: projects | 1,700 | 900 | 1,600 | 18 | 3 |
| M9 File: settings | 1,800 | 1,000 | 1,200 | 15 | 3 |
| M10 Tile maps, tilesets, crops | 1,400 | 700 | 900 | 20 | 3 |
| M11 Animate | 1,300 | 700 | 800 | 15 | 3 |
| M12 Game screens | 1,000 | 600 | 600 | 12 | 2 |
| M13 Audio | 1,000 | 600 | 500 | 10 | 2 |
| M14 Debug | 900 | 500 | 300 | 8 | 2 |
| M15 Unbuilt screens, Build, dragging tabs | 900 | 600 | 300 | 12 | 2 |
| M16 Finish | 300 (+ 2,500 of documents) | 300 | 600 | 70 | 3 |
| **All** | **about 21,300** | **about 12,500** | **about 13,500** | | **43** |

For scale: the editor crate is 53,000 lines of code and 29,000 of tests today, so this phase rewrites about two fifths of its interface code and touches most of its whole-window tests. Each session is followed by a reviewer's pass; the mock-up rounds and the lead's answers sit between parts. The numbers are judgement from reading the files, not measurement: M3 and M4 are the least certain, because their cost is mostly in tests that click through menus and panels (467 label look-ups).

### 9.2 If the phase must be shorter

In the order to cut, cheapest loss first:

1. **Dragging a document tab to split** (in M15; 250 lines and a mouse action). Groups keep their fixed splits with a draggable divider. Nothing else depends on it.
2. **The live miniature in Preferences and the marking of matched words in settings search** (in M9; about 250 lines). The settings still apply at once, which is what the miniature is for.
3. **The ghost's guides and trail** (in M6): keep the bounds-box ghost and the label.
4. **Picking a frame in the Profiler and the Console's shown completions** (in M14): the graph and timeline show the newest frame, as today.
5. **Density** (in M1): ship Default only; the setting row becomes a placeholder.
6. **Whole unbuilt screens drawn with their ribbons** (in M15): a tab and one placeholder page each. Saves a session; loses the design's layout for those screens.
7. **Move part D's Audio and Debug (M13, M14) after the phase.** After M4 the Mixer and Profiler already live in their tabs in their old look, in the new colours.
8. **Move part C (M10 to M12) after the phase.** The same is true of the 2D editors after M4. This is the big lever: parts A and B alone (nine milestones, about 26 sessions) give the new frame and every everyday screen. But the 2D editors are what the lead uses for their own game, and 2D is built next: an "evening in the new editor" spent painting tiles in the old look would not pass the gate. Recommended only if the mock-ups of round 3 are late.

What should **not** be cut: the placeholder mechanism and its tests (the lead asked for the switch by name), the settings fixtures (old files keep opening), the reach test (nothing lost), and M16's manual pass.
