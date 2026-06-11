//! Omarchy theme → egui Visuals mapping.
//!
//! Maps all 19 default Omarchy themes to egui's visual style system.
//! Live theme reloading via polling Omarchy's theme-set hook signal file.

use egui::style::{Selection, WidgetVisuals, Widgets};
use egui::{Color32, Stroke, Visuals};
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Parsed colors from an Omarchy `colors.toml`.
#[derive(Debug, Clone, Copy)]
pub struct OmarchyColors {
    pub name: &'static str,
    pub dark_mode: bool,
    pub background: Color32,
    pub foreground: Color32,
    pub accent: Color32,
    pub color0: Color32,
    pub color1: Color32,
    pub color3: Color32,
    pub color7: Color32,
    pub color8: Color32,
    pub color9: Color32,
}

/// Parse a hex color like "#1e1e2e" into `Color32`.
fn hex(c: &str) -> Color32 {
    let c = c.trim_start_matches('#');
    let r = u8::from_str_radix(&c[0..2], 16).unwrap_or(0xff);
    let g = u8::from_str_radix(&c[2..4], 16).unwrap_or(0xff);
    let b = u8::from_str_radix(&c[4..6], 16).unwrap_or(0xff);
    Color32::from_rgba_unmultiplied(r, g, b, 0xff)
}

fn darken(c: Color32, factor: f32) -> Color32 {
    let f = factor.clamp(0.0, 1.0);
    Color32::from_rgb(
        (c.r() as f32 * (1.0 - f)) as u8,
        (c.g() as f32 * (1.0 - f)) as u8,
        (c.b() as f32 * (1.0 - f)) as u8,
    )
}

fn lighten(c: Color32, factor: f32) -> Color32 {
    let f = factor.clamp(0.0, 1.0);
    Color32::from_rgb(
        (c.r() as f32 + (255.0 - c.r() as f32) * f) as u8,
        (c.g() as f32 + (255.0 - c.g() as f32) * f) as u8,
        (c.b() as f32 + (255.0 - c.b() as f32) * f) as u8,
    )
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 * (1.0 - t) + b.r() as f32 * t) as u8,
        (a.g() as f32 * (1.0 - t) + b.g() as f32 * t) as u8,
        (a.b() as f32 * (1.0 - t) + b.b() as f32 * t) as u8,
    )
}

impl OmarchyColors {
    fn new(
        name: &'static str,
        dark_mode: bool,
        bg: &str,
        fg: &str,
        acc: &str,
        c0: &str,
        c1: &str,
        c3: &str,
        c7: &str,
        c8: &str,
        c9: &str,
    ) -> Self {
        Self {
            name,
            dark_mode,
            background: hex(bg),
            foreground: hex(fg),
            accent: hex(acc),
            color0: hex(c0),
            color1: hex(c1),
            color3: hex(c3),
            color7: hex(c7),
            color8: hex(c8),
            color9: hex(c9),
        }
    }

    pub fn to_egui_visuals(&self, old: &Visuals) -> Visuals {
        let shadow_color = if self.dark_mode {
            Color32::from_black_alpha(96)
        } else {
            Color32::from_black_alpha(25)
        };

        let faint_bg = if self.dark_mode {
            lighten(self.background, 0.06)
        } else {
            darken(self.background, 0.06)
        };

        let extreme_bg = if self.dark_mode {
            lighten(self.background, 0.12)
        } else {
            darken(self.background, 0.12)
        };

        let code_bg = if self.dark_mode {
            darken(self.background, 0.10)
        } else {
            darken(self.background, 0.05)
        };

        let hovered_bg = if self.dark_mode {
            lighten(self.background, 0.10)
        } else {
            darken(self.background, 0.08)
        };

        let open_bg = if self.dark_mode {
            lighten(self.background, 0.08)
        } else {
            darken(self.background, 0.06)
        };

        let stroke_color = if self.dark_mode {
            lighten(self.background, 0.15)
        } else {
            darken(self.background, 0.12)
        };

        Visuals {
            dark_mode: self.dark_mode,
            override_text_color: Some(self.foreground),
            hyperlink_color: self.accent,
            faint_bg_color: faint_bg,
            extreme_bg_color: extreme_bg,
            code_bg_color: code_bg,
            warn_fg_color: self.color3,
            error_fg_color: self.color9,
            window_fill: self.background,
            panel_fill: self.background,
            window_stroke: Stroke {
                color: stroke_color,
                ..old.window_stroke
            },
            widgets: Widgets {
                noninteractive: WidgetVisuals {
                    bg_fill: self.background,
                    weak_bg_fill: self.background,
                    bg_stroke: Stroke {
                        color: mix(self.color0, self.background, 0.5),
                        ..old.widgets.noninteractive.bg_stroke
                    },
                    fg_stroke: Stroke {
                        color: self.color7,
                        ..old.widgets.noninteractive.fg_stroke
                    },
                    ..old.widgets.noninteractive
                },
                inactive: WidgetVisuals {
                    bg_fill: self.color0,
                    weak_bg_fill: self.color0,
                    bg_stroke: Stroke {
                        color: self.color0,
                        ..old.widgets.inactive.bg_stroke
                    },
                    fg_stroke: Stroke {
                        color: self.foreground,
                        ..old.widgets.inactive.fg_stroke
                    },
                    ..old.widgets.inactive
                },
                hovered: WidgetVisuals {
                    bg_fill: hovered_bg,
                    weak_bg_fill: hovered_bg,
                    bg_stroke: Stroke {
                        color: self.accent,
                        ..old.widgets.hovered.bg_stroke
                    },
                    fg_stroke: Stroke {
                        color: self.foreground,
                        ..old.widgets.hovered.fg_stroke
                    },
                    ..old.widgets.hovered
                },
                active: WidgetVisuals {
                    bg_fill: self.accent,
                    weak_bg_fill: self.accent,
                    bg_stroke: Stroke {
                        color: self.accent,
                        ..old.widgets.active.bg_stroke
                    },
                    fg_stroke: Stroke {
                        color: self.background,
                        ..old.widgets.active.fg_stroke
                    },
                    ..old.widgets.active
                },
                open: WidgetVisuals {
                    bg_fill: open_bg,
                    weak_bg_fill: open_bg,
                    bg_stroke: Stroke {
                        color: self.color8,
                        ..old.widgets.open.bg_stroke
                    },
                    fg_stroke: Stroke {
                        color: self.foreground,
                        ..old.widgets.open.fg_stroke
                    },
                    ..old.widgets.open
                },
            },
            selection: Selection {
                bg_fill: Color32::from_rgba_unmultiplied(
                    self.accent.r(),
                    self.accent.g(),
                    self.accent.b(),
                    if self.dark_mode { 50 } else { 40 },
                ),
                stroke: Stroke {
                    color: self.accent,
                    width: 1.0,
                },
            },
            window_shadow: egui::epaint::Shadow {
                color: shadow_color,
                ..old.window_shadow
            },
            popup_shadow: egui::epaint::Shadow {
                color: shadow_color,
                ..old.popup_shadow
            },
            ..old.clone()
        }
    }
}

/// All 19 default Omarchy themes.
static THEMES: Lazy<HashMap<&'static str, OmarchyColors>> = Lazy::new(|| {
    let mut m = HashMap::new();
    // Dark themes
    m.insert(
        "catppuccin",
        OmarchyColors::new(
            "catppuccin",
            true,
            "#1e1e2e",
            "#cdd6f4",
            "#89b4fa",
            "#45475a",
            "#f38ba8",
            "#f9e2af",
            "#bac2de",
            "#585b70",
            "#f38ba8",
        ),
    );
    m.insert(
        "ethereal",
        OmarchyColors::new(
            "ethereal", true, "#060B1E", "#ffcead", "#7d82d9", "#3C486D", "#ED5B5A", "#E9BB4F",
            "#F99957", "#6d7db6", "#faaaa9",
        ),
    );
    m.insert(
        "everforest",
        OmarchyColors::new(
            "everforest",
            true,
            "#2d353b",
            "#d3c6aa",
            "#7fbbb3",
            "#475258",
            "#e67e80",
            "#dbbc7f",
            "#d3c6aa",
            "#475258",
            "#e67e80",
        ),
    );
    m.insert(
        "gruvbox",
        OmarchyColors::new(
            "gruvbox", true, "#282828", "#d4be98", "#7daea3", "#3c3836", "#ea6962", "#d8a657",
            "#d4be98", "#3c3836", "#ea6962",
        ),
    );
    m.insert(
        "hackerman",
        OmarchyColors::new(
            "hackerman",
            true,
            "#0B0C16",
            "#ddf7ff",
            "#82FB9C",
            "#3E4058",
            "#50f872",
            "#50f7d4",
            "#85E1FB",
            "#6a6e95",
            "#85ff9d",
        ),
    );
    m.insert(
        "kanagawa",
        OmarchyColors::new(
            "kanagawa", true, "#1f1f28", "#dcd7ba", "#7e9cd8", "#090618", "#c34043", "#c0a36e",
            "#c8c093", "#727169", "#e82424",
        ),
    );
    m.insert(
        "lumon",
        OmarchyColors::new(
            "lumon", true, "#16242d", "#d6e2ee", "#8bc9eb", "#1b2d40", "#4d86b0", "#6fa4c9",
            "#d6e2ee", "#304860", "#73a6cb",
        ),
    );
    m.insert(
        "matte-black",
        OmarchyColors::new(
            "matte-black",
            true,
            "#121212",
            "#bebebe",
            "#e68e0d",
            "#333333",
            "#D35F5F",
            "#FFC107",
            "#bebebe",
            "#8a8a8d",
            "#B91C1C",
        ),
    );
    m.insert(
        "miasma",
        OmarchyColors::new(
            "miasma", true, "#222222", "#c2c2b0", "#78824b", "#000000", "#685742", "#b36d43",
            "#d7c483", "#666666", "#bb7744",
        ),
    );
    m.insert(
        "nord",
        OmarchyColors::new(
            "nord", true, "#2e3440", "#d8dee9", "#81a1c1", "#3b4252", "#bf616a", "#ebcb8b",
            "#e5e9f0", "#4c566a", "#bf616a",
        ),
    );
    m.insert(
        "osaka-jade",
        OmarchyColors::new(
            "osaka-jade",
            true,
            "#111c18",
            "#C1C497",
            "#509475",
            "#23372B",
            "#FF5345",
            "#459451",
            "#F6F5DD",
            "#53685B",
            "#db9f9c",
        ),
    );
    m.insert(
        "retro-82",
        OmarchyColors::new(
            "retro-82", true, "#05182e", "#f6dcac", "#faa968", "#303442", "#f85525", "#e97b3c",
            "#a7c9c6", "#134e5a", "#f85525",
        ),
    );
    m.insert(
        "ristretto",
        OmarchyColors::new(
            "ristretto",
            true,
            "#2c2525",
            "#e6d9db",
            "#f38d70",
            "#72696a",
            "#fd6883",
            "#f9cc6c",
            "#e6d9db",
            "#948a8b",
            "#ff8297",
        ),
    );
    m.insert(
        "tokyo-night",
        OmarchyColors::new(
            "tokyo-night",
            true,
            "#1a1b26",
            "#a9b1d6",
            "#7aa2f7",
            "#32344a",
            "#f7768e",
            "#e0af68",
            "#787c99",
            "#444b6a",
            "#ff7a93",
        ),
    );
    m.insert(
        "vantablack",
        OmarchyColors::new(
            "vantablack",
            true,
            "#000000",
            "#ffffff",
            "#8d8d8d",
            "#404040",
            "#a4a4a4",
            "#cecece",
            "#ececec",
            "#5c5c5c",
            "#a4a4a4",
        ),
    );
    // Light themes
    m.insert(
        "catppuccin-latte",
        OmarchyColors::new(
            "catppuccin-latte",
            false,
            "#eff1f5",
            "#4c4f69",
            "#1e66f5",
            "#bcc0cc",
            "#d20f39",
            "#df8e1d",
            "#5c5f77",
            "#acb0be",
            "#d20f39",
        ),
    );
    m.insert(
        "flexoki-light",
        OmarchyColors::new(
            "flexoki-light",
            false,
            "#FFFCF0",
            "#100F0F",
            "#205EA6",
            "#DAD8CE",
            "#D14D41",
            "#D0A215",
            "#B7B5AC",
            "#100F0F",
            "#D14D41",
        ),
    );
    m.insert(
        "rose-pine",
        OmarchyColors::new(
            "rose-pine",
            false,
            "#faf4ed",
            "#575279",
            "#56949f",
            "#f2e9e1",
            "#b4637a",
            "#ea9d34",
            "#575279",
            "#9893a5",
            "#b4637a",
        ),
    );
    m.insert(
        "white",
        OmarchyColors::new(
            "white", false, "#ffffff", "#000000", "#6e6e6e", "#c0c0c0", "#2a2a2a", "#4a4a4a",
            "#000000", "#c0c0c0", "#2a2a2a",
        ),
    );
    m
});

pub fn lookup_theme(name: &str) -> Option<&'static OmarchyColors> {
    let normalized = name.to_lowercase().replace([' ', '_'], "-");
    THEMES.get(normalized.as_str())
}

pub fn detect_current_theme() -> Option<String> {
    let output = std::process::Command::new("omarchy")
        .args(["theme", "current"])
        .output()
        .ok()?;
    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

pub fn apply_theme(ctx: &egui::Context, theme_name: &str) {
    let old = ctx.style().visuals.clone();
    if let Some(colors) = lookup_theme(theme_name) {
        ctx.set_visuals(colors.to_egui_visuals(&old));
        log::info!("Applied Omarchy theme: {}", colors.name);
    } else {
        log::warn!("Unknown Omarchy theme '{}', using dark default", theme_name);
        ctx.set_visuals(Visuals::dark());
    }
}

pub fn install_theme_hook() -> std::io::Result<()> {
    let hooks_dir = dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("~/.config"))
        .join("omarchy")
        .join("hooks");
    std::fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join("theme-set");
    let runtime_dir =
        std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());
    if !hook_path.exists() {
        let hook = format!(
            "#!/bin/bash\n# bMail theme hook\ntouch {}/bmail.theme-changed 2>/dev/null || true\n",
            runtime_dir
        );
        std::fs::write(&hook_path, &hook)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hook_path, std::fs::Permissions::from_mode(0o755))?;
        }
        log::info!("Installed theme-set hook");
    }

    let signal_dir = std::path::PathBuf::from(&runtime_dir);
    std::fs::create_dir_all(&signal_dir).ok();
    std::fs::write(signal_dir.join("bmail.theme-changed"), "").ok();
    Ok(())
}

pub fn theme_signal_path() -> std::path::PathBuf {
    let runtime_dir =
        std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());
    std::path::PathBuf::from(runtime_dir).join("bmail.theme-changed")
}
