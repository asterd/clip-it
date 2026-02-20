#![allow(unexpected_cfgs)]

mod clipboard;
mod commands;
mod events;
mod settings;
mod storage;

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, RwLock};

use anyhow::Context;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::menu::MenuEvent;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri_plugin_global_shortcut::ShortcutState;

use crate::events::ClipboardPausedChangedEvent;
use crate::settings::Settings;
use crate::storage::Storage;

pub struct LastWritten {
    pub fingerprint: String,
    pub written_at_ms: i64,
}

pub struct SharedState {
    pub storage: Mutex<Storage>,
    pub settings: RwLock<Settings>,
    pub paused: AtomicBool,
    pub last_written: Mutex<Option<LastWritten>>,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        commands::show_popup_window(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let app_dir = app
                .path()
                .app_data_dir()
                .context("failed to resolve app data dir")?;
            std::fs::create_dir_all(&app_dir)?;

            let db_path = app_dir.join("clipit.db");
            let storage = Storage::open(&db_path)?;
            let settings = storage.load_settings()?;

            let state = Arc::new(SharedState {
                storage: Mutex::new(storage),
                settings: RwLock::new(settings.clone()),
                paused: AtomicBool::new(!settings.capture_enabled),
                last_written: Mutex::new(None),
            });

            app.manage(state.clone());
            setup_tray(app.handle())?;

            register_global_shortcut(app.handle(), &settings.hotkey)?;
            clipboard::start_clipboard_pipeline(app.handle().clone(), state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_setting,
            commands::search_items,
            commands::get_item_preview,
            commands::open_item_path,
            commands::set_clipboard_item,
            commands::favorite_item,
            commands::pin_item,
            commands::delete_item,
            commands::clear_history,
            commands::clear_all_history,
            commands::toggle_pause_capture,
            commands::open_settings_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub fn register_global_shortcut(app: &tauri::AppHandle, shortcut_str: &str) -> anyhow::Result<()> {
    let manager = app.global_shortcut();
    let _ = manager.unregister_all();

    let shortcut = commands::parse_shortcut(shortcut_str)
        .ok_or_else(|| anyhow::anyhow!("invalid shortcut format: {shortcut_str}"))?;

    manager.register(shortcut)?;

    Ok(())
}

fn setup_tray(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let show_item = MenuItem::with_id(app, "show_popup", "Show Clipboard", true, None::<&str>)?;
    let settings_item =
        MenuItem::with_id(app, "open_settings", "Settings", true, None::<&str>)?;
    let pause_item =
        MenuItem::with_id(app, "toggle_pause", "Pause Capture", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &settings_item, &pause_item, &quit_item])?;

    let mut tray_builder = TrayIconBuilder::new()
        .icon(tray_icon_image())
        .menu(&menu)
        .on_menu_event(|app, event: MenuEvent| match event.id().as_ref() {
            "show_popup" => {
                commands::show_popup_window(app);
            }
            "open_settings" => {
                let _ = commands::open_settings_window(app.clone());
            }
            "toggle_pause" => {
                let state = app.state::<Arc<SharedState>>();
                let next = !state.paused.load(Ordering::Relaxed);
                state.paused.store(next, Ordering::Relaxed);
                let _ = app.emit(
                    "clipboard:paused_changed",
                    ClipboardPausedChangedEvent { paused: next },
                );
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray: &tauri::tray::TrayIcon, event: TrayIconEvent| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                commands::show_popup_window(&app);
            }
        });

    #[cfg(target_os = "macos")]
    {
        tray_builder = tray_builder.icon_as_template(true);
    }

    let tray = tray_builder.build(app)?;

    // Keep tray alive for whole process lifetime.
    Box::leak(Box::new(tray));

    Ok(())
}

fn tray_icon_image() -> Image<'static> {
    const W: usize = 18;
    const H: usize = 18;
    let mut rgba = vec![0u8; W * H * 4];

    for y in 0..H {
        for x in 0..W {
            let i = (y * W + x) * 4;
            let border = x == 3 || x == 14 || y == 3 || y == 14;
            let stem = x == 8 || x == 9;
            let bar = y == 8 || y == 9;
            let on = border || stem || bar;
            if on {
                rgba[i] = 0;
                rgba[i + 1] = 0;
                rgba[i + 2] = 0;
                rgba[i + 3] = 255;
            }
        }
    }

    Image::new_owned(rgba, W as u32, H as u32)
}
