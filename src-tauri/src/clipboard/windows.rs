#[cfg(target_os = "windows")]
use std::sync::mpsc::Sender;
#[cfg(target_os = "windows")]
use std::ptr::null_mut;

#[cfg(target_os = "windows")]
use windows::core::PCWSTR;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    AddClipboardFormatListener, CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW,
    RegisterClassW, TranslateMessage, CW_USEDEFAULT, HMENU, MSG, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_CLIPBOARDUPDATE, WNDCLASSW,
};

#[cfg(target_os = "windows")]
static mut GLOBAL_SENDER: Option<Sender<()>> = None;

#[cfg(target_os = "windows")]
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CLIPBOARDUPDATE {
        if let Some(sender) = &GLOBAL_SENDER {
            let _ = sender.send(());
        }
        return LRESULT(0);
    }

    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

#[cfg(target_os = "windows")]
pub fn run_clipboard_listener(sender: Sender<()>) -> anyhow::Result<()> {
    unsafe {
        GLOBAL_SENDER = Some(sender);

        let class_name: Vec<u16> = "ClipItHiddenListener\0".encode_utf16().collect();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        let _ = RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(class_name.as_ptr()),
            WINDOW_STYLE(0),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            HWND(null_mut()),
            HMENU(null_mut()),
            None,
            None,
        )
        .map_err(|err| anyhow::anyhow!("CreateWindowExW failed for clipboard listener: {err}"))?;

        if let Err(err) = AddClipboardFormatListener(hwnd) {
            anyhow::bail!("AddClipboardFormatListener failed: {err}");
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(null_mut()), 0, 0).into() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}
