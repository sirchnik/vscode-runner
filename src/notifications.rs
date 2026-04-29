use log::error;
use std::process::Command;

/// Sends a desktop notification with the given title and body.
///
/// Uses `notify-send` as a simple cross-desktop notification mechanism.
pub fn send_notification(title: &str, body: &str) {
    let full_title = format!("VS Code Runner: {title}");

    let result = Command::new("notify-send")
        .arg("--urgency=critical")
        .arg(&full_title)
        .arg(body)
        .spawn();

    match result {
        Ok(_) => {}
        Err(e) => {
            error!("Failed to send notification: {e}");
        }
    }
}
