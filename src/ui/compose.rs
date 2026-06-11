//! Compose view — reply, reply-all, forward, and new message composition.
//!
//! Provides fields for To, Subject, and body text. Sends via SMTP.

use egui::{FontId, RichText, ScrollArea, TextEdit, Ui};

/// The state of a compose session.
#[derive(Debug, Clone)]
pub struct ComposeState {
    /// Recipient(s), comma-separated.
    pub to: String,
    /// Email subject.
    pub subject: String,
    /// Message body.
    pub body: String,
    /// Whether this is in reply mode (pre-fills subject with "Re:").
    pub is_reply: bool,
    /// Whether this is a forward (pre-fills subject with "Fwd:" and body with quote).
    pub is_forward: bool,
    /// The original email we're replying to (for threading/In-Reply-To).
    pub in_reply_to: Option<String>,
    /// Sending state.
    pub sending: bool,
    /// Error message if send failed.
    pub error: Option<String>,
    /// Success message if sent.
    pub sent: bool,
}

impl ComposeState {
    /// Create a new compose for a reply.
    pub fn reply(to: &str, subject: &str, original_body: &str) -> Self {
        let re_subject = if subject.to_lowercase().starts_with("re:") {
            subject.to_string()
        } else {
            format!("Re: {subject}")
        };

        // Quote the original body
        let quoted = quote_body(original_body);

        Self {
            to: to.to_string(),
            subject: re_subject,
            body: format!("\n\n{quoted}"),
            is_reply: true,
            is_forward: false,
            in_reply_to: None,
            sending: false,
            error: None,
            sent: false,
        }
    }

    /// Create a new compose for a forward.
    pub fn forward(from: &str, date: &str, subject: &str, original_body: &str) -> Self {
        let fwd_subject = if subject.to_lowercase().starts_with("fwd:") {
            subject.to_string()
        } else {
            format!("Fwd: {subject}")
        };

        let body = format!(
            "\n\n---------- Forwarded message ----------\n\
             From: {from}\n\
             Date: {date}\n\
             Subject: {subject}\n\n\
             {original_body}"
        );

        Self {
            to: String::new(),
            subject: fwd_subject,
            body,
            is_reply: false,
            is_forward: true,
            in_reply_to: None,
            sending: false,
            error: None,
            sent: false,
        }
    }
}

/// Quote body text with "> " prefix for replies.
fn quote_body(body: &str) -> String {
    body.lines()
        .map(|line| {
            if line.is_empty() {
                ">".to_string()
            } else {
                format!("> {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render the compose view.
pub fn render(
    ui: &mut Ui,
    state: &mut ComposeState,
    on_send: &mut dyn FnMut(),
    on_cancel: &mut dyn FnMut(),
) {
    ui.vertical(|ui| {
        // Header
        ui.horizontal(|ui| {
            if ui.button("← Cancel").clicked() {
                on_cancel();
            }
            ui.separator();
            if state.is_reply {
                ui.heading(RichText::new("Reply").strong());
            } else if state.is_forward {
                ui.heading(RichText::new("Forward").strong());
            } else {
                ui.heading(RichText::new("New Message").strong());
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let send_enabled = !state.to.is_empty()
                    && !state.subject.is_empty()
                    && !state.body.is_empty()
                    && !state.sending;
                if ui
                    .add_enabled(send_enabled, egui::Button::new("Send"))
                    .clicked()
                {
                    on_send();
                }
            });
        });
        ui.separator();

        // Error / success messages
        if let Some(ref error) = state.error {
            ui.colored_label(ui.visuals().error_fg_color, format!("Error: {error}"));
        }
        if state.sent {
            ui.colored_label(ui.visuals().hyperlink_color, "Message sent!");
            return;
        }

        // To field
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("To:")
                    .strong()
                    .font(FontId::proportional(13.0)),
            );
            ui.add_sized(
                [ui.available_width(), 20.0],
                TextEdit::singleline(&mut state.to)
                    .font(FontId::proportional(13.0))
                    .hint_text("recipient@example.com"),
            );
        });

        ui.add_space(4.0);

        // Subject field
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Subject:")
                    .strong()
                    .font(FontId::proportional(13.0)),
            );
            ui.add_sized(
                [ui.available_width(), 20.0],
                TextEdit::singleline(&mut state.subject).font(FontId::proportional(13.0)),
            );
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        // Body
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_sized(
                    [ui.available_width(), ui.available_height()],
                    TextEdit::multiline(&mut state.body)
                        .font(FontId::monospace(13.0))
                        .hint_text("Write your message..."),
                );
            });
    });
}
