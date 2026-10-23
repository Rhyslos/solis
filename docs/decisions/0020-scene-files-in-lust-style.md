# 0020. Scene and prefab files: Lust-style data, made for Git

- **Status:** Accepted
- **Date:** 2026-10-23

## Problem

Scenes (`.solscene`) and prefabs (`.solprefab`) are text that has to merge in Git, field by field, with stable IDs (decisions 0006 and 0015). Unity's YAML and Godot's `.tscn` are known for merge conflicts. ROADMAP Phase 2 says to decide the format first.

## Options

1. **JSON:** every entity ends with the same closing line, which confuses Git's merging, and it's hard to read by hand.
2. **RON:** the same problem.
3. **TOML-style blocks:** one block per entity, one line per field. Valid TOML, so GitHub and VS Code colour it.
4. **Lust-style data blocks:** the same layout, written the way Lust writes `has` and `new` blocks.
5. **Real Lust code:** a scene that runs as a program. Ruled out: scenes have to load in exported games without Lust installed, and opening a file must never run code.

**Tested:** the same 30-entity scene was written in styles 3 and 4 (plus a nested variant of 4), with 200 real `git merge-file` three-way merges for each of six cases (1,200 merges per style).

| Case | Result |
|---|---|
| Different entities next to each other | No conflicts |
| Different components of one entity | No conflicts |
| Reordered children | No conflicts |
| A delete next to an edit | No conflicts |
| Two new entities | About 7% conflicted, when both landed in the same gap |
| Two edits to neighbouring lines of one component | Conflicted |

Styles 3 and 4 merged equally well. The two cases that conflict can be fixed by Solis's own merge driver (decision 0006, Phase 10).

## Decision

Lust-style data blocks (the project lead's choice), read and written only by Solis's own small reader and writer. Lust's syntax can change; this format changes only through its version line.

```
-- Solis scene. Written by Solis; safe to edit by hand.
scene version 1;

entity 2B8D4NE6TV0HC {
    name = "Sun";
    Light.kind = "sun";
    Light.color = [1.0, 0.95, 0.88];
    Light.strength = 3.0;
    Transform.rotation = [0.2588190, 0.0, 0.0, 0.9659258];
}

entity 6HXW1C9PZ3K5M {
    name = "Lamp Post (quay)";
    prefab = "4WQ0B7M2KD9TS";  -- props/lamp_post.solprefab
    Transform.position = [4.0, 2.5, 0.0];
}

entity 6HXW1C9PZ3K5M/5C0RJ7XH3N9PE {
    Light.strength = 1200.0;
}
```

The rules:

- **The first statement** is `scene version N;` or `prefab version N root ID;`. Older versions are migrated one step at a time. A newer version is refused with a message and never half-loaded.
- **IDs** are random 64-bit numbers, written as 13 Crockford base-32 characters.
  - They're random, not time-ordered, so new entities land in different places in the file rather than all at the end.
  - A duplicate ID is an error that names both entities.
- **Entities are sorted by ID,** never by hierarchy or creation. Renaming or reparenting never moves a block. The editor shows the tree.
- **A child names its parent:** `parent = "ID";`. Sibling order is `order = "a0";`, a text rank that a move can always fit between. Parents keep no list of children.
- **Inside a block:**
  - First `name`, `parent`, `order`, `prefab` and `removed`.
  - Then components, sorted by name. Each field is one `Component.field = value;` line, in the component's declared order.
  - Fields the component doesn't know come last, sorted.
  - Plain entities write every field, so a changed default never silently changes a scene. Prefab instances write only their overrides.
- **Values:**
  - `true` and `false`.
  - Whole numbers.
  - Decimals, written as the shortest text that reads back exactly, always with a `.` or `e`. NaN and infinity are refused.
  - Text in `"…"`, with `\\ \" \n \t` escapes.
  - Lists of these in `[…]`.
  - Asset references are written as the ID text, with the asset's path in a `--` comment that's refreshed on save and never read.
- **Comments** start with `--`, as in Lust. Solis rewrites its own comments and doesn't keep others.
- **Files:**
  - LF line endings, a final newline, no BOM written. The reader accepts CRLF and a BOM.
  - The reader checks size before reading (64 MiB per file, 64 KiB per line, 1,000,000 entities, 4,096 fields per entity) and gets a fuzz test.
  - Saves write a temporary file and swap it in.
- **Unknown components and fields** are kept exactly as written and written back in their sorted place. The editor shows "Missing component: Health" with a Remove button. Nothing is dropped silently.
- **Prefab instances** are one entity with `prefab = "ID";` plus overrides, never a copy.
  - Overrides on entities inside the prefab go in blocks named `instance/inner` (`/` sorts first, so they follow the instance).
  - Override keys are stable IDs, never indices or names.
  - Nesting chains them: `instance/nested/inner`.
  - Removing an inner entity is `removed = true;`. Removing a component is `remove = ["Collider"];`.
  - An override whose target is gone is kept and shown as a warning.
- **`.gitattributes`** marks these files `text eol=lf`.

## Consequences

- Files are friendly to read and edit, and read like Lust, so game makers learn one style.
- There's no colouring on GitHub until a Lust highlighter exists.
- Solis owns a small reader and writer (with fuzz tests) instead of using a library.
- The merge driver (Phase 10) handles the two cases plain Git can't.
