# Entities and components

> Everything in a scene is an entity, and what it does comes from its components.

## When to use it
Every scene in Solis is built the same way, so once you know this you can understand any of them ([concepts](concepts.md#the-scene)):

- An **entity** is one thing: the sun, a lamp, an imported model, the sky. On its own it is only a name.
- **Components** make it something. A **Transform** gives it a place in the world, a **Light** makes it shine, a **Model** draws an imported model, an **Environment** sets the sky and fog.

A lamp is just an entity with a Transform and a Light. A model with a Light added would glow like a lamp too (adding components arrives later in Phase 2). Your game's own code will add components of its own (Phase 4), shown the same way.

## How to use it

**See what something is made of**
1. Click it in the **Scene** panel.
2. **Properties** shows its **Name** and each of its components, Transform first, with their settings. Point at a setting's name to see what it does.

**Add an entity**
- **Entity > Add entity** adds an empty one (only a Transform) at the point the camera looks at. Entity > Add point light and Add spotlight add [lights](lights.md); importing a model adds an entity for it.

**Rename, move or delete**
- Type in **Name** to rename it.
- Change **Position** under Transform, or click **Move here** to bring it to where the camera looks.
- **Delete** removes it and everything under it. In the Scene panel, models also have a × that does the same.

**Parents and children:** lights brought in with a model sit indented under it in the Scene panel. They are its children: they move with it and are deleted with it.

## Settings

The **Transform** every placed entity has:

| Setting | What it does |
|---|---|
| Position | Where it is, in metres along X, Y and Z (Z is up). A child's position is measured from its parent. |
| Rotation | How it's turned, in degrees: X tilts it forward and back, Y tilts it sideways, Z turns it round like a weathervane. |
| Scale | How big it is: 1 is its own size, 2 twice as big along that axis. |

## Tips and common problems
- **Problem:** Properties says **Missing component**. **Why:** the scene uses a component Solis doesn't know, such as one from game code that isn't there. **Fix:** nothing is lost: its values are kept as they are. Click **Remove** if you don't need it.
- **Problem:** my entities are gone after closing Solis. **Why:** scenes can't be saved yet; that arrives later in Phase 2.
- Every change here can be undone with **Ctrl + Z** ([Undo and redo](undo.md)).

## Related
- [Lights and shadows](lights.md)
- [Sky, fog and exposure](sky-fog-and-exposure.md)
- [Importing models](importing-models.md)
- Concepts: [entity, component, transform, parent](concepts.md#the-scene)
