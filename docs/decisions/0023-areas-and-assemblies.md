# 0023. "Area" instead of "scene", "assembly" instead of "prefab"

- **Status:** Accepted
- **Date:** 2026-10-23

## Problem

Solis needs words for one place in a game (what Unity and Godot call a *scene*) and for a reusable thing placed many times (Unity's *prefab*). The project lead does not want "scene" anywhere in Solis, and wants Solis to be a little unique where it can while staying familiar where it must.

## Options

For a scene: Stage (USD), Place (Roblox), Level (Unreal), Map (Source), World, Room (GameMaker), Layout (Construct), Zone, Area, Space, Set, Studio. For a prefab: Template, Assembly (CAD programs), Kit, Blueprint (clashes with Unreal's visual scripting).

## Decision

- **Area** for one place in a game: a level, a menu, a room. Files end in `.solarea`; the panel listing an area's entities is the **Area** panel; the code type is `area::Area`.
- **Assembly** for a reusable, nestable thing with per-copy changes; files end in `.solassembly`; **Unpack** turns a placed one back into plain entities.
- **Library** for the asset browser (as already built).
- Everything else keeps its familiar name: Entity, Component, Properties, Gizmos, Console, Problems, Layers, Play, Undo history.
- Words from other programs (a glTF file's own "scene", "cutscene") are left as they are.

This replaces the extensions `.solscene` and `.solprefab` in [decision 0015](0015-file-extensions.md). No project has used them yet, so nothing needs converting. Earlier decision records keep their wording ("scene document", "prefab") as history; read them as "area" and "assembly".

## Consequences

- Layouts saved before the rename call the panel "Scene"; they still load (the panel reads both names).
- Tutorials from other engines need one translation for newcomers; the manual's Concepts page says "Area (called a scene in some engines)".
