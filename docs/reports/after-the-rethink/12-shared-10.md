# 12 · Shared 10: cutscenes and behaviour

ROADMAP.md (ideas 22 and 14, after effects so cutscenes and characters can use everything before them): **a cutscene timeline** (camera moves, sounds, animations and effects arranged along a timeline); **behaviour for characters** (a villager's day or an animal's habits, drawn as a diagram like the Animator). **Gate:** a short cutscene plays from the timeline, and villagers follow their day on their own.

Two milestones (each probably two pull requests: the data and runtime, then the editor).

## What exists

- **The animator** (`animation::animator`, `.solanim`; decision 0066): parameters, states, transitions with conditions, stepped each tick before sprites, in the checksum and saves; reached by game code through a generated Lust file per animator (decision 0078). The **shared graph editor** draws it.
- **The Frames tab** (decision 0064): frames along a time ruler, events in a lane: the start of a shared timeline widget.
- **By then (recommended in earlier reports):** timers, tweens, steps in order and states with enter/leave (Shared 3), paths (Shared 4), tags and world questions (Shared 2), pathfinding (2D 1; 3D in 3D 1), triggers (2D 2), rooms (Shared 6), music with sections (Shared 5), effects (Shared 8), fades and shake (Shared 3).

## M1. A cutscene timeline

**What it gives:** arrange camera moves, character moves and animations, sounds, music changes, effects, text and fades along a timeline, like a video editor; play it from game code; skip it.

**Design questions and techniques:**
- **Tracks and clips:** each track binds to a thing (the camera, an entity in the level or a prefab spawned for the scene, the music, the screen) and holds clips (a camera move along a path or between shots, a "walk to", an animation clip or animator parameter, a sound event, an effect, a line of dialogue, a fade, an event sent to game code) and keyed curves (a value over time with easing, using decision 0050's shared curve editor). This is the established timeline model (tracks bound to scene objects, clips and keys).
- **Is a cutscene game state?** It moves characters and can change the game (a door opens, an item is given). **Recommended:** the timeline is **played on the game's tick** (time in ticks, deterministic, saved: which scene and how far), so its changes land like any game code's; the camera and look-only tracks (shake, fades, letterbox) are output. Skipping jumps to the end by applying each track's final state (and firing the events it would have fired, in order), so a skipped scene leaves the world as a watched one would.
- **Bindings by name, checked:** a scene's actors are slots (`Farmer`, `Mayor`) bound when played: `cutscene:play(farmer, mayor)` through a generated Lust file per scene (rule 4), so a missing actor is a mistake before running.
- **Blending in and out:** the camera blends from the game camera into the first shot and back.
- **2D and 3D:** the same tracks; 2D cameras snap to pixels at the end (the pixel stage).
- **File:** `<name>.cutscene.lst` (Lust data, stable IDs per track and clip so two people's edits merge).

**Editor UI (mock-up first):** a Cutscene tab with the level's view on top and the timeline below (tracks, clips dragged and trimmed, keys, a playhead, the shared ruler from the Frames tab and Shared 5's music sections), preview playing in the editor without starting the game (output-only tracks) or in play mode for the full result. **Keys (rule 17):** recommended the video-editor norms the project lead knows from their own video editor project (decision 0095 brought its rules in): Space plays/pauses while the timeline has focus (but Space opens the search elsewhere, decision 0103: the timeline must own the key only while focused), J/K/L shuttle, I/O in and out, S splits a clip, arrows step a tick. Ask.

**Tests:** a scene plays identically on 1 and 4 cores and from a save mid-scene; skipping gives the same end state as watching; a missing binding is caught at check time; fuzz the file. **Size:** large.

## M2. Behaviour for characters

**What it gives:** a villager's day (wake, work the field, eat, go to the tavern, sleep) or an animal's habits, drawn as a diagram like the Animator, without writing the control flow by hand.

### Techniques compared

| Way | What it is | Strengths | Weaknesses |
| --- | --- | --- | --- |
| **Behaviour trees** (Isla's 2005 talk made them standard; Colledanchise and Ögren formalised them) | A tree of selectors, sequences, conditions and actions ticked from the root; actions report running, success or failure | Modular, readable, reusable sub-trees; reactive (higher priorities interrupt) | Ticking the tree each frame costs per agent; "memory" and long-running actions need care; deep trees get hard to read |
| **Hierarchical state machines / statecharts** (Harel) | States with substates, entry/exit actions, transitions with conditions | Exactly what the Animator already is; easy to see "what is it doing now"; cheap per tick | Transitions multiply as behaviours grow; less modular |
| **Utility AI** (Mark's infinite-axis utility system) | Each option scored from considerations through curves; the best wins | Smooth, believable choices; scales to many options; designers tune curves | Hard to predict; debugging needs score views |
| **Goal-oriented action planning** (Orkin) | Actions with preconditions and effects; a planner searches for a plan to a goal | Emergent multi-step plans | Planning cost per agent; hard to author and debug; overkill for daily routines |
| **Schedules** | A day's timetable of activities by hour, each activity a small behaviour | Exactly "a villager's day"; trivial to author | Only for routine; needs another layer for reactions |

**Recommendation:** **hierarchical state machines on the Animator's model plus utility scoring as an optional transition kind, with schedules as a ready-made top layer**, all in one **behaviour** file edited in the same graph editor:
- states are activities (Sleep, Work, Eat, Socialise, Flee), each an action (go to a place by path or tag, play an animator state, wait, run a Lust method) or a nested machine;
- transitions are conditions (the Animator's), timers (Shared 3), **time-of-day windows** (a schedule: "08:00 to 12:00 → Work"), triggers and events, or **scores** (utility: "choose the highest of Eat, Socialise, Rest by these curves");
- states have enter/leave (Shared 3's rule: what's started in a state ends with it).
This reuses the Animator's runtime and editor (decision 0050's "one graph editor") instead of a second tree language, covers daily routines (the gate) directly, and still reacts. **Behaviour trees** remain the alternative the ideas list named ("behaviour trees in the graph editor the animator uses, with `check` covering every outcome"); ask the lead which they find easier to read (question 1) with a mock-up of a villager's day in both.

**Runtime and scale:** behaviour is game state, stepped each tick in jobs (each entity's machine reads the start-of-tick world, decision 0033; writes land in order), in the checksum and saves (like animators). At 2,000 units each evaluating a small machine per tick (conditions, a few utility curves), well under 1 ms on several cores; utility scores and path requests can be evaluated less often (every N ticks, staggered by entity ID, deterministically). Actions that move use the path follower (2D 1) or the navmesh (3D 1).

**Game code:** a generated Lust file per behaviour (rule 4): `villager:current()` (a state), `villager:jumpTo(VillagerState.Flee)`, parameters as methods (`villager:setHunger(0.8)`), events `BehaviourChanged(VillagerState from, VillagerState to)`. Actions can call a component's own Lust method by name, checked (the editor lists the component's methods).

**Editor UI (mock-up first):** the graph editor with activity states, a 24-hour schedule strip for time windows, score curves on utility transitions, and a live view while playing (the current state glows, as the Animator does; scores shown as bars). No new keys beyond the graph's.

**Tests:** a villager follows a day exactly the same way on 1 and 4 cores and after save/load; a state's timers stop when it's left; utility ties broken by order; fuzz the file. **Scale case:** 2,000 villagers with schedules and utility choices. **Size:** large.

## Dependencies and order

Needs Shared 3 (timers, states), Shared 4 (paths, curves), Shared 2 (tags, nearest), 2D 1 (paths on maps), Shared 5 (music, dialogue text), Shared 6 (rooms: a villager going indoors), Shared 8 (effects). 3D characters need 3D 1's navmeshes and skeletal animation; build the action interface so a 3D navmesh mover plugs in later.

## Risks and questions for the project lead

1. **Behaviour as state machines with schedules and scores, or behaviour trees?** **Recommended:** state machines (the Animator's model) plus schedules and utility scores, shown side by side with a tree in a mock-up before deciding; alternative: behaviour trees as the ideas list said.
2. **Cutscenes as game state** (deterministic, saved, skippable to the same end state): **recommended** yes; alternative: cutscenes as pure output that pause the game (simpler, but they can't move the world).
3. **Timeline keys:** **recommended** Space (only while the timeline has focus), J/K/L, I/O, S, arrows; alternative: buttons only.

## Crates

None: both runtimes are core (decision 0051) and small. (`bonsai-bt` and similar crates exist under permissive licences but would split the editor's one graph model.)

## Size

Phase: large.

## Sources

- [Isla, Handling complexity in the Halo 2 AI (GDC 2005)](https://www.gamedeveloper.com/programming/gdc-2005-proceeding-handling-complexity-in-the-i-halo-2-i-ai); [Colledanchise and Ögren, Behavior trees in robotics and AI (arXiv)](https://arxiv.org/abs/1709.00084).
- [Harel, Statecharts (1987)](https://www.inf.ed.ac.uk/teaching/courses/seoc/2005_2006/resources/statecharts.pdf).
- [Dave Mark, Building a better centaur: AI at massive scale (GDC 2015, utility)](https://www.gdcvault.com/play/1021848/Building-a-Better-Centaur-AI).
- [Orkin, Three states and a plan (GOAP, GDC 2006)](https://www.gamedevs.org/uploads/three-states-plan-ai-of-fear.pdf).
- Rule 20: these talks name games; the plan should describe the techniques by author only.
