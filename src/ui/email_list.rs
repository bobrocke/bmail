//! Email list pane — scrollable list of emails for the selected folder.
//!
//! Shows sender, subject, date, and read/unread status.
//! Supports keyboard navigation (j/k, enter to open).

use crate::db::CachedEmail;
use egui::{Align2, Color32, FontId, RichText, Sense, Ui, Vec2};

/// Render the email list pane.
pub fn render(
    ui: &mut Ui,
    emails: &[CachedEmail],
    selected_uid: Option<u32>,
    on_select: &mut dyn FnMut(u32),
    on_open: &mut dyn FnMut(u32),
) {
    ui.vertical(|ui| {
        if emails.is_empty() {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("No emails").weak());
                ui.label(RichText::new("Select a folder to load messages").weak());
            });
            return;
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for email in emails {
                    render_email_row(ui, email, selected_uid, on_select, on_open);
                }
            });
    });
}

fn render_email_row(
    ui: &mut Ui,
    email: &CachedEmail,
    selected_uid: Option<u32>,
    on_select: &mut dyn FnMut(u32),
    on_open: &mut dyn FnMut(u32),
) {
    let is_selected = selected_uid == Some(email.uid);
    let is_unread = !email.seen;

    let row_height = 48.0;

    let (response, painter) =
        ui.allocate_painter(Vec2::new(ui.available_width(), row_height), Sense::click());

    // Background
    if is_selected {
        painter.rect_filled(response.rect, 0.0, ui.visuals().selection.bg_fill);
    } else if response.hovered() {
        painter.rect_filled(response.rect, 0.0, ui.visuals().widgets.hovered.bg_fill);
    } else if !email.seen {
        // Subtle highlight for unread
        let mut c = ui.visuals().faint_bg_color;
        c = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 80);
        painter.rect_filled(response.rect, 0.0, c);
    }

    // Unread indicator dot
    if is_unread {
        let dot_center = response.rect.left_center() + egui::vec2(10.0, 0.0);
        painter.circle_filled(dot_center, 4.0, ui.visuals().hyperlink_color);
    }

    // Sender (bold if unread)
    let x_offset = 22.0;
    let sender_text = if email.from.is_empty() {
        "(no sender)".to_string()
    } else {
        truncate_str(&email.from, 30)
    };
    let mut sender = RichText::new(&sender_text).font(FontId::proportional(13.0));
    if is_unread {
        sender = sender.strong();
    }
    if is_selected {
        sender = sender.color(ui.visuals().selection.stroke.color);
    }
    let sender_pos = response.rect.min + egui::vec2(x_offset, 4.0);
    painter.text(
        sender_pos,
        Align2::LEFT_TOP,
        sender_text,
        FontId::proportional(13.0),
        if is_selected {
            ui.visuals().selection.stroke.color
        } else if is_unread {
            ui.visuals().text_color()
        } else {
            ui.visuals().weak_text_color()
        },
    );

    // Subject
    let subject_str = if email.subject.is_empty() {
        "(no subject)".to_string()
    } else {
        truncate_str(&email.subject, 60)
    };
    let subject_pos = response.rect.min + egui::vec2(x_offset, 20.0);
    painter.text(
        subject_pos,
        Align2::LEFT_TOP,
        &subject_str,
        FontId::proportional(12.0),
        if is_selected {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().weak_text_color()
        },
    );

    // Date (right-aligned)
    let date_str = format_email_date(&email.date);
    let date_pos = response.rect.right_top() + egui::vec2(-8.0, 4.0);
    painter.text(
        date_pos,
        Align2::RIGHT_TOP,
        date_str,
        FontId::proportional(11.0),
        if is_selected {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().weak_text_color()
        },
    );

    // Separator line
    painter.line_segment(
        [
            response.rect.left_bottom() + egui::vec2(x_offset, 0.0),
            response.rect.right_bottom(),
        ],
        ui.visuals().widgets.noninteractive.bg_stroke,
    );

    // Handle clicks
    if response.clicked() {
        on_select(email.uid);
    }
    if response.double_clicked() {
        on_open(email.uid);
    }
}

/// Truncate a string to `max_len` characters, adding "..." if truncated.
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{truncated}...")
    }
}

/// Format an RFC 2822 date into a human-readable short date.
fn format_email_date(date_str: &str) -> String {
    // Try to parse the date and show a relative or short format
    if let Ok(date) = chrono::DateTime::parse_from_rfc2822(date_str) {
        let now = chrono::Utc::now();
        let duration = now.signed_duration_since(date);

        if duration.num_minutes() < 1 {
            return "just now".to_string();
        }
        if duration.num_minutes() < 60 {
            return format!("{}m ago", duration.num_minutes());
        }
        if duration.num_hours() < 24 {
            return format!("{}h ago", duration.num_hours());
        }
        if duration.num_days() < 7 {
            return format!("{}d ago", duration.num_days());
        }

        // Show date for older emails
        return date.format("%b %d").to_string();
    }

    // Fallback: try to extract just the date portion
    date_str
        .split(',')
        .nth(1)
        .unwrap_or(date_str)
        .trim()
        .to_string()
}
