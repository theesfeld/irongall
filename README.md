# irongall

One 16-color theme, one typeface, one font size — the system default on Linux.
Then a list of **installed** programs that understand those knobs, so you can
tweak any one of them without breaking the global setting.

Linux has fontconfig for fonts and nothing equivalent for colors. irongall is
the missing picker: browse a repo of 16-color schemes and libre fonts, preview
them, apply globally (including size). Fonts and size are first-class. Apply
works out of the box on a normal Arch/Hyprland box with zero template authoring.

Apply is **Linux-only**. The binary may be installed on macOS via Homebrew;
`irongall apply` there prints a clear error until a later port. No Windows.

This tool **patches user config** inside `IRONGALL-BEGIN` / `IRONGALL-END`
markers and keeps timestamped backups. Use `irongall rollback` if something
looks wrong. Open apps other than Kitty usually need a restart (Kitty reloads
on USR1).

## Install

Native binary only. No Flatpak, Snap, or AppImage.

### Arch / CachyOS (AUR)

```sh
paru -S irongall        # or irongall-bin
# yay -S irongall
```

### Homebrew

```sh
brew tap theesfeld/irongall
brew install irongall
```

Linuxbrew / Homebrew-on-Linux is the intended path. macOS can install the
binary; apply remains Linux-only.

### crates.io

```sh
cargo install irongall --locked
```

From git, if you do not want crates.io:

```sh
cargo install --git https://github.com/theesfeld/irongall --locked
```

### curl | bash (GitHub release)

```sh
curl -fsSL https://raw.githubusercontent.com/theesfeld/irongall/main/install.sh | bash
```

Installs to `~/.local/bin` (override with `IRONGALL_BIN_DIR` or `--prefix`).
No sudo. Refuses to run as root.

### From source

```sh
git clone https://github.com/theesfeld/irongall
cd irongall
cargo build --release
install -Dm755 target/release/irongall ~/.local/bin/irongall
```

### Nix

```sh
nix run github:theesfeld/irongall
nix profile install github:theesfeld/irongall
```

### GitHub Releases

Manual download of the same tarball `install.sh` uses:
<https://github.com/theesfeld/irongall/releases>

Fedora Copr / Gentoo GURU / Alpine aport: not yet.

## Quick start

```
irongall              # TUI
irongall status
irongall theme list
irongall preview theme heartbox
irongall apply --theme heartbox --size 11
irongall apps         # inventory of this machine
irongall app set kitty --size 13
irongall rollback
```

Global remains the default. `irongall apps` is the inventory;
`irongall app set` is the exception.

## How it works

Current selection lives in `~/.config/irongall/config.toml`:

```toml
[theme]
name = "heartbox"
variant = "dark"

[font]
family = "Berkeley Mono"  # used for sans, serif, AND monospace
size = 11.0

# [apps.kitty]
# size = 13.0
```

State: `~/.local/share/irongall/`. Generated snippets:
`~/.config/irongall/generated/`.

Applying theme / font / size rewrites every discovered, non-skipped program
using the global values unless that program has a tweak.

### Color format

Tinted Theming **Base16** YAML (`system: base16`, `palette.base00`–`base0F`).
Base24 (`base10`–`base17`) is accepted when present.

| Role | Base16 |
|---|---|
| background / surface | `base00` |
| darker / border | `base01` |
| selection / highlight | `base02` |
| comments | `base03` |
| foreground | `base05` |
| accent | `base0E` (fallback `base0D`) |
| red/green/yellow/blue/magenta/cyan | `base08`–`base0D` |
| orange / brown | `base09` / `base0F` |

`prefer-dark` vs `prefer-light` is inferred from `base00` luminance.

CJK and emoji fonts are fallbacks. They are never replaced. Nerd Font
families already on the system are preferred after the chosen family so
terminal icons survive.

### Commercial / owned fonts

Do not put purchased fonts in the market. Import a directory you already own:

```sh
irongall font import /path/to/ttf-dir
irongall font apply "Berkeley Mono"
```

Market fonts are OFL / SIL / Ubuntu / Apache (and similar libre licenses) only.

## CLI

```
irongall tui
irongall status
irongall apply [--theme NAME] [--font FAMILY] [--size PT] [--dry-run]
irongall rollback
irongall theme list|show|apply|search|install
irongall font  list|show|apply|search|install|import
irongall size  set <pt>
irongall apps [--json]
irongall app list [--all] [--json]
irongall app show <id>
irongall app set  <id> [--theme NAME] [--font FAMILY] [--size PT] [--follow|--hold]
irongall app reset <id>
irongall app skip <id>
irongall market update
irongall preview theme <name>
```

Exit codes: `0` ok, `1` user error, `2` apply partial failure (the report still
lists which targets succeeded).

## TUI

Keyboard-only. **Tab** / **Shift-Tab** cycle Themes → Fonts → Size → Apps →
Market → Status. `?` help, `q` quit, `A` apply all, `R` rollback. The TUI
itself is colored from the **selected** (not yet applied) scheme.

## Marketplace

No store, no accounts. `irongall market update` (or `u` in the Market pane)
downloads the [Tinted Theming schemes](https://github.com/tinted-theming/schemes)
zip and rebuilds a local index (palette previews + authors).

| Kind | Source | License |
|---|---|---|
| Color schemes | Tinted Theming Base16 YAML (`spec-0.11/base16`), plus irongall's `heartbox` / `gravas` | The **collection** is MIT (`tinted-theming/schemes/LICENSE`). Individual YAML files almost never carry their own license field; irongall labels those `MIT (tinted-theming collection)` rather than inventing a per-scheme license. Original palette authors are shown as `author`. |
| Fonts | Each project's GitHub releases → `~/.local/share/fonts/irongall/` | Only OFL-1.1 / SIL / Ubuntu Font Licence / MIT / Apache. The label on each font is that project's actual license, not a default. |

Install a scheme or font from the Market pane (`i`) or `irongall theme install NAME` / `irongall font install FAMILY`. Commercial fonts stay out of the market — `irongall font import /path/to/ttf-dir` for fonts you already own.

## License

MIT. See [LICENSE](LICENSE).
