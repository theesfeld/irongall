# Apply templates

Writers in `crates/irongall-core/src/apply/writers/` emit these snippets
inside `IRONGALL-BEGIN` / `IRONGALL-END` markers. The Rust side is the
source of truth; files here document the shape of each target.

- `fontconfig.xml` — `~/.config/fontconfig/conf.d/50-irongall.conf`
- `gtk.css` — libadwaita `@define-color` names
- `kitty.conf` — palette + font + optional nerd `symbol_map`
- `ghostty.conf` — `palette = N=#rrggbb`
- `alacritty.toml` — `[font]` + `[colors]`
