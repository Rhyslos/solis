# The logo

`logo.svg` is Solis's logo, given by the project lead. The rest is made from it:

| File | Used for |
|---|---|
| `logo.svg` | The source; the README shows it. |
| `logo-256.png` | The window's corner and the taskbar (`branding::LOGO_PNG` in `crates/app`). |
| `logo.ico` | The program file's icon (built in by `crates/app/build.rs` on Windows) and the setup program's (`installer/setup.iss`). Holds 16, 24, 32, 48, 64, 128 and 256 pixels. |

**After changing the logo**, make the other two again:

1. Draw the SVG at 1024 pixels with a see-through background, for example in a browser. Solis's own was drawn with Chromium through Playwright.
2. Make the PNG and ICO from that picture, for example with Python's Pillow:

   ```python
   from PIL import Image
   im = Image.open("logo-1024.png").convert("RGBA")
   im.resize((256, 256), Image.LANCZOS).save("logo-256.png", optimize=True)
   im.save("logo.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
   ```
