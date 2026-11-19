# docs/reports/text-layout-study

The program behind [the text layout study](../text-layout-study.md) (stage 1's text layout, rule 22): it lays the same paragraphs out with `parley` and with Solis's own way (Unicode's line breaks with `unicode-linebreak`, shaping with `harfrust`, each shaped once or remembered by its words) in the editor's Inter, and prints the table the study holds.

| File | What it is |
| --- | --- |
| `main.rs` | The three ways, the three sets of paragraphs, and the timing (the best of ten frames each). |
| `Cargo.toml` | Its libraries, each at one exact version, in a workspace of its own: it is never built with Solis, and parley is in no part of Solis. |

Run it from this folder with `cargo run --release` (it needs the network once, to fetch parley).
