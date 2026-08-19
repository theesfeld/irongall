use crate::apply::{ApplyCtx, TargetStatus};
use crate::error::Result;
use crate::markers::{self, CommentStyle};

pub fn apply(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    let path = ctx.paths.config_home.join("hypr/hyprland.conf");
    if !path.exists() {
        return Ok(TargetStatus::Skipped {
            reason: "no config".into(),
        });
    }
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let has_keys = existing.contains("col.active_border")
        || existing.contains("col.inactive_border")
        || markers::has_region(&existing, CommentStyle::Hash);
    if !has_keys {
        return Ok(TargetStatus::Skipped {
            reason: "no border color keys".into(),
        });
    }
    let p = &ctx.scheme.palette;
    let body = format!(
        "general {{\n    col.active_border = rgb({})\n    col.inactive_border = rgb({})\n}}",
        p.accent().hex_bare(),
        p.base01().hex_bare()
    );
    ctx.patch_file(&path, &body, CommentStyle::Hash, false)?;
    if !ctx.dry_run {
        let _ = std::process::Command::new("hyprctl").arg("reload").status();
    }
    Ok(if ctx.dry_run {
        TargetStatus::DryRun {
            summary: "border colors".into(),
        }
    } else {
        TargetStatus::Ok {
            detail: Some("borders".into()),
        }
    })
}
