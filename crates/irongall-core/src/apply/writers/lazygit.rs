use crate::apply::{ApplyCtx, TargetStatus};
use crate::error::Result;
use crate::markers::CommentStyle;

pub fn apply(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    let path = ctx.paths.config_home.join("lazygit/config.yml");
    let p = &ctx.scheme.palette;
    let body = format!(
        "gui:\n\
         \ttheme:\n\
         \t\tactiveBorderColor:\n\
         \t\t\t- \"{acc}\"\n\
         \t\t\t- bold\n\
         \t\tinactiveBorderColor:\n\
         \t\t\t- \"{dim}\"\n\
         \t\tselectedLineBgColor:\n\
         \t\t\t- \"{sel}\"\n\
         \t\toptionsTextColor:\n\
         \t\t\t- \"{blu}\"\n\
         \t\tdefaultFgColor:\n\
         \t\t\t- \"{fg}\"\n\
         \t\tsearchingActiveBorderColor:\n\
         \t\t\t- \"{yel}\"",
        acc = p.accent().hex_lower(),
        dim = p.base03().hex_lower(),
        sel = p.base02().hex_lower(),
        blu = p.base0d().hex_lower(),
        fg = p.base05().hex_lower(),
        yel = p.base0a().hex_lower(),
    );
    ctx.patch_file(&path, &body, CommentStyle::Hash, true)?;
    Ok(if ctx.dry_run {
        TargetStatus::DryRun {
            summary: "lazygit gui.theme".into(),
        }
    } else {
        TargetStatus::Ok {
            detail: Some("gui.theme".into()),
        }
    })
}
