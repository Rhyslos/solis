# user-guide/assets

Written into every guide the tool makes (`include_str!` in `src/guide.rs`), so the guide is one file.

| File | What it is |
| --- | --- |
| `page.html` | The page around the chapters: the top bar (logo, name, "User guide", where you are, the search with its keys, the theme switch), the contents down the side, the Top button. `{{name}}`, `{{style}}`, `{{script}}`, `{{contents}}` and `{{chapters}}` are filled in. |
| `guide.css` | The look. Dark is the editor's own greys, accent, link and warning colours (each named after its token in `crates/editor/src/tokens.rs`); light is warm paper with the same blue. A test in `src/lib_tests.rs` holds the dark colours to the tokens. |
| `guide.js` | The behaviour: the search (`/` or `Ctrl + K`), the theme (Auto, Light, Dark, remembered by the browser when it allows), where you are (lit in the contents and named in the top bar, with how much is read), copying a heading's link, the contents behind a button on a narrow window, Top. The page is whole without it. |
