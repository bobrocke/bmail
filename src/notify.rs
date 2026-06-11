//! Desktop notifications via Mako (`notify-send`).
//!
//! Sends new-email alerts through the system notification daemon (Mako on
//! Omarchy) using the `notify-rust` crate, which speaks the D-Bus
//! notification spec that Mako implements.

use notify_rust::Notification;

/// Show a new-email notification.
pub fn new_email_notification(sender: &str, subject: &str, folder: &str) {
    let summary = if sender.is_empty() {
        "New email"
    } else {
        sender
    };
    let body = if subject.is_empty() {
        format!("New message in {folder}")
    } else {
        format!("{subject}\nIn {folder}")
    };

    let result = Notification::new()
        .summary(summary)
        .body(&body)
        .appname("bMail")
        .icon("mail-unread")
        .timeout(5000) // 5 seconds
        .show();

    match result {
        Ok(_) => log::debug!("Notification sent: {} - {}", summary, body),
        Err(e) => log::warn!("Failed to send notification: {e}"),
    }
}

/// Show a notification for an error (e.g., connection failure).
pub fn error_notification(message: &str) {
    let result = Notification::new()
        .summary("bMail Error")
        .body(message)
        .appname("bMail")
        .icon("dialog-error")
        .timeout(8000)
        .show();

    match result {
        Ok(_) => log::debug!("Error notification sent: {message}"),
        Err(e) => log::warn!("Failed to send error notification: {e}"),
    }
}

/// Check if notifications are available on this system.
pub fn notifications_available() -> bool {
    // Try to create a notification — if it works, Mako is running.
    Notification::new()
        .summary("bMail")
        .body("test")
        .appname("bMail")
        .timeout(1)
        .show()
        .is_ok()
}
