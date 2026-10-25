# Entities and components

> Everything in an area is an entity, and what it does comes from its components.

## When to use it
Every area in Solis is built the same way, so once you know this you can understand any of them ([concepts](concepts.md#the-area)):

- An **entity** is one thing: the sun, a lamp, an imported model, the sky. On its own it is only a name.
- **Components** make it something. A **Transform** gives it a place in the world, a **Light** makes it shine, a **Model** draws an imported model, an **Environment** sets the sky and fog, a **Camera** is what the game looks through ([Play mode](play-mode.md)), and a **RigidBody** and a **Collider** make it take part in [physics](physics.md).

A lamp is just an entity with a Transform and a Light. Give a model a Light (**Add component > Light**) and it shines like a lamp too. Your game's own code will add components of its own (Phase 4), shown the same way.

## How to use it

**See what something is made of**
1. Click it in the **Area** panel, or in the viewport ([Selecting and moving things](selecting-and-moving.md)).
2. **Properties** shows its **Name** and each of its components, Transform first, with their settings. Point at a setting's name to see what it does.

**Add an entity**
- **Entity > Add entity** adds an empty one (only a Transform) at the point the camera looks at. Entity > Add point light and Add spotlight add [lights](lights.md); importing a model adds an entity for it.

**Add or remove a component**
- Click **Add component** at the bottom of Properties and pick one: a **Light** turns any entity into a lamp, a **Model** shows a model, and so on. It starts with its usual settings.
- The **×** beside a component's name takes it off. Game code's own components (from Phase 4) appear in the same list.

**Select several**
- **Click** one to select just it. **Ctrl + click** adds one to the selection, or takes it out. **Shift + click** selects every row from the one you picked first to this one.
- With one selected, Properties shows it. **With several selected**, Properties shows the components they all have. A setting that differs between them shows **—**; change it and every one of them gets the new value, as one step Ctrl + Z undoes. Change one number of a position or rotation (say Z), and only that number changes on each, so they keep their own X and Y.

**Find, fold, hide and lock** in the Area panel
- **Search** (top right of the Area panel): type part of a name to list only the entities called that, with what they belong to.
- **▾ / ▸** before an entity with children folds them away or shows them again. **View > Fold everything** and **Unfold everything** do it for all. Selecting something in the viewport unfolds the way to it, so you can see it in the list.
- Point at a row to see its two switches on the right: the **eye** hides the entity in the editor (it isn't drawn, and nor is anything under it), and the **padlock** locks it, so clicks in the viewport pass through it to what's behind (handy for a floor you keep clicking by mistake). The switches stay showing while they're on. **View > Show everything hidden** and **Unlock everything** turn them all off.
- Hiding and locking are only how you see the area while you work: they aren't saved in the area, aren't undo steps, and don't change the game.

**Copy settings from one entity to another**
- The **⋯** beside a component's name has **Copy values**, **Paste values** and **Reset to defaults**. Copy a Light from one lamp and paste it onto another to make them match; pasting onto an entity without that component adds it.
- **Edit > Copy components** at the top of Properties copies all of them at once, and **Paste components** gives them to everything selected.

**Rename, move, copy or delete**
- Type in **Name**, press **F2**, or **double-click** it in the Area panel to rename it (Enter to keep, Esc to cancel).
- Change **Position** under Transform, or click **Move here** to bring it to where the camera looks.
- **Ctrl + D** duplicates the selection, with everything under each one, and selects the copies.
- **Unpack** (in Properties, for a model or a placed [assembly](assemblies.md)) splits it into separate entities.
- **Delete** (the key, the button in Properties, or **Edit > Delete** in the Area panel's own menu) removes the selection and everything under it. In the Area panel, models also have a × that does the same.
- The Area panel's own **Edit** and **Entity** menus are the same as the ones in the top bar, closer to hand.
- **F** points the camera at the selection.

**Parents and children** ([concepts](concepts.md#the-area)): drag an entity in the Area panel onto another to put it under that one; it then moves, turns and is deleted with its parent. Drag it onto the empty space below the list to make it stand alone again. It stays exactly where it is in the world either way: only its numbers change, because a child's Position is measured from its parent. Lights brought in with a model start under it. Dragging one of several selected entities moves them all.

## Shortcuts
| Action | Shortcut |
|---|---|
| Add to or take from the selection | `Ctrl + click` |
| Select a range | `Shift + click` |
| Rename | `F2`, or double-click |
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
