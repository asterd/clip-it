#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

BUILD_MODE=false

usage() {
  cat <<'USAGE'
Usage: ./run.sh [--build] [--help]

Options:
  --build   Build release executable (tauri build) instead of running dev mode.
  --help    Show this help.
USAGE
}

for arg in "$@"; do
  case "$arg" in
    --build)
      BUILD_MODE=true
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg"
      usage
      exit 1
      ;;
  esac
done

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1"
    exit 1
  fi
}

echo "[clip-it] Checking toolchain..."
require_cmd npm
require_cmd cargo

if [ ! -d "ui" ] || [ ! -f "package.json" ] || [ ! -f "ui/package.json" ]; then
  echo "[clip-it] Project structure not found. Run this script from the repository root."
  exit 1
fi

echo "[clip-it] Installing root dependencies..."
npm install

echo "[clip-it] Installing UI dependencies..."
npm --prefix ui install

if [ "$BUILD_MODE" = true ]; then
  echo "[clip-it] Building release executable..."
  npm run tauri:build
else
  echo "[clip-it] Starting full app in development mode..."
  npm run tauri:dev
fi
