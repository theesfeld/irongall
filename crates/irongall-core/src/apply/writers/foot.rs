use crate::apply::{ApplyCtx, TargetStatus};
use crate::error::Result;
use crate::markers::CommentStyle;

pub fn apply(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    let path = ctx.paths.config_home.join("foot/foot.ini");
    let p = &ctx.scheme.palette;
    let ansi = p.ansi16();
    let mut body = String::new();
    body.push_str(&format!(
        "font={}:size={}\n",
        ctx.effective.font,
        crate::config::format_pt(ctx.effective.size)
    ));
    body.push_str("[colors]\n");
    body.push_str(&format!("foreground={}\n", p.base05().hex_bare().to_ascii_lowercase()));
    body.push_str(&format!("background={}\n", p.base00().hex_bare().to_ascii_lowercase()));
    body.push_str(&format!(
        "regular0={}\nregular1={}\nregular2={}\nregular3={}\nregular4={}\nregular5={}\nregular6={}\nregular7={}\n",
        hex(ansi[0]), hex(ansi[1]), hex(ansi[2]), hex(ansi[3]),
        hex(ansi[4]), hex(ansi[5]), hex(ansi[6]), hex(ansi[7]),
    ));
    body.push_str(&format!(
        "bright0={}\nbright1={}\nbright2={}\nbright3={}\nbright4={}\nbright5={}\nbright6={}\nbright7={}\n",
        hex(ansi[8]), hex(ansi[9]), hex(ansi[10]), hex(ansi[11]),
        hex(ansi[12]), hex(ansi[13]), hex(ansi[14]), hex(ansi[15]),
    ));
    ctx.patch_file(&path, &body, CommentStyle::Hash, true)?;
    Ok(if ctx.dry_run {
        TargetStatus::DryRun {
            summary: path.display().to_string(),
        }
    } else {
        TargetStatus::Ok { detail: None }
    })
}

fn hex(c: crate::color::Rgb) -> String {
    c.hex_bare().to_ascii_lowercase()
}
