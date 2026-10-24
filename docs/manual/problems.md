# Problems

> A list of the mistakes found in the open area, each one naming the entity it's on.

## When to use it
When something in the area doesn't look right, or the **Problems** tab shows a number beside its name. Problems are things Solis can't fix by itself, usually because files changed outside it: a model deleted in Explorer, an area merged by Git, a component from a newer version of your game's code. The area still opens and saves: nothing is lost, but something needs your attention.

## How to use it
1. Click the **Problems** tab (beside the Console). The number on the tab says how many there are; with none, the tab has no number.
2. Each line names the entity and what's wrong with it. Point at a line to read it all.
3. Click a line to select its entity. The Area panel unfolds to show it, and Properties shows its settings, ready to fix.
4. Fix it, and the line goes away by itself: Solis looks again whenever the area or the project changes.

## What it finds

| Problem | Why it happens | What to do |
|---|---|---|
| *X's model isn't in the project* | The model's file was deleted, or never committed to Git (with its `.solmeta` file). | Put the file back (or commit it), or choose another model in Properties, or delete the entity. |
| *X's Model has no model chosen* | A Model component was added but never given a model. | Choose one in Properties, or remove the component. |
| *X has a … component that isn't known* | The area was saved by a newer Solis, or names a component from game code that isn't built yet. The component is kept exactly as it was, so saving loses nothing. | Usually nothing: it comes back to life when the code that defines it is there. Remove it if it's left over. |
| *X is a copy of an assembly that isn't in the project* | The [assembly](assemblies.md)'s file was deleted or not committed. | Put the file back, or [unpack](concepts.md#the-area) the copy to keep it as ordinary entities. |
| *X's parent isn't in the area* | Usually a Git merge that kept a child but not its parent. | Drag it onto another entity in the Area panel, or onto empty space to make it stand alone. |
| *X is inside itself* | Usually a Git merge that combined two moves: one put A under B while the other put B under A, so neither is anywhere in the tree. | Drag it onto another entity in the Area panel, or onto empty space to make it stand alone. |

## Tips and common problems
- **Problem:** models show as missing for a moment after opening a project. **Why:** Solis is still listing the project's files. **Fix:** nothing; they disappear as soon as the Library is ready.
- Problems in game code will be listed here too when code arrives (Phase 4).

## Related
- [The Console](console.md)
- [Projects and the Library](projects-and-the-library.md)
- [Saving areas](saving-areas.md)
- Concepts: [entity, component, assembly](concepts.md#the-area)
