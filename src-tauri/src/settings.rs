use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub hotkey: String,
    pub blur_close: bool,
    pub polling_interval_ms: u64,
    pub capture_enabled: bool,
    pub max_items: i64,
    pub window_opacity: i64,
    pub colored_icons: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let hotkey = if cfg!(target_os = "macos") {
            "Cmd+Shift+P".to_string()
        } else {
            "Ctrl+Shift+P".to_string()
        };

        Self {
            hotkey,
            blur_close: true,
            polling_interval_ms: 400,
            capture_enabled: true,
            max_items: 15,
            window_opacity: 78,
            colored_icons: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PauseState {
    pub paused: bool,
}
