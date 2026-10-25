# Getting started

> How to get Solis onto your computer and open it.

## When to use it
The first time, and whenever you want the newest version.

## How to use it

**Install it (Windows, recommended):** download `solis-setup.exe` from the newest release on the repository's **Releases** page and run it. Solis then updates itself whenever a new build is merged. See [Installing and updating](installing-and-updating.md).

**Or download the program on its own:**
1. Open the Solis repository on GitHub and go to the **Actions** tab.
2. Click the newest run of **Windows EXE**.
3. At the bottom of the page, download **solis-windows**.
4. Unzip it anywhere and double-click `dist\solis.exe`. (The `setup` folder beside it holds the installer.)

A copy run this way doesn't update itself. Each download stays available for 30 days.

**Build it yourself:**
1. Install Rust from [rustup.rs](https://rustup.rs).
2. In the `solis` folder, run `cargo run --release -p app`. The first build takes a few minutes; later ones are quicker.

**Opening with a model:** `solis --import ship.glb` opens Solis and imports the model straight away (repeat `--import` for more than one).

## Settings
Solis remembers your panel layout, accent colour, camera and viewport controls in your user folder, so each start looks like the last.

## Tips and common problems
- **Problem:** Windows warns that the program is from an unknown publisher. **Why:** the program isn't signed with a paid certificate yet. **Fix:** choose **More info**, then **Run anyway**.
- **Problem:** the viewport says "No 3D view" and a reason. **Why:** Solis couldn't use the graphics card. **Fix:** update your graphics drivers. The rest of the editor still works.

## Related
- [Interface tour](interface-tour.md)
