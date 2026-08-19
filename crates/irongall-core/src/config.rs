use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::font;
use crate::paths::{write_string, Paths};

/// On-disk `~/.config/irongall/config.toml`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub theme: ThemeSection,
    #[serde(default)]
    pub font: FontSection,
    #[serde(default)]
    pub market: MarketSection,
    /// Per-program tweaks. Omitted keys inherit the global theme/font/size.
    #[serde(default)]
    pub apps: BTreeMap<String, AppOverride>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThemeSection {
    #[serde(default = "default_theme")]
    pub name: String,
    #[serde(default)]
    pub variant: Variant,
}

fn default_theme() -> String {
    "heartbox".into()
}

impl Default for ThemeSection {
    fn default() -> Self {
        Self {
            name: default_theme(),
            variant: Variant::Dark,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Variant {
    #[default]
    Dark,
    Light,
}

impl Variant {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FontSection {
    #[serde(default = "default_family")]
    pub family: String,
    #[serde(default)]
    pub sans: Option<String>,
    #[serde(default)]
    pub serif: Option<String>,
    #[serde(default)]
    pub mono: Option<String>,
    #[serde(default = "default_size")]
    pub size: f32,
    #[serde(default)]
    pub terminal_size: Option<f32>,
    #[serde(default)]
    pub ui_size: Option<f32>,
}

fn default_family() -> String {
    "monospace".into()
}

fn default_size() -> f32 {
    11.0
}

impl Default for FontSection {
    fn default() -> Self {
        Self {
            family: default_family(),
            sans: None,
            serif: None,
            mono: None,
            size: default_size(),
            terminal_size: None,
            ui_size: None,
        }
    }
}

impl FontSection {
    pub fn sans(&self) -> &str {
        self.sans.as_deref().unwrap_or(&self.family)
    }
    pub fn serif(&self) -> &str {
        self.serif.as_deref().unwrap_or(&self.family)
    }
    pub fn mono(&self) -> &str {
        self.mono.as_deref().unwrap_or(&self.family)
    }
    pub fn ui_size(&self) -> f32 {
        self.ui_size.unwrap_or(self.size)
    }
    pub fn terminal_size(&self) -> f32 {
        self.terminal_size.unwrap_or(self.size)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MarketSection {
    #[serde(default)]
    pub index_url: Option<String>,
}

/// Per-app override table (`[apps.kitty]`).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct AppOverride {
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub font: Option<String>,
    #[serde(default)]
    pub size: Option<f32>,
    /// `true` (default) inherit unset keys; `false` hold last applied values.
    #[serde(default)]
    pub follow: Option<bool>,
    #[serde(default)]
    pub skip: Option<bool>,
}

impl AppOverride {
    pub fn is_skip(&self) -> bool {
        self.skip.unwrap_or(false)
    }

    pub fn is_hold(&self) -> bool {
        self.follow == Some(false)
    }

    pub fn is_empty(&self) -> bool {
        self.theme.is_none()
            && self.font.is_none()
            && self.size.is_none()
            && self.follow.is_none()
            && self.skip.is_none()
    }

    pub fn has_tweak(&self) -> bool {
        self.theme.is_some() || self.font.is_some() || self.size.is_some()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: ThemeSection::default(),
            font: FontSection::default(),
            market: MarketSection::default(),
            apps: BTreeMap::new(),
        }
    }
}

impl Config {
    pub fn load(paths: &Paths) -> Result<Self> {
        let file = paths.config_file();
        if !file.exists() {
            return Ok(Self::default_for(paths));
        }
        let raw = fs::read_to_string(&file).map_err(|e| Error::io(e, &file))?;
        toml::from_str(&raw).map_err(|e| Error::parse(file.display().to_string(), e))
    }

    /// Default config, filling family from `fc-match` when possible.
    pub fn default_for(paths: &Paths) -> Self {
        let mut cfg = Self::default();
        if let Ok(fam) = font::fc_match_family("monospace") {
            if !fam.is_empty() {
                cfg.font.family = fam;
            }
        }
        let _ = paths;
        cfg
    }

    pub fn save(&self, paths: &Paths) -> Result<()> {
        paths.ensure_dirs()?;
        let mut out = toml::to_string_pretty(self)
            .map_err(|e| Error::parse("config.toml serialize", e))?;
        if !out.ends_with('\n') {
            out.push('\n');
        }
        write_string(&paths.config_file(), &out)
    }

    pub fn app(&self, id: &str) -> Option<&AppOverride> {
        self.apps.get(id)
    }

    pub fn app_mut(&mut self, id: &str) -> &mut AppOverride {
        self.apps.entry(id.to_string()).or_default()
    }

    pub fn reset_app(&mut self, id: &str) {
        self.apps.remove(id);
    }
}

/// Format a point size the way GTK `gtk-font-name` expects:
/// `11` not `11.0`, `11.5` kept.
pub fn format_pt(size: f32) -> String {
    let tenths = (size * 10.0).round() as i32;
    if tenths % 10 == 0 {
        format!("{}", tenths / 10)
    } else {
        format!("{:.1}", tenths as f32 / 10.0)
    }
}

/// `Berkeley Mono 11` / `Berkeley Mono 11.5`
pub fn gtk_font_name(family: &str, size: f32) -> String {
    format!("{family} {}", format_pt(size))
}

pub fn parse_pt(s: &str) -> Result<f32> {
    let v: f32 = s
        .trim()
        .parse()
        .map_err(|_| Error::user(format!("invalid size (points): {s}")))?;
    if !(8.0..=24.0).contains(&v) {
        return Err(Error::user(format!(
            "size {v} is outside the 8–24 pt range"
        )));
    }
    Ok(v)
}

pub fn load_from(path: &Path) -> Result<Config> {
    let raw = fs::read_to_string(path).map_err(|e| Error::io(e, path))?;
    toml::from_str(&raw).map_err(|e| Error::parse(path.display().to_string(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gtk_size_format() {
        assert_eq!(gtk_font_name("Berkeley Mono", 11.0), "Berkeley Mono 11");
        assert_eq!(gtk_font_name("Berkeley Mono", 11.5), "Berkeley Mono 11.5");
        assert_eq!(gtk_font_name("Inter", 10.0), "Inter 10");
    }

    #[test]
    fn reset_removes_override_table() {
        let mut cfg = Config::default();
        cfg.app_mut("kitty").size = Some(13.0);
        assert!(cfg.apps.contains_key("kitty"));
        cfg.reset_app("kitty");
        assert!(!cfg.apps.contains_key("kitty"));
    }
}
