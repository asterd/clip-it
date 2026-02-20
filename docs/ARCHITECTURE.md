# Architecture

## Runtime Components

- **Tauri App/Windows**
  - `main` popup window for clipboard list
  - `settings` window for app preferences
- **Tray Integration**
  - show popup, open settings, pause capture, quit
- **Global Shortcut**
  - configured from settings and registered at runtime

## Backend Modules

- `clipboard/`
  - OS-specific change detection
  - unified capture pipeline
  - normalization + fingerprint + dedup + self-write guard
- `storage/`
  - schema migration
  - search/filter queries
  - item actions (favorite, pin, delete, clear)
- `commands.rs`
  - Tauri command boundary for UI interaction

## Data Flow

1. Clipboard change detected
2. Capture pipeline resolves payload type
3. Self-write + dedup checks
4. Item persisted in SQLite
5. Event emitted to UI (`clipboard:item_added`)

## Design Goals

- Deterministic capture and dedup
- Fast query/filter response
- UI responsiveness under frequent clipboard updates
- Strictly local data processing
