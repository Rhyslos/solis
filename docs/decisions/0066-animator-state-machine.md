# 0066. Animators: a state machine choosing a sprite's clip, edited in the Animator tab

- **Status:** Accepted (builds on 0057, 0064 and 0065)
- **Date:** 2026-10-28

## Problem

With frame animation (decision 0064), game code picks each clip itself (`playClip`). A farmer needs idle, walk and use-tool, each in four directions, and the rules between them. Phase 9 brings the state machine (Phase 7's A4) and its graph editor (A5) for sprites first, so 3D skeletons use the same one in Phase 16 (decisions 0057 and 0059). It had to settle:
- what an animator file holds;
- how directions are chosen;
- how the running game steps it;
- what the editor looks like.

It also had to settle how game code reaches it under the new API guidelines (decision 0065).

## Options

- **Directions:** a state per direction (Walk up, Walk down…) with transitions between them; a 2D blend space by a direction; or a facing parameter that picks the nearest of a state's clips.
- **The editor:** the shared graph editor of decision 0050 in a document tab (the mock-up); or no editor, with the file written by hand. The project lead asked whether it was needed, and chose to build it.
- **Game code:** string-named setters (`setAnimSwitch(me, "moving", true)`, decision 0057's choice 9); or, under the guidelines' rule 4, a Lust file the editor writes per animator, with a state of its state names and a class with one method per parameter.

## Decision

Chosen by the project lead: the facing parameter, the editor as mocked up (a5), and rule 4.

- **An animator file** (`.solanim`) is a settings file in the Lust style. It has a block per parameter, state and transition, each by a stable random ID, so two people's changes merge as separate lines. It holds:
  - the sheet (or clips file) whose clips it names;
  - the state it starts in;
  - where the editor draws Any state.

  Read as a hostile file within limits (1 MB, 256 states, 1,024 transitions, 64 parameters, 8 conditions a transition), fuzzed, version 1 with an example.
- **Parameters** are a number, a switch, a trigger (fired for one tick) or a direction.
- **States** play one clip, or a clip per direction (up, right, down, left, then the diagonals). The clip nearest the direction parameter plays; turning changes the clip but keeps its time, so a walk stays in step. On a tie, the clip playing stays, so it never flickers.
- **A state's speed** is multiplied by a number parameter if it names one, and by the Animator's own speed.
- **Transitions** go from a state, or from Any state (to any state but its own target). Each is taken when all its conditions hold: a switch on or off, a trigger fired, a number above or below a value, or the clip finished. Any state's are tried first, then the state's own, each in order. At most one is taken a tick.
- **In the running game,** an `Animator` component (file, starting state, speed) beside a Sprite is stepped every tick, before sprites move on:
  - its transition is taken, triggers are put out;
  - the Sprite's clip and speed are set;
  - a new state restarts its clip.

  The animator's state is kept on the component (so saves keep it in the area), and its values beside it. Both are in the checksum. Saves keep the values in a section of their own, `solis save 4`, still reading 1 to 3. A changed file carries each entity's state and values over by name.
- **The Animator tab** opens first in the viewport's group when a `.solanim` is opened (Library double-click, or Library > Import > New animator). The file also shows as a second document tab in the top bar. The tab has:
  - parameters, with a preview's values, on the left;
  - the preview: it always plays, decision 0057;
  - the graph on the right, drawn on the shared graph editor with decision 0057's controls: middle-drag pans, the wheel zooms, F frames, right-click empty space adds a state, drag from a state's edge or right-click > Make transition joins two, and Delete removes.

  The preview's state glows blue, or while playing the selected entity's. Properties edits the state or transition picked, or the file. Each change is one undo step (Ctrl+Z over the tab) and is saved at once. A file changed on disk is taken in as one undo step.
- **Game code reaches an animator through a Lust file the editor writes for it** (rule 4 of the API guidelines): a state of its state names, and a class with one method per thing code can set or ask. This needs Lust's natives to take and give back states, so it arrives with the API milestone. No string-named calls are added meanwhile.

## Consequences

- **Game code can't drive animators yet.** Until the API milestone, a game's code drives sprites with `playClip`. The world already has the calls the generated class will use (`set_animator_value`, `jump_to_animator_state`, `animator_of`).
- **Blend spaces and crossfades wait for 3D** (Phase 16): frames don't blend. The file's version will grow then, with an upgrader.
- **In the editor's viewport, a sprite shows its own Clip;** the animator takes over when the game runs.
- **The shared graph editor** (`editor::graph_canvas`) is ready for materials and effects.
