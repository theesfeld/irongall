use crate::apply::{ApplyCtx, TargetStatus};
use crate::config::format_pt;
use crate::error::Result;
use crate::markers::CommentStyle;

pub fn apply(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    let path = ctx.paths.config_home.join("kdeglobals");
    if !path.exists() {
        return Ok(TargetStatus::Skipped {
            reason: "no kdeglobals".into(),
        });
    }
    ctx.patch_file(&path, &body(ctx), CommentStyle::Hash, false)?;
    Ok(if ctx.dry_run {
        TargetStatus::DryRun {
            summary: "kdeglobals colors + fonts".into(),
        }
    } else {
        TargetStatus::Ok { detail: None }
    })
}

fn kde_font(family: &str, size: f32) -> String {
    format!("{family},{},-1,5,400,0,0,0,0,0", format_pt(size))
}

fn body(ctx: &ApplyCtx<'_>) -> String {
    let p = &ctx.scheme.palette;
    let font = kde_font(&ctx.effective.font, ctx.effective.size);
    let mono = kde_font(ctx.cfg.font.mono(), ctx.cfg.font.terminal_size());
    format!(
        "[General]\n\
         font={font}\n\
         menuFont={font}\n\
         toolBarFont={font}\n\
         smallestReadableFont={font}\n\
         fixed={mono}\n\
         [WM]\n\
         activeFont={font}\n\
         activeBackground={}\n\
         activeForeground={}\n\
         inactiveBackground={}\n\
         inactiveForeground={}\n\
         [Colors:Window]\n\
         BackgroundNormal={}\n\
         ForegroundNormal={}\n\
         [Colors:View]\n\
         BackgroundNormal={}\n\
         ForegroundNormal={}\n\
         DecorationFocus={}\n\
         DecorationHover={}\n\
         [Colors:Selection]\n\
         BackgroundNormal={}\n\
         ForegroundNormal={}\n\
         [Colors:Button]\n\
         BackgroundNormal={}\n\
         ForegroundNormal={}\n\
         [Colors:Tooltip]\n\
         BackgroundNormal={}\n\
         ForegroundNormal={}\n\
         [Colors:Header]\n\
         BackgroundNormal={}\n\
         ForegroundNormal={}\n",
        p.base01().rgb_csv(),
        p.base05().rgb_csv(),
        p.base00().rgb_csv(),
        p.base04().rgb_csv(),
        p.base00().rgb_csv(),
        p.base05().rgb_csv(),
        p.base00().rgb_csv(),
        p.base05().rgb_csv(),
        p.accent().rgb_csv(),
        p.base0d().rgb_csv(),
        p.base02().rgb_csv(),
        p.base05().rgb_csv(),
        p.base01().rgb_csv(),
        p.base05().rgb_csv(),
        p.base01().rgb_csv(),
        p.base05().rgb_csv(),
        p.base01().rgb_csv(),
        p.base05().rgb_csv(),
    )
}
