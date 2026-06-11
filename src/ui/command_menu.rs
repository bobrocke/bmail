//! Command palette overlay — the `/` drop-down menu.
//!
//! Displays available commands in a scrollable popup. Typing filters the list.
//! Enter executes the selected command, Escape closes.

use crate::commands::{CommandAction, CommandPalette};
use egui::{Area, Frame, Key, Order, RichText, ScrollArea};

/// Render the command palette as a centered overlay.
/// Returns the selected action if the user confirmed one.
pub fn render(ctx: &egui::Context, palette: &mut CommandPalette) -> Option<CommandAction> {
    if !palette.visible {
        return None;
    }

    let mut action = None;

    let center = ctx.screen_rect().center();
    Area::new("command_palette".into())
        .order(Order::Foreground)
        .fixed_pos(egui::pos2(center.x - 250.0, center.y - 150.0))
        .show(ctx, |ui| {
            Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_max_width(500.0);
                ui.set_max_height(300.0);

                // Search input
                ui.horizontal(|ui| {
                    ui.label("/");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut palette.query)
                            .font(egui::FontId::proportional(16.0))
                            .hint_text("type to filter commands...")
                            .desired_width(400.0),
                    );
                    response.request_focus();
                });
                ui.separator();

                // Command list
                ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .max_height(250.0)
                    .show(ui, |ui| {
                        for (i, cmd) in palette.filtered_commands.iter().enumerate() {
                            let is_selected = i == palette.selected_index;

                            let text = if let Some(sc) = cmd.shortcut {
                                format!("{}    ({})    {}", cmd.name, sc, cmd.description)
                            } else {
                                format!("{}    {}", cmd.name, cmd.description)
                            };

                            let rich = if is_selected {
                                RichText::new(&text)
                                    .strong()
                                    .color(ui.visuals().selection.stroke.color)
                            } else {
                                RichText::new(&text)
                            };

                            let response = ui.selectable_label(is_selected, rich);

                            if response.clicked() {
                                action = Some(cmd.action.clone());
                            }
                        }

                        if palette.filtered_commands.is_empty() {
                            ui.label(RichText::new("No matching commands").weak());
                        }
                    });
            });
        });

    // Handle keyboard input
    handle_keyboard(ctx, palette, &mut action);

    action
}

fn handle_keyboard(
    ctx: &egui::Context,
    palette: &mut CommandPalette,
    action: &mut Option<CommandAction>,
) {
    let input = ctx.input(|i| i.clone());

    for event in &input.events {
        match event {
            egui::Event::Key {
                key: Key::Escape,
                pressed: true,
                ..
            } => {
                palette.close();
            }
            egui::Event::Key {
                key: Key::Enter,
                pressed: true,
                ..
            } => {
                if let Some(a) = palette.selected_action() {
                    *action = Some(a);
                    palette.close();
                }
            }
            egui::Event::Key {
                key: Key::ArrowDown,
                pressed: true,
                ..
            } => {
                palette.select_next();
            }
            egui::Event::Key {
                key: Key::ArrowUp,
                pressed: true,
                ..
            } => {
                palette.select_prev();
            }
            egui::Event::Text(text) => {
                // Don't capture the '/' that opened the palette
                if !palette.visible && text == "/" {
                    continue;
                }
                for c in text.chars() {
                    if c.is_ascii_control() {
                        continue;
                    }
                    palette.push_char(c);
                }
            }
            egui::Event::Key {
                key: Key::Backspace,
                pressed: true,
                ..
            } => {
                palette.backspace();
            }
            _ => {}
        }
    }
}
