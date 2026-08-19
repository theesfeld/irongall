use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use crate::error::{Error, Result};
use crate::paths::Paths;

static LIST_CACHE: OnceLock<Mutex<Option<Vec<FontFamily>>>> = OnceLock::new();
static NERD_CACHE: OnceLock<Mutex<Option<Option<String>>>> = OnceLock::new();

fn list_cache() -> &'static Mutex<Option<Vec<FontFamily>>> {
    LIST_CACHE.get_or_init(|| Mutex::new(None))
}

/// Drop cached `fc-list` results after installing fonts.
pub fn invalidate_cache() {
    if let Ok(mut g) = list_cache().lock() {
        *g = None;
    }
    if let Ok(mut g) = NERD_CACHE.get_or_init(|| Mutex::new(None)).lock() {
        *g = None;
    }
}

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

/// Unique families from fontconfig. Emoji / CJK / last-resort go in `Other`.
/// Results are cached until `invalidate_cache`.
pub fn list_installed() -> Result<Vec<FontFamily>> {
    if let Ok(g) = list_cache().lock() {
        if let Some(v) = g.as_ref() {
            return Ok(v.clone());
        }
    }
    let v = list_installed_uncached()?;
    if let Ok(mut g) = list_cache().lock() {
        *g = Some(v.clone());
    }
    Ok(v)
}

fn list_installed_uncached() -> Result<Vec<FontFamily>> {
    // Family-only listing is enough for the picker; styles are filled on demand.
    let output = Command::new("fc-list")
        .args(["-f", "%{family[0]}\n"])
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
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let family = line.split(',').next().unwrap_or(line).trim();
        if !family.is_empty() {
            set.insert(family.to_string());
        }
    }
    let mut out: Vec<FontFamily> = set
        .into_iter()
        .map(|family| {
            let group = if is_other(&family) {
                FontGroup::Other
            } else {
                FontGroup::Main
            };
            FontFamily {
                family,
                styles: Vec::new(),
                group,
            }
        })
        .collect();
    out.sort_by(|a, b| a.family.to_ascii_lowercase().cmp(&b.family.to_ascii_lowercase()));
    Ok(out)
}

/// Styles for one family (one extra `fc-list` — only used in the font preview).
pub fn styles_for(family: &str) -> Vec<String> {
    let output = Command::new("fc-list")
        .args([family, "-f", "%{style[0]}\n"])
        .output();
    let Ok(o) = output else { return Vec::new() };
    if !o.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&o.stdout);
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let s = line.trim();
        if !s.is_empty() {
            set.insert(s.to_string());
        }
    }
    set.into_iter().collect()
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
    invalidate_cache();
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
    if let Ok(g) = NERD_CACHE.get_or_init(|| Mutex::new(None)).lock() {
        if let Some(cached) = g.as_ref() {
            return cached.clone();
        }
    }
    let found = detect_nerd_font_uncached();
    if let Ok(mut g) = NERD_CACHE.get_or_init(|| Mutex::new(None)).lock() {
        *g = Some(found.clone());
    }
    found
}

fn detect_nerd_font_uncached() -> Option<String> {
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
    invalidate_cache();
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

const PREVIEW_SAMPLE: &str = "Ag Hamburgefontsiv\nabcdefghijkmnopq\n0123456789 => !=";

/// Font file for a family via fontconfig (`fc-match`). Portable; not distro-specific.
pub fn file_for(family: &str) -> Result<PathBuf> {
    let output = Command::new("fc-match")
        .args(["-f", "%{file}", family])
        .output()
        .map_err(|e| Error::Command {
            cmd: format!("fc-match {family}"),
            status: None,
            detail: e.to_string(),
        })?;
    if !output.status.success() {
        return Err(Error::Command {
            cmd: format!("fc-match {family}"),
            status: Some(output.status),
            detail: String::from_utf8_lossy(&output.stderr).into(),
        });
    }
    let p = String::from_utf8_lossy(&output.stdout);
    let p = p.trim();
    if p.is_empty() {
        return Err(Error::user(format!("no file for font '{family}'")));
    }
    Ok(PathBuf::from(p))
}

/// Rasterize a sample of `family` to Unicode braille.
/// Works in any terminal that can show braille; no graphics protocol.
pub fn preview_braille(family: &str, cols: usize, rows: usize) -> Result<String> {
    let cols = cols.clamp(8, 80);
    let rows = rows.clamp(3, 24);
    let path = file_for(family)?;
    let bytes = fs::read(&path).map_err(|e| Error::io(e, &path))?;
    let font = fontdue::Font::from_bytes(bytes.as_slice(), fontdue::FontSettings::default())
        .map_err(|e| Error::user(format!("cannot rasterize '{}': {e}", path.display())))?;

    let w = cols * 2;
    let h = rows * 4;
    let px = ((h as f32) * 0.28).clamp(10.0, 22.0);
    let mut buf = vec![0u8; w * h];

    let mut pen_x = 1.0f32;
    let mut baseline = px * 0.85;
    for ch in PREVIEW_SAMPLE.chars() {
        if ch == '\n' {
            baseline += px * 1.2;
            pen_x = 1.0;
            continue;
        }
        if baseline as usize + 4 >= h {
            break;
        }
        let (metrics, bitmap) = font.rasterize(ch, px);
        let gx = (pen_x + metrics.xmin as f32).round() as i32;
        let gy = (baseline + metrics.ymin as f32).round() as i32;
        let bw = metrics.width as i32;
        let bh = metrics.height as i32;
        for yy in 0..bh {
            for xx in 0..bw {
                let dx = gx + xx;
                let dy = gy + yy;
                if dx < 0 || dy < 0 || dx >= w as i32 || dy >= h as i32 {
                    continue;
                }
                let src = bitmap[(yy * bw + xx) as usize];
                let i = dy as usize * w + dx as usize;
                if src > buf[i] {
                    buf[i] = src;
                }
            }
        }
        pen_x += metrics.advance_width;
        if pen_x > w as f32 - 2.0 {
            baseline += px * 1.2;
            pen_x = 1.0;
        }
    }

    Ok(bitmap_to_braille(&buf, w, h, cols, rows))
}

fn bitmap_to_braille(buf: &[u8], w: usize, h: usize, cols: usize, rows: usize) -> String {
    // Braille dots: 1 4 / 2 5 / 3 6 / 7 8
    const DOT: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];
    let mut out = String::with_capacity(rows * (cols + 1));
    for row in 0..rows {
        for col in 0..cols {
            let mut bits: u32 = 0;
            for dy in 0..4 {
                for dx in 0..2 {
                    let x = col * 2 + dx;
                    let y = row * 4 + dy;
                    if x < w && y < h && buf[y * w + x] > 90 {
                        bits |= DOT[dy][dx] as u32;
                    }
                }
            }
            out.push(char::from_u32(0x2800 + bits).unwrap_or('\u{2800}'));
        }
        if row + 1 < rows {
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::bitmap_to_braille;

    #[test]
    fn braille_full_block() {
        let buf = vec![255u8; 2 * 4];
        let s = bitmap_to_braille(&buf, 2, 4, 1, 1);
        assert_eq!(s.chars().count(), 1);
        assert_ne!(s, "\u{2800}");
    }
}
