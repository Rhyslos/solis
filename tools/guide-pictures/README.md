# tools/guide-pictures

The user guide's pictures, taken from the real editor ([decision 0113](../../docs/decisions/0113-pictures-and-three-languages-in-the-guide.md)). A script makes every picture the same way each time, so they're taken again whenever the interface changes and stay accurate.

| File | What it is |
| --- | --- |
| `take-pictures.sh` | Starts the editor on a virtual screen (1440 × 900, Mesa's software driver), in a throwaway home folder, on a new copy of one of File › Learn's samples (`--sample`), drives it with `xdotool` as `pictures.list` says, and saves each picture, cropped and made small, into [`docs/manual/pictures`](../../docs/README.md). Then it writes `interface.txt` and builds the guide again (`cargo run -p user-guide`). The program and its window's title come from `branding.rs`, so it never names the program. |
| `pictures.list` | Every picture, one block each: its file, the manual page that shows it, what it shows, and the steps that make it (keys, clicks, drags, a file written into the project, then `shot`, the part of the screen to keep). A `session` line starts the editor afresh on a sample. Read the top of the file for every step. |
| `interface.txt` | The stamp: the editor's approved picture-test images (`crates/editor/tests/pictures`) and a hash of each, as they were when the pictures were last taken. Written by the script; never by hand. |

```
cargo build -p app
tools/guide-pictures/take-pictures.sh                # every picture, the stamp and the guide
tools/guide-pictures/take-pictures.sh launcher.png   # one picture, to try a change to its steps
```

It needs Xvfb, xdotool and ImageMagick (as the window test does), and runs itself on a virtual screen when there's none. Set `BIN` to use another build of the program.

**When to run it:** whenever the interface changes. A test in [`tools/user-guide`](../user-guide/README.md) fails when the editor's approved picture-test images have changed since the stamp was written, saying to run it; commit the pictures, the stamp and the guide together, and look at the pictures first. A picture whose steps no longer fit the layout (a button moved) shows the wrong thing: change its steps in `pictures.list`.
