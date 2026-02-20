use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

use crate::clipboard::{normalize_text, set_clipboard_image, set_clipboard_text, sha256_hex};
use crate::events::ClipboardPausedChangedEvent;
use crate::settings::{PauseState, Settings};
use crate::storage::{ItemPreview, SearchResponse};
use crate::SharedState;

#[tauri::command]
pub fn get_settings(state: State<'_, std::sync::Arc<SharedState>>) -> Result<Settings, String> {
    Ok(state.settings.read().map_err(err_to_string)?.clone())
}

#[tauri::command]
pub fn set_setting(
    app: AppHandle,
    state: State<'_, std::sync::Arc<SharedState>>,
    key: String,
    value: Value,
) -> Result<(), String> {
    {
        let mut settings = state.settings.write().map_err(err_to_string)?;
        crate::storage::apply_setting_value(&mut settings, &key, value.clone());
    }

    {
        let storage = state.storage.lock().map_err(err_to_string)?;
        storage.upsert_setting(&key, &value).map_err(err_to_string)?;
        if key == "max_items" {
            let settings = state.settings.read().map_err(err_to_string)?;
            storage
                .enforce_max_items(settings.max_items)
                .map_err(err_to_string)?;
        }
    }

    if key == "hotkey" {
        let settings = state.settings.read().map_err(err_to_string)?.clone();
        crate::register_global_shortcut(&app, &settings.hotkey).map_err(err_to_string)?;
    }

    Ok(())
}

#[tauri::command]
pub fn search_items(
    state: State<'_, std::sync::Arc<SharedState>>,
    query: String,
    limit: u32,
    offset: u32,
    filter: Option<String>,
) -> Result<SearchResponse, String> {
    let storage = state.storage.lock().map_err(err_to_string)?;
    storage
        .search_items(&query, limit, offset, filter.as_deref().unwrap_or("all"))
        .map_err(err_to_string)
}

#[tauri::command]
pub fn get_item_preview(
    state: State<'_, std::sync::Arc<SharedState>>,
    item_id: i64,
) -> Result<ItemPreview, String> {
    let storage = state.storage.lock().map_err(err_to_string)?;
    storage
        .get_item_preview(item_id)
        .map_err(err_to_string)?
        .ok_or_else(|| "item not found".to_string())
}

#[tauri::command]
pub fn open_item_path(
    state: State<'_, std::sync::Arc<SharedState>>,
    item_id: i64,
) -> Result<(), String> {
    let payload = {
        let storage = state.storage.lock().map_err(err_to_string)?;
        storage
            .get_item_clipboard_payload(item_id)
            .map_err(err_to_string)?
    }
    .ok_or_else(|| "item not found".to_string())?;

    if payload.kind != "file" {
        return Err("item is not a file/folder path".to_string());
    }

    let raw = payload.text.unwrap_or_default();
    let first = raw
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| "empty file path".to_string())?;
    let path = normalize_path(first);

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(err_to_string)?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(err_to_string)?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(err_to_string)?;
    }

    Ok(())
}

#[tauri::command]
pub fn set_clipboard_item(
    state: State<'_, std::sync::Arc<SharedState>>,
    item_id: i64,
) -> Result<(), String> {
    let payload = {
        let storage = state.storage.lock().map_err(err_to_string)?;
        storage
            .get_item_clipboard_payload(item_id)
            .map_err(err_to_string)?
    }
    .ok_or_else(|| "item not found".to_string())?;

    let fingerprint = match payload.kind.as_str() {
        "image" => {
            let rgba = payload
                .image_rgba
                .ok_or_else(|| "image payload missing".to_string())?;
            let width = payload
                .image_width
                .ok_or_else(|| "image width missing".to_string())? as usize;
            let height = payload
                .image_height
                .ok_or_else(|| "image height missing".to_string())? as usize;
            set_clipboard_image(rgba.clone(), width, height).map_err(err_to_string)?;

            let mut hasher = sha2::Sha256::new();
            use sha2::Digest;
            hasher.update(b"image:");
            hasher.update((width as u64).to_le_bytes());
            hasher.update((height as u64).to_le_bytes());
            hasher.update(&rgba);
            format!("{:x}", hasher.finalize())
        }
        _ => {
            let text = payload.text.unwrap_or_default();
            let normalized = normalize_text(&text);
            if normalized.is_empty() {
                return Ok(());
            }
            set_clipboard_text(&normalized).map_err(err_to_string)?;
            sha256_hex(&format!("{}:{}", payload.kind, normalized))
        }
    };

    let now = now_ms();
    let mut guard = state.last_written.lock().map_err(err_to_string)?;
    *guard = Some(crate::LastWritten {
        fingerprint,
        written_at_ms: now,
    });

    Ok(())
}

#[tauri::command]
pub fn favorite_item(
    state: State<'_, std::sync::Arc<SharedState>>,
    item_id: i64,
    favorite: bool,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(err_to_string)?;
    storage.set_favorite(item_id, favorite).map_err(err_to_string)
}

#[tauri::command]
pub fn pin_item(
    state: State<'_, std::sync::Arc<SharedState>>,
    item_id: i64,
    pinned: bool,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(err_to_string)?;
    storage.pin_item(item_id, pinned).map_err(err_to_string)
}

#[tauri::command]
pub fn delete_item(
    state: State<'_, std::sync::Arc<SharedState>>,
    item_id: i64,
) -> Result<(), String> {
    let storage = state.storage.lock().map_err(err_to_string)?;
    storage.delete_item(item_id).map_err(err_to_string)
}

#[tauri::command]
pub fn clear_history(state: State<'_, std::sync::Arc<SharedState>>) -> Result<(), String> {
    let storage = state.storage.lock().map_err(err_to_string)?;
    storage.clear_history().map_err(err_to_string)
}

#[tauri::command]
pub fn clear_all_history(state: State<'_, std::sync::Arc<SharedState>>) -> Result<(), String> {
    let storage = state.storage.lock().map_err(err_to_string)?;
    storage.clear_all_history().map_err(err_to_string)
}

#[tauri::command]
pub fn toggle_pause_capture(
    app: AppHandle,
    state: State<'_, std::sync::Arc<SharedState>>,
) -> Result<PauseState, String> {
    let next = !state.paused.load(Ordering::Relaxed);
    state.paused.store(next, Ordering::Relaxed);

    let _ = app.emit(
        "clipboard:paused_changed",
        ClipboardPausedChangedEvent { paused: next },
    );

    Ok(PauseState { paused: next })
}

#[tauri::command]
pub fn open_settings_window(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show().map_err(err_to_string)?;
        window.set_focus().map_err(err_to_string)?;
    }
    Ok(())
}

pub fn parse_shortcut(shortcut: &str) -> Option<Shortcut> {
    let s = shortcut.trim().to_lowercase();
    let mut mods = Modifiers::empty();
    let mut code: Option<Code> = None;

    for part in s.split('+').map(|p| p.trim()) {
        match part {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "shift" => mods |= Modifiers::SHIFT,
            "alt" | "option" => mods |= Modifiers::ALT,
            "cmd" | "command" | "super" => mods |= Modifiers::SUPER,
            key => code = key_code_from_token(key).or(code),
        }
    }

    code.map(|c| Shortcut::new(Some(mods), c))
}

pub fn show_popup_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_always_on_top(true);
        let _ = window.show();
        let popup_size = match window.outer_size() {
            Ok(size) => (size.width as i32, size.height as i32),
            Err(_) => (460_i32, 420_i32),
        };

        let monitor = app
            .cursor_position()
            .ok()
            .and_then(|pos| app.monitor_from_point(pos.x, pos.y).ok().flatten())
            .or_else(|| app.primary_monitor().ok().flatten());

        if let Some(monitor) = monitor {
            let work = monitor.work_area();
            let x = work.position.x + (work.size.width as i32 - popup_size.0) / 2;
            let y = work.position.y + work.size.height as i32 - popup_size.1;
            let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(
                x.max(work.position.x),
                y.max(work.position.y),
            )));
        }

        let _ = window.set_focus();
        let _ = app.emit("popup:opened", serde_json::json!({}));
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn err_to_string<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn key_code_from_token(token: &str) -> Option<Code> {
    match token {
        "a" => Some(Code::KeyA),
        "b" => Some(Code::KeyB),
        "c" => Some(Code::KeyC),
        "d" => Some(Code::KeyD),
        "e" => Some(Code::KeyE),
        "f" => Some(Code::KeyF),
        "g" => Some(Code::KeyG),
        "h" => Some(Code::KeyH),
        "i" => Some(Code::KeyI),
        "j" => Some(Code::KeyJ),
        "k" => Some(Code::KeyK),
        "l" => Some(Code::KeyL),
        "m" => Some(Code::KeyM),
        "n" => Some(Code::KeyN),
        "o" => Some(Code::KeyO),
        "p" => Some(Code::KeyP),
        "q" => Some(Code::KeyQ),
        "r" => Some(Code::KeyR),
        "s" => Some(Code::KeyS),
        "t" => Some(Code::KeyT),
        "u" => Some(Code::KeyU),
        "v" => Some(Code::KeyV),
        "w" => Some(Code::KeyW),
        "x" => Some(Code::KeyX),
        "y" => Some(Code::KeyY),
        "z" => Some(Code::KeyZ),
        _ => None,
    }
}

fn normalize_path(input: &str) -> String {
    if let Some(rest) = input.strip_prefix("file://") {
        return rest.replace("%20", " ");
    }
    input.to_string()
}

#[cfg(test)]
mod tests {
    use super::normalize_path;

    #[test]
    fn normalize_path_decodes_file_url() {
        let out = normalize_path("file:///Users/alice/My%20Folder/test.txt");
        assert_eq!(out, "/Users/alice/My Folder/test.txt");
    }

    #[test]
    fn normalize_path_keeps_plain_paths() {
        let out = normalize_path("/tmp/file.txt");
        assert_eq!(out, "/tmp/file.txt");
    }
}
