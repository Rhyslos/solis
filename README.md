# Solis

A free, open-source 3D game engine for stylized games, strategy games, online co-op and PvP, and big open worlds. Lightweight and beginner-friendly. The engine is written in Rust, and games in [Lust](https://github.com/Rhyslos/Lust), Rust or Lua.

**Status:** Phase 1 (seeing things) has begun: the editor opens with its dockable panels, theme and a viewport you can fly around in. Nothing can be made in it yet; models arrive next.

## Running it

**On Windows, without building it yourself:** open the repo's **Actions** tab, click the latest **Windows EXE** run, and download **solis-windows**. Unzip it and run `solis.exe`. A new one is built whenever code is merged; to build one now, choose **Windows EXE** and **Run workflow**.

**From the source code:**

You need [Rust](https://rustup.rs). Then, in this folder:

```
cargo run --release -p app
```

The first build takes a few minutes. `cargo run -p app -- --help` lists the options.

**Moving around the viewport** ([decision 0009](docs/decisions/0009-fly-camera-controls.md)): hold the right mouse button and move the mouse to look around; while it's held, W A S D move, Q and E go down and up, the wheel changes speed, and Shift goes faster. The wheel on its own moves forward and back, and dragging with the middle button slides the view.

- [DESIGN.md](DESIGN.md): what Solis is and how it's planned
- [ROADMAP.md](ROADMAP.md): the phases to Solis 0.1
- [docs/decisions](docs/decisions): decisions and their reasons

MIT licensed: see [LICENSE](LICENSE).
