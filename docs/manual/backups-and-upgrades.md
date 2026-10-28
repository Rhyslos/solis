# Backups and upgrades

> Files saved by an older Solis keep opening, and a copy of each is kept before Solis changes it.

## When to use it
You don't need to do anything: it works by itself. This page explains what happens when you open a project after updating Solis, so the messages in the [Console](console.md) make sense.

Each kind of file Solis saves (areas, assemblies, sound events, the Mixer's buses, input actions, recordings, the project file and [sidecars](concepts.md#projects)) has a **[format version](concepts.md#projects)**: a number at its top saying how it's laid out. When an update changes how a kind of file is laid out, its number goes up, and the update brings an **upgrader** that turns the old layout into the new one.

## How it works
1. **Opening an older file:** Solis upgrades it as it reads it, in memory. The file itself isn't changed. For an area, an assembly you open, the input actions and the Mixer, the Console says so: *"input.solinput was saved by an older version (format 1), so it's been upgraded to format 2 as it was read…"*.
2. **Saving it:** before the file is written in the new layout, Solis:
   1. checks that the new file reads back exactly as what's being saved;
   2. copies the old file to the project's `.solis/backups/` folder, into a folder named by the date and time (such as `.solis/backups/2026-10-28 09-15-02/input.solinput`), keeping its place in the project;
   3. writes the new file, and checks it on disk.

   This happens for areas, assemblies, sound events, the input actions and the Mixer, the files you change in the editor. If any step fails, the old file stays exactly as it was and the Console says why. When it works, the Console says where the copy went.
3. **After that** the file is in the new layout, so later saves don't make more copies. Your project's Git history is the undo for ordinary changes.

A file saved by a **newer** Solis than yours isn't opened at all: the Console says to update. Solis never changes a file it doesn't fully understand.

## Settings
There are none. Old backups are tidied away by themselves:

| Rule | What it means |
|---|---|
| Older than 90 days | Removed the next time a backup is made. |
| More than 1 GB in all | The oldest are removed until they fit. |
| The newest | Always kept, however old or big. |

## Tips and common problems
- **Problem:** you want the old file back. **Why:** an upgrade changed something you didn't expect. **Fix:** copy the file from `.solis/backups/<date and time>/` back over the new one, or use Git to go back. To open it in its old layout, use the Solis build that saved it (the last ten are kept on the Releases page; see [Installing and updating](installing-and-updating.md)).
- **Problem:** the backups are gone. **Why:** `.solis/` is also Solis's [cache](concepts.md#projects); deleting the whole folder deletes the backups with it. **Fix:** nothing is lost while the project is in Git: its history has every committed version. Deleting `.solis/cache` alone keeps them.
- **Problem:** the Console says *"this … file was saved by a newer version"*. **Why:** someone saved it with a newer Solis. **Fix:** update Solis (Help > Check for updates).
- The backups folder isn't committed to Git (new projects' `.gitignore` leaves out `.solis/`), so each computer keeps its own.

## Related
- [Saving areas](saving-areas.md): how areas are saved, and why they merge in Git.
- [Installing and updating](installing-and-updating.md): how updates arrive.
- [Concepts](concepts.md): [format version](concepts.md#projects), [cache](concepts.md#projects), [sidecar](concepts.md#projects).
