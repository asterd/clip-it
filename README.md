<p align="center">
  <img src="src-tauri/icons/icon.svg" alt="Clip It logo" width="140" height="140" />
</p>

<h1 align="center">Clip It</h1>

<p align="center">
  Cross-platform clipboard manager built with Tauri 2, Rust, and React/TypeScript.
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-0b1220.svg" alt="MIT License" /></a>
  <img src="https://img.shields.io/badge/Tauri-2-24C8DB.svg" alt="Tauri 2" />
  <img src="https://img.shields.io/badge/Rust-stable-DEA584.svg" alt="Rust stable" />
  <img src="https://img.shields.io/badge/React-18-61DAFB.svg" alt="React 18" />
</p>

## License

This project is licensed under the MIT License. See `LICENSE`.

## Features

- Global hotkey popup (`Cmd/Ctrl+Shift+P` by default)
- Clipboard history with full-text search (SQLite + FTS5)
- Item types: text, image, file/folder path
- Favorite and pinned items
- Filters: `All`, `Favorites`, `Pinned`
- Preview modal per item type
- Copy-to-clipboard with self-write guard (2s)
- Local-only storage (no cloud services)

## Tech Stack

- **Desktop shell:** Tauri 2
- **Backend:** Rust
- **Frontend:** React + TypeScript + Vite
- **Database:** SQLite (rusqlite, bundled)

## Repository Layout

- `src-tauri/`: Rust backend and Tauri app config
- `src-tauri/src/clipboard/`: OS-specific clipboard detection and capture pipeline
- `src-tauri/src/storage/`: SQLite models, migration, search/filter APIs
- `ui/`: React frontend

## Quick Start

```bash
./run.sh
```

Build release:

```bash
./run.sh --build
```

## Development

### Commands

```bash
# backend check
cd src-tauri && cargo check

# backend tests
cd src-tauri && cargo test

# frontend build
npm --prefix ui run build
```

## CI/CD

GitHub Actions workflows included:

- `CI` (`.github/workflows/ci.yml`)
  - Runs on push and pull request
  - Builds frontend and runs Rust checks/tests on Linux, macOS, Windows
- `Release` (`.github/workflows/release.yml`)
  - Runs on tag push (`v*`)
  - Builds Tauri bundles and publishes installable artifacts to GitHub Releases

Create a version tag to trigger a release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## Architecture Notes

### Clipboard Detection Strategy

- **Windows:** event-driven listener (`AddClipboardFormatListener`, `WM_CLIPBOARDUPDATE`)
- **macOS:** polling using `NSPasteboard.changeCount`
- **Linux:** polling (best effort)

### Type Priority (Capture)

1. File/folder payload
2. Text
3. Image

On macOS, file/folder copy is read from native pasteboard file URLs to avoid false image classification from Finder icon previews.

## Data Model (MVP)

`items` columns:

- `id`, `created_at`, `kind`, `text`, `fingerprint`
- `image_rgba`, `image_width`, `image_height`
- `favorite`, `pinned`, `deleted`

## Open-Source Notes

- Keep cross-platform behavior behind feature flags and `cfg(target_os = ...)` modules.
- Add regression tests for clipboard classification before changing the capture pipeline.
- Preserve backwards-compatible DB migrations (never remove previously shipped columns without migration plan).

## Known Limitations

- Linux clipboard behavior varies by desktop environment/compositor.
- Wayland clipboard access may be limited by session policies.
- File-path detection outside macOS still relies on text payload heuristics.

## Security

If you discover a vulnerability, please open a security advisory or private report before publishing details.
