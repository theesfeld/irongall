use crate::apply::{ApplyCtx, TargetStatus};
use crate::config::format_pt;
use crate::error::Result;
use crate::markers::CommentStyle;

pub fn apply(ctx: &mut ApplyCtx<'_>, major: u8) -> Result<TargetStatus> {
    let dir_name = if major >= 6 { "qt6ct" } else { "qt5ct" };
    let dir = ctx.paths.config_home.join(dir_name);
    let colors = dir.join("colors/irongall.conf");
    ctx.write_file(&colors, &color_scheme(ctx))?;

    let conf = dir.join(format!("{dir_name}.conf"));
    let font = qt_font(&ctx.effective.font, ctx.effective.size);
    let body = format!(
        "[Appearance]\ncolor_scheme_path = {}/colors/irongall.conf\ncustom_palette = true\n\n[Fonts]\ngeneral={font}\nfixed={font}",
        dir.display()
    );
    ctx.patch_file(&conf, &body, CommentStyle::Hash, true)?;
    Ok(if ctx.dry_run {
        TargetStatus::DryRun {
            summary: format!("{dir_name} colors + fonts"),
        }
    } else {
        TargetStatus::Ok {
            detail: Some(format!("{dir_name}")),
        }
    })
}

fn qt_font(family: &str, size: f32) -> String {
    format!(
        "\"{family},{},-1,5,400,0,0,0,0,0,0,0,0,0,0,1\"",
        format_pt(size)
    )
}

fn qcolor(c: crate::color::Rgb) -> String {
    format!("#ff{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

fn palette_line(ctx: &ApplyCtx<'_>) -> String {
    let p = &ctx.scheme.palette;
    let window = p.base00();
    let window_text = p.base05();
    let base = p.base00();
    let text = p.base05();
    let button = p.base01();
    let button_text = p.base05();
    let highlight = p.accent();
    let highlighted = p.base00();
    let link = p.base0d();
    let visited = p.base0e();
    let alt = p.base01();
    let tooltip_bg = p.base01();
    let tooltip_fg = p.base05();
    let bright = p.base07();
    let dark = p.base02();
    let mid = p.base03();
    let shadow = p.base00();
    let placeholder = p.base03();
    // QPalette roles 0–20 as qt6ct expects.
    let roles = [
        window_text, // 0 WindowText
        button,      // 1 Button
        bright,      // 2 Light
        p.base04(),  // 3 Midlight
        dark,        // 4 Dark
        mid,         // 5 Mid
        text,        // 6 Text
        bright,      // 7 BrightText
        button_text, // 8 ButtonText
        base,        // 9 Base
        window,      // 10 Window
        shadow,      // 11 Shadow
        highlight,   // 12 Highlight
        highlighted, // 13 HighlightedText
        link,        // 14 Link
        visited,     // 15 LinkVisited
        alt,         // 16 AlternateBase
        window,      // 17 NoRole
        tooltip_bg,  // 18 ToolTipBase
        tooltip_fg,  // 19 ToolTipText
        placeholder, // 20 PlaceholderText
    ];
    roles.iter().map(|c| qcolor(*c)).collect::<Vec<_>>().join(", ")
}

fn color_scheme(ctx: &ApplyCtx<'_>) -> String {
    let line = palette_line(ctx);
    format!(
        "[ColorScheme]\nactive_colors={line}\ndisabled_colors={line}\ninactive_colors={line}\n"
    )
}
