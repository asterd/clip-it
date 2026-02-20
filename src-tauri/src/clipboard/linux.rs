#[cfg(target_os = "linux")]
use std::sync::mpsc::Sender;
#[cfg(target_os = "linux")]
use std::sync::Arc;
#[cfg(target_os = "linux")]
use std::thread;
#[cfg(target_os = "linux")]
use std::time::Duration;

#[cfg(target_os = "linux")]
use crate::SharedState;

#[cfg(target_os = "linux")]
pub fn run_polling_loop(sender: Sender<()>, state: Arc<SharedState>) {
    loop {
        let interval = {
            let s = state.settings.read().expect("settings poisoned");
            s.polling_interval_ms
        };
        thread::sleep(Duration::from_millis(interval));
        let _ = sender.send(());
    }
}
