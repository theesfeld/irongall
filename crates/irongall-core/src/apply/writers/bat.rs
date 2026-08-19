use std::process::Command;

use crate::apply::{ApplyCtx, TargetStatus};
use crate::error::Result;
use crate::markers::CommentStyle;

pub fn apply(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    let dir = ctx.paths.config_home.join("bat");
    let tm = dir.join("themes/irongall.tmTheme");
    ctx.write_file(&tm, &tmtheme(ctx))?;
    let conf = dir.join("config");
    ctx.patch_file(&conf, "--theme=\"irongall\"", CommentStyle::Hash, true)?;
    if !ctx.dry_run {
        let _ = Command::new("bat").args(["cache", "--build"]).status();
    }
    Ok(if ctx.dry_run {
        TargetStatus::DryRun {
            summary: "bat theme + cache".into(),
        }
    } else {
        TargetStatus::Ok {
            detail: Some("theme".into()),
        }
    })
}

fn tmtheme(ctx: &ApplyCtx<'_>) -> String {
    let p = &ctx.scheme.palette;
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\"><dict>\n\
         <key>name</key><string>irongall</string>\n\
         <key>settings</key><array>\n\
         <dict><key>settings</key><dict>\n\
           <key>background</key><string>{bg}</string>\n\
           <key>foreground</key><string>{fg}</string>\n\
           <key>caret</key><string>{fg}</string>\n\
           <key>lineHighlight</key><string>{bg2}</string>\n\
           <key>selection</key><string>{sel}</string>\n\
         </dict></dict>\n\
         <dict><key>scope</key><string>comment</string><key>settings</key><dict><key>foreground</key><string>{dim}</string></dict></dict>\n\
         <dict><key>scope</key><string>string</string><key>settings</key><dict><key>foreground</key><string>{grn}</string></dict></dict>\n\
         <dict><key>scope</key><string>constant.numeric</string><key>settings</key><dict><key>foreground</key><string>{ora}</string></dict></dict>\n\
         <dict><key>scope</key><string>keyword</string><key>settings</key><dict><key>foreground</key><string>{acc}</string></dict></dict>\n\
         <dict><key>scope</key><string>entity.name.function</string><key>settings</key><dict><key>foreground</key><string>{blu}</string></dict></dict>\n\
         <dict><key>scope</key><string>entity.name.type</string><key>settings</key><dict><key>foreground</key><string>{yel}</string></dict></dict>\n\
         <dict><key>scope</key><string>variable</string><key>settings</key><dict><key>foreground</key><string>{fg}</string></dict></dict>\n\
         <dict><key>scope</key><string>invalid</string><key>settings</key><dict><key>foreground</key><string>{red}</string></dict></dict>\n\
         </array>\n\
         <key>uuid</key><string>irongall-base16</string>\n\
         </dict></plist>\n",
        bg = p.base00().hex_lower(),
        fg = p.base05().hex_lower(),
        bg2 = p.base01().hex_lower(),
        sel = p.base02().hex_lower(),
        dim = p.base03().hex_lower(),
        grn = p.base0b().hex_lower(),
        ora = p.base09().hex_lower(),
        acc = p.accent().hex_lower(),
        blu = p.base0d().hex_lower(),
        yel = p.base0a().hex_lower(),
        red = p.base08().hex_lower(),
    )
}
