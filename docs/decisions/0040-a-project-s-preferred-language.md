# 0040. A project's preferred language

- **Status:** Accepted
- **Date:** 2026-10-25

## Problem

Games can be written in Lust, Rust or Lua, all three in one project (decisions 0002 and 0003). When someone makes a new code file in Solis, which language should it be?

## Decision

Chosen by the project lead: **setting up a new project asks for its preferred language**, offered in this order: **Lust, Rust, Lua** (Lust first, as it's the language taught first). It's kept in `project.solis` and can be changed later.

The preference only decides what a new code file starts as: its file ending (`.lst`, `.rs` or `.lua`) and its starting template. It never limits a project: any of the three can be made at any time, and all three can be used together.

## Consequences

- File > New project gets a window of settings before the project is made, starting with the preferred language; the new-project templates (Phase 11: RTS, first person, third person) join that window.
- Making a new code file (in the Library and Code panels) offers the preferred language first, with the other two beside it.
- Game code docs keep showing all three languages; they don't depend on the preference.
- Built when project setup is next worked on: the new-project window and templates (Phase 11), or sooner if the project lead asks.
