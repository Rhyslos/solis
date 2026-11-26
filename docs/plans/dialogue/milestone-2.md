# Dialogue, milestone 2: conversations and their runtime

The plan for the second milestone of [the dialogue system](README.md): conversations as Lust data files, speakers, the four node kinds, conditions and effects through game code, the runtime and its events, holding the game still, and saves. It follows [the scope reset](../scope-reset.md): game code is Lust only, and the work is split into steps a builder finishes in well under an hour, one pull request each.

**This milestone takes the full path** (the scope reset's table): a conversation is a new file format and a conversation in progress is saved game state. So every step runs the full checks in CLAUDE.md's Builds, anything read from a file gets a fuzz target, the running conversation joins the checksum, and the decision record, the tracker, ARCHITECTURE.md and the CLAUDE.md sentence are updated.

**It has no UI of its own.** Until the conversation editor (milestone 4), conversations are written by hand or by tests, and game code shows lines with a screen of its own. The dialogue box (milestone 3) and the editor (milestone 4) are mocked up already, for approval before they're built:

| Milestone | Mock-up | What it shows |
| --- | --- | --- |
| 3, the dialogue box | [Dialogue box mock-up](https://claude.ai/artifact/77CrihfvXyDwM318Q4fEFp) | A line typing, a line with three choices (one locked), a long line that grows the box upward, narration with no speaker, the same box restyled by a game maker, the box while playing in the editor, and its file on the UI tab |
| 4, the conversation editor | [Conversation editor mock-up](https://claude.ai/artifact/N2YvGVm2oZ8G9j7jN4LXfD) | The Dialogue tab with a small conversation that starts with a branch (the branch picked), a choice node picked with its options in Properties, and a problem shown and picked from the Problems list |

Wherever there's a real choice, the plan follows the recommended answer and says what changes under the others. The choices are listed at the end, under [Questions for the project lead](#questions-for-the-project-lead), one question each.

## What the milestone gives a game maker

When it's done, a game maker can:

1. write `conversations/village/gardener.conversation.lst` and `speakers/mira.speaker.lst` by hand;
2. start it from game code: `startConversation(Conversation.Village.Gardener);` inside a component;
3. hear `ConversationStarted`, `LineShown`, `ChoicesShown` and `ConversationEnded` on that entity;
4. move it on with `Dialogue:advance()` and `Dialogue:choose(2)`, and read what to show with `Dialogue:current()`, from game code and from screens;
5. gate options and routes with conditions, and tell the game what happened with effects, both one line of Lust, checked before the game runs;
6. save and load in the middle of a conversation, and get the same game back on any number of cores.

A screen of their own shows it until milestone 3 brings the ready-made box.

## The conversation file

A conversation is a Lust data file (decision 0081), `<name>.conversation.lst`, anywhere in the Library; `conversations/` is the folder the manual suggests. Its name in game code comes from its place, folders as options inside options: `conversations/village/gardener.conversation.lst` is `Conversation.Village.Gardener` (the leading `conversations` folder is left out, as `ui/` is for screens).

Lust lists hold one kind of thing, so each node kind gets its own list (as decision 0081 says each format gets its own shape). Nodes point at each other by ID, never by place in a list; an empty `next` ends the conversation, as an End node does.

```lust
-- A conversation. Written by Solis; safe to edit by hand.
fetch "solis/data/conversation":[Conversation, Uses, Line, Choice, Option, Branch, Route, End];

lock int version = 1;
lock Conversation conversation = new Conversation { start = "check", holdsGame = true };

-- The game's names the conditions and effects use, as a code file's fetch lines.
lock Uses array uses = [
    new Uses { file = "quest", names = ["Quest", "GaveSeeds", "SeedsReturned"] }
];

lock Line array lines = [
    new Line { id = "firstMeeting", speaker = "Mira", key = "Gardener.firstMeeting", text = "Oh! A new face in the village. Have you seen my [colour=#e8955a]red roses[/colour]?", effects = [], next = "menu", x = 224, y = 40 },
    new Line { id = "roses", speaker = "Mira", key = "Gardener.roses", text = "They came from my grandmother's garden by the sea.", effects = [], next = "menu", x = 672, y = 22 },
    new Line { id = "thanks", speaker = "Mira", key = "Gardener.thanks", text = "My seeds! You found them.", effects = ["send SeedsReturned() to World:the(Component.Quest);"], next = "bye", x = 440, y = 262 }
];
lock Choice array choices = [
    new Choice { id = "menu", effects = [], x = 440, y = 20 }
];
lock Option array options = [
    new Option { choice = "menu", key = "Gardener.menu.choice1", text = "Tell me about the roses.", condition = "", hideWhenUnavailable = true, effects = [], next = "roses" },
    new Option { choice = "menu", key = "Gardener.menu.choice2", text = "I brought your rose seeds.", condition = "World:the(Component.Quest):quest().hasSeeds", hideWhenUnavailable = false, effects = ["send GaveSeeds() to talker;"], next = "thanks" }
];
lock Branch array branches = [
    new Branch { id = "check", effects = [], otherwise = "firstMeeting", x = 16, y = 168 }
];
lock Route array routes = [
    new Route { branch = "check", condition = "World:the(Component.Quest):quest().hasSeeds", next = "thanks" }
];
lock End array ends = [
    new End { id = "bye", effects = [], x = 672, y = 300 }
];
end
```

- **IDs** are unique in a conversation: letters, numbers and `_`, starting with a letter (`firstMeeting`, `line_3`). They're the file's own; game code never names a node.
- **A choice's options and a branch's routes** are kept in their own lists, in order, each naming its node (`choice = "menu"`). Order in the list is the order shown or tried.
- **`speaker`** names a speaker as game code does, without `Speaker.` (`"Mira"`, or `"Village.Baker"` for one in a folder); empty is narration.
- **`text`** is the main language's words, with the text effects screens know (`[wave]`, `[colour=…]`, `[pause 0.5]`). **`key`** is its [text key](../../manual/concepts.md#text-key), made once and never changed: `<Conversation>.<node id>`, and `<Conversation>.<choice id>.choice<n>` for an option, filled in by Solis when it writes the file and worked out the same way while a hand-written file lacks one.
- **`condition`** is one line of Lust giving yes or no; empty means always. **`effects`** are lines of Lust that send events (see [Conditions and effects](#conditions-and-effects)).
- **`holdsGame`** says whether the game stands still while it runs (on by default; see [Holding the game still](#holding-the-game-still)).
- **`x` and `y`** are where the node sits in the editor; the runtime ignores them.
- **Limits**, refused in plain words when a file passes them, keeping what came before with a note: 10,000 nodes, 32 options a choice, 32 routes a branch, 16 effects a node or option, 10,000 characters of text a line, 1,000 characters a condition or effect, 64 `uses`. A damaged node, option or route is left out with a note naming it; a link to a node that isn't there is kept (the editor's Problems list shows it, and the runtime ends there with a Console message).

### Speakers

A speaker is a file of its own, `<name>.speaker.lst` (`speakers/mira.speaker.lst` is `Speaker.Mira`), shared by every conversation:

```lust
-- A speaker. Written by Solis; safe to edit by hand.
fetch "solis/data/speaker":Speaker;

lock int version = 1;
lock Speaker speaker = new Speaker { name = "Mira", key = "Speakers.Mira", portrait = "portraits/mira.png" };
end
```

`name` is the main language's words for the name, `key` its text key (`Speakers.<Name>`, the form subtitles already use), and `portrait` a picture by its place in the Library, as a screen's `image` names one (`"portraits/faces.png#Mira"` for a crop); empty for none.

## Conditions and effects

The dialogue system has no store of its own for story values (decision 0124): a condition asks the game a question, and an effect tells the game something happened.

- **A condition is one line of Lust that gives yes or no.** It reads the game **as the tick started**, as all game code does: game-wide objects (`Story.metMira`), the entity that started the conversation (`talker`), and any other entity's components through Fixes 2a (`talker:shopkeeper().met`, `World:the(Component.Quest):quest().hasSeeds`). It can't change anything: a call that would (`World:say`, `moveTo`) stops it with a message naming the conversation and node, as a screen is stopped.
- **An effect is a line of Lust that sends an event**: `send GaveSeeds() to talker;`, `send MetMira() to World:the(Component.Quest);`. The game's own `on` blocks decide what happens, as they do for a screen's button. An effect line must start with `send`; anything else is a mistake in Problems.
- **Names are checked before the game runs** (rule 25). Solis writes one Lust file per conversation, `solis/conversations/village/gardener.lst`, holding a method per condition and per effect list, fetching the conversation's `uses`. Lust checks it with the rest of the game's code, and a mistake is shown on the conversation, not the written file: "gardener, choice menu, option 2's condition: Quest has no field hasSeds. Did you mean hasSeeds?".

What Solis writes for the example above (never edited by hand; `talker` is the entity that started the conversation):

```lust
-- solis/conversations/village/gardener.lst, written by Solis from
-- conversations/village/gardener.conversation.lst. Never edited by hand.
fetch "solis/world":World;
fetch "solis/project":Component;
fetch "quest":[Quest, GaveSeeds, SeedsReturned];

class VillageGardenerConversation {
    method bool when1(entity talker) { return World:the(Component.Quest):quest().hasSeeds; }
    method bool when2(entity talker) { return World:the(Component.Quest):quest().hasSeeds; }
    method do1(entity talker) { send SeedsReturned() to World:the(Component.Quest); }
    method do2(entity talker) { send GaveSeeds() to talker; }
}
pass VillageGardenerConversation;
end
```

**A line can't escape its method.** Each line is pasted into the file, so Solis refuses, before writing, a condition or effect holding `{`, `}`, `--`, a line break, or (for a condition) `;`, and checks that brackets and quotes pair up. A refused line is a mistake in Problems naming the node, and its method is written as `return false;` (a condition) or empty (an effect), so the rest of the game still checks.

## Game code

A new engine file, `solis/dialogue.lst`, for Lust only (decision 0135). Each call checked against the guidelines' checklist:

| Call | What it does |
| --- | --- |
| `startConversation(Conversation.Village.Gardener)` | Inside a component: asks to start that conversation, with this entity as its **talker**. It starts as the next tick begins. Asking again for the one that's running with the same talker does nothing (safe every frame). |
| `Dialogue:advance()` | Moves on from the line shown, as the next tick begins. Several calls in one tick count once. Does nothing while choices show or nothing runs. |
| `Dialogue:choose(2)` | Picks the second choice shown (counting from 1, as the editor numbers them), as the next tick begins. A number past the choices shown, or a locked choice, does nothing, and the Console says so once. |
| `Dialogue:stop()` | Ends the conversation, as the next tick begins. |
| `Dialogue:current()` | What to show, as the tick started: a `DialogueNow` (below). Game code and screens both read it. |

`DialogueNow` holds: `talking` (yes or no), `conversation` (a `Conversation`), `talker` (an entity, or nothing), `waiting` (`DialogueWait.Nothing`, `.Line` or `.Choices`), `hasSpeaker`, `speaker` (a `Speaker`), `name` and `text` (in the player's language, with text effects), `portrait` (a picture's name, empty for none) and `choices`, a list of `DialogueChoice` with `text` and `available`. Locked choices are in the list with `available` false; hidden ones aren't.

**Events**, sent to the talker, carrying the conversation as a state:

| Event | When |
| --- | --- |
| `ConversationStarted(Conversation which)` | It started. |
| `LineShown(Conversation which)` | A line is waiting for the player. |
| `ChoicesShown(Conversation which)` | Choices are waiting for the player. |
| `ConversationEnded(Conversation which)` | It ended: an End node, an empty next, `Dialogue:stop()`, a mistake, or the talker gone. |
| `ConversationRefused(Conversation which)` | Another conversation was running, or another entity asked first in the same tick. |

Events carry no words: the words are for showing, never for deciding (the translations page's rule). Game code that wants the words reads `Dialogue:current()`.

**When things happen in a tick.** Requests (start, advance, choose, stop) are kept as changes in the tick's order (entity, then component, then call), like timers' starts. As the next tick begins, after the timers make their events and before the game-wide objects run, the dialogue steps once on the first shard: it takes the requests in order, enters nodes (branches at once, at most 256 steps, as the prototype), calls conditions and effects in that shard's Luau against the world as the tick started, and makes its events. Its events and the effects' sends are heard in that tick's first round. So the press that starts a conversation is a tick old when the first line shows, and can't also move it on.

### An example

```lust
-- gardener.lst: Mira talks when the player presses Interact beside her.
fetch "solis/input":Input;
fetch "solis/project":[Action, Conversation];
fetch "solis/dialogue":ConversationEnded;
fetch "solis/zones":[Entered, Exited];

component Gardener {
    has {
        bool playerNear = false;
        bool talking = false;
    }

    on {
        -- Mira's trigger zone (Only tags: Player) tells us when the player is close.
        Entered(entity who) { playerNear = true; }

        Exited(entity who) { playerNear = false; }

        ConversationEnded(Conversation which) {
            talking = false;
        }
    }

    method update(float dt) {
        -- Interact beside her starts the conversation; Solis does the rest.
        is playerNear && not talking && Input:pressed(Action.Interact)? then {
            startConversation(Conversation.Village.Gardener);
            talking = true;
        }
    }
}
pass Gardener;
end
```

## Holding the game still

A conversation whose `holdsGame` is on holds the game still while it runs: game time, physics, sprites and timers stand as at game speed 0, while game code, events and screens go on (so the box types out and the player can answer). It's a hold of its own, on top of the game's speed: `World:gameSpeed()` still says what the game set, a pause menu's `World:setGameSpeed(1)` doesn't end the conversation's hold, and the conversation ending doesn't end the menu's pause. The Game panel's corner says *Held by a conversation* meanwhile.

## What's saved

A conversation in progress is game state:

- **Saved:** which conversation, its talker, the node it's on, the choices shown (which options, in order, and whether each was available), and the requests waiting for the next tick. Save games go up one version (14, unless another milestone takes it first); older saves load with no conversation running.
- **Not saved:** the words (they're worked out again in the player's language), and how far a line has typed (it's what a screen shows, as text effects say).
- **In the checksum** once a conversation runs or a request waits, so games recorded before it keep their sums.
- **A changed file while playing or loading:** new code or a changed conversation reaches the game at the next phase, and a save may be loaded after its conversation changed. Either way, if the node it's on is gone, the conversation ends, the talker hears `ConversationEnded`, and the Console says why. Conversations' contents aren't hashed into saves as tables' are: a conversation that still has its node carries on.

## What this leans on from Fixes 2a

Fixes 2a is being built now. The runtime doesn't need it, but what game makers write in conditions and effects does:

- **`other:knight()`**, reading another entity's component as the tick started: conditions read the talker's and other entities' components this way (`talker:shopkeeper().met`). It's also why conditions see the world as the tick started: that's the view 2a gives, on whichever shard asks.
- **`World:the(Component.Quest)`**, the one entity with a component: the usual place a game keeps its story's state, for conditions to read and effects to send to. The `Component` state it takes is 2a's.
- **`entity:exists()`**: the runtime ends a conversation whose talker no longer exists, using the same check, and the manual shows conditions guarding another entity with it.

Steps 1, 2 and 4 can be built before 2a is merged. Steps 3 and 5 onward use its calls in tests and the manual, so they start once 2a is in `main`; if 2a's names change when it lands, only the examples and tests change.

## The steps

Each step is one pull request on the full path, built and checked before the next starts. Times are a builder's, aiming well under an hour.

### Step 1: the conversation and speaker files (about 45 minutes)

- `level::conversation` and `level::speaker`: read and write version 1 as above; the classes in `solis/data/conversation` and `solis/data/speaker`; the kinds `conversation` and `speaker` in `reflect::data_files`, so the Library knows them (with their own icon and colour, as tables have).
- Missing keys worked out from IDs when read; written when Solis writes the file.
- The decision record (the next free number) with the answers to the questions below, and its line in DESIGN.md §17.
- **Tests:** a full conversation reads and writes back the same bytes (round trip); every node kind, empty lists, narration; each limit refused in words with what came before kept; a damaged node, option or route left out with its note; duplicate IDs and dangling links kept for the editor to show; an unknown version refused plainly; a fuzz target `conversation_file` (and `speaker_file`) with seeds from the tests.

### Step 2: names for game code (about 30 minutes)

- `project_names`: `state Conversation` and `state Speaker` in `solis/project.lst`, folders as options inside options, the leading `conversations` and `speakers` folders left out.
- A file whose name can't be an option gets a Problem when added, as other names do; two conversations with one name get one.
- **Tests:** names from nested folders; a misspelt `Conversation.Village.Gardnr` is a mistake with its line and "did you mean"; a renamed file makes old uses mistakes (code isn't rewritten, as tables' renames).

### Step 3a: conditions and effects written as Lust (about 45 minutes)

- `script::conversation_code`: writes `solis/conversations/<path>.lst` for each conversation (the class, a method per condition and per effect list, the `uses` as fetches), handed to Lust by `reach.rs` whenever the game has conversations.
- The guard for lines that would escape their method, and effects that don't start with `send`.
- **Tests:** the written file for the example, word for word; hostile lines (`true; } method x() {`, `-- `, a line break, unpaired quotes) refused, with the game still checking; an empty condition writes no method and means always; a conversation with no conditions writes a class with no methods.

### Step 3b: mistakes shown on the conversation (about 30 minutes)

- `code.rs`: a Lust mistake in a written file becomes a `CodeProblem` on the conversation file, naming the node and the slot ("option 2's condition", "effect 1"), from a map `conversation_code` keeps of which line is which.
- A condition that changes something stops at run time with the same naming.
- **Tests:** a misspelt field, a missing fetch, a wrong type (a condition giving a number) each land on the right node and slot.

### Step 4: the runtime, on its own (about 45 minutes)

- `script::dialogue`: the conversation running as plain Rust, with conditions and effects behind a small trait so it's tested without Luau. Start, entering nodes (the 256-step limit), Line, Choice, Branch, End, advance, choose (checking the option's condition again, then its effects, then on), stop, and the events it makes, as a list.
- **Tests, one each for the prototype's "things that are easy to miss":** a node's effects run every time it's entered, loops back to a menu included; an option's effects run before moving on, a node's on entry; an empty next ends like an End; an empty condition is met; and from its runtime section: branches that loop end after 256 steps with "branches loop"; a missing node ends with its ID named; a choice with none available ends with a warning, so the player can't be stuck; hidden options don't count in the numbering, locked ones do; `choose` on a locked option does nothing.

### Step 5a: conversations in the game (about 55 minutes)

- `solis/dialogue.lst` with `startConversation`, `Dialogue:advance`, `choose` and `stop` and the five events. Requests as `Change::Dialogue`; `Game::dialogue` steps as each tick begins, after the timers' events and before the objects, on the first shard, its conditions and effects called in that shard's Luau with a reading host (as screens have) and the world as the tick started; its events and the effects' sends go into the first round.
- The talker gone (`exists()`), or its component removed, ends it.
- **Tests:** the example plays through on 1 shard and on 4 with the same events and the same checksum each tick; two starts in one tick, from entities on different shards, give the same winner every time, the other hearing `ConversationRefused`; `advance` called three times in a tick moves one line; an effect's event is heard in the same tick's first round; `startConversation` every tick while it runs does nothing; the manual's example compiles (`manual_examples_tests`).

### Step 5b: reading the conversation (about 35 minutes)

- `Dialogue:current()` and `DialogueNow` for game code and for screens (`WorldView::dialogue`, the screens' reader).
- **Tests:** what a line, choices (locked and hidden), narration and nothing running read as; a screen showing `Dialogue:current().text` while playing (a screen test).

### Step 6: the words in every language (about 40 minutes)

- Lines, options and speakers' names given to `translations::Texts` the way a table's translated cells are: the main language from the files, others in `locale/<code>/<group>.ftl`, where the group is the conversation's first name part (`Gardener.firstMeeting` in `gardener.ftl`). They're listed in the Translations tab, counted in the missing check and in the spreadsheet.
- A conversation whose first name part is also a text group or a table's name gets a Problem.
- **Tests:** a line in Norsk reads in Norsk, and falls back to the main language when missing; the missing check counts conversation lines; the spreadsheet carries them out and back in; text effects in a translated line still work.

### Step 7: holding the game still (about 30 minutes)

- The hold, on top of the game's speed, and the Game panel's words; `holdsGame = false` for chatter that plays while walking.
- **Tests:** game time and timers stand still while a holding conversation runs and carry on after; a pause menu setting speed 1 doesn't end the hold, and the conversation ending doesn't undo speed 0; a recording made during a conversation plays back the same.

### Step 8: saves, the checksum and changes while playing (about 50 minutes)

- What's saved (above), save games one version up, the checksum once something runs or waits; new code and changed conversations at the next phase; a gone node ends it with a message.
- **Tests:** save on a line, on choices and with a request waiting, load, and play on: the same events and checksum as the game that wasn't saved, on 1 shard and 4; an older save loads with nothing running; a save whose node was deleted ends cleanly; the save fuzz target gains a conversation section; the two-PC check plays a short conversation.

### Step 9: finishing the milestone (about 40 minutes)

- The manual page `docs/manual/conversations.md` (begun in step 5a and grown by each step after it, as each change a user notices updates the manual): what a conversation is, writing one by hand, starting it, the events, conditions and effects, holding the game, saves, tips and common problems; Lust only. Terms in `concepts.md` (conversation, speaker, talker, condition, effect), the page in `SUMMARY.md`, the user guide built again.
- A sample: a small conversation, its speakers and a plain screen showing it, in the test projects, played by `--smoke`.
- The "Can I make this?" page (if it's in `main` by then): dialogue moves to "works with a workaround: write conversations by hand and show them with your own screen, until milestones 3 and 4".
- The tracker, ARCHITECTURE.md, CHANGELOG, the crate READMEs and the CLAUDE.md sentence.

## Not in this milestone

- The dialogue box and the Talker component that turns Interact and MenuChoose into `advance` and `choose` (milestone 3), the editor (milestone 4), starting from the world with ordered entries, timed lines and states that follow a conversation (milestone 5), voice and moments (milestone 6), the overview (milestone 7).
- Values inside a line (`{ $name }`, the player's name): a line holds only its words for now. Worth asking about when milestone 3 is planned.
- Portraits that change with a line's mood: one portrait a speaker for now.
- Lua and Rust: the catch-up phase at the end of 2D.

## The project lead's answers (2026-11-23)

These were asked as questions and answered. **Where an answer differs from the sections above, the answer wins**, and the step that builds that part brings its section into line first.

1. **Conditions and effects:** one line of Lust each, as planned. A condition reads the game as the tick started and gives yes or no; an effect sends an event the game's own code acts on.
2. **Words:** the main language's words in the conversation file, other languages in the translation files, as planned.
3. **Pausing: a setting.** A project setting decides whether a conversation holds the game still. New projects start with **holds the game still**, and each conversation can still turn it off. *Differs from "Holding the game still": step 7 adds the setting.*
4. **How many at once: a setting.** A project setting allows **one at a time** (a dialogue box game) or **many at once** (speech popups over characters, each talker running its own). New projects start with one at a time; with one at a time, asking for a second is refused and the asker hears `ConversationRefused`. *Differs from the plan's one-in-the-game rule: steps 4 and 5a keep one runner per talker, the setting limiting it to one.*
5. **Speakers:** a small file each (`speakers/mira.speaker.lst`), as planned.
6. **Calls: everything on `Dialogue`.** `Dialogue:start(Conversation.Village.Gardener, this)` hands over the talker, which hears the events, beside `Dialogue:advance()`, `Dialogue:choose(2)`, `Dialogue:stop()` and `Dialogue:current()`. With many at once allowed, the calls after `start` name the talker too. *Differs from "Game code": step 2 writes the calls this way.*
7. **The conversation editor:** its own top tab, **Dialogue**, after UI (the mock-up as drawn).
8. **The ready-made box:** written into the project as `ui/dialogue.lst` with its first conversation, then the project's own (the mock-up as drawn).
9. **The graph's keys and mouse:** the animator graph's, plus dragging a wire's end off a node to unjoin it and dragging from a port to empty canvas to add a node already joined.

The mock-ups themselves still wait for the project lead's approval before milestones 3 and 4 are built.
