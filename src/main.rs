//! bMail — IMAP email client for Omarchy Linux.
//!
//! An egui-based email client optimized for the Omarchy desktop environment.
//! Features IMAP access, Markdown email rendering, nested folder tree,
//! keyboard-driven commands (/ palette), and live theme sync with Omarchy.

mod app;
mod commands;
mod config;
mod db;
mod html2md;
mod imap;
mod notify;
mod smtp;
mod theme;
mod ui;

use app::BmailApp;

fn main() -> Result<(), eframe::Error> {
    // Initialize logging
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    log::info!("bMail starting...");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("bMail")
            .with_icon(egui::IconData::default()), // We'd load an actual icon here
        ..Default::default()
    };

    eframe::run_native(
        "bMail",
        options,
        Box::new(|cc| {
            // Apply the Omarchy theme on startup
            let theme_name = theme::detect_current_theme()
                .unwrap_or_else(|| "catppuccin".to_string());
            theme::apply_theme(&cc.egui_ctx, &theme_name);

            Ok(Box::new(BmailApp::new()))
        }),
    )
}
