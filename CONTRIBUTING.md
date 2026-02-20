# Contributing

## Prerequisites

- Rust stable toolchain
- Node.js 18+
- Platform dependencies required by Tauri 2

## Local Workflow

1. Run `./run.sh` for dev mode.
2. Keep changes scoped by module (`clipboard`, `storage`, `ui`).
3. Add/adjust tests for behavior changes.
4. Run checks before opening PR:
   - `cd src-tauri && cargo test`
   - `cd src-tauri && cargo check`
   - `npm --prefix ui run build`

## PR Guidelines

- Include problem statement and why the approach was chosen.
- Document OS-specific tradeoffs for clipboard behavior.
- Add migration notes if DB schema changes.

## Release Process

- Version releases are created from Git tags matching `v*` (for example `v0.1.0`).
- Pushing a matching tag triggers the GitHub Actions release workflow.
- The workflow builds platform bundles and attaches installable artifacts to a GitHub Release.
