# installer

The Windows installer (decision 0043), built by the Windows EXE workflow on every merge into main.

| File | What it does |
| --- | --- |
| `setup.iss` | The installer script. It installs for the current user only (no administrator needed), adds a Start menu entry and an uninstall entry in Settings > Apps, and leaves `installed.txt` beside the program so it knows it may update itself. Beside the program it puts its licences and the user guide (`user-guide/index.html`, decision 0112), which the editor's Documentation button opens. It never names the program: the workflow reads the name, ID and repository from `crates/app/src/branding.rs` and passes them in. |

The workflow compiles it with one exact version of Inno Setup (`INNO_SETUP_VERSION` in `windows-exe.yml`), which it checks before using.

The program's updater (`crates/updater`) downloads a newer installer, checks it, and starts it with the arguments in `crates/updater/src/install.rs`.

**Read next:** [`.github/workflows/README.md`](../.github/workflows/README.md) for the workflow that builds it, and [`crates/updater/src`](../crates/updater/src/README.md).
