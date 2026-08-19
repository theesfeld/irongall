use crate::apply::{ApplyCtx, TargetStatus};
use crate::error::Result;
use crate::markers::CommentStyle;

pub fn apply(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    let path = ctx.paths.config_home.join("starship.toml");
    let p = &ctx.scheme.palette;
    let body = format!(
        "palette = \"irongall\"\n\
         \n\
         [palettes.irongall]\n\
         blue      = \"{blu}\"\n\
         red       = \"{red}\"\n\
         green     = \"{grn}\"\n\
         yellow    = \"{yel}\"\n\
         cyan      = \"{cyn}\"\n\
         magenta   = \"{acc}\"\n\
         white     = \"{fg}\"\n\
         black     = \"{bg}\"\n\
         text      = \"{fg}\"\n\
         subtext1  = \"{fg}\"\n\
         subtext0  = \"{dim}\"\n\
         overlay2  = \"{dim}\"\n\
         overlay1  = \"{dim}\"\n\
         overlay0  = \"{bg}\"\n\
         surface2  = \"{bg2}\"\n\
         surface1  = \"{bg2}\"\n\
         surface0  = \"{bg}\"\n\
         base      = \"{bg}\"\n\
         mantle    = \"{bg}\"\n\
         crust     = \"{bg}\"",
        blu = p.base0d().hex_lower(),
        red = p.base08().hex_lower(),
        grn = p.base0b().hex_lower(),
        yel = p.base0a().hex_lower(),
        cyn = p.base0c().hex_lower(),
        acc = p.accent().hex_lower(),
        fg = p.base05().hex_lower(),
        bg = p.base00().hex_lower(),
        bg2 = p.base01().hex_lower(),
        dim = p.base03().hex_lower(),
    );
    ctx.patch_file(&path, &body, CommentStyle::Hash, true)?;
    Ok(if ctx.dry_run {
        TargetStatus::DryRun {
            summary: "starship palette".into(),
        }
    } else {
        TargetStatus::Ok {
            detail: Some("palette".into()),
        }
    })
}
