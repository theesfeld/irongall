mod alacritty;
mod fontconfig;
mod ghostty;
mod gsettings;
mod gtk;
mod helix;
mod hyprland;
mod kdeglobals;
mod kitty;
mod neovim;
mod qtct;
mod wezterm;
mod xresources;
mod xsettingsd;
mod zed;
mod foot;

use crate::apply::{ApplyCtx, TargetStatus};
use crate::error::Result;

pub fn apply_id(id: &str, ctx: &mut ApplyCtx<'_>) -> Result<TargetStatus> {
    match id {
        "fontconfig" => fontconfig::apply(ctx),
        "gtk3" => gtk::apply_gtk3(ctx),
        "gtk4" => gtk::apply_gtk4(ctx),
        "gsettings" => gsettings::apply(ctx),
        "qt6ct" => qtct::apply(ctx, 6),
        "qt5ct" => qtct::apply(ctx, 5),
        "kdeglobals" => kdeglobals::apply(ctx),
        "xresources" => xresources::apply(ctx),
        "xsettingsd" => xsettingsd::apply(ctx),
        "hyprland" => hyprland::apply(ctx),
        "kitty" => kitty::apply(ctx),
        "ghostty" => ghostty::apply(ctx),
        "alacritty" => alacritty::apply(ctx),
        "foot" => foot::apply(ctx),
        "wezterm" => wezterm::apply(ctx),
        "neovim" => neovim::apply(ctx),
        "zed" => zed::apply(ctx),
        "helix" => helix::apply(ctx),
        _ => Ok(TargetStatus::Skipped {
            reason: "no-writer".into(),
        }),
    }
}
