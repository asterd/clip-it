use std::borrow::Cow;
use std::path::Path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use arboard::{Clipboard, ImageData};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use crate::events::ClipboardItemAddedEvent;
use crate::SharedState;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub fn start_clipboard_pipeline(app: AppHandle, state: Arc<SharedState>) {
    let (tx, rx) = mpsc::channel::<()>();

    #[cfg(target_os = "windows")]
    {
        let tx_clone = tx.clone();
        thread::spawn(move || {
            if let Err(err) = windows::run_clipboard_listener(tx_clone) {
                eprintln!("windows clipboard listener failed: {err}");
            }
        });
    }

    #[cfg(target_os = "macos")]
    {
        let tx_clone = tx.clone();
        let state_clone = state.clone();
        thread::spawn(move || macos::run_polling_loop(tx_clone, state_clone));
    }

    #[cfg(target_os = "linux")]
    {
        let tx_clone = tx.clone();
        let state_clone = state.clone();
        thread::spawn(move || linux::run_polling_loop(tx_clone, state_clone));
    }

    thread::spawn(move || {
        while rx.recv().is_ok() {
            if let Err(err) = capture_once(&app, &state) {
                eprintln!("capture loop error: {err}");
            }
        }
    });
}

pub fn normalize_text(input: &str) -> String {
    let without_null = input.replace('\0', "");
    without_null
        .trim_matches(|c| c == '\n' || c == '\r' || c == '\u{000B}' || c == '\u{000C}')
        .to_string()
}

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn set_clipboard_text(text: &str) -> anyhow::Result<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(text.to_string())?;
    Ok(())
}

pub fn set_clipboard_image(rgba: Vec<u8>, width: usize, height: usize) -> anyhow::Result<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_image(ImageData {
        width,
        height,
        bytes: Cow::Owned(rgba),
    })?;
    Ok(())
}

fn capture_once(app: &AppHandle, state: &Arc<SharedState>) -> anyhow::Result<()> {
    let capture_enabled = {
        let settings = state.settings.read().expect("settings poisoned");
        settings.capture_enabled
    };

    if !capture_enabled || state.paused.load(std::sync::atomic::Ordering::Relaxed) {
        return Ok(());
    }

    let mut clipboard = Clipboard::new()?;
    let now = now_ms();

    #[cfg(target_os = "macos")]
    let file_candidate = macos::read_file_urls_from_pasteboard();
    #[cfg(not(target_os = "macos"))]
    let file_candidate: Option<String> = None;

    let text_candidate = clipboard
        .get_text()
        .ok()
        .map(|raw| normalize_text(&raw))
        .filter(|t| !t.is_empty());

    let (kind, text, image_rgba, image_width, image_height, fingerprint) =
        if let Some(file_payload) = file_candidate
            .or_else(|| text_candidate.clone().filter(|t| looks_like_file_payload(t)))
        {
            let fp = sha256_hex(&format!("file:{}", file_payload));
            ("file".to_string(), Some(file_payload), None, None, None, fp)
        } else if let Some(text_payload) = text_candidate {
            let fp = sha256_hex(&format!("text:{}", text_payload));
            ("text".to_string(), Some(text_payload), None, None, None, fp)
        } else if let Ok(img) = clipboard.get_image() {
            let width = img.width as i64;
            let height = img.height as i64;
            let bytes = img.bytes.into_owned();
            let mut hasher = Sha256::new();
            hasher.update(b"image:");
            hasher.update((width as u64).to_le_bytes());
            hasher.update((height as u64).to_le_bytes());
            hasher.update(&bytes);
            let fp = format!("{:x}", hasher.finalize());
            let label = format!("image://{}x{}", width, height);
            (
                "image".to_string(),
                Some(label),
                Some(bytes),
                Some(width),
                Some(height),
                fp,
            )
        } else {
            return Ok(());
        };

    {
        let guard = state.last_written.lock().expect("last_written poisoned");
        if let Some(last) = &*guard {
            if last.fingerprint == fingerprint && now - last.written_at_ms < 2000 {
                return Ok(());
            }
        }
    }

    let storage = state.storage.lock().expect("storage poisoned");

    if let Some(last_fp) = storage.last_fingerprint()? {
        if last_fp == fingerprint {
            return Ok(());
        }
    }

    let id = storage.insert_item(
        &kind,
        text.as_deref(),
        &fingerprint,
        image_rgba.as_deref(),
        image_width,
        image_height,
    )?;
    let max_items = {
        let s = state.settings.read().expect("settings poisoned");
        s.max_items
    };
    storage.enforce_max_items(max_items)?;

    let preview_text = match kind.as_str() {
        "image" => "Image copied".to_string(),
        "file" => text.clone().unwrap_or_default(),
        _ => text
            .clone()
            .unwrap_or_default()
            .replace('\n', " ")
            .replace('\r', " ")
            .chars()
            .take(140)
            .collect::<String>(),
    };

    let payload = ClipboardItemAddedEvent {
        id,
        preview_text,
        created_at: now,
        pinned: false,
    };
    let _ = app.emit("clipboard:item_added", payload);

    Ok(())
}

fn looks_like_file_payload(text: &str) -> bool {
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return false;
    }

    lines.iter().all(|line| {
        line.starts_with("file://")
            || line.starts_with('/')
            || line.starts_with("~/")
            || (line.len() > 2
                && line.as_bytes()[1] == b':'
                && (line.as_bytes()[2] == b'\\' || line.as_bytes()[2] == b'/'))
            || Path::new(line).exists()
    })
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::{looks_like_file_payload, normalize_text};

    #[test]
    fn normalize_text_removes_null_and_soft_trims() {
        let input = "\nhello\0 world\r\n";
        let out = normalize_text(input);
        assert_eq!(out, "hello world");
    }

    #[test]
    fn detects_unix_path_payload() {
        let payload = "/Users/alice/Documents/project";
        assert!(looks_like_file_payload(payload));
    }

    #[test]
    fn detects_file_url_payload() {
        let payload = "file:///Users/alice/Desktop/Test%20Folder";
        assert!(looks_like_file_payload(payload));
    }

    #[test]
    fn detects_windows_path_payload() {
        let payload = "C:\\Users\\alice\\Desktop\\example.txt";
        assert!(looks_like_file_payload(payload));
    }

    #[test]
    fn does_not_misclassify_plain_text() {
        let payload = "This is a normal sentence.";
        assert!(!looks_like_file_payload(payload));
    }
}
