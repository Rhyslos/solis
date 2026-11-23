# Playing a game from the command line

> Plays a level without opening the editor, with a script of what the player presses, and prints what happened: what the Console said, tick by tick, and the values of the components you ask about.

## When to use it
- **To try a change quickly**, without clicking through the game: "walk right for a second, then jump" is three lines of a script.
- **To test your game automatically**, in a workflow that runs on every change: if anything the Console says is an error, the command fails.
- **To see inside the game**: `--print Quest,Knight` shows those components' values at the end, as game code has them.

It plays the game exactly as **Play** does, screens and all, on the same [ticks](concepts.md#tick) (60 a second), so what happens here happens in the editor too.

## How to use it
1. **Write a script** of what the player does, in a plain text file (below), such as `walk.txt`.
2. **Run it** from a command prompt:

   ```text
   solis --play "C:\Games\My farm" --inputs walk.txt --print Quest,Knight
   ```

3. **Read what it prints:** what the [Console](console.md) said, each line with its tick, then the components you asked for. The command ends with 1 (failed) if the Console said any error, and 0 otherwise, so a workflow can tell.

| Option | What it does |
|---|---|
| `--play <folder>` | The project to play. |
| a level, after the folder | Which level to play, as the Library shows it: `levels/barn.sollevel`. Without it, the game's [start level](concepts.md#start-level). |
| `--ticks <n>` | How many ticks to play, from 1 to 36,000 (ten minutes). Without it: a script's last tick and one more, or 600 ticks (ten seconds) with no script. |
| `--inputs <script>` | The script of input to play with. Without it, nothing is pressed. |
| `--print <Component,…>` | The components whose values are printed at the end, on every entity that has one, separated by commas: `--print Quest,Knight`. |

## The script
Each line says what the player does on one tick, by the same [action](input.md) names game code uses:

```text
-- walk right for a second, jumping half way
at 0 hold Move 1 0
at 30 press Jump
at 60 hold Move 0 0
at 90 hold Sprint
at 120 release Sprint
```

- **`at <tick> press <Action>`** presses a button for one tick, as a quick tap does.
- **`at <tick> hold <Action>`** holds a button down until a `release`, and **`at <tick> release <Action>`** lets it go.
- **`at <tick> hold <Action> <across> <up>`** holds an [axis](input.md) (Move, Look) at two numbers from -1 to 1: `hold Move 1 0` is right, `hold Move 0 1` is up. `hold Move 0 0` lets it rest.
- **Ticks count from 0**, the first tick played, at 60 a second: tick 60 is one second in.
- `--` starts a comment. Lines can come in any order, and several can share a tick.

Game code sees exactly what it would see from a keyboard or gamepad: `Input:pressed(Action.Jump)` is true on the tick of a `press`, and `Input:held` while it's held. A script names actions, not keys, so it doesn't change when you rebind a key.

## What it prints

```text
Playing levels/farm.sollevel for 121 ticks, with walk.txt
tick 34, said: The gate opens.
After 121 ticks:
  Quest on "Quest" (5A329QT768NJB): hero = 5A329QT768NJC, stage = "Gate"
  Knight on "Knight" (5A329QT768NJC): coins = 3, hasKey = true
```

What the Console says as the level opens is on tick 0. A save the game makes while playing is kept for the play, never written into your project.

## Tips and common problems
- **Problem:** the script names an action that isn't one of the project's. **Why:** a misspelt action, or one renamed in File › Project settings › Input. **Fix:** the play says which and carries on, pressing nothing for it; correct the script.
- **Problem:** a mistake on a line of the script. **Why:** each line is `at <tick>`, then `hold`, `press` or `release`, then the action. **Fix:** the message says which line and how to write it.
- **Problem:** `no entity has a Knight`. **Why:** no entity in the level has that component, as the game has it at the end (a despawned one is gone). **Fix:** check the component's name, which is written as in its `component` line.
- **Problem:** the play stopped before its ticks ran out. **Why:** the game stopped itself (`Window:quit()`, say). **Fix:** nothing, if that's what it should do.

## Related
- [Input](input.md): the actions a script presses, and recordings
- [Checking a project before updating](checking-a-project.md): `--smoke` plays with nothing pressed
- [Game code](game-code.md)
- Concepts: [tick](concepts.md#tick), [start level](concepts.md#start-level)
