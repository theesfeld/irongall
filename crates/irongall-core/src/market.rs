use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::font;
use crate::paths::{write_string, Paths};

pub const BUNDLED_INDEX: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/market/index.json"
));

pub const DEFAULT_INDEX_URL: &str =
    "https://raw.githubusercontent.com/theesfeld/irongall/main/market/index.json";

/// Tinted Theming's Base16 scheme tree — the real color-scheme marketplace.
pub const TINTED_SCHEMES_API: &str =
    "https://api.github.com/repos/tinted-theming/schemes/contents/base16?ref=spec-0.11";
pub const TINTED_SCHEME_RAW: &str =
    "https://raw.githubusercontent.com/tinted-theming/schemes/spec-0.11/base16";
/// One zipball so we can read each YAML's palette / author / license field.
pub const TINTED_ZIP: &str =
    "https://github.com/tinted-theming/schemes/archive/refs/heads/spec-0.11.zip";

/// What we print when a scheme YAML has no license of its own.
/// The collection repo is MIT; individual original palettes are not re-audited.
pub const TINTED_COLLECTION_LICENSE: &str = "MIT (tinted-theming collection)";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Index {
    pub version: u32,
    #[serde(default)]
    pub schemes: Vec<SchemeEntry>,
    #[serde(default)]
    pub fonts: Vec<FontEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchemeEntry {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub preview: Vec<String>,
    #[serde(default)]
    pub author: Option<String>,
    /// `irongall` (vendored / this repo) or `tinted-theming` (upstream Base16).
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub variant: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FontEntry {
    pub family: String,
    pub license: String,
    pub source: String,
    #[serde(default)]
    pub install: String,
    #[serde(default)]
    pub notes: Option<String>,
    /// Direct zip URL when known (avoids GitHub API for offline-friendly docs).
    #[serde(default)]
    pub zip_url: Option<String>,
}

const LIBRE: &[&str] = &[
    "OFL", "OFL-1.1", "SIL", "SIL OFL", "UFL", "UBUNTU", "UBUNTU-FONT-LICENCE",
    "APACHE", "APACHE-2.0", "APACHE 2.0", "MIT", "BSD", "BSD-2-CLAUSE",
    "BSD-3-CLAUSE", "ISC", "GPL", "GPL-2.0", "GPL-3.0", "LGPL", "CC0",
    "CC-BY", "CC-BY-4.0", "CC0-1.0",
];

pub fn is_libre(license: &str) -> bool {
    let n = license.trim().to_ascii_uppercase().replace('_', "-");
    LIBRE.iter().any(|l| n == *l || n.contains(*l))
}

pub fn load_index(paths: &Paths) -> Result<Index> {
    let bundled = bundled_index()?;
    let cached_path = paths.market_index();
    if cached_path.is_file() {
        if let Ok(raw) = fs::read_to_string(&cached_path) {
            if let Ok(idx) = serde_json::from_str::<Index>(&raw) {
                // v1 indexes stamped every scheme "MIT" with empty previews.
                if idx.version >= bundled.version {
                    return Ok(idx);
                }
            }
        }
    }
    Ok(bundled)
}

pub fn bundled_index() -> Result<Index> {
    serde_json::from_str(BUNDLED_INDEX).map_err(|e| Error::parse("bundled market index", e))
}

pub fn update(paths: &Paths, url: Option<&str>) -> Result<Index> {
    let mut index = if let Some(url) = url {
        let body = http_get(url)?;
        serde_json::from_str(&body).map_err(|e| Error::parse("market index", e))?
    } else {
        refresh_from_upstream()?
    };
    // Always keep bundled OFL fonts even if the remote index omitted them.
    let bundled = bundled_index()?;
    if index.fonts.is_empty() {
        index.fonts = bundled.fonts;
    }
    if let Some(parent) = paths.market_index().parent() {
        fs::create_dir_all(parent).map_err(|e| Error::io(e, parent))?;
    }
    let raw = serde_json::to_string_pretty(&index).map_err(|e| Error::parse("market index", e))?;
    write_string(&paths.market_index(), &raw)?;
    Ok(index)
}

/// Pull the live Base16 zip from tinted-theming/schemes and merge.
fn refresh_from_upstream() -> Result<Index> {
    let mut index = bundled_index()?;
    let bytes = http_get_bytes(TINTED_ZIP)?;
    let tinted = parse_tinted_zip(&bytes)?;
    index
        .schemes
        .retain(|s| s.source.as_deref() == Some("irongall"));
    let mut have: std::collections::BTreeSet<String> = index
        .schemes
        .iter()
        .map(|s| s.name.to_ascii_lowercase())
        .collect();
    for s in tinted {
        if have.insert(s.name.to_ascii_lowercase()) {
            index.schemes.push(s);
        }
    }
    Ok(index)
}

/// Read palette + author + optional YAML license from a Tinted Theming zipball.
pub fn parse_tinted_zip(bytes: &[u8]) -> Result<Vec<SchemeEntry>> {
    let reader = Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(reader)
        .map_err(|e| Error::user(format!("tinted-theming zip: {e}")))?;
    let mut out = Vec::new();
    for i in 0..zip.len() {
        let mut file = zip
            .by_index(i)
            .map_err(|e| Error::user(format!("tinted-theming zip: {e}")))?;
        let name = file.name().to_string();
        if !name.contains("/base16/") || !name.ends_with(".yaml") {
            continue;
        }
        let slug = std::path::Path::new(&name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if slug.is_empty() {
            continue;
        }
        let mut yaml = String::new();
        file.read_to_string(&mut yaml)
            .map_err(|e| Error::user(format!("tinted-theming zip read: {e}")))?;
        if let Ok(entry) = scheme_entry_from_yaml(&slug, &yaml) {
            out.push(entry);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

fn scheme_entry_from_yaml(slug: &str, yaml: &str) -> Result<SchemeEntry> {
    #[derive(Deserialize)]
    struct Y {
        #[serde(default)]
        system: Option<String>,
        #[serde(default)]
        author: Option<String>,
        #[serde(default)]
        variant: Option<String>,
        #[serde(default)]
        license: Option<String>,
        #[serde(default)]
        palette: std::collections::BTreeMap<String, String>,
    }
    let y: Y = serde_yaml::from_str(yaml).map_err(|e| Error::parse(format!("scheme {slug}"), e))?;
    let mut preview = Vec::new();
    for key in [
        "base00", "base01", "base02", "base03", "base04", "base05", "base06", "base07",
        "base08", "base09", "base0A", "base0B", "base0C", "base0D", "base0E", "base0F",
    ] {
        let v = y
            .palette
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str());
        if let Some(v) = v {
            let hex = v.trim().split_whitespace().next().unwrap_or(v).trim();
            let hex = hex.trim_start_matches('#');
            if hex.len() >= 6 {
                preview.push(format!("#{}", hex[..6].to_ascii_uppercase()));
            }
        }
    }
    let license = y
        .license
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| TINTED_COLLECTION_LICENSE.to_string());
    Ok(SchemeEntry {
        name: slug.to_string(),
        url: format!("{TINTED_SCHEME_RAW}/{slug}.yaml"),
        license,
        system: y.system.unwrap_or_else(|| "base16".into()),
        preview,
        author: y.author.filter(|s| !s.is_empty()),
        source: Some("tinted-theming".into()),
        variant: y.variant.filter(|s| !s.is_empty()),
    })
}

pub fn install_scheme(paths: &Paths, name: &str) -> Result<PathBuf> {
    let index = load_index(paths)?;
    let entry = index
        .schemes
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| Error::user(format!("scheme '{name}' is not in the market index")))?;
    let yaml = http_get(&entry.url)?;
    // Validate it parses.
    crate::scheme::Scheme::parse(&entry.name, &yaml, crate::scheme::SchemeSource::Installed)?;
    let dest = paths.schemes_dir().join(format!("{}.yaml", entry.name));
    write_string(&dest, &yaml)?;
    Ok(dest)
}

pub fn install_font(paths: &Paths, family: &str) -> Result<PathBuf> {
    let index = load_index(paths)?;
    let entry = index
        .fonts
        .iter()
        .find(|f| f.family.eq_ignore_ascii_case(family))
        .ok_or_else(|| Error::user(format!("font '{family}' is not in the market index")))?;
    if !is_libre(&entry.license) {
        return Err(Error::user(format!(
            "refusing to install '{}': license '{}' is not a known libre license (OFL/SIL/Ubuntu/Apache/MIT/…)",
            entry.family, entry.license
        )));
    }
    let zip_url = match entry.zip_url.as_deref() {
        Some(u) => u.to_string(),
        None => resolve_zip_url(entry)?,
    };
    let bytes = http_get_bytes(&zip_url)?;
    let dest = paths.fonts_dir().join(sanitize_family(&entry.family));
    extract_fonts(&bytes, &dest)?;
    font::fc_cache()?;
    font::invalidate_cache();
    Ok(dest)
}

fn sanitize_family(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

fn resolve_zip_url(entry: &FontEntry) -> Result<String> {
    if entry.install == "github-release-zip" || entry.source.contains("github.com") {
        if let Some(api) = github_latest_api(&entry.source) {
            let json = http_get(&api)?;
            let v: serde_json::Value =
                serde_json::from_str(&json).map_err(|e| Error::parse("github release", e))?;
            if let Some(assets) = v.get("assets").and_then(|a| a.as_array()) {
                for a in assets {
                    let name = a.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let url = a
                        .get("browser_download_url")
                        .and_then(|u| u.as_str())
                        .unwrap_or("");
                    if name.ends_with(".zip") && !name.to_ascii_lowercase().contains("source") {
                        return Ok(url.to_string());
                    }
                }
            }
            if let Some(zip) = v.get("zipball_url").and_then(|u| u.as_str()) {
                return Ok(zip.to_string());
            }
        }
    }
    Err(Error::user(format!(
        "cannot resolve a zip download for '{}' (source: {})",
        entry.family, entry.source
    )))
}

fn github_latest_api(source: &str) -> Option<String> {
    // https://github.com/JetBrains/JetBrainsMono/releases/latest
    let url = source.trim_end_matches('/');
    let rest = url.strip_prefix("https://github.com/")?;
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() >= 2 {
        return Some(format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            parts[0], parts[1]
        ));
    }
    None
}

fn extract_fonts(bytes: &[u8], dest: &Path) -> Result<usize> {
    fs::create_dir_all(dest).map_err(|e| Error::io(e, dest))?;
    let reader = Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(reader)
        .map_err(|e| Error::user(format!("invalid zip: {e}")))?;
    let mut n = 0usize;
    for i in 0..zip.len() {
        let mut file = zip
            .by_index(i)
            .map_err(|e| Error::user(format!("zip read: {e}")))?;
        if file.is_dir() {
            continue;
        }
        let name = file.name().to_string();
        let ext = Path::new(&name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "ttf" | "otf" | "ttc" | "otc") {
            continue;
        }
        let fname = Path::new(&name)
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("font-{i}.{ext}")));
        let out_path = dest.join(fname);
        let mut out = fs::File::create(&out_path).map_err(|e| Error::io(e, &out_path))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .map_err(|e| Error::user(format!("zip extract: {e}")))?;
        out.write_all(&buf).map_err(|e| Error::io(e, &out_path))?;
        n += 1;
    }
    if n == 0 {
        return Err(Error::user("zip contained no TTF/OTF files"));
    }
    Ok(n)
}

fn http_get(url: &str) -> Result<String> {
    let bytes = http_get_bytes(url)?;
    String::from_utf8(bytes).map_err(|e| Error::Network(format!("response is not utf-8: {e}")))
}

fn http_get_bytes(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("irongall/0.1")
        .build()
        .map_err(|e| Error::Network(e.to_string()))?;
    let resp = client
        .get(url)
        .send()
        .map_err(|e| Error::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(Error::Network(format!(
            "GET {url} → {}",
            resp.status()
        )));
    }
    resp.bytes()
        .map(|b| b.to_vec())
        .map_err(|e| Error::Network(e.to_string()))
}

pub fn search_schemes(index: &Index, q: &str) -> Vec<SchemeEntry> {
    let q = q.to_ascii_lowercase();
    index
        .schemes
        .iter()
        .filter(|s| s.name.to_ascii_lowercase().contains(&q))
        .cloned()
        .collect()
}

pub fn search_fonts(index: &Index, q: &str) -> Vec<FontEntry> {
    let q = q.to_ascii_lowercase();
    index
        .fonts
        .iter()
        .filter(|f| f.family.to_ascii_lowercase().contains(&q))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_index_parses() {
        let idx = bundled_index().unwrap();
        assert!(idx.schemes.iter().any(|s| s.name == "heartbox"));
        assert!(idx.fonts.iter().any(|f| f.family == "JetBrains Mono"));
        // Tinted schemes must not be blindly stamped "MIT" — collection license.
        let tinted: Vec<_> = idx
            .schemes
            .iter()
            .filter(|s| s.source.as_deref() == Some("tinted-theming"))
            .collect();
        assert!(tinted.len() > 100);
        assert!(tinted.iter().all(|s| s.license != "MIT"));
        assert!(tinted.iter().all(|s| s.preview.len() == 16));
        assert!(idx.fonts.iter().all(|f| is_libre(&f.license)));
        assert!(idx.fonts.iter().any(|f| f.license == "OFL-1.1"));
    }

    #[test]
    fn libre_licenses() {
        assert!(is_libre("OFL-1.1"));
        assert!(is_libre("MIT"));
        assert!(is_libre("Apache-2.0"));
        assert!(!is_libre("proprietary"));
        assert!(!is_libre("Commercial"));
    }
}
