use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::color::Rgb;
use crate::error::{Error, Result};
use crate::paths::Paths;

macro_rules! vendored {
    ($($name:literal),* $(,)?) => {
        pub const VENDORED: &[(&str, &str)] = &[
            $(
                ($name, include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/schemes/",
                    $name,
                    ".yaml"
                ))),
            )*
        ];
    };
}

vendored![
    "heartbox",
    "gravas",
    "default-dark",
    "default-light",
    "tokyo-night",
    "catppuccin-mocha",
    "dracula",
    "nord",
    "gruvbox-dark-hard",
    "one-dark",
    "solarized-dark",
    "rose-pine",
    "everforest",
    "kanagawa",
    "monokai",
    "github",
    "material",
    "oceanicnext",
    "tomorrow-night",
    "ayu-dark",
];

/// A Tinted Theming Base16 (optionally Base24) scheme.
#[derive(Clone, Debug)]
pub struct Scheme {
    pub slug: String,
    pub name: String,
    pub author: String,
    pub variant: Option<String>,
    pub system: String,
    pub palette: Palette,
    pub source: SchemeSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemeSource {
    Vendored,
    Installed,
}

/// base00–base0F required; base10–base17 optional (Base24).
#[derive(Clone, Debug)]
pub struct Palette {
    pub colors: BTreeMap<String, Rgb>,
}

impl Palette {
    pub fn get(&self, key: &str) -> Result<Rgb> {
        self.colors
            .get(key)
            .copied()
            .ok_or_else(|| Error::user(format!("scheme missing {key}")))
    }

    pub fn base00(&self) -> Rgb {
        self.get("base00").unwrap_or(Rgb::new(0, 0, 0))
    }
    pub fn base01(&self) -> Rgb {
        self.get("base01").unwrap_or(self.base00())
    }
    pub fn base02(&self) -> Rgb {
        self.get("base02").unwrap_or(self.base01())
    }
    pub fn base03(&self) -> Rgb {
        self.get("base03").unwrap_or(self.base02())
    }
    pub fn base04(&self) -> Rgb {
        self.get("base04").unwrap_or(self.base05())
    }
    pub fn base05(&self) -> Rgb {
        self.get("base05").unwrap_or(Rgb::new(255, 255, 255))
    }
    pub fn base06(&self) -> Rgb {
        self.get("base06").unwrap_or(self.base05())
    }
    pub fn base07(&self) -> Rgb {
        self.get("base07").unwrap_or(self.base06())
    }
    pub fn base08(&self) -> Rgb {
        self.get("base08").unwrap_or(Rgb::new(255, 0, 0))
    }
    pub fn base09(&self) -> Rgb {
        self.get("base09").unwrap_or(self.base08())
    }
    pub fn base0a(&self) -> Rgb {
        self.get("base0A").unwrap_or(self.base09())
    }
    pub fn base0b(&self) -> Rgb {
        self.get("base0B").unwrap_or(Rgb::new(0, 255, 0))
    }
    pub fn base0c(&self) -> Rgb {
        self.get("base0C").unwrap_or(Rgb::new(0, 255, 255))
    }
    pub fn base0d(&self) -> Rgb {
        self.get("base0D").unwrap_or(Rgb::new(0, 0, 255))
    }
    pub fn base0e(&self) -> Rgb {
        self.get("base0E").unwrap_or(self.base0d())
    }
    pub fn base0f(&self) -> Rgb {
        self.get("base0F").unwrap_or(self.base09())
    }

    /// Accent: prefer base0E (magenta), fall back to base0D (blue).
    pub fn accent(&self) -> Rgb {
        self.get("base0E").unwrap_or_else(|_| self.base0d())
    }

    pub fn prefer_dark(&self) -> bool {
        self.base00().is_dark()
    }

    /// ANSI 0–15. Uses Base24 `base10`–`base17` when present; otherwise
    /// repeats the 8 chromatic colors and uses base03/base07 for brights.
    pub fn ansi16(&self) -> [Rgb; 16] {
        let brights = |i: &str, fallback: Rgb| self.get(i).unwrap_or(fallback);
        [
            self.base00(),
            self.base08(),
            self.base0b(),
            self.base0a(),
            self.base0d(),
            self.base0e(),
            self.base0c(),
            self.base05(),
            brights("base10", self.base03()),
            brights("base11", self.base08().lighten(0.15)),
            brights("base12", self.base0b().lighten(0.15)),
            brights("base13", self.base0a().lighten(0.15)),
            brights("base14", self.base0d().lighten(0.15)),
            brights("base15", self.base0e().lighten(0.15)),
            brights("base16", self.base0c().lighten(0.15)),
            brights("base17", self.base07()),
        ]
    }
}

#[derive(Deserialize)]
struct SchemeFile {
    #[serde(default)]
    system: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    variant: Option<String>,
    palette: BTreeMap<String, String>,
}

impl Scheme {
    pub fn is_dark(&self) -> bool {
        match self.variant.as_deref().map(|s| s.to_ascii_lowercase()) {
            Some(v) if v == "light" => false,
            Some(v) if v == "dark" => true,
            _ => self.palette.prefer_dark(),
        }
    }

    pub fn parse(slug: &str, yaml: &str, source: SchemeSource) -> Result<Self> {
        let file: SchemeFile =
            serde_yaml::from_str(yaml).map_err(|e| Error::parse(format!("scheme {slug}"), e))?;
        let mut colors = BTreeMap::new();
        for (k, v) in file.palette {
            let key = k.to_ascii_lowercase();
            colors.insert(key, Rgb::parse(&v)?);
        }
        for i in 0..16 {
            let key = format!("base0{i:X}");
            if !colors.contains_key(&key.to_ascii_lowercase()) {
                return Err(Error::user(format!("scheme {slug} missing {key}")));
            }
        }
        // Store with canonical mixed-case keys base00..base0F plus any extras.
        let mut canon = BTreeMap::new();
        for (k, v) in colors {
            let canon_key = if let Some(rest) = k.strip_prefix("base") {
                format!("base{}", rest.to_ascii_uppercase())
            } else {
                k
            };
            canon.insert(canon_key, v);
        }
        Ok(Self {
            slug: slug.to_string(),
            name: file.name.unwrap_or_else(|| slug.to_string()),
            author: file.author.unwrap_or_default(),
            variant: file.variant,
            system: file.system.unwrap_or_else(|| "base16".into()),
            palette: Palette { colors: canon },
            source,
        })
    }

    pub fn parse_file(path: &Path, source: SchemeSource) -> Result<Self> {
        let yaml = fs::read_to_string(path).map_err(|e| Error::io(e, path))?;
        let slug = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("scheme");
        Self::parse(slug, &yaml, source)
    }

    /// 16-color truecolor preview for `irongall preview theme`.
    pub fn ansi_preview(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("{}  ({})\n", self.name, self.slug));
        if !self.author.is_empty() {
            out.push_str(&format!("author: {}\n", self.author));
        }
        let dark = if self.palette.prefer_dark() {
            "dark"
        } else {
            "light"
        };
        out.push_str(&format!(
            "variant: {}  system: {}\n",
            self.variant.as_deref().unwrap_or(dark),
            self.system
        ));
        out.push('\n');
        let keys = [
            "base00", "base01", "base02", "base03", "base04", "base05", "base06", "base07",
            "base08", "base09", "base0A", "base0B", "base0C", "base0D", "base0E", "base0F",
        ];
        for (i, key) in keys.iter().enumerate() {
            let c = self.palette.get(key).unwrap_or(Rgb::new(0, 0, 0));
            let chip = format!(
                "\x1b[48;2;{};{};{}m          \x1b[0m",
                c.r, c.g, c.b
            );
            out.push_str(&format!("{key} {}  {}\n", c.hex(), chip));
            if i == 7 {
                out.push('\n');
            }
        }
        out.push('\n');
        out.push_str("ANSI 0–15:\n");
        for (i, c) in self.palette.ansi16().iter().enumerate() {
            out.push_str(&format!(
                "\x1b[48;2;{};{};{}m  {:2}  \x1b[0m",
                c.r, c.g, c.b, i
            ));
            if i == 7 {
                out.push('\n');
            }
        }
        out.push('\n');
        out
    }
}

/// Load vendored + user-installed schemes. Installed overrides vendored on slug clash.
pub fn load_all(paths: &Paths) -> Result<Vec<Scheme>> {
    let mut by_slug: BTreeMap<String, Scheme> = BTreeMap::new();
    for (slug, yaml) in VENDORED {
        let scheme = Scheme::parse(slug, yaml, SchemeSource::Vendored)?;
        by_slug.insert(slug.to_string(), scheme);
    }
    let dir = paths.schemes_dir();
    if dir.is_dir() {
        for entry in fs::read_dir(&dir).map_err(|e| Error::io(e, &dir))? {
            let entry = entry.map_err(|e| Error::io(e, &dir))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                || path.extension().and_then(|e| e.to_str()) == Some("yml")
            {
                let scheme = Scheme::parse_file(&path, SchemeSource::Installed)?;
                by_slug.insert(scheme.slug.clone(), scheme);
            }
        }
    }
    Ok(by_slug.into_values().collect())
}

pub fn load_named(paths: &Paths, name: &str) -> Result<Scheme> {
    let needle = name.to_ascii_lowercase();
    for s in load_all(paths)? {
        if s.slug.to_ascii_lowercase() == needle || s.name.to_ascii_lowercase() == needle {
            return Ok(s);
        }
    }
    Err(Error::user(format!(
        "scheme '{name}' not found — try `irongall theme list` or `irongall market update`"
    )))
}

pub fn search(paths: &Paths, query: &str) -> Result<Vec<Scheme>> {
    let q = query.to_ascii_lowercase();
    Ok(load_all(paths)?
        .into_iter()
        .filter(|s| {
            s.slug.to_ascii_lowercase().contains(&q)
                || s.name.to_ascii_lowercase().contains(&q)
                || s.author.to_ascii_lowercase().contains(&q)
        })
        .collect())
}

pub fn vendored_path_list() -> Vec<PathBuf> {
    VENDORED
        .iter()
        .map(|(n, _)| PathBuf::from(format!("schemes/{n}.yaml")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_every_vendored_scheme() {
        for (slug, yaml) in VENDORED {
            let s = Scheme::parse(slug, yaml, SchemeSource::Vendored)
                .unwrap_or_else(|e| panic!("{slug}: {e}"));
            assert_eq!(s.slug, *slug);
            assert!(s.palette.get("base00").is_ok());
            assert!(s.palette.get("base0F").is_ok());
        }
    }

    #[test]
    fn heartbox_colors() {
        let s = Scheme::parse("heartbox", include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/schemes/heartbox.yaml"
        )), SchemeSource::Vendored).unwrap();
        assert_eq!(s.palette.base00().hex(), "#0A1528");
        assert_eq!(s.palette.base08().hex(), "#E03818");
        assert!(s.palette.prefer_dark());
    }

    #[test]
    fn dark_light_inference() {
        let dark = Scheme::parse("default-dark", VENDORED.iter().find(|(n,_)| *n=="default-dark").unwrap().1, SchemeSource::Vendored).unwrap();
        let light = Scheme::parse("default-light", VENDORED.iter().find(|(n,_)| *n=="default-light").unwrap().1, SchemeSource::Vendored).unwrap();
        assert!(dark.palette.prefer_dark());
        assert!(!light.palette.prefer_dark());
    }
}
