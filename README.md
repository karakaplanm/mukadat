# r3data

A desktop tool for digitizing data points from plot images — load a chart image, calibrate its axes, and extract the underlying (x, y) values by clicking points or auto-tracing a curve.

Built with [Dioxus](https://dioxuslabs.com/) (desktop renderer) and [Kuva](https://github.com/Psy-Fer/kuva) for plot rendering/export.

## Features

- Open PNG/JPEG/WebP/BMP plot images and pan/zoom around them
- Calibrate X/Y axes (with optional log scale) by picking two reference points per axis
- Place data points manually, or auto-trace a curve between/after manual guide points
- Live preview plot and publication-quality SVG export via Kuva
- Save/load projects (`.r3data` JSON files) with calibration, points, and view state
- Copy extracted points to the clipboard or export as CSV

## Running

```sh
cargo run
```

Requires a recent stable Rust toolchain (edition 2024).

## Testing

```sh
cargo test
```

## Project layout

- `src/main.rs` — entry point
- `src/app.rs` — the `App` Dioxus component (state, event handlers, UI)
- `src/model.rs` — serializable data types (`DataPoint`, `ProjectData`, enums)
- `src/calibration.rs` — pixel-to-value calibration math
- `src/image_ops.rs` — image loading, demo graph generation, curve auto-tracing, Kuva SVG/plot formatting
- `src/project_io.rs` — shared project load/apply logic and the "last opened project" pointer file
- `src/style.css` — application theme
