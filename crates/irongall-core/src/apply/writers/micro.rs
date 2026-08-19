use crate::apply::{ApplyCtx, TargetStatus};
use crate::error::Result;
use crate::markers::CommentStyle;

pub fn apply(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    let dir = ctx.paths.config_home.join("micro");
    let scheme = dir.join("colorschemes/irongall.micro");
    ctx.write_file(&scheme, &colorscheme(ctx))?;
    let settings = dir.join("settings.json");
    let body = "  // IRONGALL-BEGIN\n  \"colorscheme\": \"irongall\"\n  // IRONGALL-END";
    if settings.exists() {
        let old = std::fs::read_to_string(&settings).unwrap_or_else(|_| "{}".into());
        let new = crate::markers::patch(&old, body, CommentStyle::Slash);
        ctx.write_file(&settings, &new)?;
    } else {
        ctx.write_file(&settings, "{\n  \"colorscheme\": \"irongall\"\n}\n")?;
    }
    Ok(if ctx.dry_run {
        TargetStatus::DryRun {
            summary: "micro colorscheme".into(),
        }
    } else {
        TargetStatus::Ok {
            detail: Some("colorscheme".into()),
        }
    })
}

fn colorscheme(ctx: &ApplyCtx<'_>) -> String {
    let p = &ctx.scheme.palette;
    format!(
        "color-link default \"{fg},{bg}\"\n\
         color-link comment \"{dim}\"\n\
         color-link identifier \"{blu}\"\n\
         color-link constant \"{ora}\"\n\
         color-link constant.string \"{grn}\"\n\
         color-link symbol \"{acc}\"\n\
         color-link type \"{yel}\"\n\
         color-link statement \"{acc}\"\n\
         color-link special \"{cyn}\"\n\
         color-link preproc \"{acc}\"\n\
         color-link error \"bold {red}\"\n\
         color-link todo \"bold {yel}\"\n\
         color-link statusline \"{fg},{bg2}\"\n\
         color-link tabbar \"{fg},{bg2}\"\n\
         color-link indent-char \"{sel}\"\n\
         color-link line-number \"{dim}\"\n\
         color-link current-line-number \"{fg}\"\n\
         color-link cursor-line \"{sel},{fg}\"\n\
         color-link color-column \"{bg2}\"\n\
         color-link diff-added \"{grn}\"\n\
         color-link diff-modified \"{yel}\"\n\
         color-link diff-deleted \"{red}\"\n",
        fg = p.base05().hex(),
        bg = p.base00().hex(),
        bg2 = p.base01().hex(),
        sel = p.base02().hex(),
        dim = p.base03().hex(),
        blu = p.base0d().hex(),
        ora = p.base09().hex(),
        grn = p.base0b().hex(),
        acc = p.accent().hex(),
        yel = p.base0a().hex(),
        cyn = p.base0c().hex(),
        red = p.base08().hex(),
    )
}
