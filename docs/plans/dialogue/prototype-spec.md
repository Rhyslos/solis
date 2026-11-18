# The dialogue prototype: the project lead's specification

The project lead, 18 November 2026. Kept as written, as the starting point for [Solis's dialogue system](README.md); only the prototype's engine is no longer named (rule 20). Solis builds on it and goes further: see the plan.

This describes the whole dialogue system — data, runtime, dialogue box and graph editor — without relying on any feature of the engine it was first made in, so it can be rebuilt from scratch in other software.

## At a glance

The system has three layers. **Data:** conversations stored as files. **Editor:** a developer tool that draws those files as node graphs. **Runtime:** the in-game part that plays a conversation and shows it in a dialogue box.

Only the dialogue manager reads conversation data during play; the dialogue box sees nothing but ready-to-show text, and everything that gates content goes through conditions and effects reading the game state store.

## Data model

A conversation is a **directed graph of nodes**, stored as one data file. Nodes point to each other by a **text ID**, never by direct reference, so the file can be saved, loaded and edited as plain data.

### Conversation (graph)

| Field | Type | Meaning |
| --- | --- | --- |
| `GraphId` | text | Unique name of the conversation, e.g. `ExampleGardener`. Used to build translation keys. Defaults to the file name. |
| `StartNodeId` | text | ID of the node the conversation starts at. |
| `Nodes` | list of nodes | Every node in the conversation, in any order. |

One operation: **FindNode(id)** — return the node whose `NodeId` equals `id`, or nothing. A linear search is fine; conversations are small.

### Node (shared by all node types)

| Field | Type | Meaning |
| --- | --- | --- |
| `NodeId` | text | Unique within the conversation, e.g. `start`, `askAboutFlowers`, `line_3`. |
| `Effects` | list of effects | Run the moment the conversation reaches this node. |
| `GraphPosition` | 2D point | Where the node sits in the editor. Ignored at runtime. |

There are four node types. Every "next" field holds a node ID; **an empty next ID ends the conversation**.

| Type | Extra fields | Behaviour |
| --- | --- | --- |
| **Line** | `Speaker` (speaker or none), `TextKey`, `Text` (multi-line, supports bold/italic markup), `NextNodeId` | Shows one line, waits for the player to continue. |
| **Choice** | `Choices` (list of choice options) | Shows options; waits for the player to pick one. The previous line stays on screen. |
| **Branch** | `Branches` (list of branch routes), `DefaultNextNodeId` | Invisible. Jumps to the first route whose conditions are all met, else the default. |
| **End** | none | Ends the conversation. |

### Choice option

| Field | Type | Meaning |
| --- | --- | --- |
| `TextKey`, `Text` | text | Translation key and English text shown on the option. |
| `Conditions` | list of conditions | All must be met for the option to be pickable. |
| `HideWhenUnavailable` | yes/no (default yes) | Yes: hidden when conditions fail. No: shown as "(Locked)" and not pickable. |
| `Effects` | list of effects | Run when the player picks this option, before moving on. |
| `NextNodeId` | text | Where picking it goes. |

### Branch route

| Field | Type | Meaning |
| --- | --- | --- |
| `Conditions` | list of conditions | All must be met for this route. |
| `NextNodeId` | text | Where this route goes. |

### Speaker (its own reusable file, shared across conversations)

| Field | Type | Meaning |
| --- | --- | --- |
| `SpeakerId` | text | e.g. `exampleGardener`. |
| `DisplayNameKey`, `DisplayName` | text | Translation key and English name shown above the text. |
| `Portrait` | image or none | Shown beside the text; hidden when empty. |

### Dialogue entry (used by NPCs, see "Starting conversations")

| Field | Type | Meaning |
| --- | --- | --- |
| `Conditions` | list of conditions | All must be met for this entry. |
| `Dialogue` | conversation | The conversation to play. |

An entry is **available** when it has a conversation and all its conditions are met.

## Game state, conditions and effects

The dialogue system never checks quests, relationships or inventory directly. It only knows two abstract things: **conditions** (a yes/no question) and **effects** (something that happens). Other systems add their own condition and effect types later without changing the dialogue code.

### Game state store (one global instance)

A single store that lives for the whole game session and holds two dictionaries:

| Store | Key → value | Missing key reads as |
| --- | --- | --- |
| Flags | text → yes/no (e.g. `metMira`) | no |
| Values | text → whole number (e.g. `miraFriendship`) | 0 |

Operations: `GetFlag`, `SetFlag`, `GetValue`, `SetValue`, `AddToValue(name, amount)`. Setting a flag or value to what it already is does nothing. Any real change raises a **StateChanged(name)** notification. Names are case-sensitive. Nothing is saved to disk yet.

### Conditions

A condition is a small data object with one operation, **IsMet() → yes/no**. A **list** of conditions passes only when every condition in it is met; an empty or missing list passes. Empty slots in a list are skipped.

| Condition | Settings | IsMet when |
| --- | --- | --- |
| Flag | `FlagName`, `ExpectedValue` (default yes) | the flag equals the expected value |
| Value | `ValueName`, `Operator`, `Target` | value *operator* target, where operator is =, ≠, <, ≤, >, ≥ (default ≥) |
| Any of | list of conditions | at least one inner condition is met (an empty list is **not** met) |
| All of | list of conditions | every inner condition is met |
| Not | one inner condition | the inner condition is set and **not** met |

Any of, All of and Not can be nested to any depth, e.g. "(met Mira AND friendship ≥ 3) OR quest done". If the game state store doesn't exist, Flag and Value conditions return no.

### Effects

An effect is a small data object with one operation, **Apply()**. Applying a list runs each effect in order, skipping empty slots.

| Effect | Settings | Apply does |
| --- | --- | --- |
| Set flag | `FlagName`, `Value` (default yes) | sets the flag |
| Change value | `ValueName`, `Mode` (Set or Add, default Add), `Amount` (default 1) | sets the value, or adds to it |

### Extending it

A new system (quests, relationships, money) adds its own subclasses, e.g. `QuestStateCondition` or `StartQuestEffect`. The dialogue runtime, the editor and the dialogue data don't change: they only call `IsMet()` and `Apply()`. In the editor, the new types simply appear in the list of things you can add.

## Runtime: how a conversation runs

One global **dialogue manager** runs at most one conversation at a time. Branch nodes are resolved instantly; only Line and Choice nodes wait for the player.

### State

The current conversation (or none), the current node, and the list of choice options currently on screen. **IsActive** = a conversation is set.

### Notifications sent to the dialogue box

| Notification | Data |
| --- | --- |
| Started | none |
| LineShown | speaker name (translated, empty for narration), portrait (or none), line text (translated) |
| ChoicesShown | list of (index, text (translated), is available) |
| Ended | none |

The box never touches the conversation data; everything it shows arrives already translated.

### Operations

**StartDialogue(conversation) → yes/no**

1. Refuse (return no) if a conversation is already running or none was given.
2. Set it as current, ask the pause manager to pause the game, send **Started**.
3. Enter the conversation's start node. Return yes.

**Enter node (id)** — a loop, at most **256** steps per call:

1. If no conversation is running any more, stop.
2. Empty ID → end the conversation.
3. Find the node. Not found → report an error naming the conversation and ID, end the conversation.
4. Make it the current node and **apply its effects**.
5. By type:
   - **Line:** send LineShown, stop and wait.
   - **Choice:** show choices (below), stop and wait.
   - **Branch:** take the next ID of the first route whose conditions are all met, else the default ID, and go round the loop again.
   - **End:** end the conversation.
   - Unknown type: report an error, end.
6. If 256 steps pass without reaching a Line, Choice or End (branches pointing at each other), report "branches loop" and end.

**Show choices:**

1. Clear the on-screen option list.
2. For each option (skipping empty slots): check its conditions. If not met and it hides when unavailable, skip it. Otherwise add it to the on-screen list and to the notification, with its index in the **on-screen** list.
3. If no listed option is available, report a warning and end the conversation (so the player can't get stuck).
4. Otherwise send **ChoicesShown**.

**Advance():** only when the current node is a Line, enter its next ID.

**Choose(index):** only when the current node is a Choice and the index is within the on-screen list. Re-check the option's conditions (ignore if not met), apply the option's effects, enter its next ID.

**EndDialogue():** if running, clear the conversation, current node and on-screen options, release the pause request, send **Ended**.

### Pausing

The pause manager keeps a set of "who wants the game paused". The game is paused while the set isn't empty. The dialogue manager adds itself on start and removes itself on end, so other systems (menus, inventory) can pause independently without unpausing each other. The world (movement, clock, triggers) stops while paused; the dialogue box keeps running.

## The dialogue box

The dialogue box is a screen-space panel drawn on top of the game world, above other HUD layers. It never reads the conversation data itself: it only reacts to the runtime's four notifications (Started, LineShown, ChoicesShown, Ended) and calls back `Advance()` or `Choose(index)`.

### Layout

Measured for a 640 × 360 game resolution (scaled up to the window). Scale proportionally for other resolutions.

- **Panel:** stretches across the bottom of the screen, 16 px in from the left and right edges. Its bottom edge sits 52 px above the screen bottom (originally to clear the hotbar) and its starting height is 58 px. It **grows upward** when its content needs more room, for example when several choices are listed. Hidden when no conversation is running.
- **Inner padding:** 6 px left and right, 4 px top and bottom.
- **Row inside:** portrait on the left, text column on the right, 6 px apart.
  - **Portrait:** 48 × 48 px, aligned to the top, image scaled to fit and centred with its aspect ratio kept. Hidden when the speaker has no portrait or there is no speaker.
  - **Text column** fills the remaining width, items 2 px apart, top to bottom:
    1. **Speaker name:** font size 10. Hidden when empty (narration).
    2. **Line text:** font size 10, wraps to the column width, grows to fit its text, no scrollbar. Supports simple markup: bold, italic.
    3. **Continue marker:** the character "»", font size 10, right-aligned. Shown only when the line has finished appearing and there are no choices.
    4. **Choice list:** one button per shown choice, stacked vertically 2 px apart, text left-aligned, font size 10. Hidden unless choices are showing.
- **Style:** plain default panel and font. All final visuals (box frame, font, portrait frame, continue marker) are for the artist.

### Text reveal (typewriter)

- A setting `CharactersPerSecond`, default 40. **0 means instant text** (an accessibility option).
- When a line arrives: set the full text, count its visible characters (markup tags excluded), hide the continue marker, start revealing from 0.
- Every frame: add `CharactersPerSecond × frame time` to a running counter and show that many characters. When the counter reaches the total, show everything and switch to "finished".
- Finishing shows the continue marker (unless choices are showing).

### Choices

- On ChoicesShown: clear old buttons, make sure the current line is fully revealed, hide the continue marker, create one button per choice.
- An available choice shows its text. A locked choice shows "*text* (Locked)" (the word "Locked" comes from translation key `DIALOGUE_CHOICE_LOCKED`) and is disabled, so lock state is shown in words, not colour alone.
- Keyboard/controller focus goes to the **first available** choice (after the buttons exist, i.e. one frame later).
- Up/down (arrow keys, d-pad, stick) moves between choices using the standard UI navigation.
- Pressing a button calls `Choose(that button's index)`.

### Input (all through remappable actions, never hard-coded keys)

The box listens for two actions: **interact** (E key, controller A) and **accept** (Enter, Space, controller A). Only while the box is visible:

| Situation | Interact or accept does |
| --- | --- |
| Text still revealing | Reveal the rest instantly |
| Choices showing | Press the focused choice (accept on a focused button presses it directly) |
| Line finished | `Advance()` |

The press is consumed so nothing else in the game reacts to it. The mouse can click choice buttons but cannot advance lines.

### Ending

On Ended: stop revealing, remove choice buttons, hide the panel.

### Pausing

The box keeps running while the game is paused (the runtime pauses the game during conversations), so its input and text reveal still work.

## Starting conversations from NPCs and objects

Anything the player can talk to (an NPC, a sign, a door) gets a **dialogue trigger** component. It has:

- an **interaction area**: a trigger zone (e.g. a circle with a 24 px radius) around the object
- an ordered list of **dialogue entries** (conditions + conversation)

Behaviour:

1. When a body tagged **player** enters the area, remember "player in range". Forget it when the player leaves.
2. When the **interact** action is pressed and the player is in range, pick the **first available entry** from the top of the list (all its conditions met).
3. If one was found and the runtime agrees to start it, consume the key press so the dialogue box doesn't also react to it.
4. If none are available, nothing happens.

The trigger only listens for input while the game is **not** paused, so pressing interact on the last line of a conversation (which unpauses the game) can't instantly restart it.

Order entries from most specific to least specific, and leave the last one without conditions as a fallback:

| # | Conditions | Conversation |
| --- | --- | --- |
| 1 | flag `deliveredFlowers` is on | thank-you conversation |
| 2 | flag `metMira` is on | small talk |
| 3 | none | first meeting (its first node sets `metMira`) |

## Localisation

Every piece of player-facing text is stored twice: a **translation key** and the **English text**. Display goes through one helper:

**Resolve(key, englishText):**

1. If the key is empty, return the English text.
2. Look the key up in the active language's translation table.
3. If the table returns the key itself (no translation found), return the English text. Otherwise return the translation.

The game works fully in English with no translation files at all.

### Where keys come from

| Text | Key format | Example |
| --- | --- | --- |
| Line | `<GraphId>.<NodeId>` | `ExampleGardener.firstMeeting` |
| Choice option | `<GraphId>.<NodeId>.choice<n>` (n from 1) | `ExampleGardener.menu.choice2` |
| Speaker name | set by hand on the speaker | `SPEAKER_EXAMPLE_GARDENER` |
| "(Locked)" label | fixed | `DIALOGUE_CHOICE_LOCKED` |

The editor fills in **missing** keys automatically every time a conversation is saved. Keys are never changed afterwards, so renaming a node doesn't break existing translations.

### Export format

The editor exports one CSV spreadsheet per conversation:

- First row: `keys,en`.
- One row per key: key, then English text. Both columns are wrapped in double quotes, with any `"` inside doubled (`""`). Line breaks inside a quoted cell are allowed.
- Row order: walk the nodes in list order. For each line, its speaker's name (if the speaker has a key) and then the line text. For each choice node, each option's text. Each key appears only once.
- Translators add one column per language (e.g. `nb`, `de`), which the game then loads as translation tables.

## The graph editor

The editor is a developer tool, separate from the game. It edits conversation files visually and never runs during play. In the prototype it was a full-screen tab next to the engine's 2D, 3D and script views; elsewhere it can be its own window or tool.

### Window layout (top to bottom)

1. **Toolbar** (one row, left to right): `New` · `Open` · `Save` · separator · `Add Node ▾` (dropdown: Line, Choice, Branch, End) · `Set as Start` · `Arrange` · separator · `Export Translations` · a right-aligned status label. The label shows the file name, plus "(unsaved)" when there are changes, or "No dialogue open. Use New or Open, or double-click a conversation file."
2. **Graph canvas**: an infinite, pannable, zoomable area holding the node boxes and wires. It fills most of the window.
3. **Problems list**: a strip at least 80 px tall under the canvas, with a draggable divider between them.

A separate **property panel** (in the prototype, the engine's normal inspector) edits the selected node's fields.

### Node boxes

Each node is drawn as a box with a **title bar** and a column of **rows**. Rows are 220 px wide with word-wrapped text. A row can have an **input port** on its left edge, an **output port** on its right edge, or neither.

Title: `<Type>: <NodeId>`, plus `   [Start] ` on the start node. The marker is text, so it isn't shown by colour alone.

| Node | Rows, top to bottom | Ports |
| --- | --- | --- |
| Line | "*Speaker*: *text preview*" (speaker part left out when none) | that row: input + output |
| Choice | "Player chooses:" · one row per option: "*n*. *text preview*" plus "  \[if, hidden\]" or "  \[if, locked\]" when it has conditions · a `+ Choice` button | first row: input · each option row: output |
| Branch | "Go to the first match:" · one row per route: "If *n* condition(s) met" · "Otherwise" · a `+ Branch` button | first row: input · each route row and Otherwise: output |
| End | "End of conversation" | input only |

Every node also gets a last row, "Effects on enter: *n*", when it has effects (no ports).

Text preview: line breaks become spaces; text over 60 characters is cut to 60 plus "…"; empty text shows "(no text)".

### Ports and wires

- **Input port index:** every node has exactly one input, index 0.
- **Output port indexes** count only rows that have an output, top to bottom. This gives the mapping:

| Node | Output port *i* writes to |
| --- | --- |
| Line | port 0 → `NextNodeId` |
| Choice | port *i* → option *i*'s `NextNodeId` |
| Branch | port *i* (i < route count) → route *i*'s `NextNodeId`; port = route count → `DefaultNextNodeId` |

- A wire is drawn from output port *i* of node A to the input of node B whenever that field holds B's ID. An output can hold only one wire, so a new connection replaces the old one.
- IDs pointing at nodes that don't exist draw no wire; the problems list reports them instead.

### Interactions

| Action | How |
| --- | --- |
| Add a node | Right-click empty canvas → pick a type (placed where you clicked); or toolbar `Add Node` (placed in the middle of the view) |
| Add a node already wired | Drag from an output and release on empty canvas → pick a type; the new node is connected to that output |
| Connect | Drag from an output port to a node's input port |
| Disconnect | Drag a wire's end off the input port and release on empty space |
| Move | Drag nodes; positions are written to `GraphPosition` when the drag ends |
| Select | Click a node; its fields open in the property panel |
| Delete | Select one or more nodes, press Delete. Every link pointing at a deleted node is cleared; if the start node is deleted, the start is cleared |
| Add option or route | `+ Choice` adds an option with text "New choice"; `+ Branch` adds an empty route |
| Set start | Select a node, press `Set as Start` |
| Tidy layout | `Arrange` selects all nodes and runs an automatic left-to-right layout that minimises crossing wires, then stores the positions |
| Jump to a problem | Click a problem; that node is selected and scrolled to the centre of the view |
| Undo / redo | Ctrl+Z / Ctrl+Shift+Z |
| Save | `Save` or Ctrl+S; closing with unsaved changes asks first |
| Open | Double-click a conversation file, or `Open` |

### New nodes

- **ID:** `<type>_<n>`, with the lowest *n* from 1 that isn't taken, e.g. `line_1`, `choice_2`, `branch_1`, `end_1`.
- **Contents:** a new Choice node starts with one option, "New choice".
- **Start:** if the conversation has no start node yet, the new node becomes the start.
- **New file:** creates an empty conversation whose `GraphId` is the file name.

## Editor internals

### One source of truth

The conversation data is the only truth. Every edit writes to the data first, then the editor **redraws the whole canvas from the data**: it removes every node box and wire, recreates a box per node, restores the selection, rewires, and re-runs validation. Conversations are small, so a full redraw is cheap and avoids the view and the data ever disagreeing.

Node boxes are named internally `Node0`, `Node1`, … in list order rather than by node ID. That way, two nodes accidentally sharing an ID can't break the canvas; wires are resolved by looking up IDs (the first node with that ID wins).

### Editing in the property panel

Whenever any field of the open conversation or one of its nodes changes in the property panel, the editor:

1. **Detects renames:** compares every node's current ID with the ID it had at the last redraw. For each one that changed, every link that pointed at the old ID (start node, next IDs, choice and route targets) is rewritten to the new ID.
2. Marks the conversation unsaved.
3. Redraws.

### Validation (the problems list)

Run after every change. Each problem has an optional node ID ("Graph" when it applies to the whole conversation) and a message.

| Check | Message |
| --- | --- |
| An empty slot in the node list | The node list contains an empty entry. |
| A node with a blank ID | A node has no ID. |
| Two nodes with the same ID | Another node already uses this ID. |
| No start node set | No start node is set. |
| Start ID doesn't match a node | The start node '*id*' doesn't exist. |
| Line with empty text | The line has no text. |
| Choice node with no options | The choice node has no choices. |
| Option with empty text | Choice *n* has no text. |
| Branch with no routes | The branch has no conditions, so it always goes to Otherwise. |
| A next ID pointing at a missing node | Links to '*id*', which doesn't exist. |
| Node not reachable from the start | Can't be reached from the start node. |

Reachability: breadth-first search from the start node, following every output target that exists; only run when the start node exists. With no problems, the list shows "No problems found." (not selectable).

### Undo and redo

Edits made on the canvas (add, delete, connect, disconnect, move, set start, arrange, add option or route) are recorded as named undo steps, e.g. "Delete Dialogue Nodes".

- **Snapshot:** before the edit, capture a snapshot of the conversation's structure. Make the edit. Capture another snapshot. Register undo = restore *before*, redo = restore *after*.
- **What a snapshot holds:** the start ID, the ordered list of node **objects** (references, not copies), and per node its position, its output targets, and (for choice and branch nodes) the ordered list of option or route objects.
- **Restoring** puts those exact objects back in that order and rewrites their positions and targets. Because object identity is kept, undo steps made in the property panel on those same nodes stay valid.
- **Property panel edits** use the host program's normal undo; the canvas redraws (and re-checks renames) whenever the undo history changes.

### Saving

1. Store every box's current position into its node.
2. Fill in missing translation keys.
3. Write the file.

A conversation embedded inside another file can't be saved on its own; the editor tells the user to save the containing file instead.

## Rebuild order

Each step can be tested before the next one starts.

1. **Game state store, conditions and effects.** Flags, values, the five condition types, the two effect types, and "all met" / "apply all" over lists. Test with plain code.
2. **Data model and file format.** Conversation, the four node types, choice options, branch routes, speakers, entries. Make sure a conversation can be saved and loaded with nested conditions and effects intact.
3. **Localisation helper.** `Resolve(key, englishText)` with the fall-back rules.
4. **Runtime.** Start, the node-entering loop, Advance, Choose, End, the step limit and the four notifications. Test by printing notifications from a hand-written conversation.
5. **Pause hook.** A pause manager with pause requests, so the conversation pauses the world.
6. **Dialogue box.** Layout, typewriter with an instant option, choices with focus, locked labels, interact/accept input.
7. **Dialogue trigger.** Area, player tag, ordered entries.
8. **Graph editor.** Canvas, node boxes with ports, wires and the port mapping, add/connect/disconnect/delete/move, property panel, rename handling, set start, arrange, open/new/save.
9. **Validation, translation keys and CSV export.**
10. **Undo/redo** using structure snapshots.

### Things that are easy to miss

- Effects on a node run **every time** the node is entered, including on loops back to a menu.
- A choice option's effects run **before** moving to its next node; a node's own effects run on **entry**.
- An empty next ID ends the conversation, the same as an End node.
- "Any of" with an empty list is **not** met; an empty condition list on an option, route or entry **is** met.
- The key press that starts a conversation must not also advance its first line.
- Locked choices must say so in text, not only by colour or greyed-out styling.
- Translation keys are filled once and never regenerated.
