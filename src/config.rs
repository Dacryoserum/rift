use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub tab_size: u8,
    pub follow_poll_interval_ms: u64,
    pub index_sample_interval_bytes: u64,
    pub max_line_bytes: usize,
    pub minimap_enabled: bool,
    pub theme: ThemeName,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tab_size: 4,
            follow_poll_interval_ms: 250,
            index_sample_interval_bytes: 65536,
            max_line_bytes: 4096,
            minimap_enabled: true,
            theme: ThemeName::Dark,
        }
    }
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config_path = config_path();
        if !config_path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}

fn config_path() -> PathBuf {
    dirs_config()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rift")
        .join("config.toml")
}

fn dirs_config() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
        })
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ThemeName {
    Dark,
    Light,
    Solarized,
}

pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub gutter_fg: Color,
    pub search_highlight_bg: Color,
    pub current_line_bg: Color,
    pub log_error_fg: Color,
    pub log_warn_fg: Color,
    pub log_info_fg: Color,
    pub log_debug_fg: Color,
    pub json_key_fg: Color,
    pub json_string_fg: Color,
    pub json_number_fg: Color,
    pub csv_header_style: Style,
    pub csv_odd_col_fg: Color,
    pub csv_even_col_fg: Color,
    pub bookmark_fg: Color,
    pub minimap_hit_fg: Color,
    pub minimap_viewport_fg: Color,
    pub minimap_bookmark_fg: Color,
    pub statusbar_bg: Color,
    pub statusbar_fg: Color,
    pub cmdline_bg: Color,
    pub cmdline_fg: Color,
    pub popup_bg: Color,
    pub popup_border_fg: Color,
}

impl Theme {
    pub fn from_name(name: &ThemeName) -> Self {
        match name {
            ThemeName::Dark => Self::dark(),
            ThemeName::Light => Self::light(),
            ThemeName::Solarized => Self::solarized(),
        }
    }

    fn dark() -> Self {
        Self {
            background: Color::Rgb(28, 28, 28),
            foreground: Color::Rgb(212, 212, 212),
            gutter_fg: Color::Rgb(90, 90, 90),
            search_highlight_bg: Color::Rgb(100, 80, 0),
            current_line_bg: Color::Rgb(40, 40, 50),
            log_error_fg: Color::Rgb(240, 80, 80),
            log_warn_fg: Color::Rgb(240, 180, 50),
            log_info_fg: Color::Rgb(100, 200, 100),
            log_debug_fg: Color::Rgb(100, 160, 240),
            json_key_fg: Color::Rgb(86, 182, 194),
            json_string_fg: Color::Rgb(152, 195, 121),
            json_number_fg: Color::Rgb(229, 192, 123),
            csv_header_style: Style::default()
                .fg(Color::Rgb(255, 255, 255))
                .add_modifier(Modifier::BOLD),
            csv_odd_col_fg: Color::Rgb(180, 180, 240),
            csv_even_col_fg: Color::Rgb(140, 200, 180),
            bookmark_fg: Color::Rgb(255, 200, 50),
            minimap_hit_fg: Color::Rgb(240, 160, 60),
            minimap_viewport_fg: Color::Rgb(100, 180, 240),
            minimap_bookmark_fg: Color::Rgb(255, 200, 50),
            statusbar_bg: Color::Rgb(50, 50, 80),
            statusbar_fg: Color::Rgb(200, 200, 220),
            cmdline_bg: Color::Rgb(28, 28, 28),
            cmdline_fg: Color::Rgb(200, 200, 200),
            popup_bg: Color::Rgb(40, 40, 60),
            popup_border_fg: Color::Rgb(100, 120, 200),
        }
    }

    fn light() -> Self {
        Self {
            background: Color::Rgb(250, 250, 250),
            foreground: Color::Rgb(30, 30, 30),
            gutter_fg: Color::Rgb(160, 160, 160),
            search_highlight_bg: Color::Rgb(255, 230, 100),
            current_line_bg: Color::Rgb(230, 235, 245),
            log_error_fg: Color::Rgb(200, 30, 30),
            log_warn_fg: Color::Rgb(160, 100, 0),
            log_info_fg: Color::Rgb(0, 140, 0),
            log_debug_fg: Color::Rgb(0, 80, 180),
            json_key_fg: Color::Rgb(0, 100, 160),
            json_string_fg: Color::Rgb(0, 130, 60),
            json_number_fg: Color::Rgb(160, 80, 0),
            csv_header_style: Style::default()
                .fg(Color::Rgb(0, 0, 0))
                .add_modifier(Modifier::BOLD),
            csv_odd_col_fg: Color::Rgb(40, 40, 160),
            csv_even_col_fg: Color::Rgb(0, 100, 80),
            bookmark_fg: Color::Rgb(200, 100, 0),
            minimap_hit_fg: Color::Rgb(200, 120, 0),
            minimap_viewport_fg: Color::Rgb(0, 80, 180),
            minimap_bookmark_fg: Color::Rgb(200, 100, 0),
            statusbar_bg: Color::Rgb(200, 210, 230),
            statusbar_fg: Color::Rgb(30, 30, 60),
            cmdline_bg: Color::Rgb(240, 240, 240),
            cmdline_fg: Color::Rgb(30, 30, 30),
            popup_bg: Color::Rgb(240, 242, 250),
            popup_border_fg: Color::Rgb(80, 100, 180),
        }
    }

    fn solarized() -> Self {
        Self {
            background: Color::Rgb(0, 43, 54),
            foreground: Color::Rgb(131, 148, 150),
            gutter_fg: Color::Rgb(88, 110, 117),
            search_highlight_bg: Color::Rgb(101, 123, 131),
            current_line_bg: Color::Rgb(7, 54, 66),
            log_error_fg: Color::Rgb(220, 50, 47),
            log_warn_fg: Color::Rgb(203, 75, 22),
            log_info_fg: Color::Rgb(133, 153, 0),
            log_debug_fg: Color::Rgb(38, 139, 210),
            json_key_fg: Color::Rgb(42, 161, 152),
            json_string_fg: Color::Rgb(133, 153, 0),
            json_number_fg: Color::Rgb(203, 75, 22),
            csv_header_style: Style::default()
                .fg(Color::Rgb(253, 246, 227))
                .add_modifier(Modifier::BOLD),
            csv_odd_col_fg: Color::Rgb(38, 139, 210),
            csv_even_col_fg: Color::Rgb(42, 161, 152),
            bookmark_fg: Color::Rgb(181, 137, 0),
            minimap_hit_fg: Color::Rgb(211, 54, 130),
            minimap_viewport_fg: Color::Rgb(38, 139, 210),
            minimap_bookmark_fg: Color::Rgb(181, 137, 0),
            statusbar_bg: Color::Rgb(7, 54, 66),
            statusbar_fg: Color::Rgb(147, 161, 161),
            cmdline_bg: Color::Rgb(0, 43, 54),
            cmdline_fg: Color::Rgb(131, 148, 150),
            popup_bg: Color::Rgb(7, 54, 66),
            popup_border_fg: Color::Rgb(38, 139, 210),
        }
    }
}
