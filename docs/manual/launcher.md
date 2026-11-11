# The launcher

> The page that lists your projects, where you open, pin and tidy them, and find the examples to start from.

## When to use it

Solis always starts on the launcher, in a small window, and you pick the project to work on there; the window then grows to the editor's size. Closing the editor's window brings you back to the launcher (the project stays open behind it: **← Back to** returns to it), and closing the launcher quits Solis, asking about unsaved work first. **File > Projects…** brings it back at any time too. Use it to:

- open a project you've worked on before, without hunting for its folder;
- keep the projects you're working on now at the top (pin them);
- see, before opening a project, which build of Solis last opened it: useful after an update, or when a project comes from someone with a newer copy;
- tidy the list: a project you've finished with, or one whose folder has gone, can be removed from it, and one you moved can be found again;
- rename a project, make a copy of it to try something risky, or open its folder in Explorer;
- find projects already on your computer (copied from a USB stick, cloned from Git) without opening each one;
- start something new, or make one of the examples to learn from.

Solis doesn't open the last project by itself: the projects you opened most recently are at the top, so it's one double-click.

When a newer build of Solis is out, a blue bar over your projects says so, with **Update now**, **See what's new** (the Updates page) and **Not now** (until Solis starts again). Nothing interrupts you as Solis starts. A copy you didn't install with the setup file has no bar, since it never updates itself: the Updates page says where to get the new build.

## How to use it

1. **Projects** (on the left) lists every project Solis knows, the [pinned](#settings) ones first. Each row shows the project's first letter, its name, its folder, the [build](concepts.md) that last opened it, and when you last opened it.
2. **Double-click** a project to open it, or click it once and press **Enter**. With unsaved changes in the open area, Solis asks first.
3. **☆** pins a project so it stays at the top under **Pinned**; **★** unpins it.
4. **⋯** on a row has:
   - **Open**.
   - **Show in Explorer**: opens the project's folder.
   - **Rename…**: changes the project's name (in the title bar and the list). Its folder keeps its name, so nothing that points at the folder breaks. Type the new name and press **Enter**; **Esc** leaves it as it was.
   - **Make a copy…**: choose an empty folder, and the whole project is copied there, named "… copy". Handy before trying something you may want to undo. The import cache is left out (the copy makes its own when it opens), and so are links to elsewhere on the computer. A big project takes a while: a note at the top says when it's done, and the copy joins the list.
   - **Remove from list**: only forgets the project here. Its folder and files stay exactly where they are, and **Open project…** brings it back.
5. **Search projects** narrows the list as you type, by name or folder. Every word you type must appear somewhere, in any order.
6. **Sort by** orders the list by when each was last opened, or by name. Pinned projects always come first.
7. **+ New project** makes a folder into a new project; **Open project…** opens a project folder that isn't in the list yet. **Scan a folder…** looks through a folder you choose, and the folders inside it (four levels down), and adds every project it finds to the list, without opening them. It doesn't look inside projects or hidden folders.
8. **Updates** shows the build you have and the newest one. When a newer build is out, Updates has a **New** mark and the bottom of the sidebar says so. The page says what the new build brings (from its release on GitHub) and when it was published, and has the next step: **Download** (it downloads while you work), then **Restart to update** (Solis asks about unsaved work, closes, installs it and opens again). In a copy you didn't install with the setup file, the step is **Open the releases page** instead. **Check now** asks GitHub at once; **Look for updates when Solis starts** is the same setting as in the Help menu. See [Installing and updating](installing-and-updating.md).
9. **Learn** has four examples, each made as a new project in a folder you choose (the Playground, the Turret sample, the UI sample and the Benchmark), the first steps for someone new, and a link to this manual.
10. To go back to the editor without opening anything, press **Esc**, or click **← Back to** your project (or **Carry on without a project**) at the bottom left.

## Settings

| Setting | What it does |
|---|---|
| Pinned (☆ / ★) | Keeps a project at the top of the list, whatever the sort. Use it for the projects you're working on now. |
| Sort by | **Last opened** (the most recent first; projects from before the launcher, whose time isn't known, come last) or **Name** (A to Z). |

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
| In **Rename…**: keep the new name, or leave it as it was | `Enter`, `Esc` |

## Tips and common problems

- **Problem:** a row says **Folder not found** and its letter is a **?**. **Why:** the folder was moved, renamed or deleted, or it's on a drive that isn't plugged in. **Fix:** plug the drive in and open the launcher again (it looks each time). If you moved the folder, click **Locate…** and choose where it is now: the project keeps its pin. Otherwise click **Remove**.
- **Problem:** a row says **project.solis is gone from its folder**. **Why:** the folder is there, but the file that makes it a project isn't (it was deleted, or the folder was emptied). **Fix:** if the project is in Git, get the file back from there; otherwise **Remove** it.
- **Problem:** `project.solis` shows as changed in Git after opening a project. **Why:** Solis noted its build number in it (`saved_by`). **Fix:** commit it with your other changes: it tells your teammates which build last opened the project.

- **Problem:** Make a copy… says the folder **isn't empty**. **Why:** a copy never goes over other files. **Fix:** make a new, empty folder for it (the folder window has a New folder button).
- **Problem:** Scan a folder… says **it stopped early**. **Why:** it looks through at most 20,000 folders, so choosing a whole drive can't keep it busy for long. **Fix:** choose the folder your projects are in, such as Documents.

## Related

- [Projects and the Library](projects-and-the-library.md): what a project is and what's in it.
- [Installing and updating](installing-and-updating.md): getting a newer build.
- [Backups and upgrades](backups-and-upgrades.md): what happens to a project made with an older build.
- [Concepts](concepts.md): [launcher](concepts.md#projects), [project](concepts.md#projects), [build](concepts.md#projects).
