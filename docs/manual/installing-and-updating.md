# Installing and updating

> Install Solis once, and File › Updates tells you about every new build from GitHub.

## When to use it
The first time you get Solis, and any time you want to know which build you have or get the newest one now. After installing, you don't need to do anything: new builds arrive by themselves.

If you'd rather not install anything, you can still download the program on its own (see [Getting started](getting-started.md)). That copy doesn't update itself.

## How to use it

**Install it (once):**
1. Open the Solis repository on GitHub and click **Releases** on the right (or go to `github.com/Rhyslos/solis/releases`).
2. Under the newest release (**Solis build** and a number), download `solis-setup.exe`.
3. Run it. If Windows says it protected your PC, click **More info**, then **Run anyway**. (Solis isn't signed with a paid certificate yet, so Windows doesn't know the publisher.)
4. Accept the licence.
5. Choose where to install it, or keep the suggested folder (`%LOCALAPPDATA%\Programs\Solis`, in your own user folder). Any folder your Windows account can write to works, on any drive. It never asks for an administrator, so only your Windows account gets Solis.
6. Choose whether you want a desktop shortcut, click **Install**, leave **Open Solis** ticked and click **Finish**.

Solis is now in the Start menu (with its logo, a glowing swirl round a dark centre), and in Windows' **Settings > Apps** where you can uninstall it. If you ticked the box, there's a desktop shortcut too.

**How updates work:**
1. When Solis starts, it asks GitHub in the background whether there's a newer build, and opens [Recent projects](launcher.md) straight away: nothing waits for the answer, and offline nothing happens.
2. If there's a newer build, the status bar says so at its right: *Build 58 is out* (click it), and **Updates** in the File column gets a **New** mark. **File › Updates** has a card across its top about it.
3. Choose, on that card:
   - **Download:** the card says *Downloading build 58…* while Solis checks the download's [SHA-256](concepts.md#getting-solis) against the one GitHub published (and throws it away if they differ). Then it says the build is downloaded: **Restart to update** closes Solis (asking about unsaved work first), installs it and opens the new build; or keep working, and it installs when you close Solis.
   - What the build brings is beside it, on the same page.
   - **Not now:** the card goes until Solis starts again.
4. A copy you didn't install with the setup file (a downloaded `solis.exe`, run from its zip) never updates itself: it downloads nothing and installs nothing. Updating it would put a second, installed copy somewhere else on the computer and leave the one you run as old as before. So it only tells you: there's no **Download**; the Console says *Build 58 of Solis is out, but this copy wasn't installed with the installer*, **Updates** in the File column still gets its **New** mark, and the Updates page offers **Open the releases page** in place of Download. Get `solis-setup.exe` there and run it once, or download the newer zip.

**Check now:** **File › Updates › Check now** (or press **Space** and type *Check for updates*). If there's a newer build, the same window asks, even about a build you skipped. **Update now** then downloads it while you keep working, and **Restart to update** appears in the tab bar at the top, just left of the project's name: click it (Solis asks about unsaved work first), or keep working and the update installs when you close Solis. In a copy you didn't install, the window's button is **Open the download page** instead. The line under Check for updates says how it went: *Up to date (build 57)*, *Build 58 is available*, *Downloading build 58…*, *Build 58 is out: install it to update by itself* (a copy you didn't install), or what went wrong.

**The Updates page:** **File › Updates** puts the same steps in one place: the build you have, the newest build and what it brings, **Check now**, **Download** (or **Open the releases page**, in a copy you didn't install) and **Restart to update**. A **New** mark beside Updates says a newer build is out.

**Which build do I have?** **File › About** shows the version and build number, as does the window's title.

## Settings
| Setting | What it does |
|---|---|
| Check for updates when starting (press Space and type it) | On (the usual): Solis asks GitHub for a newer build each time it starts, and File › Updates and the status bar say so when there is one. The same switch is on Preferences › General and on the Updates page. Off: nothing is asked of GitHub unless you choose Check for updates. Remembered between starts. |
| Skip this version (in the update window from File › Updates › Check now) | Stops Solis mentioning that one build. A newer build is shown again; Check for updates always shows what's newest. |

## Shortcuts
None: updating has no keys.

## Tips and common problems
- **Problem:** **File › Project settings** says *Built from source: no updates* beside **Engine version**. **Why:** you built Solis yourself with Cargo, so it has no build number to compare. **Fix:** install it with the setup file to get updates, or update your own copy with `git pull`.
- **Problem:** the Console says a newer build is out, but *this copy wasn't installed with the installer*, and there's no **Update now**. **Why:** you're running a downloaded `solis.exe` (from a zip). Solis only updates a copy its installer put there, so this one is never downloaded for or changed. **Fix:** install with `solis-setup.exe` once (the Updates page's **Open the releases page** goes there); from then on it updates.
- **Problem:** *Couldn't check: couldn't reach GitHub*. **Why:** the computer is offline, or something blocks GitHub. **Fix:** nothing to do; it tries again later. Your work isn't affected.
- **Problem:** *GitHub asked to wait before checking again*. **Why:** GitHub limits how often one computer may ask (60 times an hour). **Fix:** wait an hour.
- **Problem:** *Couldn't check for updates: the download stopped: the response body is larger than request limit*. **Why:** the first builds, published on 25 and 26 October 2026, had a mistake that refused every finished download. **Fix:** download the newest `solis-setup.exe` from the Releases page and run it once; it updates itself from then on.
- **Problem:** a download *didn't match its checksum*. **Why:** it was damaged or cut short on the way. **Fix:** it was thrown away; Solis downloads it again next time.
- **What Solis sends:** only the request itself, naming the program and its version (GitHub asks every program to). Nothing about you or your projects.
- **Problem:** the installer says it *can't be installed in* the folder you chose. **Why:** your account can't write there without an administrator (Program Files, say), and Solis must be able to replace itself when it updates. **Fix:** choose a folder of your own.
- **Moving Solis to another folder:** uninstall it (Settings > Apps), then run the setup file again and choose the new folder. Updates always go to the folder you installed in.
- **What's in the installed folder:** the program, its licence (`LICENSE`, MIT), `THIRD-PARTY-NOTICES.txt` (the open-source libraries built into Solis, with their licences) and the fonts' licences in `licenses`.
- **Your projects and settings** are kept through every update: they live in your project folders and your user folder, not where Solis is installed.

## Related
- [Getting started](getting-started.md)
- [Concepts: builds, releases, installers and SHA-256](concepts.md#getting-solis)
