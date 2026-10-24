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
   - **File extensions** (`project.solis`, `.solarea`, `.solassembly`, `.solmat`, [decision 0015](docs/decisions/0015-file-extensions.md); `.solmeta` sidecars and `.solbin` cache files, [decision 0021](docs/decisions/0021-asset-database-and-models.md); the cache folder `.solis/`, also named in the `.gitignore` new projects get, `crates/asset_db/src/project.rs`): keep them, since existing projects use them, or read both the old and new ones.
   - **The engine's code folder** (`CODE_FOLDER` in `branding.rs`: `assets/solis/world.lst` and the rest, which game code fetches as `"solis/world"`): keep it, or existing games' code stops finding it. The first line of those files names the program from `APP_NAME`, so it follows by itself.
9. **Docs**: search and replace the name in `README.md`, `DESIGN.md`, `ROADMAP.md`, `CLAUDE.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md` and `docs/manual/`. Add a `CHANGELOG.md` entry about the rename.
10. **GitHub**: rename the repository in its settings. GitHub redirects the old URL automatically.
11. **Decision record**: add one to `docs/decisions/` explaining the new name.

Places get added to this list whenever the name appears somewhere new.
