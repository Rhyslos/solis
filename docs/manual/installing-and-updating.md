# Installing and updating

> Install Solis once, and it keeps itself up to date with every new build on GitHub.

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
1. Each time Solis opens, and every six hours while it's open, it asks GitHub whether there's a newer build.
2. If there is, it downloads it in the background while you work. Nothing changes yet.
3. It checks the download's [SHA-256](concepts.md#getting-solis) against the one GitHub published, and throws the download away if they differ.
4. The Console says the build is ready, and a **Restart to update** button appears at the top right, just left of the Play buttons.
5. Either click **Restart to update** (Solis asks about unsaved work first, closes, installs and opens again), or keep working: the update installs when you close Solis, and the next start is the new build.

**Check now:** Help > **Check for updates**. The line under it says how it went: *Up to date (build 57)*, *Downloading build 58…*, or what went wrong.

**Which build do I have?** Help > **About Solis** shows the version and build number, as does the window's title.

## Settings
| Setting | What it does |
|---|---|
| Help > Update automatically | On (the usual): checks when Solis opens and every six hours, and downloads new builds by itself. Off: checks only when you choose Check for updates. Either way, nothing installs until you close or restart. Remembered between starts. |

## Shortcuts
None: updating has no keys.

## Tips and common problems
- **Problem:** Help says *Built from source: no updates*. **Why:** you built Solis yourself with Cargo, so it has no build number to compare. **Fix:** install it with the setup file to get updates, or update your own copy with `git pull`.
- **Problem:** the Console says a newer build is out, but *this copy wasn't installed with the installer*. **Why:** you're running a downloaded `solis.exe` (from a zip), which doesn't replace itself. **Fix:** install with `solis-setup.exe` once; from then on it updates.
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
