use std::process::Command;

use crate::apply::{ApplyCtx, TargetStatus};
use crate::config::gtk_font_name;
use crate::error::Result;
use crate::markers::CommentStyle;

pub fn apply(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    let candidates = [
        ctx.paths.config_home.join("xsettingsd/xsettingsd.conf"),
        ctx.paths.home.join(".xsettingsd"),
    ];
    let path = candidates.into_iter().find(|p| p.exists());
    let Some(path) = path else {
        return Ok(TargetStatus::Skipped {
            reason: "no xsettingsd config".into(),
        });
    };
    let font = gtk_font_name(&ctx.effective.font, ctx.effective.size);
    let body = format!("Gtk/FontName \"{font}\"");
    ctx.patch_file(&path, &body, CommentStyle::Hash, false)?;
    if !ctx.dry_run {
        let _ = Command::new("pkill").args(["-HUP", "xsettingsd"]).status();
    }
    Ok(if ctx.dry_run {
        TargetStatus::DryRun {
            summary: path.display().to_string(),
        }
    } else {
        TargetStatus::Ok { detail: None }
    })
}
