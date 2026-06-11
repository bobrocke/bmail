//! First-run setup screen for IMAP/SMTP configuration.
//!
//! Collects IMAP host/port/username/password, suggests SMTP settings,
//! and allows testing the connection before saving.

use crate::config::{Config, Identity, ImapConfig, SmtpConfig};
use egui::{FontId, RichText, Ui};

/// Steps in the setup wizard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupStep {
    Imap,
    Smtp,
    Identity,
    Test,
}

/// State for the setup screen.
#[derive(Debug, Clone)]
pub struct SetupState {
    pub step: SetupStep,
    pub imap_host: String,
    pub imap_port: String,
    pub imap_username: String,
    pub imap_password: String,
    pub smtp_host: String,
    pub smtp_port: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub display_name: String,
    pub email: String,
    pub test_result: Option<Result<(), String>>,
    pub testing: bool,
    /// Track which field is waiting for a clipboard paste response.
    pub pending_paste: Option<PasteField>,
    pub show_imap_password: bool,
    pub show_smtp_password: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteField {
    ImapPassword,
    SmtpPassword,
}

impl Default for SetupState {
    fn default() -> Self {
        Self {
            step: SetupStep::Imap,
            imap_host: String::new(),
            imap_port: "993".to_string(),
            imap_username: String::new(),
            imap_password: String::new(),
            smtp_host: String::new(),
            smtp_port: "587".to_string(),
            smtp_username: String::new(),
            smtp_password: String::new(),
            display_name: String::new(),
            email: String::new(),
            test_result: None,
            testing: false,
            pending_paste: None,
            show_imap_password: false,
            show_smtp_password: false,
        }
    }
}

impl SetupState {
    /// Build a Config from the current setup state.
    pub fn to_config(&self) -> Config {
        Config {
            imap: ImapConfig {
                host: self.imap_host.clone(),
                port: self.imap_port.parse().unwrap_or(993),
                username: self.imap_username.clone(),
                password: self.imap_password.clone(),
            },
            smtp: SmtpConfig {
                host: self.smtp_host.clone(),
                port: self.smtp_port.parse().unwrap_or(587),
                username: self.smtp_username.clone(),
                password: self.smtp_password.clone(),
            },
            identity: Identity {
                display_name: self.display_name.clone(),
                email: self.email.clone(),
            },
            preferences: Default::default(),
        }
    }

    /// Auto-detect SMTP from the IMAP host.
    pub fn auto_detect_smtp(&mut self) {
        if let Some((host, port)) = Config::suggest_smtp(&self.imap_host) {
            self.smtp_host = host;
            self.smtp_port = port.to_string();
        }
    }
}

/// Render the setup wizard. Returns `Some(Config)` when the user completes it.
pub fn render(ctx: &egui::Context, state: &mut SetupState) -> Option<Config> {
    let mut result = None;

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.heading(
                RichText::new("bMail Setup")
                    .strong()
                    .font(FontId::proportional(24.0)),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new("Configure your email account").font(FontId::proportional(14.0)),
            );
            ui.add_space(24.0);
        });

        ui.vertical_centered(|ui| {
            let max_width = 400.0;
            ui.set_max_width(max_width + 80.0);

            // Step indicator
            ui.horizontal(|ui| {
                let steps = [
                    (SetupStep::Imap, "IMAP"),
                    (SetupStep::Smtp, "SMTP"),
                    (SetupStep::Identity, "Identity"),
                    (SetupStep::Test, "Test"),
                ];
                for (step, label) in &steps {
                    let is_current = state.step == *step;
                    let is_done = state.step as usize > *step as usize;
                    let color = if is_current {
                        ui.visuals().hyperlink_color
                    } else if is_done {
                        ui.visuals().weak_text_color()
                    } else {
                        ui.visuals().text_color()
                    };
                    ui.label(RichText::new(*label).color(color));
                    if step != &SetupStep::Test {
                        ui.label(" → ");
                    }
                }
            });
            ui.add_space(16.0);

            match state.step {
                SetupStep::Imap => render_imap_step(ui, state),
                SetupStep::Smtp => render_smtp_step(ui, state),
                SetupStep::Identity => render_identity_step(ui, state),
                SetupStep::Test => {
                    if render_test_step(ui, state) {
                        result = Some(state.to_config());
                    }
                }
            }
        });
    });

    result
}

fn field(
    ui: &mut Ui,
    label: &str,
    value: &mut String,
    hint: &str,
    default: &str,
    password: bool,
    show_password: Option<&mut bool>,
) -> bool {
    let mut paste_clicked = false;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label)
                .strong()
                .font(FontId::proportional(13.0)),
        );
        let dim = ui.visuals().weak_text_color();
        let bright = ui.visuals().text_color();
        let text_color = if value == default { dim } else { bright };
        let response = ui.add_sized(
            [200.0, 20.0],
            if password {
                let hide = show_password.as_ref().is_none_or(|s| !**s);
                egui::TextEdit::singleline(value)
                    .password(hide)
                    .font(FontId::proportional(13.0))
                    .hint_text(hint)
                    .text_color(text_color)
            } else {
                egui::TextEdit::singleline(value)
                    .font(FontId::proportional(13.0))
                    .hint_text(hint)
                    .text_color(text_color)
            },
        );

        // Password visibility toggle
        if let Some(show) = show_password {
            let icon = if *show { "👁" } else { "👁‍🗨" };
            if ui
                .button(icon)
                .on_hover_text("Show/hide password")
                .clicked()
            {
                *show = !*show;
            }
        }

        // Paste button
        if ui
            .button("📋")
            .on_hover_text("Paste from clipboard")
            .clicked()
        {
            response.request_focus();
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            paste_clicked = true;
        }
    });
    ui.add_space(4.0);
    paste_clicked
}

/// Check if a paste event arrived and apply it to the target field.
fn apply_paste(
    ui: &mut Ui,
    target: &mut String,
    pending: &mut Option<PasteField>,
    which: PasteField,
) {
    if *pending == Some(which) {
        ui.input(|i| {
            for event in &i.events {
                if let egui::Event::Paste(text) = event {
                    target.push_str(text);
                    *pending = None;
                }
            }
        });
    }
}

fn render_imap_step(ui: &mut Ui, state: &mut SetupState) {
    ui.heading("IMAP Server");

    if field(
        ui,
        "Host:",
        &mut state.imap_host,
        "imap.gmail.com",
        "",
        false,
        None,
    ) {
        state.pending_paste = Some(PasteField::ImapPassword);
    }
    if field(ui, "Port:", &mut state.imap_port, "993", "993", false, None) {
        state.pending_paste = Some(PasteField::ImapPassword);
    }
    if field(
        ui,
        "Username:",
        &mut state.imap_username,
        "you@gmail.com",
        "",
        false,
        None,
    ) {
        state.pending_paste = Some(PasteField::ImapPassword);
    }
    if field(
        ui,
        "Password:",
        &mut state.imap_password,
        "app-specific password",
        "",
        true,
        Some(&mut state.show_imap_password),
    ) {
        state.pending_paste = Some(PasteField::ImapPassword);
    }

    apply_paste(
        ui,
        &mut state.imap_password,
        &mut state.pending_paste,
        PasteField::ImapPassword,
    );

    ui.add_space(12.0);

    ui.horizontal(|ui| {
        if ui.button("← Quit").clicked() {
            std::process::exit(0);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let can_continue = !state.imap_host.is_empty() && !state.imap_username.is_empty();
            if ui
                .add_enabled(can_continue, egui::Button::new("Continue →"))
                .clicked()
            {
                // Auto-detect SMTP
                state.auto_detect_smtp();
                // Pre-fill SMTP credentials from IMAP
                if state.smtp_username.is_empty() {
                    state.smtp_username = state.imap_username.clone();
                }
                if state.smtp_password.is_empty() {
                    state.smtp_password = state.imap_password.clone();
                }
                // Pre-fill identity email
                if state.email.is_empty() && state.imap_username.contains('@') {
                    state.email = state.imap_username.clone();
                }
                state.step = SetupStep::Smtp;
            }
        });
    });
}

fn render_smtp_step(ui: &mut Ui, state: &mut SetupState) {
    ui.heading("SMTP Server (sending)");

    if field(
        ui,
        "Host:",
        &mut state.smtp_host,
        "smtp.gmail.com",
        "",
        false,
        None,
    ) {
        state.pending_paste = Some(PasteField::SmtpPassword);
    }
    if field(ui, "Port:", &mut state.smtp_port, "587", "587", false, None) {
        state.pending_paste = Some(PasteField::SmtpPassword);
    }
    if field(
        ui,
        "Username:",
        &mut state.smtp_username,
        "you@gmail.com",
        "",
        false,
        None,
    ) {
        state.pending_paste = Some(PasteField::SmtpPassword);
    }
    if field(
        ui,
        "Password:",
        &mut state.smtp_password,
        "app-specific password",
        "",
        true,
        Some(&mut state.show_smtp_password),
    ) {
        state.pending_paste = Some(PasteField::SmtpPassword);
    }

    apply_paste(
        ui,
        &mut state.smtp_password,
        &mut state.pending_paste,
        PasteField::SmtpPassword,
    );

    ui.add_space(12.0);

    ui.horizontal(|ui| {
        if ui.button("← Back").clicked() {
            state.step = SetupStep::Imap;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let can_continue = !state.smtp_host.is_empty();
            if ui
                .add_enabled(can_continue, egui::Button::new("Continue →"))
                .clicked()
            {
                state.step = SetupStep::Identity;
            }
        });
    });
}

fn render_identity_step(ui: &mut Ui, state: &mut SetupState) {
    ui.heading("Your Identity");

    ui.label("This is what recipients will see when you send email.");
    ui.add_space(8.0);

    field(
        ui,
        "Name:",
        &mut state.display_name,
        "Alice Smith",
        "",
        false,
        None,
    );
    field(
        ui,
        "Email:",
        &mut state.email,
        "alice@gmail.com",
        "",
        false,
        None,
    );

    ui.add_space(12.0);

    ui.horizontal(|ui| {
        if ui.button("← Back").clicked() {
            state.step = SetupStep::Smtp;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let can_continue = !state.display_name.is_empty() && !state.email.is_empty();
            if ui
                .add_enabled(can_continue, egui::Button::new("Test & Save →"))
                .clicked()
            {
                state.step = SetupStep::Test;
            }
        });
    });
}

fn render_test_step(ui: &mut Ui, state: &mut SetupState) -> bool {
    ui.heading("Test Connection");

    if state.test_result.is_none() {
        ui.label("Click 'Test' to verify your IMAP connection.");
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            if ui.button("← Back").clicked() {
                state.step = SetupStep::Identity;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(!state.testing, egui::Button::new("Test Connection"))
                    .clicked()
                {
                    state.testing = true;
                    // The actual test is handled by the app's update loop
                    // which runs async IMAP connection test
                }
            });
        });
    } else {
        match &state.test_result {
            Some(Ok(())) => {
                ui.colored_label(
                    ui.visuals().hyperlink_color,
                    RichText::new("✓ Connection successful!").strong(),
                );
                ui.add_space(8.0);
                ui.label(
                    "Your account is configured. Click 'Save & Continue' to start using bMail.",
                );
                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    if ui.button("← Back").clicked() {
                        state.step = SetupStep::Identity;
                        state.test_result = None;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Save & Continue →").clicked() {
                            return true;
                        }
                        false
                    })
                    .inner
                });
            }
            Some(Err(e)) => {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    RichText::new(format!("✗ Connection failed: {e}")).strong(),
                );
                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    if ui.button("← Back").clicked() {
                        state.step = SetupStep::Identity;
                        state.test_result = None;
                    }
                    if ui.button("Retry").clicked() {
                        state.test_result = None;
                        state.testing = true;
                    }
                });
            }
            None => {}
        }
    }

    false
}
