# Feature name

> One sentence: what it does.

## When to use it
The situations this is for, in plain language. Mention the alternative if something else is better for some cases.

## How to use it
1. Step one (which panel, what to select).
2. Step two: press `Ctrl + S`.
3. Step three.

> **Good to know:** something worth stopping for, such as a choice that can't be undone or a reason that isn't obvious.

## Settings
| Setting | What it does |
|---|---|
| Example | Explain the effect, not just the name. Give a typical value where it helps. |

## Shortcuts
| Action | Shortcut |
|---|---|
| Example | `Ctrl + S` |

## Tips and common problems
- **Problem:** what you see. **Why:** what causes it. **Fix:** what to do.

## Related
- Links to related pages, and to [Concepts](concepts.md) terms used on this page.

## How the manual becomes the user guide (delete this part)
Every page here is also a chapter of the user guide (`docs/user-guide/index.html`, built by `cargo run -p user-guide`; see [The user guide](user-guide.md)). Write so it reads well in both:

- **Keys:** write a key, or keys pressed together, in backticks with ` + ` between them: `` `Ctrl + K` ``, `` `Esc` ``, `` `F2` ``, `` `Shift + click` ``. The guide draws them as keys. Code in backticks that isn't key names (`` `world.lst` ``) stays code. `<kbd>K</kbd>` works too.
- **Steps:** a numbered list is drawn as numbered steps.
- **Boxes:** a quote starting **Good to know:** (or **Note:**) is a blue box; one starting **Careful:** (or **Warning:**) an amber one. The quote straight under the title is the page's opening sentence.
- **Links:** to another page (`physics.md`), a heading (`physics.md#how-to-use-it`, the heading's words in lower case with hyphens) or a file elsewhere in the repository (`../decisions/0112-the-user-guide-is-the-manual-built.md`, shown on GitHub). A link to nothing fails the checks.
- **A new page** goes in [SUMMARY.md](SUMMARY.md), in its group: that list is the guide's chapters, in order. A page missing from it fails the checks.
- After changing any page, build the guide again (`cargo run -p user-guide`) and commit it with the change.
