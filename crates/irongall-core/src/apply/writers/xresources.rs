use std::process::Command;

use crate::apply::{ApplyCtx, TargetStatus};
use crate::error::Result;
use crate::markers::CommentStyle;

pub fn apply(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    let path = if ctx.paths.home.join(".Xresources").exists() {
        ctx.paths.home.join(".Xresources")
    } else if ctx.paths.home.join(".Xdefaults").exists() {
        ctx.paths.home.join(".Xdefaults")
    } else {
        return Ok(TargetStatus::Skipped {
            reason: "no .Xresources".into(),
        });
    };
    ctx.patch_file(&path, &body(ctx), CommentStyle::Hash, false)?;
    if !ctx.dry_run {
        let _ = Command::new("xrdb")
            .args(["-merge", &path.display().to_string()])
            .status();
    }
    Ok(if ctx.dry_run {
        TargetStatus::DryRun {
            summary: path.display().to_string(),
        }
    } else {
        TargetStatus::Ok {
            detail: Some("xrdb -merge".into()),
        }
    })
}

fn body(ctx: &ApplyCtx<'_>) -> String {
    let p = &ctx.scheme.palette;
    let ansi = p.ansi16();
    let mut s = String::new();
    s.push_str(&format!("*foreground: {}\n", p.base05().hex()));
    s.push_str(&format!("*background: {}\n", p.base00().hex()));
    s.push_str(&format!("*cursorColor: {}\n", p.base05().hex()));
    for (i, c) in ansi.iter().enumerate() {
        s.push_str(&format!("*color{i}: {}\n", c.hex()));
    }
    s
}
