# The launcher

> The page that lists your projects, where you open, pin and tidy them, and find the examples to start from.

## When to use it

The launcher fills the window when Solis starts without a project, and **File > Projects…** brings it back at any time. Use it to:

- open a project you've worked on before, without hunting for its folder;
- keep the projects you're working on now at the top (pin them);
- see, before opening a project, which build of Solis last opened it: useful after an update, or when a project comes from someone with a newer copy;
- tidy the list: a project you've finished with, or one whose folder has gone, can be removed from it;
- start something new, or make one of the examples to learn from.

Solis also opens the project you had open last when it starts, so if you only ever work on one game you may rarely see the launcher.

## How to use it

1. **Projects** (on the left) lists every project Solis knows, the [pinned](#settings) ones first. Each row shows the project's first letter, its name, its folder, the [build](concepts.md) that last opened it, and when you last opened it.
2. **Double-click** a project to open it, or click it once and press **Enter**. With unsaved changes in the open area, Solis asks first.
3. **☆** pins a project so it stays at the top under **Pinned**; **★** unpins it.
4. **⋯** on a row has **Open** and **Remove from list**. Removing only forgets the project here: its folder and files stay exactly where they are, and **Open project…** brings it back.
5. **Search projects** narrows the list as you type, by name or folder. Every word you type must appear somewhere, in any order.
6. **Sort by** orders the list by when each was last opened, or by name. Pinned projects always come first.
7. **+ New project** makes a folder into a new project; **Open project…** opens a project folder that isn't in the list yet.
8. **Learn** has three examples, each made as a new project in a folder you choose (the Playground, the Turret sample and the Benchmark), the first steps for someone new, and a link to this manual.
9. To go back to the editor without opening anything, press **Esc**, or click **← Back to** your project (or **Carry on without a project**) at the bottom left.

## Settings

| Setting | What it does |
|---|---|
| Pinned (☆ / ★) | Keeps a project at the top of the list, whatever the sort. Use it for the projects you're working on now. |
| Sort by | **Last opened** (the most recent first; projects from before the launcher, whose time isn't known, come last) or **Name** (A to Z). |
| Show this page when Solis starts without a project | Under **Learn**. Turn it off to start in the empty editor instead. |

## The build each project was last opened with

Every copy of Solis installed from GitHub has a build number (build 57, then 58, and so on). When a project opens, Solis writes its build number into the project's `project.solis` file, so the launcher can tell you about it before you open the project:

| Mark | Meaning |
|---|---|
| **build 57 · this one** | Last opened with the build you're using. Nothing to think about. |
| **build 52 · older** | Last opened with an older build. It opens as usual; any of its files laid out the old way are upgraded when saved, with a backup first ([Backups and upgrades](backups-and-upgrades.md)). |
| **build 60 · newer** | Last opened with a newer build than yours. Update Solis first ([Installing and updating](installing-and-updating.md)): parts made with the newer build may not open in yours. |
| **—** | Not known: the project hasn't been opened since builds were noted, or only by a copy built from source. |

A copy built from source has no build number of its own: it shows each project's build without comparing.

## Shortcuts

| Action | Shortcut |
|---|---|
| Open the project picked | `Enter`, or double-click |
| Move along the list | `↑` and `↓` |
| Back to the editor | `Esc` |

## Tips and common problems

- **Problem:** a row says **Folder not found** and its letter is a **?**. **Why:** the folder was moved, renamed or deleted, or it's on a drive that isn't plugged in. **Fix:** plug the drive in and open the launcher again (it looks each time), or click **Remove** and open the project from its new place with **Open project…**.
- **Problem:** a row says **project.solis is gone from its folder**. **Why:** the folder is there, but the file that makes it a project isn't (it was deleted, or the folder was emptied). **Fix:** if the project is in Git, get the file back from there; otherwise **Remove** it.
- **Problem:** `project.solis` shows as changed in Git after opening a project. **Why:** Solis noted its build number in it (`saved_by`). **Fix:** commit it with your other changes: it tells your teammates which build last opened the project.

## Related

- [Projects and the Library](projects-and-the-library.md): what a project is and what's in it.
- [Installing and updating](installing-and-updating.md): getting a newer build.
- [Backups and upgrades](backups-and-upgrades.md): what happens to a project made with an older build.
- [Concepts](concepts.md): [launcher](concepts.md#projects), [project](concepts.md#projects), [build](concepts.md#projects).
