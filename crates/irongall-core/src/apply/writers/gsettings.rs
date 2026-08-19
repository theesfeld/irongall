use std::process::Command;

use crate::apply::{ApplyCtx, TargetStatus};
use crate::config::gtk_font_name;
use crate::error::Result;

pub fn apply(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    let font = gtk_font_name(&ctx.effective.font, ctx.effective.size);
    let mono = gtk_font_name(
        ctx.cfg.font.mono(),
        ctx.cfg.font.terminal_size(),
    );
    // gsettings fonts are system-level: use effective (which is global unless
    // someone overrode the gsettings adapter itself).
    let scheme = if ctx.scheme.palette.prefer_dark() {
        "prefer-dark"
    } else {
        "prefer-light"
    };

    if ctx.dry_run {
        return Ok(TargetStatus::DryRun {
            summary: format!("{font}; {scheme}"),
        });
    }

    let pairs = [
        ("org.gnome.desktop.interface", "font-name", font.as_str()),
        (
            "org.gnome.desktop.interface",
            "document-font-name",
            font.as_str(),
        ),
        (
            "org.gnome.desktop.interface",
            "monospace-font-name",
            mono.as_str(),
        ),
        (
            "org.gnome.desktop.wm.preferences",
            "titlebar-font",
            font.as_str(),
        ),
        (
            "org.gnome.desktop.interface",
            "color-scheme",
            scheme,
        ),
    ];

    let mut ok = 0usize;
    let mut last_err = String::new();
    for (schema, key, value) in pairs {
        match gset(schema, key, value) {
            Ok(()) => ok += 1,
            Err(e) => last_err = e,
        }
    }
    if ok == 0 {
        return Ok(TargetStatus::Skipped {
            reason: format!("gsettings unavailable ({last_err})"),
        });
    }
    Ok(TargetStatus::Ok {
        detail: Some(format!("{ok} keys, {scheme}")),
    })
}

fn gset(schema: &str, key: &str, value: &str) -> std::result::Result<(), String> {
    let out = Command::new("gsettings")
        .args(["set", schema, key, value])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(())
}
