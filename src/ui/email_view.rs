//! Email view — full email display replacing the main pane.
//!
//! Shows sender, subject, date header, then the Markdown body.
//! Markdown is rendered as rich text in an egui scroll area.

use crate::db::CachedEmail;
use egui::{FontId, RichText, ScrollArea, Ui};

/// Render the full email view.
pub fn render(ui: &mut Ui, email: &CachedEmail, on_close: &mut dyn FnMut()) {
    ui.vertical(|ui| {
        // ── Header bar with close button ──
        ui.horizontal(|ui| {
            if ui.button("← Back").clicked() {
                on_close();
            }
            ui.separator();
            if ui.button("🗑").on_hover_text("Delete").clicked() {
                // handled by parent via command
            }
            if ui.button("↩").on_hover_text("Reply").clicked() {
                // handled by parent via command
            }
        });
        ui.separator();

        // ── Email header ──
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("From:")
                    .strong()
                    .font(FontId::proportional(13.0)),
            );
            ui.label(RichText::new(&email.from).font(FontId::proportional(13.0)));
        });
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("To:")
                    .strong()
                    .font(FontId::proportional(13.0)),
            );
            ui.label(RichText::new(&email.to).font(FontId::proportional(13.0)));
        });
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Date:")
                    .strong()
                    .font(FontId::proportional(13.0)),
            );
            ui.label(RichText::new(&email.date).font(FontId::proportional(13.0)));
        });
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(&email.subject)
                    .strong()
                    .font(FontId::proportional(14.0))
                    .color(ui.visuals().hyperlink_color),
            );
        });
        ui.separator();

        // ── Email body (Markdown) ──
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                render_markdown_body(ui, &email.body_text);
            });
    });
}

/// Render Markdown text as egui rich text.
///
/// Supports:
/// - **bold**
/// - *italic*
/// - `code`
/// - # headers
/// - - bullet lists
/// - > blockquotes
/// - [links](url)
/// - --- horizontal rules
fn render_markdown_body(ui: &mut Ui, markdown: &str) {
    let font_size = 14.0;
    let code_font = FontId::monospace(font_size - 1.0);
    let body_font = FontId::proportional(font_size);
    let header_font = FontId::proportional(font_size + 4.0);

    for line in markdown.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            ui.add_space(4.0);
            continue;
        }

        // Headers
        if let Some(header) = trimmed.strip_prefix("#### ") {
            ui.label(
                RichText::new(header)
                    .strong()
                    .font(FontId::proportional(font_size + 1.0)),
            );
        } else if let Some(header) = trimmed.strip_prefix("### ") {
            ui.label(
                RichText::new(header)
                    .strong()
                    .font(FontId::proportional(font_size + 2.0)),
            );
        } else if let Some(header) = trimmed.strip_prefix("## ") {
            ui.add_space(8.0);
            ui.label(
                RichText::new(header)
                    .strong()
                    .font(FontId::proportional(font_size + 3.0))
                    .color(ui.visuals().hyperlink_color),
            );
            ui.add_space(2.0);
        } else if let Some(header) = trimmed.strip_prefix("# ") {
            ui.add_space(8.0);
            ui.label(
                RichText::new(header)
                    .strong()
                    .font(header_font.clone())
                    .color(ui.visuals().hyperlink_color),
            );
            ui.separator();
        }
        // Horizontal rule
        else if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            ui.separator();
        }
        // Blockquote
        else if let Some(quote) = trimmed.strip_prefix("> ") {
            let _indent = ui.next_widget_position().x + 8.0;
            let _available = ui.available_width() - 16.0;
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.colored_label(ui.visuals().widgets.noninteractive.bg_stroke.color, "▎");
                ui.add_space(4.0);
                render_inline_markdown(ui, quote, &body_font, &code_font);
            });
        }
        // Bullet list
        else if let Some(item) = trimmed.strip_prefix("- ") {
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                ui.label("•");
                ui.add_space(4.0);
                render_inline_markdown(ui, item, &body_font, &code_font);
            });
        }
        // Numbered list
        else if trimmed
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
            && trimmed.contains(". ")
        {
            if let Some(dot_pos) = trimmed.find(". ") {
                let num = &trimmed[..dot_pos];
                let item = &trimmed[dot_pos + 2..];
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(format!("{num}."));
                    ui.add_space(4.0);
                    render_inline_markdown(ui, item, &body_font, &code_font);
                });
            }
        }
        // Code block (indented)
        else if trimmed.starts_with("```") {
            // Code block marker — skip (block code is handled as regular text for now)
            ui.add_space(2.0);
        }
        // Regular paragraph
        else {
            ui.horizontal(|ui| {
                render_inline_markdown(ui, line, &body_font, &code_font);
            });
        }
    }
}

/// Render a single line with inline Markdown formatting.
fn render_inline_markdown(ui: &mut Ui, text: &str, body_font: &FontId, code_font: &FontId) {
    let mut remaining = text;
    let mut first = true;

    while !remaining.is_empty() {
        // Find the next formatting token
        let (bold_start, bold_end) = find_token(remaining, "**");
        let (italic_start, italic_end) = find_token(remaining, "*");
        let (code_start, code_end) = find_token(remaining, "`");
        let (link_start, link_end) = find_link(remaining);

        let next = [
            bold_start.map(|p| (p, 0)),
            italic_start.map(|p| (p, 1)),
            code_start.map(|p| (p, 2)),
            link_start.map(|p| (p, 3)),
        ]
        .into_iter()
        .flatten()
        .min_by_key(|(pos, _)| *pos);

        match next {
            None => {
                // No more formatting — output remaining text
                if first {
                    ui.label(RichText::new(remaining).font(body_font.clone()));
                } else {
                    ui.label(RichText::new(remaining).font(body_font.clone()));
                }
                break;
            }
            Some((pos, kind)) => {
                // Output text before the token
                if pos > 0 {
                    ui.label(RichText::new(&remaining[..pos]).font(body_font.clone()));
                }

                match kind {
                    0 => {
                        // Bold: **text**
                        if let (Some(start), Some(end)) = (bold_start, bold_end) {
                            let inner = &remaining[start + 2..end];
                            ui.label(RichText::new(inner).strong().font(body_font.clone()));
                            remaining = &remaining[end + 2..];
                        }
                    }
                    1 => {
                        // Italic: *text*
                        if let (Some(start), Some(end)) = (italic_start, italic_end) {
                            let inner = &remaining[start + 1..end];
                            ui.label(RichText::new(inner).italics().font(body_font.clone()));
                            remaining = &remaining[end + 1..];
                        }
                    }
                    2 => {
                        // Code: `text`
                        if let (Some(start), Some(end)) = (code_start, code_end) {
                            let inner = &remaining[start + 1..end];
                            ui.label(
                                RichText::new(inner)
                                    .font(code_font.clone())
                                    .background_color(ui.visuals().code_bg_color),
                            );
                            remaining = &remaining[end + 1..];
                        }
                    }
                    3 => {
                        // Link: [text](url)
                        if let (Some(start), Some(end)) = (link_start, link_end) {
                            let link_text = &remaining[start..end];
                            if let Some((display, url)) = parse_link_parts(link_text) {
                                ui.hyperlink_to(
                                    RichText::new(&display).font(body_font.clone()),
                                    url,
                                );
                            }
                            remaining = &remaining[end..];
                        }
                    }
                    _ => {}
                }
                first = false;
            }
        }
    }
}

/// Find the start and end positions of a token like `**text**`.
fn find_token(text: &str, token: &str) -> (Option<usize>, Option<usize>) {
    let start = text.find(token);
    let start = match start {
        Some(s) => s,
        None => return (None, None),
    };
    if start + token.len() >= text.len() {
        return (Some(start), None);
    }
    let end = text[start + token.len()..].find(token);
    let end = match end {
        Some(e) => start + token.len() + e,
        None => return (Some(start), None),
    };
    if end <= start + token.len() {
        return (Some(start), None);
    }
    (Some(start), Some(end))
}

/// Find a Markdown link `[text](url)`.
fn find_link(text: &str) -> (Option<usize>, Option<usize>) {
    let start = match text.find('[') {
        Some(s) => s,
        None => return (None, None),
    };
    let mid = match text[start..].find("](") {
        Some(m) => start + m,
        None => return (Some(start), None),
    };
    let end = match text[mid..].find(')') {
        Some(e) => mid + e + 1,
        None => return (Some(start), None),
    };
    (Some(start), Some(end))
}

/// Parse `[display](url)` into (display, url) parts.
fn parse_link_parts(link_text: &str) -> Option<(String, String)> {
    let link_text = link_text.strip_prefix('[')?;
    let link_text = link_text.strip_suffix(')')?;
    let mut parts = link_text.splitn(2, "](");
    let display = parts.next()?.to_string();
    let url = parts.next()?.to_string();
    Some((display, url))
}
