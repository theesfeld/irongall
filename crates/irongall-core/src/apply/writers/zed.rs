use crate::apply::{ApplyCtx, TargetStatus};
use crate::error::Result;
use crate::markers::CommentStyle;

pub fn apply(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    let dir = ctx.paths.config_home.join("zed");
    let theme_path = dir.join("themes/irongall.json");
    ctx.write_file(&theme_path, &theme_json(ctx))?;

    let settings = dir.join("settings.json");
    let body = settings_block(ctx);
    if settings.exists() {
        let old = std::fs::read_to_string(&settings).unwrap_or_else(|_| "{}".into());
        let new = patch_jsonc(&old, &body);
        ctx.write_file(&settings, &new)?;
    } else {
        let new = format!("{{\n{body}\n}}\n");
        ctx.write_file(&settings, &new)?;
    }
    Ok(if ctx.dry_run {
        TargetStatus::DryRun {
            summary: "settings.json + theme".into(),
        }
    } else {
        TargetStatus::Ok {
            detail: Some("theme + fonts".into()),
        }
    })
}

fn settings_block(ctx: &ApplyCtx<'_>) -> String {
    let size = ctx.effective.size;
    format!(
        "  // IRONGALL-BEGIN\n\
         \t\"theme\": \"Irongall\",\n\
         \t\"ui_font_family\": \"{font}\",\n\
         \t\"buffer_font_family\": \"{font}\",\n\
         \t\"ui_font_size\": {size},\n\
         \t\"buffer_font_size\": {size}\n\
         \t// IRONGALL-END",
        font = ctx.effective.font.replace('\\', "\\\\").replace('"', "\\\""),
        size = crate::config::format_pt(size),
    )
}

fn patch_jsonc(old: &str, block: &str) -> String {
    crate::markers::patch(old, block.trim(), CommentStyle::Slash)
}

fn theme_json(ctx: &ApplyCtx<'_>) -> String {
    let p = &ctx.scheme.palette;
    // Zed theme v0.2.0 style — enough keys for syntax + UI.
    format!(
        "{{\n\
          \"$schema\": \"https://zed.dev/schema/themes/v0.2.0.json\",\n\
          \"name\": \"Irongall\",\n\
          \"author\": \"irongall\",\n\
          \"themes\": [{{\n\
            \"name\": \"Irongall\",\n\
            \"appearance\": \"{appearance}\",\n\
            \"style\": {{\n\
              \"background\": \"{bg}\",\n\
              \"foreground\": \"{fg}\",\n\
              \"border\": \"{b01}\",\n\
              \"border.focused\": \"{acc}\",\n\
              \"elevated_surface.background\": \"{b01}\",\n\
              \"surface.background\": \"{bg}\",\n\
              \"element.background\": \"{b01}\",\n\
              \"element.hover\": \"{b02}\",\n\
              \"element.selected\": \"{b02}\",\n\
              \"drop_target.background\": \"{b02}\",\n\
              \"ghost_element.background\": \"#00000000\",\n\
              \"ghost_element.hover\": \"{b01}\",\n\
              \"ghost_element.selected\": \"{b02}\",\n\
              \"text\": \"{fg}\",\n\
              \"text.muted\": \"{b03}\",\n\
              \"text.accent\": \"{acc}\",\n\
              \"icon\": \"{fg}\",\n\
              \"icon.muted\": \"{b03}\",\n\
              \"icon.accent\": \"{acc}\",\n\
              \"status_bar.background\": \"{b01}\",\n\
              \"title_bar.background\": \"{b01}\",\n\
              \"toolbar.background\": \"{bg}\",\n\
              \"tab_bar.background\": \"{b01}\",\n\
              \"tab.inactive_background\": \"{b01}\",\n\
              \"tab.active_background\": \"{bg}\",\n\
              \"search.match_background\": \"{b02}\",\n\
              \"panel.background\": \"{b01}\",\n\
              \"panel.focused_border\": \"{acc}\",\n\
              \"editor.foreground\": \"{fg}\",\n\
              \"editor.background\": \"{bg}\",\n\
              \"editor.gutter.background\": \"{bg}\",\n\
              \"editor.subheader.background\": \"{b01}\",\n\
              \"editor.highlighted_line.background\": \"{b01}\",\n\
              \"editor.line_number\": \"{b03}\",\n\
              \"editor.active_line_number\": \"{b04}\",\n\
              \"editor.invisible\": \"{b02}\",\n\
              \"editor.wrap_guide\": \"{b02}\",\n\
              \"editor.active_wrap_guide\": \"{b03}\",\n\
              \"editor.document_highlight.read_background\": \"{b02}\",\n\
              \"editor.document_highlight.write_background\": \"{b02}\",\n\
              \"terminal.background\": \"{bg}\",\n\
              \"terminal.foreground\": \"{fg}\",\n\
              \"terminal.ansi.black\": \"{b00}\",\n\
              \"terminal.ansi.red\": \"{b08}\",\n\
              \"terminal.ansi.green\": \"{b0b}\",\n\
              \"terminal.ansi.yellow\": \"{b0a}\",\n\
              \"terminal.ansi.blue\": \"{b0d}\",\n\
              \"terminal.ansi.magenta\": \"{b0e}\",\n\
              \"terminal.ansi.cyan\": \"{b0c}\",\n\
              \"terminal.ansi.white\": \"{b05}\",\n\
              \"terminal.ansi.bright_black\": \"{b03}\",\n\
              \"terminal.ansi.bright_red\": \"{b08}\",\n\
              \"terminal.ansi.bright_green\": \"{b0b}\",\n\
              \"terminal.ansi.bright_yellow\": \"{b0a}\",\n\
              \"terminal.ansi.bright_blue\": \"{b0d}\",\n\
              \"terminal.ansi.bright_magenta\": \"{b0e}\",\n\
              \"terminal.ansi.bright_cyan\": \"{b0c}\",\n\
              \"terminal.ansi.bright_white\": \"{b07}\",\n\
              \"error\": \"{b08}\",\n\
              \"warning\": \"{b0a}\",\n\
              \"info\": \"{b0d}\",\n\
              \"success\": \"{b0b}\",\n\
              \"hint\": \"{b0c}\",\n\
              \"syntax\": {{\n\
                \"comment\": {{ \"color\": \"{b03}\", \"font_style\": \"italic\" }},\n\
                \"string\": {{ \"color\": \"{b0b}\" }},\n\
                \"keyword\": {{ \"color\": \"{b0e}\" }},\n\
                \"function\": {{ \"color\": \"{b0d}\" }},\n\
                \"type\": {{ \"color\": \"{b0a}\" }},\n\
                \"number\": {{ \"color\": \"{b09}\" }},\n\
                \"constant\": {{ \"color\": \"{b09}\" }},\n\
                \"variable\": {{ \"color\": \"{fg}\" }},\n\
                \"punctuation\": {{ \"color\": \"{b04}\" }},\n\
                \"operator\": {{ \"color\": \"{b0c}\" }},\n\
                \"tag\": {{ \"color\": \"{b08}\" }},\n\
                \"attribute\": {{ \"color\": \"{b09}\" }},\n\
                \"property\": {{ \"color\": \"{b08}\" }}\n\
              }}\n\
            }}\n\
          }}]\n\
        }}\n",
        appearance = if p.prefer_dark() { "dark" } else { "light" },
        bg = p.base00().hex_lower(),
        fg = p.base05().hex_lower(),
        b00 = p.base00().hex_lower(),
        b01 = p.base01().hex_lower(),
        b02 = p.base02().hex_lower(),
        b03 = p.base03().hex_lower(),
        b04 = p.base04().hex_lower(),
        b05 = p.base05().hex_lower(),
        b07 = p.base07().hex_lower(),
        b08 = p.base08().hex_lower(),
        b09 = p.base09().hex_lower(),
        b0a = p.base0a().hex_lower(),
        b0b = p.base0b().hex_lower(),
        b0c = p.base0c().hex_lower(),
        b0d = p.base0d().hex_lower(),
        b0e = p.base0e().hex_lower(),
        acc = p.accent().hex_lower(),
    )
}
