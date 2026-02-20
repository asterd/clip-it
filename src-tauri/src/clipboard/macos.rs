#![allow(unexpected_cfgs)]

#[cfg(target_os = "macos")]
use std::ffi::CStr;
#[cfg(target_os = "macos")]
use std::os::raw::c_char;
#[cfg(target_os = "macos")]
use std::sync::mpsc::Sender;
#[cfg(target_os = "macos")]
use std::sync::Arc;
#[cfg(target_os = "macos")]
use std::thread;
#[cfg(target_os = "macos")]
use std::time::Duration;

#[cfg(target_os = "macos")]
use objc::{class, msg_send, sel, sel_impl};

#[cfg(target_os = "macos")]
use crate::SharedState;

#[cfg(target_os = "macos")]
fn change_count() -> i64 {
    unsafe {
        let pb: *mut objc::runtime::Object = msg_send![class!(NSPasteboard), generalPasteboard];
        let count: i64 = msg_send![pb, changeCount];
        count
    }
}

#[cfg(target_os = "macos")]
pub fn run_polling_loop(sender: Sender<()>, state: Arc<SharedState>) {
    let mut last = change_count();

    loop {
        let interval = {
            let s = state.settings.read().expect("settings poisoned");
            s.polling_interval_ms
        };
        thread::sleep(Duration::from_millis(interval));

        let current = change_count();
        if current != last {
            last = current;
            let _ = sender.send(());
        }
    }
}

#[cfg(target_os = "macos")]
pub fn read_file_urls_from_pasteboard() -> Option<String> {
    unsafe {
        let pb: *mut objc::runtime::Object = msg_send![class!(NSPasteboard), generalPasteboard];
        if pb.is_null() {
            return None;
        }

        let classes: *mut objc::runtime::Object = msg_send![class!(NSArray), arrayWithObject: class!(NSURL)];
        let options: *mut objc::runtime::Object = msg_send![class!(NSDictionary), dictionary];
        let urls: *mut objc::runtime::Object = msg_send![pb, readObjectsForClasses: classes options: options];
        if urls.is_null() {
            return None;
        }

        let count: usize = msg_send![urls, count];
        if count == 0 {
            return None;
        }

        let mut out = Vec::new();
        for i in 0..count {
            let url: *mut objc::runtime::Object = msg_send![urls, objectAtIndex: i];
            if url.is_null() {
                continue;
            }

            let path_ns: *mut objc::runtime::Object = msg_send![url, path];
            if path_ns.is_null() {
                continue;
            }

            let c_str_ptr: *const c_char = msg_send![path_ns, UTF8String];
            if c_str_ptr.is_null() {
                continue;
            }

            let path = CStr::from_ptr(c_str_ptr).to_string_lossy().to_string();
            if !path.is_empty() {
                out.push(path);
            }
        }

        if out.is_empty() {
            None
        } else {
            Some(out.join("\n"))
        }
    }
}
