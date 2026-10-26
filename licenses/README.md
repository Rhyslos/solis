# Licences of material inside the program

These files ship with every build of Solis, next to the program.

| Material | Used for | Licence | File |
| --- | --- | --- | --- |
| Inter (regular and semibold) | Interface text | SIL Open Font License 1.1 | `fonts/Inter-OFL-1.1.txt` |
| JetBrains Mono NL | Monospace text: the console, the frame rate | SIL Open Font License 1.1 | `fonts/JetBrainsMono-OFL-1.1.txt` |
| Hack | Fallback for symbols (built into egui) | MIT and Bitstream Vera | `fonts/Hack-MIT-and-Bitstream-Vera.txt` |
| Noto Emoji | Fallback for symbols (built into egui) | SIL Open Font License 1.1 | `fonts/NotoEmoji-OFL-1.1.txt` |
| Ubuntu Light | Fallback font (built into egui) | Ubuntu Font Licence 1.0 | `fonts/Ubuntu-Font-Licence-1.0.txt` |
| emoji-icon-font | Fallback for symbols (built into egui) | MIT | `fonts/emoji-icon-font-MIT.txt` |

**Code libraries** built into the program are listed, with their copyright notices and licence texts, in `THIRD-PARTY-NOTICES.txt` beside the program. The Windows EXE workflow writes it for every build from the exact libraries in it, with [cargo-about](https://github.com/EmbarkStudios/cargo-about) (settings in `about.toml`, layout in `third-party.hbs`, decision 0044). Their licences are checked by `cargo deny` (see `deny.toml`) on every pull request.

To write it yourself: `cargo install cargo-about --locked --features cli`, then from the repository's top folder `cargo about generate -c about.toml licenses/third-party.hbs -m crates/app/Cargo.toml -o THIRD-PARTY-NOTICES.txt`.
