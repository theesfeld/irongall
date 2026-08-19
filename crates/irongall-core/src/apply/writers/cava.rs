use crate::apply::{ApplyCtx, TargetStatus};
use crate::error::Result;
use crate::markers::CommentStyle;

pub fn apply(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    let path = ctx.paths.config_home.join("cava/config");
    let p = &ctx.scheme.palette;
    let body = format!(
        "[color]\n\
         background = '{bg}'\n\
         foreground = '{acc}'\n\
         gradient = 1\n\
         gradient_color_1 = '{cyn}'\n\
         gradient_color_2 = '{blu}'\n\
         gradient_color_3 = '{acc}'\n\
         gradient_color_4 = '{red}'",
        bg = p.base00().hex_lower(),
        acc = p.accent().hex_lower(),
        cyn = p.base0c().hex_lower(),
        blu = p.base0d().hex_lower(),
        red = p.base08().hex_lower(),
    );
    ctx.patch_file(&path, &body, CommentStyle::Hash, true)?;
    Ok(if ctx.dry_run {
        TargetStatus::DryRun {
            summary: "cava colors".into(),
        }
    } else {
        TargetStatus::Ok { detail: None }
    })
}
