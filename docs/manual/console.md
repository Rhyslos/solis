# The Console

> Messages from the editor and the engine, and a command line for doing things by typing.

## When to use it
- **To see what's happening:** the Console reports the editor starting, the graphics card in use, models imported and reloaded, files saved, and anything that went wrong. Warnings and errors from the libraries Solis is built on (the graphics layer, for example) show here too, named after the library.
- **To do things by typing:** some jobs are quicker typed than clicked, such as selecting every entity called "Lamp", or setting the exact position of ten things at once. Everything a [command](concepts.md#tools) changes is an ordinary [undo](undo.md) step, the same as doing it with the mouse.

## How to use it

**Reading messages**
1. Each message shows its time, an icon for how serious it is, and the text. A long message is cut short; point at it to read it all.
2. The three buttons at the top count the messages of each kind: information (ⓘ), warnings (⚠) and errors (✕). Click one to hide those messages, and again to show them.
3. Type in **Search** to show only messages containing that text (capitals don't matter).
4. **Clear** empties the Console.

**Typing commands**
1. Press **'** (on a Norwegian keyboard, the key with ' and * next to Æ) to jump to the command line at the bottom of the Console. You can also click in it.
2. Type a command, such as `select Crate`, and press **Enter**. The Console repeats it after a `›`, then answers: what it found, or, if something's wrong, why (in red).
3. The command line keeps the keyboard, ready for the next command. Press **Esc** or click elsewhere to leave it.
4. **Up** and **Down** bring back earlier commands. Solis remembers the last 100, even after it's closed.
5. **Tab** finishes what you're typing: a command's name, an entity's name after `select`, or a component's setting after `get` and `set`. If several fit, they're listed above the command line; type a little more and press Tab again.

Names never care about capitals. A name with spaces goes in quotes: `select "Point light 1"`.

## Commands

| Command | What it does |
|---|---|
| `help` | Lists every command. `help set` says how one is used. |
| `clear` | Empties the Console. |
| `select Crate Lamp` | Selects every entity with those names (or [IDs](concepts.md#the-area)). `select all` selects everything, `select none` nothing. |
| `find har` | Lists the entities whose names contain "har", with their IDs. |
| `frame` | Points the camera at the selection, like **F**. |
| `new Harbour wall` | Adds an empty entity where the camera looks, named as you say, and selects it. |
| `delete` | Deletes the selection and everything under it. |
| `duplicate` | Duplicates the selection, like **Ctrl + D**. |
| `get Transform` | Shows a [component's](concepts.md#the-area) settings on the selected entity. `get Light.color` shows one. |
| `set Transform.position 0 0 2` | Changes a setting on every selected entity that has that component, as one undo step. |
| `undo`, `redo` | Like **Ctrl + Z** and **Ctrl + Y**. |
| `save` | Saves the area, like **Ctrl + S**. |
| `look unlit` | Chooses the viewport's [look](looks.md): standard, stylized or unlit. |
| `tool rotate` | Chooses a tool: select, move, rotate, scale or transform. |
| `play`, `pause`, `step`, `stop` | Like the buttons at the top (`step` runs one tick while paused): see [Play mode](play-mode.md). |
| `stats` | Counts the entities, models, lights and triangles, and says how fast the editor is drawing. |
| `seed`, `seed 42`, `seed new` | The [seed](concepts.md#game-code) the running game started with (where its random numbers come from), and the one in use if game code or a load changed it since. With a number, every Play starts from that seed, so a play can be repeated; `seed new` gives each Play a new one again. |
| `checksum` | While playing, the game's [checksums](concepts.md#simulation) at this tick: two plays that agree at the same tick are in exactly the same state. |

**Values for `set`:**
- Numbers: `set Light.strength 250`. A unit after the number is fine (`2m`).
- Three numbers (positions, scales): `0 0 2` or `0,0,2`.
- Rotations: three angles in degrees, around X, Y and Z, as Properties shows them: `set Transform.rotation 0 0 90`.
- Colours: `#ff8800` (as in a paint program), or three numbers from 0 to 1.
- Choices: one of the choices Properties offers, such as `set Light.kind spot`.
- On or off: `on`, `off`, `yes`, `no`, `true`, `false`.
- Models and entities can't be typed yet; choose them in Properties.

## Shortcuts
| Action | Shortcut |
|---|---|
| Jump to the command line | **'** (not while typing in another field) |
| Run the command | Enter |
| Earlier and later commands | Up, Down |
| Finish a name | Tab |
| Leave the command line | Esc |

## Tips and common problems
- **Problem:** pressing **'** types a ' instead of jumping to the command line. **Why:** another text field (a search box, a name) has the keyboard. **Fix:** press Esc or click the viewport first.
- **Problem:** "nothing is called …". **Why:** no entity has exactly that name. **Fix:** `find` with part of the name lists the real names; Tab finishes them for you.
- **Problem:** `set` says "nothing selected has a Light". **Why:** `set` only changes entities that have that component; it never adds one. **Fix:** select the right entities, or add the component in [Properties](entities.md) first.
- The Console keeps the newest 2,000 messages. Messages from other libraries that repeat every frame are shown once.
- Mistakes in the area itself (a missing model, say) are listed in the [Problems](problems.md) panel, beside the Console.

## Related
- [Problems](problems.md)
- [Undo and redo](undo.md)
- [Entities and components](entities.md)
- [Keyboard shortcuts](keyboard-shortcuts.md)
- [Interface tour](interface-tour.md)
- Concepts: [command, console](concepts.md#tools)
