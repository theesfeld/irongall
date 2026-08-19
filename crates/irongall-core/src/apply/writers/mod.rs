mod alacritty;
mod bat;
mod btop;
mod cava;
mod fontconfig;
mod foot;
mod ghostty;
mod gsettings;
mod gtk;
mod helix;
mod hyprland;
mod kdeglobals;
mod kitty;
mod lazygit;
mod micro;
mod neovim;
mod qtct;
mod starship;
mod wezterm;
mod xresources;
mod xsettingsd;
mod yazi;
mod zathura;
mod zed;

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
        "micro" => micro::apply(ctx),
        "btop" => btop::apply(ctx),
        "cava" => cava::apply(ctx),
        "starship" => starship::apply(ctx),
        "bat" => bat::apply(ctx),
        "yazi" => yazi::apply(ctx),
        "lazygit" => lazygit::apply(ctx),
        "zathura" => zathura::apply(ctx),
        _ => Ok(TargetStatus::Skipped {
            reason: "no-writer".into(),
        }),
    }
}
