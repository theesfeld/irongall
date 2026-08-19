use crate::apply::{ApplyCtx, TargetStatus};
use crate::error::Result;
use crate::font;

const ALIASED: &[&str] = &[
    "Noto Sans",
    "Noto Serif",
    "Noto Sans Mono",
    "Adwaita Sans",
    "Adwaita Mono",
    "Cantarell",
    "Arial",
    "Helvetica",
    "Helvetica Neue",
    "Inter",
    "DejaVu Sans",
    "DejaVu Serif",
    "DejaVu Sans Mono",
    "Liberation Sans",
    "Liberation Serif",
    "Liberation Mono",
    "FreeSans",
    "FreeSerif",
    "FreeMono",
    "Ubuntu",
    "Roboto",
    "Source Sans 3",
    "Source Serif 4",
    "Source Code Pro",
];

pub fn apply(ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    let sans = ctx.cfg.font.sans();
    let serif = ctx.cfg.font.serif();
    let mono = ctx.cfg.font.mono();
    // Per-app fontconfig is forbidden; this writer always uses global families.
    let _ = &ctx.effective;

    let nerd = font::detect_nerd_font();
    let xml = render(sans, serif, mono, nerd.as_deref());

    let dir = ctx.paths.config_home.join("fontconfig/conf.d");
    let path = dir.join("50-irongall.conf");
    // The whole file is ours; still wrap the guts in XML markers.
    let existing = if path.exists() {
        std::fs::read_to_string(&path).unwrap_or_default()
    } else {
        String::new()
    };
    let body = xml.trim();
    let new = if existing.contains("<!-- IRONGALL-BEGIN -->") {
        crate::markers::patch(&existing, body, crate::markers::CommentStyle::Xml)
    } else {
        format!(
            "<?xml version=\"1.0\"?>\n\
             <!DOCTYPE fontconfig SYSTEM \"urn:fontconfig:fonts.dtd\">\n\
             <fontconfig>\n\
             <!-- IRONGALL-BEGIN -->\n\
             {body}\n\
             <!-- IRONGALL-END -->\n\
             </fontconfig>\n"
        )
    };
    ctx.write_file(&path, &new)?;

    if !ctx.dry_run {
        font::fc_cache()?;
    }
    Ok(if ctx.dry_run {
        TargetStatus::DryRun {
            summary: path.display().to_string(),
        }
    } else {
        TargetStatus::Ok {
            detail: Some("fc-cache".into()),
        }
    })
}

pub fn render(sans: &str, serif: &str, mono: &str, nerd: Option<&str>) -> String {
    let mut s = String::new();
    s.push_str("  <!-- irongall: strong-alias generic families -->\n");
    alias(&mut s, "sans-serif", sans);
    alias(&mut s, "serif", serif);
    alias(&mut s, "monospace", mono);
    alias(&mut s, "system-ui", sans);

    s.push_str("  <!-- common named fonts → chosen family (never emoji / CJK / nerd) -->\n");
    for name in ALIASED {
        if name.eq_ignore_ascii_case(sans)
            || name.eq_ignore_ascii_case(serif)
            || name.eq_ignore_ascii_case(mono)
        {
            continue;
        }
        let dest = if looks_mono(name) { mono } else if looks_serif(name) { serif } else { sans };
        alias(&mut s, name, dest);
    }

    s.push_str("  <!-- fallbacks: nerd icons, then CJK, then emoji. Never alias those families. -->\n");
    s.push_str("  <alias>\n    <family>sans-serif</family>\n    <prefer>\n");
    if let Some(n) = nerd {
        s.push_str(&format!("      <family>{n}</family>\n"));
    }
    s.push_str("      <family>Noto Sans CJK JP</family>\n");
    s.push_str("      <family>Noto Sans CJK SC</family>\n");
    s.push_str("      <family>Noto Color Emoji</family>\n");
    s.push_str("    </prefer>\n  </alias>\n");
    s.push_str("  <alias>\n    <family>monospace</family>\n    <prefer>\n");
    if let Some(n) = nerd {
        s.push_str(&format!("      <family>{n}</family>\n"));
    }
    s.push_str("      <family>Noto Sans Mono CJK JP</family>\n");
    s.push_str("      <family>Noto Color Emoji</family>\n");
    s.push_str("    </prefer>\n  </alias>\n");
    s
}

fn alias(s: &mut String, from: &str, to: &str) {
    s.push_str("  <alias binding=\"strong\">\n");
    s.push_str(&format!("    <family>{}</family>\n", xml_escape(from)));
    s.push_str("    <prefer>\n");
    s.push_str(&format!("      <family>{}</family>\n", xml_escape(to)));
    s.push_str("    </prefer>\n");
    s.push_str("  </alias>\n");
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn looks_mono(name: &str) -> bool {
    let l = name.to_ascii_lowercase();
    l.contains("mono") || l.contains("code")
}

fn looks_serif(name: &str) -> bool {
    name.to_ascii_lowercase().contains("serif") && !name.to_ascii_lowercase().contains("sans")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_is_well_formed_enough() {
        let xml = render("Berkeley Mono", "Berkeley Mono", "Berkeley Mono", Some("MesloLGS Nerd Font"));
        assert!(xml.contains("<alias binding=\"strong\">"));
        assert!(xml.contains("<family>sans-serif</family>"));
        assert!(xml.contains("<family>Berkeley Mono</family>"));
        assert!(xml.contains("Noto Color Emoji"));
        assert!(!xml.contains("<alias binding=\"strong\">\n    <family>Noto Color Emoji"));
        assert!(xml.contains("MesloLGS Nerd Font"));
        // no unmatched tags in the snippet (file wrapper adds fontconfig)
        let opens = xml.matches("<alias").count();
        let closes = xml.matches("</alias>").count();
        assert_eq!(opens, closes);
    }
}
