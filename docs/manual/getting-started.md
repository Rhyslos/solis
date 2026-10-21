# Getting started

> How to get Solis onto your computer and open it.

## When to use it
The first time, and whenever you want the newest version.

## How to use it

**Download it (Windows):**
1. Open the Solis repository on GitHub and go to the **Actions** tab.
2. Click the newest run of **Windows EXE**.
3. At the bottom of the page, download **solis-windows**.
4. Unzip it anywhere and double-click `solis.exe`.

A new `solis.exe` is built every time code is merged into the main version. To build one straight away, open **Windows EXE** in the Actions tab and press **Run workflow**. Each download stays available for 30 days.

**Build it yourself (Windows, macOS or Linux):**
1. Install Rust from [rustup.rs](https://rustup.rs).
2. In the `solis` folder, run `cargo run --release -p app`. The first build takes a few minutes; later ones are quicker.

## Settings
Solis remembers your panel layout, accent colour, camera and viewport controls in your user folder, so each start looks like the last.

## Tips and common problems
- **Problem:** Windows warns that the program is from an unknown publisher. **Why:** the EXE isn't signed yet (that comes with installers). **Fix:** choose **More info**, then **Run anyway**.
- **Problem:** the viewport says "No 3D view" and a reason. **Why:** Solis couldn't use the graphics card. **Fix:** update your graphics drivers. The rest of the editor still works.

## Related
- [Interface tour](interface-tour.md)
