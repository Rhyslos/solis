# tools/smoke

The window test: opens the real editor on a virtual screen and checks what a script can, from outside the program.

| File | What it does |
| --- | --- |
| `window-smoke.sh` | Starts the editor with a throwaway home folder, then checks: the window opens, the viewport is drawn in the right colours, a tool key picks a tool, the camera flies (right mouse and W), a panel can be dragged to a new place, File > Quit quits, and the next launch puts the panel and camera back. With `SAMPLE_MODEL` set to a `.glb` file it also checks an imported model is drawn. Screenshots go to the output folder. |

It runs on Linux only (it needs Xvfb, xdotool, ImageMagick and a GPU driver such as Mesa's software one). The program's name is passed in as the window title, so the script never names the program:

```
cargo build -p app
cargo run -p assets --example sample -- smoke-output/sample.glb
SAMPLE_MODEL=smoke-output/sample.glb xvfb-run -s "-screen 0 1440x900x24" tools/smoke/window-smoke.sh target/debug/<program> "<program name>" smoke-output
```

The binary's name and the window title come from `APP_ID` and `APP_NAME` in `crates/app/src/branding.rs`; CONTRIBUTING.md, "Setting up", and `.github/workflows/ci.yml` have the command filled in. Screen positions in the script are for the Default layout on a 1440x900 screen, so a layout change may need them updated.
