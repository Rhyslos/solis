# Interface tour

> What's where in the editor, and how to rearrange it.

## When to use it
When finding your way around, or making the layout suit your screen.

## How to use it

**The top bar**, from left to right:
- **Menus:** File, Edit, View, Entity, Build, Window and Help. Items that don't work yet are greyed out; point at one to see which phase brings it.
- **The scene tab:** the open scene ("Untitled scene"). Opening more scenes, prefabs and code side by side arrives in Phase 2.
- **Play**, **Play with two players** and **Stop**, then **Build** and **Build & Play**, on the right. They tell you in the Console which phase brings them.

**The panels:**

| Panel | Shares a space with | What it's for |
|---|---|---|
| Viewport | Game | The 3D world. See [Moving around the viewport](navigating-the-viewport.md). |
| Game | Viewport | The running game, when you press Play (Phase 5). |
| Scene | Layers | Everything in the scene, as a tree (Phase 2). |
| Layers | Scene | The scene's layers (Phase 2). |
| Properties | | The settings of whatever is selected (Phase 2). |
| Library | Code | Models, textures, sounds and prefabs (Phase 2). |
| Code | Library | Game code in Lust, Rust or Lua (Phase 7). |
| Console | Problems | Messages from the editor. See [The Console](console.md). |
| Problems | Console | Mistakes found in code and scenes. |

**Rearranging panels ([docking](concepts.md#the-window)):**
1. Drag a panel by its tab.
2. As you drag, a highlighted area shows where it will land. Drop it on the edge of another panel to put the two side by side, or in the middle to add it to that panel's tab group.
3. Drag the thin line between two panels to make one bigger.
4. The ▼ at the left of a panel's tab row folds the panel away; click it again to unfold.

Your layout is remembered next time.

## Settings
| Setting | What it does |
|---|---|
| Window > Layouts | **Default:** Library and Console side by side under the viewport. **Wide screen:** Library in a column on the left. **Small screen:** Library, Code, Console and Problems as tabs in one space. **Reset layout** goes back to Default. |
| Window > Accent colour | Petrol blue (the standard), red, green or plum. Remembered next time. |
| View | Switches the viewport's corner controls on and off (also in the viewport's ≡ menu). |

## Tips and common problems
- **Problem:** a panel has disappeared. **Why:** it may be folded away, or behind another tab. **Fix:** look for its tab in each tab group, or use Window > Layouts > Reset layout.

## Related
- [Concepts](concepts.md): panel, tab, docking, layout, accent colour.
