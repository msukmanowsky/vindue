# Brand assets

The shipping mark is **`icon-d-tricolor.svg`** — a 4×4 grid tiled by three
panes in strong primaries (blue = left vertical, red = upper right, yellow =
lower right) on the app's own dark slate: the product — multiple windows,
tiled — in one glance.

| File | Role |
| --- | --- |
| `icon-d-tricolor.{svg,png}` | **Shipping icon.** 1024×1024 master; feeds `npm run tauri icon`, which generates `src-tauri/icons/` |
| `tray-template.svg` | Monochrome menu-bar glyph (silhouette of the same composition, widened gaps for ~18px legibility). Template images render from the alpha channel only — the full-color icon would be a solid blob in the menu bar. Rendered to `src-tauri/assets/tray-template.png` and embedded in the binary |
| `icon-a-panel`, `icon-b-window`, `icon-c-snap`, `icon-e-tricolor-narrow` | The exploration trail: grid-selection, lit window, snap result, narrow-strip variant |

Re-render any PNG at any size:

```sh
rsvg-convert -w 1024 -h 1024 design/<name>.svg -o design/<name>.png
```

Colors come from the app palette (`src/styles.css`): slate `#1c1c22`,
cell `#2b2b35`, selection blue `#3478f6`.
