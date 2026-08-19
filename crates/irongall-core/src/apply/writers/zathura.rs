use crate::apply::{ApplyCtx, TargetStatus};
use crate::config::gtk_font_name;
use crate::error::Result;
use crate::markers::CommentStyle;

pub fn apply(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    let path = ctx.paths.config_home.join("zathura/zathurarc");
    let p = &ctx.scheme.palette;
    let font = gtk_font_name(&ctx.effective.font, ctx.effective.size);
    let body = format!(
        "set font \"{font}\"\n\
         set default-bg \"{bg}\"\n\
         set default-fg \"{fg}\"\n\
         set statusbar-bg \"{bg}\"\n\
         set statusbar-fg \"{fg}\"\n\
         set inputbar-bg \"{bg}\"\n\
         set inputbar-fg \"{fg}\"\n\
         set notification-bg \"{bg2}\"\n\
         set notification-fg \"{fg}\"\n\
         set completion-bg \"{bg2}\"\n\
         set completion-fg \"{fg}\"\n\
         set completion-highlight-bg \"{sel}\"\n\
         set completion-highlight-fg \"{fg}\"\n\
         set highlight-color \"{yel}\"\n\
         set highlight-active-color \"{acc}\"\n\
         set recolor true\n\
         set recolor-lightcolor \"{bg}\"\n\
         set recolor-darkcolor \"{fg}\"",
        bg = p.base00().hex_lower(),
        fg = p.base05().hex_lower(),
        bg2 = p.base01().hex_lower(),
        sel = p.base02().hex_lower(),
        yel = p.base0a().hex_lower(),
        acc = p.accent().hex_lower(),
    );
    ctx.patch_file(&path, &body, CommentStyle::Hash, true)?;
    Ok(if ctx.dry_run {
        TargetStatus::DryRun {
            summary: "zathurarc".into(),
        }
    } else {
        TargetStatus::Ok { detail: None }
    })
}
