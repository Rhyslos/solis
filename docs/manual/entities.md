# Entities and components

> Everything in an area is an entity, and what it does comes from its components.

## When to use it
Every area in Solis is built the same way, so once you know this you can understand any of them ([concepts](concepts.md#the-area)):

- An **entity** is one thing: the sun, a lamp, an imported model, the sky. On its own it is only a name.
- **Components** make it something. A **Transform** gives it a place in the world, a **Light** makes it shine, a **Model** draws an imported model, an **Environment** sets the sky and fog, a **Camera** is what the game looks through ([Play mode](play-mode.md)), and a **RigidBody** and a **Collider** make it take part in [physics](physics.md).

A lamp is just an entity with a Transform and a Light. Give a model a Light (**Add component > Light**) and it shines like a lamp too. Your game's own code adds components of its own ([Game code](game-code.md#components)), shown the same way.

## How to use it

**See what something is made of**
1. Click it in the **Area** panel, or in the viewport ([Selecting and moving things](selecting-and-moving.md)).
2. **Properties** shows its **Name** and each of its components in a card of its own, Transform first, with their settings. Point at a setting's name to see what it does.
3. Each card's **?** opens this manual's page about that component. A component from your game code shows the file it's written in instead, with **Open in Ledger ↗** to open that file in the program you write code in. A component written in Lua says **Show in folder ↗** and shows its file in its folder for you to open from your editor. Why? Solis only ever opens pictures, text and Lust in another program: a downloaded project's `.lua` file, opened by a program on your computer that runs Lua, would run outside Solis's sandbox (see [Projects and the Library](projects-and-the-library.md)).

**What's been changed:** a small dot in the accent colour before a setting's name means it isn't the component's standard value (what it starts with). Beside the setting, **↺** and the standard value (`↺ 1000`) put it back, as one step Ctrl + Z undoes. Why? So you can see at a glance what you've tuned, and undo an experiment without remembering the old number. Places, turns and picked files aren't marked: they start empty and are meant to be set. (A component that came set up for a job, such as the Sun's Light, shows dots on what makes it a sun.)

**Shared files:** a file several entities use, such as a model or sprite sheet, says **shared by 3** beside it. Changing the file itself (in Aseprite, say) changes all of them, so this tells you before you do.

**Add an entity**
- **Home > Empty** adds an empty one (only a Transform) at the point the camera looks at. **Light** beside it adds a point light, and its small arrow lists the spotlight ([lights](lights.md)); **Camera**, **Sprite** and **Tile map** are there too; importing a model adds an entity for it.

**Add or remove a component**
- Click **+ Component** at the top right of Properties, type part of a name to narrow the list, and pick one (or press Enter for the first): a **Light** turns any entity into a lamp, a **Model** shows a model, and so on. It starts with its usual settings. The list is in two groups: **Built in** (Solis's own) and **Game code** (the ready-made kit's and your own); point at one to see what it does.
- A setting that points at another entity (a camera's **target**, say) shows that entity's name. Click it for a list of the area's entities, with a box to find one by name, and **None** to point at nothing.
- A card's **⋯** (or a right-click anywhere on the card) has **Remove**, which takes the component off. Game code's own components appear in the same list.

**Select several**
- **Click** one to select just it. **Ctrl + click** adds one to the selection, or takes it out. **Shift + click** selects every row from the one you picked first to this one.
- With one selected, Properties shows it. **With several selected**, Properties shows the components they all have. A setting that differs between them shows **—**; change it and every one of them gets the new value, as one step Ctrl + Z undoes. Change one number of a position or rotation (say Z), and only that number changes on each, so they keep their own X and Y.

**Find, fold, hide and lock** in the Area column
- **Search** (the magnifier in the Area column's header): a field opens under the header; type part of a name to list only the entities called that, with what they belong to. **Esc** empties and closes it.
- **▾ / ▸** before an entity with children folds them away or shows them again. The header's fold button folds everything; its **⋯** has **Unfold everything**, **Fold everything**, **Show everything hidden** and **Unlock everything**. Selecting something in the viewport unfolds the way to it, so you can see it in the list.
- Point at a row to see its two switches on the right: the **eye** hides the entity in the editor (it isn't drawn, and nor is anything under it), and the **padlock** locks it, so clicks in the viewport pass through it to what's behind (handy for a floor you keep clicking by mistake). The switches stay showing while they're on. **View > Show everything hidden** and **Unlock everything** turn them all off.
- Hiding and locking are only how you see the area while you work: they aren't saved in the area, aren't undo steps, and don't change the game.

**Copy settings from one entity to another**
- A card's **⋯** has **Copy values**, **Paste values** and **Reset to defaults**. Copy a Light from one lamp and paste it onto another to make them match; pasting onto an entity without that component adds it.
- The **⋯** beside the entity's name has **Copy components**, which copies all of them at once, and **Paste components**, which gives them to everything selected.

**Rename, move, copy or delete**
- Click its name at the top of Properties and type, press **F2**, or **double-click** it in the Area column (or choose **Rename** in its right-click menu) to rename it (Enter to keep, Esc to cancel).
- Change **Position** under Transform, or choose **Move here** in the **⋯** beside its name to bring it to where the camera looks.
- **Ctrl + D** duplicates the selection, with everything under each one, and selects the copies.
- **Unpack** (in the **⋯** beside a model's name, or beside a placed [assembly](assemblies.md)'s note) splits it into separate entities.
- **Delete** (the key, the **⋯** beside its name in Properties, or **Delete** in a row's right-click menu in the Area column) removes the selection and everything under it.
- **Right-click a row** in the Area column for its menu, close to hand: Rename, Duplicate, Delete; Add entity, point light, spotlight or camera; Create assembly from selection; Select all and Select nothing. The row right-clicked is selected first, so the menu acts on it. These are the same things the Home tab's ribbon and the search do.
- **F** points the camera at the selection.

**Parents and children** ([concepts](concepts.md#the-area)): drag an entity in the Area panel onto another to put it under that one; it then moves, turns and is deleted with its parent. Drag it onto the empty space below the list to make it stand alone again. It stays exactly where it is in the world either way: only its numbers change, because a child's Position is measured from its parent. Lights brought in with a model start under it. Dragging one of several selected entities moves them all.

## Shortcuts
| Action | Shortcut |
|---|---|
| Add to or take from the selection | `Ctrl + click` |
| Select a range | `Shift + click` |
| Rename | `F2`, or double-click (`Enter` keeps the name, `Esc` leaves it) |
| Select everything | `Ctrl + A` |
| Select nothing | `Esc` |
| Duplicate | `Ctrl + D` |
| Delete | `Delete` |
| Point the camera at the selection | `F` |
| Put under another | drag onto it in the Area panel |
| Make it stand alone | drag onto empty space in the Area panel |

## Settings

The **Transform** every placed entity has:

| Setting | What it does |
|---|---|
| Position | Where it is, in metres along X, Y and Z (Z is up). A child's position is measured from its parent. |
| Rotation | How it's turned, in degrees: X tilts it forward and back, Y tilts it sideways, Z turns it round like a weathervane. |
| Scale | How big it is: 1 is its own size, 2 twice as big along that axis. |

## Tips and common problems
- **Problem:** Properties says **Missing component**. **Why:** the area uses a component Solis doesn't know, such as one from game code that isn't there. **Fix:** nothing is lost: its values are kept as they are. Click **Remove** if you don't need it.
- **Problem:** my entities are gone after closing Solis. **Why:** the area wasn't saved. **Fix:** save with **Ctrl + S** ([Saving areas](saving-areas.md)); Solis asks before closing with unsaved changes.
- **Problem:** I dragged an entity onto another and nothing happened. **Why:** an entity can't go under itself or its own children (that would make a loop).
- **Problem:** something is missing from the viewport. **Why:** it (or something it belongs to) is hidden with the eye in the Area panel. **Fix:** View > Show everything hidden.
- **Problem:** clicking something in the viewport selects what's behind it. **Why:** it's locked (the padlock). **Fix:** click its padlock, or View > Unlock everything.
- Every change here can be undone with **Ctrl + Z** ([Undo and redo](undo.md)).

## Related
- [Lights and shadows](lights.md)
- [Sky, fog and exposure](sky-fog-and-exposure.md)
- [Importing models](importing-models.md)
- Concepts: [entity, component, transform, parent](concepts.md#the-area)
