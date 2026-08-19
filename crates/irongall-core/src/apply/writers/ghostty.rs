use crate::apply::{ApplyCtx, TargetStatus};
use crate::error::Result;
use crate::markers::CommentStyle;

pub fn apply(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    let path = ctx.paths.config_home.join("ghostty/config");
    let p = &ctx.scheme.palette;
    let ansi = p.ansi16();
    let mut body = String::new();
    body.push_str(&format!("font-family = {}\n", ctx.effective.font));
    body.push_str(&format!(
        "font-size = {}\n",
        crate::config::format_pt(ctx.effective.size)
    ));
    body.push_str(&format!("background = {}\n", p.base00().hex_bare()));
    body.push_str(&format!("foreground = {}\n", p.base05().hex_bare()));
    body.push_str(&format!("cursor-color = {}\n", p.base05().hex_bare()));
    body.push_str(&format!(
        "selection-background = {}\n",
        p.base02().hex_bare()
    ));
    body.push_str(&format!(
        "selection-foreground = {}\n",
        p.base05().hex_bare()
    ));
    for (i, c) in ansi.iter().enumerate() {
        body.push_str(&format!("palette = {i}={}\n", c.hex()));
    }
    ctx.patch_file(&path, &body, CommentStyle::Hash, true)?;
    Ok(if ctx.dry_run {
        TargetStatus::DryRun {
            summary: path.display().to_string(),
        }
    } else {
        TargetStatus::Ok { detail: None }
    })
}
