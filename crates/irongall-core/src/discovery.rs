use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::catalog::{self, CatalogEntry, Kind};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::paths::{write_string, Paths};

/// How a catalog entry relates to this machine and the current config.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppState {
    Global,
    Tweak,
    Hold,
    Skip,
    Missing,
    NoWriter,
}

impl AppState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Tweak => "tweak",
            Self::Hold => "hold",
            Self::Skip => "skip",
            Self::Missing => "missing",
            Self::NoWriter => "no-writer",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Discovered {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub present: bool,
    pub has_writer: bool,
    pub matched: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscoveryCache {
    pub timestamp: u64,
    pub apps: Vec<Discovered>,
}

/// Live scan of the catalog against PATH / configs / desktop files.
/// Pacman is skipped: a 60ms spawn for a signal we already get from PATH
/// and `.desktop` files.
pub fn scan(paths: &Paths) -> Result<Vec<Discovered>> {
    let isolated = std::env::var_os("IRONGALL_TEST_ROOT").is_some();
    let path_dirs = path_dirs();
    let desktop_dirs = desktop_dirs(paths, isolated);
    let mut apps = Vec::new();
    for entry in catalog::all() {
        let (present, matched) = is_present(entry, paths, &path_dirs, &desktop_dirs);
        apps.push(Discovered {
            id: entry.id.to_string(),
            name: entry.name.to_string(),
            kind: entry.kind.as_str().to_string(),
            present,
            has_writer: entry.has_writer,
            matched,
        });
    }
    Ok(apps)
}

pub fn scan_and_cache(paths: &Paths) -> Result<Vec<Discovered>> {
    let apps = scan(paths)?;
    let cache = DiscoveryCache {
        timestamp: now_secs(),
        apps: apps.clone(),
    };
    paths.ensure_dirs()?;
    if let Some(parent) = paths.discovery_cache().parent() {
        fs::create_dir_all(parent).ok();
    }
    let json = serde_json::to_string_pretty(&cache)
        .map_err(|e| Error::parse("discovery.json", e))?;
    write_string(&paths.discovery_cache(), &json)?;
    Ok(apps)
}

pub fn load_cache(paths: &Paths) -> Option<DiscoveryCache> {
    let raw = fs::read_to_string(paths.discovery_cache()).ok()?;
    serde_json::from_str(&raw).ok()
}

fn is_present(
    entry: &CatalogEntry,
    paths: &Paths,
    path_dirs: &[PathBuf],
    desktop_dirs: &[PathBuf],
) -> (bool, Vec<String>) {
    let mut matched = Vec::new();

    for bin in entry.binaries {
        if which(bin, path_dirs).is_some() {
            matched.push(format!("binary:{bin}"));
        }
    }
    for glob in entry.config_globs {
        for hit in expand_glob(paths, glob) {
            matched.push(format!("config:{}", hit.display()));
        }
    }
    for desk in entry.desktop_ids {
        for dir in desktop_dirs {
            let p = dir.join(desk);
            if p.is_file() {
                matched.push(format!("desktop:{}", p.display()));
            }
        }
    }

    // System adapters that write user files should show as present on a
    // normal Linux box even without a pre-existing config file.
    if matched.is_empty() && is_always_present(entry) && std::env::var_os("IRONGALL_TEST_ROOT").is_none() {
        matched.push("platform:linux".into());
    }

    (!matched.is_empty(), matched)
}

fn is_always_present(entry: &CatalogEntry) -> bool {
    cfg!(target_os = "linux")
        && matches!(
            entry.id,
            "fontconfig" | "gtk3" | "gtk4" | "gsettings"
        )
}

fn expand_glob(paths: &Paths, pattern: &str) -> Vec<PathBuf> {
    let expanded = paths.expand(pattern);
    let s = expanded.to_string_lossy();
    if s.contains('*') {
        if let Some(parent) = expanded.parent() {
            let name = expanded.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if let Ok(rd) = fs::read_dir(parent) {
                return rd
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| glob_match(name, &p.file_name().unwrap_or_default().to_string_lossy()))
                    .collect();
            }
        }
        return Vec::new();
    }
    if expanded.exists() {
        vec![expanded]
    } else {
        Vec::new()
    }
}

fn glob_match(pat: &str, name: &str) -> bool {
    if let Some((pre, post)) = pat.split_once('*') {
        name.starts_with(pre) && name.ends_with(post)
    } else {
        pat == name
    }
}

pub fn which(bin: &str, path_dirs: &[PathBuf]) -> Option<PathBuf> {
    for dir in path_dirs {
        let p = dir.join(bin);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

pub fn path_dirs() -> Vec<PathBuf> {
    env::var_os("PATH")
        .map(|p| env::split_paths(&p).collect())
        .unwrap_or_default()
}

fn desktop_dirs(paths: &Paths, isolated: bool) -> Vec<PathBuf> {
    let mut dirs = vec![paths.data_home.join("applications")];
    if !isolated {
        dirs.push(PathBuf::from("/usr/share/applications"));
        dirs.push(PathBuf::from("/usr/local/share/applications"));
    }
    dirs
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Effective theme / font / size for one program.
#[derive(Clone, Debug, PartialEq)]
pub struct Effective {
    pub theme: String,
    pub font: String,
    pub size: f32,
    pub state: AppState,
    pub skip: bool,
}

impl Effective {
    pub fn dash_theme(&self) -> String {
        if self.skip || matches!(self.state, AppState::Missing | AppState::NoWriter) {
            "—".into()
        } else {
            self.theme.clone()
        }
    }
}

pub fn effective(cfg: &Config, entry: &CatalogEntry, present: bool) -> Effective {
    let ov = cfg.app(entry.id);
    let skip = ov.map(|o| o.is_skip()).unwrap_or(false);

    if !present {
        return Effective {
            theme: cfg.theme.name.clone(),
            font: cfg.font.family.clone(),
            size: default_size(cfg, entry.kind),
            state: AppState::Missing,
            skip: false,
        };
    }
    if skip {
        return Effective {
            theme: cfg.theme.name.clone(),
            font: cfg.font.family.clone(),
            size: default_size(cfg, entry.kind),
            state: AppState::Skip,
            skip: true,
        };
    }
    if !entry.has_writer {
        return Effective {
            theme: cfg.theme.name.clone(),
            font: cfg.font.family.clone(),
            size: default_size(cfg, entry.kind),
            state: AppState::NoWriter,
            skip: false,
        };
    }

    let mut theme = cfg.theme.name.clone();
    let mut font = match entry.kind {
        Kind::Terminal | Kind::Editor | Kind::Cli => cfg.font.mono().to_string(),
        _ => cfg.font.sans().to_string(),
    };
    // Default: one family everywhere unless sans/serif/mono overrides exist.
    // For terminals we still start from the global family (or mono override).
    if cfg.font.mono.is_none() && cfg.font.sans.is_none() {
        font = cfg.font.family.clone();
    }
    let mut size = default_size(cfg, entry.kind);

    if let Some(o) = ov {
        if let Some(t) = &o.theme {
            theme = t.clone();
        }
        if let Some(f) = &o.font {
            font = f.clone();
        }
        if let Some(s) = o.size {
            size = s;
        }
    }

    let state = if ov.map(|o| o.is_hold()).unwrap_or(false) {
        AppState::Hold
    } else if ov.map(|o| o.has_tweak()).unwrap_or(false) {
        AppState::Tweak
    } else {
        AppState::Global
    };

    Effective {
        theme,
        font,
        size,
        state,
        skip: false,
    }
}

fn default_size(cfg: &Config, kind: Kind) -> f32 {
    if kind.uses_terminal_size() {
        cfg.font.terminal_size()
    } else if kind.uses_ui_size() {
        cfg.font.ui_size()
    } else {
        cfg.font.size
    }
}

/// Row used by `irongall apps` / TUI.
#[derive(Clone, Debug, Serialize)]
pub struct AppRow {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub state: String,
    pub theme: String,
    pub font: String,
    pub size: Option<f32>,
    pub present: bool,
    pub has_writer: bool,
    pub matched: Vec<String>,
}

pub fn rows(paths: &Paths, cfg: &Config, include_missing: bool) -> Result<Vec<AppRow>> {
    let discovered = scan(paths)?;
    let mut out = Vec::new();
    for d in discovered {
        let entry = match CatalogEntry::get(&d.id) {
            Some(e) => e,
            None => continue,
        };
        if !include_missing && !d.present {
            continue;
        }
        let eff = effective(cfg, entry, d.present);
        let (theme, font, size) = match eff.state {
            AppState::Missing | AppState::Skip | AppState::NoWriter => {
                ("—".into(), "—".into(), None)
            }
            _ => (eff.theme, eff.font, Some(eff.size)),
        };
        out.push(AppRow {
            id: d.id,
            name: d.name,
            kind: d.kind,
            state: eff.state.as_str().to_string(),
            theme,
            font,
            size,
            present: d.present,
            has_writer: d.has_writer,
            matched: d.matched,
        });
    }
    Ok(out)
}

pub fn format_table(rows: &[AppRow]) -> String {
    let headers = ("id", "name", "kind", "state", "theme", "font", "size");
    let mut widths = [
        headers.0.len(),
        headers.1.len(),
        headers.2.len(),
        headers.3.len(),
        headers.4.len(),
        headers.5.len(),
        headers.6.len(),
    ];
    let rendered: Vec<(String, String, String, String, String, String, String)> = rows
        .iter()
        .map(|r| {
            (
                r.id.clone(),
                r.name.clone(),
                r.kind.clone(),
                r.state.clone(),
                r.theme.clone(),
                r.font.clone(),
                r.size
                    .map(crate::config::format_pt)
                    .unwrap_or_else(|| "—".into()),
            )
        })
        .collect();
    for r in &rendered {
        widths[0] = widths[0].max(r.0.len());
        widths[1] = widths[1].max(r.1.len());
        widths[2] = widths[2].max(r.2.len());
        widths[3] = widths[3].max(r.3.len());
        widths[4] = widths[4].max(r.4.len());
        widths[5] = widths[5].max(r.5.len());
        widths[6] = widths[6].max(r.6.len());
    }
    let mut out = String::new();
    out.push_str(&format!(
        "{:<w0$}  {:<w1$}  {:<w2$}  {:<w3$}  {:<w4$}  {:<w5$}  {:>w6$}\n",
        headers.0,
        headers.1,
        headers.2,
        headers.3,
        headers.4,
        headers.5,
        headers.6,
        w0 = widths[0],
        w1 = widths[1],
        w2 = widths[2],
        w3 = widths[3],
        w4 = widths[4],
        w5 = widths[5],
        w6 = widths[6],
    ));
    for r in &rendered {
        out.push_str(&format!(
            "{:<w0$}  {:<w1$}  {:<w2$}  {:<w3$}  {:<w4$}  {:<w5$}  {:>w6$}\n",
            r.0,
            r.1,
            r.2,
            r.3,
            r.4,
            r.5,
            r.6,
            w0 = widths[0],
            w1 = widths[1],
            w2 = widths[2],
            w3 = widths[3],
            w4 = widths[4],
            w5 = widths[5],
            w6 = widths[6],
        ));
    }
    out
}

pub fn counts(rows: &[AppRow]) -> Counts {
    let mut c = Counts::default();
    for r in rows {
        match r.state.as_str() {
            "global" => c.global += 1,
            "tweak" => c.tweak += 1,
            "hold" => c.hold += 1,
            "skip" => c.skip += 1,
            "missing" => c.missing += 1,
            "no-writer" => c.no_writer += 1,
            _ => {}
        }
    }
    c
}

#[derive(Clone, Debug, Default)]
pub struct Counts {
    pub global: usize,
    pub tweak: usize,
    pub hold: usize,
    pub skip: usize,
    pub missing: usize,
    pub no_writer: usize,
}

impl Counts {
    pub fn one_line(&self) -> String {
        format!(
            "{} global · {} tweaked · {} skipped · {} missing",
            self.global, self.tweak, self.skip, self.missing
        )
    }
}

/// Test helper: scan using an explicit PATH and home, without touching the real user.
pub fn scan_with_env(paths: &Paths, extra_path: &[PathBuf]) -> Result<Vec<Discovered>> {
    let _ = extra_path;
    // Tests set PATH / HOME / XDG_* and IRONGALL_TEST_ROOT.
    scan(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppOverride;

    #[test]
    fn effective_kitty_size_override() {
        let mut cfg = Config::default();
        cfg.font.size = 11.0;
        cfg.font.family = "Berkeley Mono".into();
        cfg.apps.insert(
            "kitty".into(),
            AppOverride {
                size: Some(13.0),
                ..Default::default()
            },
        );
        let kitty = CatalogEntry::get("kitty").unwrap();
        let gtk = CatalogEntry::get("gtk3").unwrap();
        let k = effective(&cfg, kitty, true);
        let g = effective(&cfg, gtk, true);
        assert_eq!(k.size, 13.0);
        assert_eq!(g.size, 11.0);
        assert_eq!(k.state, AppState::Tweak);
        assert_eq!(g.state, AppState::Global);
    }

    #[test]
    fn skip_state() {
        let mut cfg = Config::default();
        cfg.apps.insert(
            "neovim".into(),
            AppOverride {
                skip: Some(true),
                ..Default::default()
            },
        );
        let nvim = CatalogEntry::get("neovim").unwrap();
        let e = effective(&cfg, nvim, true);
        assert!(e.skip);
        assert_eq!(e.state, AppState::Skip);
    }
}
