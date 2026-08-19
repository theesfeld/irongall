use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Error, Result};
use crate::paths::Paths;

/// A font family as seen by fontconfig.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFamily {
    pub family: String,
    pub styles: Vec<String>,
    pub group: FontGroup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontGroup {
    Main,
    Other,
}

/// Run `fc-list` and return unique families. Emoji / CJK / last-resort
/// go in `Other`; everything else is `Main`.
pub fn list_installed() -> Result<Vec<FontFamily>> {
    let output = Command::new("fc-list")
        .args([":", "family", "style"])
        .output();
    let output = match output {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            return Err(Error::Command {
                cmd: "fc-list".into(),
                status: Some(o.status),
                detail: String::from_utf8_lossy(&o.stderr).into(),
            });
        }
        Err(e) => {
            return Err(Error::Command {
                cmd: "fc-list".into(),
                status: None,
                detail: e.to_string(),
            });
        }
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut map: std::collections::BTreeMap<String, BTreeSet<String>> =
        std::collections::BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // `Family,Alias:style=Regular,Bold` or `Family:style=Regular`
        let (fam_part, style_part) = line.split_once(':').unwrap_or((line, ""));
        let family = fam_part
            .split(',')
            .next()
            .unwrap_or(fam_part)
            .trim()
            .to_string();
        if family.is_empty() {
            continue;
        }
        let styles = style_part
            .strip_prefix("style=")
            .unwrap_or(style_part)
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let entry = map.entry(family).or_default();
        for s in styles {
            entry.insert(s);
        }
    }
    let mut out: Vec<FontFamily> = map
        .into_iter()
        .map(|(family, styles)| {
            let group = if is_other(&family) {
                FontGroup::Other
            } else {
                FontGroup::Main
            };
            FontFamily {
                family,
                styles: styles.into_iter().collect(),
                group,
            }
        })
        .collect();
    out.sort_by(|a, b| a.family.to_ascii_lowercase().cmp(&b.family.to_ascii_lowercase()));
    Ok(out)
}

pub fn is_other(family: &str) -> bool {
    let l = family.to_ascii_lowercase();
    l.contains("lastresort")
        || l.contains("last resort")
        || l.contains("emoji")
        || l.contains("color emoji")
        || l.contains("cjk")
        || l.contains("noto sans jp")
        || l.contains("noto sans kr")
        || l.contains("noto sans sc")
        || l.contains("noto sans tc")
        || l.contains("noto serif cjk")
        || l.contains("source han")
        || l.contains("wenquanyi")
        || l.contains("nerd font")
        || l.contains("symbols only")
        || l.contains("font awesome")
}

pub fn fc_match_family(query: &str) -> Result<String> {
    let output = Command::new("fc-match")
        .args(["-f", "%{family}", query])
        .output()
        .map_err(|e| Error::Command {
            cmd: format!("fc-match {query}"),
            status: None,
            detail: e.to_string(),
        })?;
    if !output.status.success() {
        return Err(Error::Command {
            cmd: format!("fc-match {query}"),
            status: Some(output.status),
            detail: String::from_utf8_lossy(&output.stderr).into(),
        });
    }
    let fam = String::from_utf8_lossy(&output.stdout);
    let fam = fam.split(',').next().unwrap_or(&fam).trim();
    Ok(fam.to_string())
}

pub fn fc_cache() -> Result<()> {
    let status = Command::new("fc-cache")
        .arg("-f")
        .status()
        .map_err(|e| Error::Command {
            cmd: "fc-cache -f".into(),
            status: None,
            detail: e.to_string(),
        })?;
    if !status.success() {
        return Err(Error::Command {
            cmd: "fc-cache -f".into(),
            status: Some(status),
            detail: "fc-cache failed".into(),
        });
    }
    Ok(())
}

pub fn family_installed(family: &str) -> bool {
    list_installed()
        .map(|list| {
            list.iter()
                .any(|f| f.family.eq_ignore_ascii_case(family))
        })
        .unwrap_or(false)
}

/// Detect an installed Nerd Font family for `symbol_map` / fallback.
pub fn detect_nerd_font() -> Option<String> {
    let list = list_installed().ok()?;
    const PREFERRED: &[&str] = &[
        "MesloLGS Nerd Font",
        "MesloLGS NF",
        "FantasqueSansM Nerd Font",
        "JetBrainsMono Nerd Font",
        "FiraCode Nerd Font",
        "Hack Nerd Font",
        "Symbols Nerd Font",
        "Symbols Nerd Font Mono",
    ];
    for p in PREFERRED {
        if list.iter().any(|f| f.family.eq_ignore_ascii_case(p)) {
            return Some((*p).to_string());
        }
    }
    list.into_iter()
        .find(|f| {
            let l = f.family.to_ascii_lowercase();
            l.contains("nerd font") && !l.contains("symbols only")
        })
        .map(|f| f.family)
}

pub fn search_installed(query: &str) -> Result<Vec<FontFamily>> {
    let q = query.to_ascii_lowercase();
    Ok(list_installed()?
        .into_iter()
        .filter(|f| f.family.to_ascii_lowercase().contains(&q))
        .collect())
}

/// Copy a user-owned font directory into `~/.local/share/fonts/irongall/imported/<name>/`.
pub fn import_dir(paths: &Paths, src: &Path) -> Result<PathBuf> {
    if !src.exists() {
        return Err(Error::user(format!(
            "font import path does not exist: {}",
            src.display()
        )));
    }
    let name = src
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("imported");
    let dest = paths.fonts_dir().join("imported").join(name);
    copy_fonts(src, &dest)?;
    fc_cache()?;
    Ok(dest)
}

pub fn copy_fonts(src: &Path, dest: &Path) -> Result<usize> {
    fs::create_dir_all(dest).map_err(|e| Error::io(e, dest))?;
    let mut n = 0usize;
    let walker = walk_fonts(src)?;
    for file in walker {
        let ext = file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(ext.as_str(), "ttf" | "otf" | "ttc" | "otc" | "woff" | "woff2") {
            let name = file.file_name().unwrap();
            let target = dest.join(name);
            fs::copy(&file, &target).map_err(|e| Error::io(e, &target))?;
            n += 1;
        }
    }
    if n == 0 {
        return Err(Error::user(format!(
            "no font files found under {}",
            src.display()
        )));
    }
    Ok(n)
}

fn walk_fonts(src: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if src.is_file() {
        out.push(src.to_path_buf());
        return Ok(out);
    }
    fn rec(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(dir).map_err(|e| Error::io(e, dir))? {
            let entry = entry.map_err(|e| Error::io(e, dir))?;
            let p = entry.path();
            if p.is_dir() {
                rec(&p, out)?;
            } else {
                out.push(p);
            }
        }
        Ok(())
    }
    rec(src, &mut out)?;
    Ok(out)
}

pub const PANGRAM: &str = "The quick brown fox jumps over the lazy dog 0123456789 => != ===";
