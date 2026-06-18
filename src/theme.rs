use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use ratatui::style::Color;
use serde::{Deserialize, Serialize};

/// Built-in Catppuccin Mocha palette — the single source of truth for the
/// default colors. The config file and the resolved fallbacks both derive
/// from these. Values match the canonical Ghostty/terminal palette (the one
/// terminalcolors.com exports), so accents use ANSI palette colors rather than
/// Catppuccin's extended hues.
mod default {
    use ratatui::style::Color;

    pub const TEXT: Color = Color::Rgb(205, 214, 244); // #cdd6f4  foreground
    pub const SURFACE: Color = Color::Rgb(69, 71, 90); // #45475a  palette 0 (Surface1)
    pub const ACTIVE_BORDER: Color = Color::Rgb(245, 194, 231); // #f5c2e7  palette 5 (Pink)
    pub const INACTIVE_BORDER: Color = Color::Rgb(88, 91, 112); // #585b70  palette 8 (Surface2)
    pub const POPUP_BORDER: Color = Color::Rgb(148, 226, 213); // #94e2d5  palette 6 (Teal)
    pub const DIR_FG: Color = Color::Rgb(137, 180, 250); // #89b4fa  palette 4 (Blue)
    pub const SELECTED_FG: Color = Color::Rgb(166, 227, 161); // #a6e3a1  palette 2 (Green)
    pub const COPY_TARGET_BORDER: Color = Color::Rgb(249, 226, 175); // #f9e2af  palette 3 (Yellow)
    pub const MOVE_TARGET_BORDER: Color = Color::Rgb(243, 139, 168); // #f38ba8  palette 1 (Red)
}

/// Resolved color set used throughout the UI.
pub struct Theme {
    pub text: Color,
    pub active_border: Color,
    pub inactive_border: Color,
    pub popup_border: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
    pub dir_fg: Color,
    pub selected_fg: Color,
    pub copy_target_border: Color,
    pub move_target_border: Color,
}

/// Serializable color settings, stored as `#rrggbb` hex strings so the file is
/// human-friendly. Missing fields fall back to their defaults via `serde`.
#[derive(Serialize, Deserialize)]
#[serde(default)]
struct ThemeConfig {
    text: String,
    active_border: String,
    inactive_border: String,
    popup_border: String,
    highlight_bg: String,
    highlight_fg: String,
    dir_fg: String,
    selected_fg: String,
    copy_target_border: String,
    move_target_border: String,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            text: to_hex(default::TEXT),
            active_border: to_hex(default::ACTIVE_BORDER),
            inactive_border: to_hex(default::INACTIVE_BORDER),
            popup_border: to_hex(default::POPUP_BORDER),
            highlight_bg: to_hex(default::SURFACE),
            highlight_fg: to_hex(default::TEXT),
            dir_fg: to_hex(default::DIR_FG),
            selected_fg: to_hex(default::SELECTED_FG),
            copy_target_border: to_hex(default::COPY_TARGET_BORDER),
            move_target_border: to_hex(default::MOVE_TARGET_BORDER),
        }
    }
}

impl ThemeConfig {
    /// Parse each hex string into a `Color`, falling back to the built-in
    /// default for any field that fails to parse.
    fn resolve(&self) -> Theme {
        Theme {
            text: parse_hex(&self.text).unwrap_or(default::TEXT),
            active_border: parse_hex(&self.active_border).unwrap_or(default::ACTIVE_BORDER),
            inactive_border: parse_hex(&self.inactive_border).unwrap_or(default::INACTIVE_BORDER),
            popup_border: parse_hex(&self.popup_border).unwrap_or(default::POPUP_BORDER),
            highlight_bg: parse_hex(&self.highlight_bg).unwrap_or(default::SURFACE),
            highlight_fg: parse_hex(&self.highlight_fg).unwrap_or(default::TEXT),
            dir_fg: parse_hex(&self.dir_fg).unwrap_or(default::DIR_FG),
            selected_fg: parse_hex(&self.selected_fg).unwrap_or(default::SELECTED_FG),
            copy_target_border: parse_hex(&self.copy_target_border)
                .unwrap_or(default::COPY_TARGET_BORDER),
            move_target_border: parse_hex(&self.move_target_border)
                .unwrap_or(default::MOVE_TARGET_BORDER),
        }
    }
}

static THEME: OnceLock<Theme> = OnceLock::new();

/// Install the active theme. Called once at startup. A no-op if a theme has
/// already been resolved (e.g. by an accessor used before `init`).
pub fn init(theme: Theme) {
    let _ = THEME.set(theme);
}

fn current() -> &'static Theme {
    THEME.get_or_init(|| ThemeConfig::default().resolve())
}

pub fn text() -> Color {
    current().text
}
pub fn active_border() -> Color {
    current().active_border
}
pub fn inactive_border() -> Color {
    current().inactive_border
}
pub fn popup_border() -> Color {
    current().popup_border
}
pub fn highlight_bg() -> Color {
    current().highlight_bg
}
pub fn highlight_fg() -> Color {
    current().highlight_fg
}
pub fn dir_fg() -> Color {
    current().dir_fg
}
pub fn selected_fg() -> Color {
    current().selected_fg
}
pub fn copy_target_border() -> Color {
    current().copy_target_border
}
pub fn move_target_border() -> Color {
    current().move_target_border
}

/// Load colors from `~/.config/lfm/colors.json`. If the file is missing, write
/// a default file so the user has something to edit, then use the defaults. A
/// malformed file silently falls back to the defaults.
pub fn load() -> Theme {
    let Some(path) = config_path() else {
        return ThemeConfig::default().resolve();
    };
    if let Ok(content) = fs::read_to_string(&path) {
        serde_json::from_str::<ThemeConfig>(&content)
            .unwrap_or_default()
            .resolve()
    } else {
        // No readable file yet — write defaults so the user has something to
        // edit, then fall back to them.
        let _ = write_default(&path);
        ThemeConfig::default().resolve()
    }
}

/// Resolve `~/.config/lfm/colors.json` (respecting `XDG_CONFIG_HOME`).
fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("lfm").join("colors.json"))
}

fn write_default(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&ThemeConfig::default()).map_err(io::Error::other)?;
    fs::write(path, json)
}

/// Format an `Rgb` color as `#rrggbb`. Non-`Rgb` colors render as `#000000`,
/// but the defaults are all `Rgb`.
pub(crate) fn to_hex(color: Color) -> String {
    match color {
        Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        _ => "#000000".to_string(),
    }
}

/// Parse a `#rrggbb` (or `rrggbb`) hex string into a `Color::Rgb`.
pub(crate) fn parse_hex(s: &str) -> Option<Color> {
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}
