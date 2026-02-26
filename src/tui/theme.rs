use ratatui::style::{Color, Style};
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub accent: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub muted: Color,
    pub highlight: Color,
    pub selection_bg: Color,
    pub selection_fg: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::load("omarchy")
    }
}

impl Theme {
    pub fn load(name: &str) -> Self {
        match name {
            "omarchy" => Self::try_omarchy_tasuki()
                .or_else(Self::try_omarchy_colors)
                .unwrap_or_else(Self::dark),
            "dark" => Self::dark(),
            "light" => Self::light(),
            custom => Self::try_custom(custom).unwrap_or_else(Self::dark),
        }
    }

    fn try_omarchy_tasuki() -> Option<Self> {
        let path = Self::omarchy_theme_path()?;
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok()?;
        let theme_file: ThemeFile = toml::from_str(&content).ok()?;
        Some(theme_file.colors.into())
    }

    fn try_omarchy_colors() -> Option<Self> {
        let path = Self::omarchy_colors_path()?;
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok()?;
        let colors: OmarchyColorFile = toml::from_str(&content).ok()?;
        Some(Self {
            background: hex_to_color(&colors.background)?,
            foreground: hex_to_color(&colors.foreground)?,
            accent: hex_to_color(&colors.accent)?,
            success: hex_to_color(&colors.color2)?,
            warning: hex_to_color(&colors.color3)?,
            error: hex_to_color(&colors.color1)?,
            muted: hex_to_color(&colors.color8)?,
            highlight: hex_to_color(&colors.color5)?,
            selection_bg: hex_to_color(&colors.selection_background)?,
            selection_fg: hex_to_color(&colors.selection_foreground)?,
        })
    }

    fn try_custom(name: &str) -> Option<Self> {
        let path = dirs::config_dir()?
            .join("tasuki")
            .join("themes")
            .join(format!("{}.toml", name));
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok()?;
        let theme_file: ThemeFile = toml::from_str(&content).ok()?;
        Some(theme_file.colors.into())
    }

    pub fn watch_path(&self) -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home).join(".config/omarchy/current"))
    }

    fn omarchy_theme_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home).join(".config/omarchy/current/theme/tasuki.toml"))
    }

    fn omarchy_colors_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home).join(".config/omarchy/current/theme/colors.toml"))
    }

    #[cfg(test)]
    fn omarchy_available() -> bool {
        Self::omarchy_colors_path()
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    pub fn dark() -> Self {
        Self {
            background: Color::Rgb(30, 30, 30),
            foreground: Color::Rgb(220, 220, 220),
            accent: Color::Rgb(100, 149, 237),
            success: Color::Rgb(95, 135, 95),
            warning: Color::Rgb(218, 165, 32),
            error: Color::Rgb(205, 92, 92),
            muted: Color::Rgb(128, 128, 128),
            highlight: Color::Rgb(147, 112, 219),
            selection_bg: Color::Rgb(70, 70, 70),
            selection_fg: Color::Rgb(255, 255, 255),
        }
    }

    pub fn light() -> Self {
        Self {
            background: Color::Rgb(250, 250, 250),
            foreground: Color::Rgb(50, 50, 50),
            accent: Color::Rgb(65, 105, 225),
            success: Color::Rgb(34, 139, 34),
            warning: Color::Rgb(218, 165, 32),
            error: Color::Rgb(220, 20, 60),
            muted: Color::Rgb(128, 128, 128),
            highlight: Color::Rgb(138, 43, 226),
            selection_bg: Color::Rgb(200, 220, 255),
            selection_fg: Color::Rgb(50, 50, 50),
        }
    }

    pub fn style_default(&self) -> Style {
        Style::default().bg(self.background).fg(self.foreground)
    }

    pub fn style_selected(&self) -> Style {
        Style::default().bg(self.selection_bg).fg(self.selection_fg)
    }

    pub fn style_accent(&self) -> Style {
        Style::default().fg(self.accent)
    }

    pub fn style_success(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn style_warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    pub fn style_error(&self) -> Style {
        Style::default().fg(self.error)
    }

    pub fn style_muted(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn style_highlight(&self) -> Style {
        Style::default().fg(self.highlight)
    }
}

pub struct DynamicTheme {
    theme: Arc<RwLock<Theme>>,
}

impl DynamicTheme {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme: Arc::new(RwLock::new(theme)),
        }
    }

    pub fn get(&self) -> Theme {
        self.theme
            .read()
            .map(|t| t.clone())
            .unwrap_or_else(|_| Theme::default())
    }

    pub fn update(&self, new_theme: Theme) {
        if let Ok(mut theme) = self.theme.write() {
            *theme = new_theme;
        }
    }
}

impl Clone for DynamicTheme {
    fn clone(&self) -> Self {
        Self {
            theme: Arc::clone(&self.theme),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ThemeFile {
    name: String,
    description: Option<String>,
    colors: ColorScheme,
}

#[derive(Debug, Deserialize)]
struct ColorScheme {
    background: String,
    foreground: String,
    accent: String,
    success: String,
    warning: String,
    error: String,
    muted: String,
    highlight: String,
    selection_bg: String,
    selection_fg: String,
}

impl From<ColorScheme> for Theme {
    fn from(scheme: ColorScheme) -> Self {
        Self {
            background: hex_to_color(&scheme.background).unwrap_or(Color::Black),
            foreground: hex_to_color(&scheme.foreground).unwrap_or(Color::White),
            accent: hex_to_color(&scheme.accent).unwrap_or(Color::Cyan),
            success: hex_to_color(&scheme.success).unwrap_or(Color::Green),
            warning: hex_to_color(&scheme.warning).unwrap_or(Color::Yellow),
            error: hex_to_color(&scheme.error).unwrap_or(Color::Red),
            muted: hex_to_color(&scheme.muted).unwrap_or(Color::Gray),
            highlight: hex_to_color(&scheme.highlight).unwrap_or(Color::Magenta),
            selection_bg: hex_to_color(&scheme.selection_bg).unwrap_or(Color::Blue),
            selection_fg: hex_to_color(&scheme.selection_fg).unwrap_or(Color::White),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct OmarchyColorFile {
    accent: String,
    cursor: String,
    foreground: String,
    background: String,
    selection_foreground: String,
    selection_background: String,
    color0: String,
    color1: String,
    color2: String,
    color3: String,
    color4: String,
    color5: String,
    color6: String,
    color7: String,
    color8: String,
    color9: String,
    color10: String,
    color11: String,
    color12: String,
    color13: String,
    color14: String,
    color15: String,
}

fn hex_to_color(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_omarchy_detection() {
        let _available = Theme::omarchy_available();
    }

    #[test]
    fn test_dark_theme() {
        let theme = Theme::dark();
        assert_ne!(theme.background, Color::Black);
    }

    #[test]
    fn test_load_default() {
        let theme = Theme::default();
        assert_ne!(theme.background, Color::Black);
    }

    #[test]
    fn test_watch_path() {
        let theme = Theme::load("omarchy");
        let path = theme.watch_path();
        println!("Watch path: {:?}", path);
        if Theme::omarchy_available() {
            assert!(path.is_some());
        }
    }
}
