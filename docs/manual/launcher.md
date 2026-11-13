# Projects

> File › Recent projects, where Solis starts: every project you've worked on as a card, the templates to start a new one from, and beside it Learn, Updates and About.

## When to use it

Solis always starts here, in its own full window: the **File** tab's **Recent projects** page. You pick the project to work on, and the editor opens on it. Solis never reopens the last project by itself, so you always choose; the projects you opened most recently come first, so it's one double-click.

Until a project is open, the other tabs along the top are dimmed and the top right says **No project open**. They still work: clicking one, or **Carry on without a project** at the top of the File column (or **Esc**), opens the editor with nothing open, which is handy for looking at a model on its own. The editor opens on the tab you left in front last time.

Closing the editor's window brings you back to this page, with the project still open behind it (**Back to editor** returns to it). Closing the window while a File page is in front quits Solis, asking about unsaved work first. The page is also there at any time from **File › Recent projects**.

Use it to:

- open a project you've worked on before, without hunting for its folder;
- keep the projects you're working on now at the front (pin them);
- see, before opening a project, which build of Solis last opened it: useful after an update, or when a project comes from someone with a newer copy;
- tidy the list: a project you've finished with, or one whose folder has gone, can be removed from it, and one you moved can be found again;
- rename a project, make a copy of it to try something risky, or open its folder in Explorer;
- find projects already on your computer (copied from a USB stick, cloned from Git) without opening each one;
- start something new: an empty project, or one of the four examples to learn from.

## How to use it

1. Each project is a **card**: a large letter where its picture will be (pictures are an [unfinished part](unfinished-parts.md)), its name, its folder, when you last opened it, and the [build](concepts.md#projects) that last opened it. [Pinned](#settings) projects come first.
2. **Double-click** a card to open it, or click it once and press **Enter** (Enter in **Search projects** opens it too, while the search still shows it; with no card picked, Enter there opens nothing). The **arrow keys** walk from card to card: left and right along a row, up and down between rows. With unsaved changes in the open area, Solis asks first.
3. The **pin** at the right of a project's name keeps it at the front; click it again to unpin.
4. **Right-click** a card for its menu:
   - **Open**.
   - **Pin** (or **Unpin**).
   - **Show in Explorer**: opens the project's folder.
   - **Rename…**: changes the project's name (in the title bar and on its card). Its folder keeps its name, so nothing that points at the folder breaks. Type the new name and press **Enter**; **Esc** leaves it as it was.
   - **Make a copy…**: choose an empty folder, and the whole project is copied there, named "… copy". Handy before trying something you may want to undo. The import cache is left out (the copy makes its own when it opens), and so are links to elsewhere on the computer. A big project takes a while: a note under the heading says when it's done, and the copy joins the list.
   - **Remove from list**: only forgets the project here. Its folder and files stay exactly where they are, and **Open…** brings it back.
5. **Search projects** narrows the cards as you type, by name or folder. Every word you type must appear somewhere, in any order. Click it to type: when Solis starts, the keys are the cards' (the arrows walk them, **Esc** carries on into the editor).
6. **Sort by** orders the cards by when each was last opened, or by name. Pinned projects always come first.
7. **New project** makes a folder into a new project. **Open… ▾** has **Open a project folder…** (one that isn't in the list yet) and **Scan a folder for projects…**, which looks through a folder you choose, and the folders inside it (four levels down), and adds every project it finds, without opening them. It doesn't look inside projects or hidden folders.
8. **Start from a template**, under the cards: **Blank project** is the same as New project; **Playground**, **Turret sample**, **UI sample** and **Benchmark** each make one of Solis's [examples](getting-started.md) as a new project, in a folder you choose. The faint tiles after them (First-person, Top-down, Platformer, and **All templates**) are [unfinished parts](unfinished-parts.md): templates to come.

## Learn, Updates and About

The File column has three more pages:

- **Learn**: the four examples again, each with a sentence about it and **Make it in a folder…**; **Your first hour**, five steps for someone new; **Open the manual** (this manual, in your browser); and **Add the sample area**, which adds a small ready-made area to the open project to look around in.
- **Updates**: when a newer build of Solis is out, Updates has a **New** mark in the File column, the status bar says **Build N is out** (click it to come here), and a card across the top of the page says what it is, with **Not now** and **Download** (it downloads while you work), then **Restart to update** (Solis asks about unsaved work, closes, installs it and opens again). In a copy you didn't install with the setup file, the step is **Open the releases page** instead. Under the card: the build you have, the newest build with **Check now**, the **Look for updates when Solis starts** switch, the releases page and the list of every change. Beside them, what the new build brings, from its release on GitHub. See [Installing and updating](installing-and-updating.md).
- **About**: the program's name, version and build, what it is, **Open LICENSE** (its licence), **Open the notices** (the libraries and fonts built into it, each with its licence), and the way to Updates and the manual.

## Settings

| Setting | What it does |
|---|---|
| Pin | Keeps a project at the front, whatever the order. Use it for the projects you're working on now. |
| Sort by | **Last opened** (the most recent first; projects whose time isn't known come last) or **Name** (A to Z). |
| Look for updates when Solis starts (Updates) | Asks GitHub for a newer build at each start, in the background. Off, nothing is asked until you press **Check now**. The same switch is on **Preferences › General**. |

## The build each project was last opened with

Every copy of Solis installed from GitHub has a build number (build 57, then 58, and so on). When a project opens, Solis writes its build number into the project's `project.solis` file, so its card can tell you about it before you open the project:

| Mark | Meaning |
|---|---|
| **build 57 · this one** | Last opened with the build you're using. Nothing to think about. |
| **build 52 · older** (amber) | Last opened with an older build. It opens as usual; any of its files laid out the old way are upgraded when saved, with a backup first ([Backups and upgrades](backups-and-upgrades.md)). |
| **build 60 · newer** (amber) | Last opened with a newer build than yours. Update Solis first ([Installing and updating](installing-and-updating.md)): parts made with the newer build may not open in yours. |
| **—** | Not known: the project hasn't been opened since builds were noted, or only by a copy built from source. |

A copy built from source has no build number of its own: it shows each project's build without comparing.

## Shortcuts

| Action | Shortcut |
|---|---|
| Open the project picked | `Enter`, or double-click |
| Walk the cards | `←` `→` `↑` `↓` |
| A card's menu | Right-click it |
| Back to the editor (or carry on without a project) | `Esc` |
| In **Rename…**: keep the new name, or leave it as it was | `Enter`, `Esc` |

## Tips and common problems

- **Problem:** a card says **Folder not found** and its letter is a **?**. **Why:** the folder was moved, renamed or deleted, or it's on a drive that isn't plugged in. **Fix:** plug the drive in and come back to Recent projects (it looks each time). If you moved the folder, click **Locate…** and choose where it is now: the project keeps its pin. Otherwise click **Remove**.
- **Problem:** a card says **project.solis is gone from its folder**. **Why:** the folder is there, but the file that makes it a project isn't. **Fix:** if the project is in Git, get the file back from there; otherwise **Remove** it.
- **Problem:** `project.solis` shows as changed in Git after opening a project. **Why:** Solis noted its build number in it (`saved_by`). **Fix:** commit it with your other changes: it tells your teammates which build last opened the project.
- **Problem:** Make a copy… says the folder **isn't empty**. **Why:** a copy never goes over other files. **Fix:** make a new, empty folder for it (the folder window has a New folder button).
- **Problem:** Scan a folder for projects… says **it stopped early**. **Why:** it looks through at most 20,000 folders, so choosing a whole drive can't keep it busy for long. **Fix:** choose the folder your projects are in, such as Documents.
- **Problem:** the arrow keys move the search's caret instead of walking the cards. **Why:** the search has the keys while you type in it. **Fix:** click a card first; the arrows walk from there.

## Related

- [Projects and the Library](projects-and-the-library.md): what a project is and what's in it.
- [Settings](settings.md): Preferences and Project settings, the File column's other pages.
- [Installing and updating](installing-and-updating.md): getting a newer build.
- [Backups and upgrades](backups-and-upgrades.md): what happens to a project made with an older build.
- [Concepts](concepts.md): [project](concepts.md#projects), [build](concepts.md#projects), [File page](concepts.md#the-window).
