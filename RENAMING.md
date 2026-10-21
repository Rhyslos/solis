# Renaming the program

The engine is currently called **Solis**. The name is kept in as few places as possible, so changing it is a short, mechanical job.

## What does NOT change on a rename

- **Crate and folder names** inside `crates/` are generic (`render`, `editor`, `app`).
- **Settings and saved layouts**, as long as `APP_ID` stays the same (see below).

## Checklist

1. **`crates/app/src/branding.rs`**: update `APP_NAME`. Everything in the editor (window title, About box, `--help`) reads from here.
2. **`APP_ID`** in the same file: leave it alone unless you really want to. It decides where settings are stored, so changing it makes the editor start with fresh settings. If you do change it, add a one-time step that copies the old settings folder.
3. **`crates/app/Cargo.toml`**: the `[[bin]] name` (the program's file name, `solis` or `solis.exe`).
4. **`.github/workflows/ci.yml`**: the program's path and the window title passed to `window-smoke.sh` in the window test job; they must match the file name and `APP_NAME`.
5. **`.github/workflows/windows-exe.yml`**: the file copied into the download (`solis.exe`) and the download's name (`solis-windows`).
6. **`Cargo.toml`** (root): the `repository` URL, if the GitHub repository is renamed.
7. **`LICENSE`**: only if the copyright line should name the program.
8. **Installer and icons** (when they exist).
9. **Docs**: search and replace the name in `README.md`, `DESIGN.md`, `ROADMAP.md`, `CLAUDE.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md` and `docs/manual/`. Add a `CHANGELOG.md` entry about the rename.
10. **GitHub**: rename the repository in its settings. GitHub redirects the old URL automatically.
11. **Decision record**: add one to `docs/decisions/` explaining the new name.

Places get added to this list whenever the name appears somewhere new.
