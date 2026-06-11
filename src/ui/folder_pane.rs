//! Folder pane — nested tree view of IMAP folders.
//!
//! Displays folders in a hierarchical tree with unread counts.
//! Left-click selects a folder and loads its emails.
//! Supports keyboard navigation (j/k, enter).

use crate::db::CachedFolder;
use crate::imap::FolderNode;
use egui::{Color32, RichText, Ui, Vec2};

/// Which folder is currently selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderSelection {
    /// Full IMAP path of the selected folder.
    pub path: String,
    /// Display name (last component).
    pub name: String,
}

/// Render the folder tree pane.
pub fn render(
    ui: &mut Ui,
    tree: &[FolderNode],
    folder_cache: &[CachedFolder],
    selected: &Option<FolderSelection>,
    on_select: &mut dyn FnMut(String, String),
) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.heading(RichText::new("Folders").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄").clicked() {
                    // Refresh triggered by parent
                }
            });
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for node in tree {
                    render_node(ui, node, folder_cache, selected, on_select, 0);
                }
            });
    });
}

fn render_node(
    ui: &mut Ui,
    node: &FolderNode,
    folder_cache: &[CachedFolder],
    selected: &Option<FolderSelection>,
    on_select: &mut dyn FnMut(String, String),
    depth: usize,
) {
    let indent = depth as f32 * 16.0;
    let is_selected = selected
        .as_ref()
        .map(|s| s.path == node.full_path)
        .unwrap_or(false);

    // Get unread count from cache
    let unread = folder_cache
        .iter()
        .find(|f| f.name == node.full_path)
        .map(|f| f.unread)
        .unwrap_or(0);

    let icon = if node.children.is_empty() {
        "📧"
    } else {
        "📁"
    };

    let mut text = RichText::new(format!("{icon}  {}", node.name));

    if is_selected {
        text = text.strong().color(Color32::WHITE);
    }

    let (response, painter) =
        ui.allocate_painter(Vec2::new(ui.available_width(), 22.0), egui::Sense::click());

    if is_selected {
        painter.rect_filled(response.rect, 0.0, ui.visuals().selection.bg_fill);
    } else if response.hovered() {
        painter.rect_filled(response.rect, 0.0, ui.visuals().widgets.hovered.bg_fill);
    }

    // Draw the text
    let text_pos = response.rect.left_center() + egui::vec2(indent + 4.0, 0.0);
    painter.text(
        text_pos,
        egui::Align2::LEFT_CENTER,
        node.name.clone(),
        egui::FontId::proportional(14.0),
        if is_selected {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().text_color()
        },
    );

    // Draw unread badge
    if unread > 0 {
        let badge_text = if unread > 999 {
            "999+".to_string()
        } else {
            unread.to_string()
        };

        let badge_pos = response.rect.right_center() - egui::vec2(40.0, 0.0);
        let galley = ui.painter().layout(
            badge_text.clone(),
            egui::FontId::proportional(11.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        let badge_rect = egui::Rect::from_center_size(
            badge_pos + egui::vec2(galley.size().x / 2.0, 0.0),
            galley.size() + egui::vec2(8.0, 4.0),
        );
        painter.rect_filled(badge_rect, 6.0, ui.visuals().hyperlink_color);
        painter.text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            badge_text,
            egui::FontId::proportional(11.0),
            Color32::WHITE,
        );
    }

    if response.clicked() && node.selectable {
        on_select(node.full_path.clone(), node.name.clone());
    }

    // Render children
    if !node.children.is_empty() {
        for child in &node.children {
            render_node(ui, child, folder_cache, selected, on_select, depth + 1);
        }
    }
}
