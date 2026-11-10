# Ideas for later: tools that help make games

**Now milestones** (10 November 2026, [decision 0098](../decisions/0098-ideas-become-milestones.md)): each idea below says which phase of [ROADMAP.md](../../ROADMAP.md) builds it. Two are not planned: photo mode and screenshots, and Steam's features built in, which are for each game's developer, not the engine.

Written down on 8 November 2026 so nothing is forgotten, when none of it was planned or decided; two days later all but two became milestones (above). Each idea still gets its own decision (DESIGN.md §17) before it's built.

These came from comparing Solis with other engines (Godot, Unity, Unreal, GameMaker, Defold) and from Lust's own rulebook. They're general tools any game could use: game-specific systems, like an inventory or crafting, belong in games (CLAUDE.md).

**Status:** each idea ends with the milestone that builds it (*Milestone: Shared 3*, a phase of ROADMAP.md's three tracks), or with *Not planned*.

## In plain words: the first ones worth doing

If only three things are built from this list, these would help the farming game most:

1. **Timers:** "do this in 2 seconds", "do this every day". Crops growing, a shop opening at 9.
2. **Tween:** make something move, grow or fade smoothly instead of jumping. A hoe bouncing when used, a menu sliding in, the screen fading to black at bedtime.
3. **Tags:** label things ("crop", "tree", "animal") and ask "which crops are near me?".

After those: **field hints** (Properties shows sliders and explanations for your own settings) and **debug drawing** (draw lines and circles over the game to see what your code is doing).

## The list

Each idea: what it is in everyday words, then a note for whoever builds it.

### Time and flow

1. **Timers.** Something happens after a while, or again and again. They pause when the game pauses and are kept in save games. **Milestone: Shared 3.**
   - Build note: `every 5.0 { }` and `after 2.0 { }` in components, on game time. Lust's screens already have `every`.
2. **Tween.** A value changes smoothly over time, with a choice of how it speeds up and slows down (easing). **Milestone: Shared 3.**
   - Build note: the word is `tween` (what other engines call it). `tween position to [10, 4, 0] over 0.5 [easeOut];`, sending an event when it's done. Not `animate`, which already plays sprite clips in Solis and means gliding in Lust's screens. Lust gets the maths (`Maths:lerp` for numbers, easing curves); Solis runs tweens on game time, so they pause, slow down and replay with the game.
3. **Steps in order.** "Walk there, wait a second, say this", written top to bottom. Good for little cutscenes. **Milestone: Shared 3.**
   - Build note: `after` blocks inside one another. Lust's built-in `wait` is for console programs; inside a game it would have to wait on game time and work with saves and replays, or game code uses `after` instead. Decide which.
4. **States that know when they start and stop.** A character that's "stunned" can run something as it becomes stunned and as it recovers. And anything started while stunned (a timer, a tween, a sound) stops by itself when the stun ends, so nothing is left behind. No other engine does that second part. **Milestone: Shared 3.**
   - Build note: `on enter Stunned { }` and `on leave Stunned { }`, and Lust's rule that what's made in a state ends with it (LUST.md §8) applied to timers, tweens, sounds and spawned entities.

### Feel

5. **Screen shake, flash and fade.** The camera shakes on a hit, a sprite flashes white, the screen fades between places. **Milestone: Shared 3.**
6. **Smooth randomness (noise).** Random values that drift gently instead of jumping: swaying grass, flickering lamps, a slowly drifting camera. **Milestone: Shared 3.**
   - Build note: `Noise[x, y]`, next to Lust's `Random[1, 6]`.
7. **Controller rumble.** **Milestone: Shared 3.**

### Finding things in the world

8. **Tags.** Label entities ("crop", "enemy"), with spelling checked before the game runs. **Milestone: Shared 2.**
   - Build note: `Tag.Crop`, written into `solis/project.lst` like actions and sounds.
9. **Questions about the world.** "All crops within 3 metres", "the nearest enemy". **Milestone: Shared 2.**
   - Build note: the same chain as Lust's databases: `World:tagged(Tag.Crop):where(distance < 3):orderBy(distance):first(1)`.
10. **Rooms and moving between them.** Go into a house or a cellar, with a fade or a loading screen, and everything that belonged to the room goes away when you leave. (Rooms on a world map are in 2D 1.) **Milestone: Shared 6.**
    - Build note: an area as a Lust `node`: `Cellar:open()` and `Cellar:close()`, with what it made ending when it closes.
11. **Reusing things instead of making new ones (pools).** Bullets, sparks and dropped items are reused, so huge numbers stay smooth. **Milestone: Shared 6.**
    - Build note: an assembly that keeps what it makes for reuse.
12. **Paths.** Draw a curve in the editor for a guard to walk, a platform to follow, or a camera to glide along. **Milestone: Shared 4.**
13. **Finding the way (pathfinding).** Characters walk around walls to where they're going. **Milestone: 2D 1 (tile maps) and 3D 1 (navmeshes).**
14. **Behaviour for characters (AI).** A villager's day or an animal's habits, drawn as a diagram like the Animator. **Milestone: Shared 10.**
    - Build note: behaviour trees in the graph editor the animator uses, with `check` covering every outcome.

### Game data

15. **Data tables.** Lists like "every crop: days to grow, price, season", kept as a file and shown as a table in the editor. **Milestone: Shared 4.**
    - Build note: Lust's `cMap` tables, read by name with spelling checked.
16. **Weighted random.** "Usually common, sometimes rare, very seldom legendary", and dealing from a shuffled deck so nothing repeats too soon. **Milestone: Shared 3.**
    - Build note: `Random[Common 70, Rare 25, Legendary 5]` over a state's options; shuffle bags.
17. **Translations.** The game's text in several languages, with a check for missing lines. **Milestone: Shared 5.**
18. **Save slots.** Several saves with a picture, the date and time played, and autosave; the player's own settings (volume, controls, language) kept apart from saves. **Milestone: Shared 4.**
    - Build note: Lust's own `save` list (LUST.md §14).
    - Not needed as its own feature: **quest flags** ("met the mayor", "bridge repaired"). A passed object with a state per quest already does this and is kept in saves.

### Properties and the editor

19. **Field hints.** Your own settings in Properties get sliders with limits and a sentence saying what they do. Unity and Godot both have this. **Milestone: Shared 4.**
    - Build note: `float speed = 4 [range 0 10, tip "metres a second"];`, in the `[ ]` style of Lust's screens.
20. **More kinds of settings.** A setting can be a colour, a sound, a sprite, an assembly or a list, so a component can say "play this sound". Today it can only be a number, text, a choice, a position, a rotation or an entity. **Milestone: Shared 4.**
21. **Components that draw in the editor.** For example a guard's view cone, visible while you place it. **Milestone: Shared 4.**
22. **A cutscene timeline.** Arrange camera moves, sounds and animations along a timeline, like a video editor. **Milestone: Shared 10.**

### Game screens

23. **Text effects.** Text that types itself out letter by letter, wavy or shaking words, small icons inside text. **Milestone: Shared 5.**
24. **Button pictures that match the player's controller,** switching between keyboard and gamepad. **Milestone: Shared 5.**
25. **Tooltips, and drag and drop** (for inventories and shops). **Milestone: Shared 5.**

### Sound

26. **Music that follows the game.** Extra layers fade in when danger is near, and changes land on the beat. Kira, which Solis uses for sound, keeps a beat clock already. **Milestone: Shared 5.**
27. **Subtitles and captions.** **Milestone: Shared 5.**

### Testing and sharing

28. **Seeing inside the game while it runs.** Debug drawing (lines and circles over the game), a cheat box (type `give seeds 10` or `time 18:00`, with your own commands), and live values. **Milestone: Shared 6.**
29. **Game tests.** Small automatic checks like "a crop grows in three days", run the way `lust test` runs Lust's. **Milestone: Shared 2.**
30. **Bug reports from playtesters** that carry a recording of everything they pressed, so you can replay exactly what went wrong. Solis records inputs already, which few engines do. **Milestone: Shared 6.**
31. **Mods.** Your players change and add to your game with LustScript (`.lsc`), which is what LustScript was made for. **Milestone: Shared 11.**
32. **Steam:** achievements, cloud saves, the Workshop and the Steam Deck. **Not planned:** left to each game's developer, not the engine (decision 0098).
33. **Photo mode and screenshots.** **Not planned:** left to each game's developer, not the engine (decision 0098).
34. **A note for code files nothing fetches.** A `.lst` file is only part of the game if `main.lst` fetches it, or a file it fetches. One that isn't is quietly left out: its components never show in Add component, and nothing says why. Problems would show a soft hint ("`enemies/dragon.lst` isn't fetched from `main.lst`, so its components aren't in the game"), with a quick fix to fetch it and a way to silence it for parked files or folders. A small change. Making *every* `.lst` file count, as `.lua` files already do, is a bigger question for Lust's own design, and is left to the Lust project. Also worth a scale case one day: checking thousands of code files on every save. **Milestone: Shared 2.**

### Left out on purpose

- **Visual scripting** (Unreal's Blueprints): Lust is how games are written in Solis.

## Making them feel like Lust

So these don't feel bolted on, each builds on something Lust already has:

| Lust already has | Becomes |
| --- | --- |
| `every` and `[animate]` in screens | timers and tweens in game code |
| What's made in a state ends with it | timers, tweens and sounds that stop when their state ends |
| `node` opens and closes | rooms and levels |
| `assembly`, a factory with recipes | pools and spawn lists |
| A database's `:where`, `:orderBy`, `:first` | questions about the world |
| `check` covers every option | AI and dialogue that can't forget a case |
| Names checked before running (`Action.Jump`) | tags, data tables and translated text |
| Hooks aren't saved | "when done" is an event or a block, never a stored hook, so saves and replays work |
| `Random[1, 6]` | `Noise[x, y]` and weighted random |
| The `[ ]` look list in screens | field hints in Properties |
