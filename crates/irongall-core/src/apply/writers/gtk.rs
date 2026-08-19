use crate::apply::{ApplyCtx, TargetStatus};
use crate::config::gtk_font_name;
use crate::error::Result;
use crate::markers::CommentStyle;

pub fn apply_gtk3(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    apply_gtk(ctx, "gtk-3.0")
}

pub fn apply_gtk4(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    apply_gtk(ctx, "gtk-4.0")
}

fn apply_gtk(ctx: &mut ApplyCtx<'_>, dir_name: &str) -> Result<TargetStatus> {
    let dir = ctx.paths.config_home.join(dir_name);
    let settings = dir.join("settings.ini");
    let css = dir.join("gtk.css");
    let font = gtk_font_name(&ctx.effective.font, ctx.effective.size);

    let ini_body = format!("gtk-font-name={font}");
    ctx.patch_file(&settings, &ini_body, CommentStyle::Hash, true)?;

    let css_body = gtk_css(ctx);
    ctx.patch_file(&css, &css_body, CommentStyle::Css, true)?;

    // GTK 2 (only when applying gtk3, once).
    if dir_name == "gtk-3.0" {
        let gtk2 = ctx.paths.home.join(".gtkrc-2.0");
        let body = format!("gtk-font-name=\"{font}\"");
        if gtk2.exists() || true {
            ctx.patch_file(&gtk2, &body, CommentStyle::Hash, true)?;
        }
    }

    Ok(status(ctx, &format!("{dir_name} settings + css")))
}

pub fn gtk_css(ctx: &ApplyCtx<'_>) -> String {
    let p = &ctx.scheme.palette;
    let bg = p.base00().hex();
    let bg2 = p.base01().hex();
    let sel = p.base02().hex();
    let fg = p.base05().hex();
    let acc = p.accent().hex();
    let red = p.base08().hex();
    let green = p.base0b().hex();
    let yellow = p.base0a().hex();
    format!(
        "@define-color window_bg_color {bg};\n\
         @define-color window_fg_color {fg};\n\
         @define-color view_bg_color {bg};\n\
         @define-color view_fg_color {fg};\n\
         @define-color headerbar_bg_color {bg2};\n\
         @define-color headerbar_fg_color {fg};\n\
         @define-color headerbar_border_color {bg2};\n\
         @define-color headerbar_backdrop_color {bg};\n\
         @define-color popover_bg_color {bg2};\n\
         @define-color popover_fg_color {fg};\n\
         @define-color theme_bg_color {bg};\n\
         @define-color theme_fg_color {fg};\n\
         @define-color theme_selected_bg_color {sel};\n\
         @define-color theme_selected_fg_color {fg};\n\
         @define-color accent_bg_color {acc};\n\
         @define-color accent_fg_color {bg};\n\
         @define-color accent_color {acc};\n\
         @define-color destructive_bg_color {red};\n\
         @define-color destructive_fg_color {bg};\n\
         @define-color error_bg_color {red};\n\
         @define-color error_color {red};\n\
         @define-color success_bg_color {green};\n\
         @define-color warning_bg_color {yellow};\n\
         @define-color card_bg_color {bg2};\n\
         @define-color card_fg_color {fg};\n\
         @define-color dialog_bg_color {bg2};\n\
         @define-color dialog_fg_color {fg};\n\
         @define-color sidebar_bg_color {bg2};\n\
         @define-color sidebar_fg_color {fg};\n\
         @define-color secondary_sidebar_bg_color {bg};\n\
         @define-color shade_color {bg2};\n"
    )
}

fn status(ctx: &ApplyCtx<'_>, detail: &str) -> TargetStatus {
    if ctx.dry_run {
        TargetStatus::DryRun {
            summary: detail.into(),
        }
    } else {
        TargetStatus::Ok {
            detail: Some(detail.into()),
        }
    }
}
