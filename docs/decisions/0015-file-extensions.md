# 0015. File extensions: .solscene, .solprefab, .solmat and project.solis

- **Status:** Accepted
- **Date:** 2026-10-22

## Problem

Scenes, prefabs and materials become files in Phase 2, and each needs its own extension, so Windows can open them in Solis and people can tell them apart in a folder. The obvious short one, `.sol`, is already taken: it's the extension for Solidity programs (smart contracts on Ethereum), which code editors such as VS Code and GitHub recognise and colour. Windows would show Solis files with the wrong icon and type, and GitHub would count them as Solidity code.

## Options

1. **`.sol` anyway:** short, but clashes as above.
2. **One extension for everything (`.solis`):** no clash, but a folder full of `.solis` files doesn't say which is a scene and which a material.
3. **One extension per kind, starting `sol`** (the project lead's choice): long enough to be unique, and the kind is in the name.

## Decision

Option 3:

| File | Extension | Example |
|---|---|---|
| The project (one per game, at its top folder) | `project.solis` | `my-game/project.solis` |
| A scene | `.solscene` | `levels/harbour.solscene` |
| A prefab | `.solprefab` | `props/crate.solprefab` |
| A material | `.solmat` | `materials/painted-wood.solmat` |

Searches found no common program using these. All are text files, made to merge in Git (decision 0006); their contents are decided with the scene milestone in Phase 2. New kinds of file (animations, sounds settings, UI) follow the same pattern when they arrive.

## Consequences

- The extensions carry the program's name, so they're listed in RENAMING.md. A rename would keep them (changing them would break every existing project), or read both old and new.
- Windows file associations and icons for them come with the installer.
