# Testing Guide

## Automated

```bash
cd src-tauri && cargo test
cd src-tauri && cargo check
npm --prefix ui run build
```

## Manual Regression Checklist

1. Open popup via global shortcut.
2. Copy plain text; verify it appears in history.
3. Copy file/folder in Finder/Explorer; verify item kind is file/path.
4. Copy image; verify item kind is image.
5. Select item; verify clipboard is updated and toast appears.
6. Favorite and pin items; verify filters and ordering.
7. Delete item; verify it is removed.
8. Open preview modal for text/file/image and validate content.
9. For file preview, run "Open in Finder/Explorer" and verify target opens.
10. Change hotkey in settings and validate re-registration.
