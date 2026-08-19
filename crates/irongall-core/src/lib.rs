//! Core library for irongall: schemes, fonts, catalog, discovery, apply.

pub mod apply;
pub mod backup;
pub mod catalog;
pub mod color;
pub mod config;
pub mod discovery;
pub mod error;
pub mod font;
pub mod market;
pub mod markers;
pub mod paths;
pub mod scheme;

pub use color::Rgb;
pub use config::Config;
pub use error::{Error, Result};
pub use paths::Paths;
pub use scheme::Scheme;

/// Status text for `irongall status`.
pub fn status_text(paths: &Paths, cfg: &Config) -> Result<String> {
    let mut out = String::new();
    out.push_str("irongall\n");
    out.push_str(&format!(
        "  theme   {} ({})\n",
        cfg.theme.name,
        cfg.theme.variant.as_str()
    ));
    out.push_str(&format!("  font    {}\n", cfg.font.family));
    if let Some(s) = &cfg.font.sans {
        out.push_str(&format!("  sans    {s}\n"));
    }
    if let Some(s) = &cfg.font.serif {
        out.push_str(&format!("  serif   {s}\n"));
    }
    if let Some(s) = &cfg.font.mono {
        out.push_str(&format!("  mono    {s}\n"));
    }
    out.push_str(&format!(
        "  size    {} pt\n",
        config::format_pt(cfg.font.size)
    ));
    if let Some(s) = cfg.font.terminal_size {
        out.push_str(&format!("  term    {} pt\n", config::format_pt(s)));
    }
    if let Some(s) = cfg.font.ui_size {
        out.push_str(&format!("  ui      {} pt\n", config::format_pt(s)));
    }
    out.push_str(&format!("  config  {}\n", paths.config_file().display()));

    out.push('\n');
    out.push_str("fc-match\n");
    for q in ["sans-serif", "serif", "monospace", "system-ui"] {
        match font::fc_match_family(q) {
            Ok(fam) => out.push_str(&format!("  {q:<12} {fam}\n")),
            Err(e) => out.push_str(&format!("  {q:<12} ({e})\n")),
        }
    }

    let rows = discovery::rows(paths, cfg, true)?;
    let counts = discovery::counts(&rows);
    out.push('\n');
    out.push_str("apps\n");
    out.push_str(&format!("  {}\n", counts.one_line()));
    if counts.no_writer > 0 {
        out.push_str(&format!("  {} no-writer\n", counts.no_writer));
    }
    Ok(out)
}
