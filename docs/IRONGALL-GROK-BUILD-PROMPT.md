# Grok Build prompt — `irongall`

Copy everything below the line into a **new empty git repo** and send it as the first message.

---

Build **irongall**, a native Linux CLI + TUI that makes **one 16-color theme, one typeface, and one font size** the system default — then **finds installed programs** that understand those knobs and lets the user **tweak any one of them** without breaking the global setting.

I am on CachyOS (Arch) + Hyprland + Wayland. Fish shell. Kitty is the main terminal; also Alacritty and Ghostty. GTK 3/4 apps, Qt 6 via `qt6ct`, some KDE apps via `kdeglobals`. Do **not** depend on any desktop shell, bar, or rice GUI. Do **not** generate palettes from wallpapers. Do **not** build a graphical app.

Ship a real, runnable tool in this repo. Rust. `clap` for CLI, `ratatui` + `crossterm` for TUI. Apache-2.0 or MIT. Native binary only — no Flatpak, Snap, AppImage, Electron, or Node.

## Why this exists

Linux has fontconfig for fonts and nothing equivalent for colors. Existing tools (`tinty`, `flavours`, `wallust`, `pywal`) either only do colors, or require the user to write templates, or are wallpaper-driven. **irongall** is the missing “picker”: browse a repo of 16-color schemes and libre fonts, preview them, apply globally, including size.

Differentiator vs tinty: **fonts and size are first-class**, **apply works out of the box** on a normal Arch/Hyprland box with zero template authoring, and it **discovers what is actually installed** so the user is not staring at a list of 70 templates they do not have.

## Non-goals (v1)

- No GUI, no daemon, no background watcher.
- No wallpaper sampling / Material You / image extraction.
- No icon theme, cursor theme, or GTK widget-theme generation (Adwaita stays; we only set colors + font + size).
- Apply engine is **Linux-only** (fontconfig, GTK, Qt, Hyprland, etc.). Do not spend v1 on a macOS/Windows apply backend.
- Homebrew on macOS may still ship the **binary** so `brew install` works; `irongall apply` on macOS should print a clear “Linux only” error until a later port.
- No Windows.
- No paid marketplace, accounts, or telemetry.
- Do not redistribute commercial/proprietary fonts. Marketplace fonts are OFL/SIL/Ubuntu/Apache licensed only. Local import exists for fonts the user already owns (e.g. a purchased mono).
- Do not overwrite whole config files. Patch managed markers.

## Product

```
irongall              # launches TUI
irongall tui
irongall status
irongall apply [--theme NAME] [--font FAMILY] [--size PT]
irongall rollback
irongall theme list|show|apply|search|install
irongall font  list|show|apply|search|install
irongall size  set <pt>
irongall apps                         # discover installed themable programs
irongall app list [--all]             # installed by default; --all includes missing
irongall app show <id>
irongall app set  <id> [--theme NAME] [--font FAMILY] [--size PT] [--follow|--hold]
irongall app reset <id>               # back to inheriting global
irongall app skip <id>                # leave this program entirely alone
irongall market update
irongall preview theme <name>
```

Applying theme / font / size rewrites **every discovered, non-skipped program** using the global values unless that program has a tweak. `irongall apply` with no args reapplies the current selection (global + per-app). `irongall app set kitty --size 13` writes only Kitty, leaves GTK/Qt/etc. on the global size.

### Current selection (source of truth)

`~/.config/irongall/config.toml`:

```toml
[theme]
name = "heartbox"
variant = "dark"          # dark | light (if scheme provides both)

[font]
family = "Berkeley Mono"  # used for sans, serif, AND monospace
# optional overrides; if omitted, family is used for all three
# sans = "Inter"
# serif = "Source Serif 4"
# mono = "Berkeley Mono"
size = 11.0               # points; this is the global size
# optional:
# terminal_size = 11.0    # if set, terminals use this instead of size
# ui_size = 11.0          # if set, GTK/Qt/KDE use this instead of size

# Optional per-program tweaks. Omitted keys inherit the global [theme]/[font].
# follow = true  (default) — inherit whatever is not overridden
# follow = false — freeze this program at the last applied values (hold)
# skip   = true  — never touch this program's files
#
# [apps.kitty]
# size = 13.0
#
# [apps.zed]
# font = "Berkeley Mono"
# size = 15.0
#
# [apps.neovim]
# skip = true
```

State lives in `~/.local/share/irongall/` (installed schemes, font cache, market index, apply history, last discovery). Generated snippets live in `~/.config/irongall/generated/`.

Default: **one family, one size, everywhere**. Per-app tweaks are the escape hatch, not the default path.

## Color format

Use **Tinted Theming Base16** as the on-disk scheme format (YAML with `system: base16` and `palette.base00`–`base0F`). Optionally accept Base24 (`base10`–`base17`) when present; if only Base16, derive bright ANSI from the 16.

Do **not** invent a new palette format. Vendor a starter set of ~20 schemes in-repo under `schemes/` so the tool works offline. On `irongall market update`, fetch the rest from a **git/HTTP index** (see Marketplace).

Map Base16 → system roles like this (document in README):

| Role | Base16 |
|---|---|
| background / surface | `base00` |
| darker / border | `base01` |
| selection / highlight | `base02` |
| comments | `base03` |
| foreground | `base05` |
| accent | `base0E` or `base0D` (prefer `base0E` magenta/accent, fallback `base0D` blue) |
| red/green/yellow/blue/magenta/cyan | `base08`–`base0D` |
| orange / brown | `base09` / `base0F` |

Infer `prefer-dark` vs `prefer-light` from `base00` luminance.

## Font + size apply (this is as important as colors)

**Family**

1. Resolve via `fc-list` / fontconfig. Refuse to apply a family that is not installed, unless the user just installed it from the market in the same operation.
2. Write `~/.config/fontconfig/conf.d/50-irongall.conf` (not `/etc`, user-level) that:
   - strong-aliases `sans-serif`, `serif`, `monospace`, `system-ui` to the chosen family (or the sans/serif/mono overrides)
   - aliases common named fonts (Noto Sans, Adwaita Sans, Cantarell, Arial, Helvetica, Inter, etc.) to the chosen family
   - does **not** alias `Noto Color Emoji`, `Noto Sans CJK *`, or Nerd Font / icon families
   - after the chosen family, prefer a Nerd Font already on the system (detect `MesloLGS Nerd Font`, `FantasqueSansM Nerd Font`, etc.) so terminal icons survive, then CJK, then emoji
3. `fc-cache -f` for the user.

**Size** (`size` in points, float allowed: `10`, `11`, `12.5`)

Write the same number (or the ui/terminal overrides) into every target that has a size knob:

- GTK 3/4 `settings.ini`: `gtk-font-name=Family 11`
- `~/.gtkrc-2.0`: `gtk-font-name="Family 11"`
- `gsettings`: `org.gnome.desktop.interface {font-name,document-font-name,monospace-font-name}` and `org.gnome.desktop.wm.preferences titlebar-font`
- `xsettingsd` if the file exists: `Gtk/FontName`
- qt6ct/qt5ct `[Fonts] general` and `fixed`
- `kdeglobals` `[General] font, menuFont, toolBarFont, fixed` and `[WM] activeFont`
- Kitty: `font_size`
- Alacritty: `[font] size`
- Ghostty: `font-size`
- Hyprland: only if a font size key already exists in the user’s config; do not invent a dummy `misc:font_size`

TUI control: a size stepper (←/→ or `[` / `]`) in 0.5pt increments, range 8–24, live preview of the number, apply on Enter.

CLI: `irongall size set 12` must change size **without** requiring a theme re-pick.

## Apply engine (colors)

Patch, don’t clobber. Every file irongall touches gets a managed region:

```
# IRONGALL-BEGIN
...generated...
# IRONGALL-END
```

(CSS uses `/* IRONGALL-BEGIN */` … `/* IRONGALL-END */`.)

If the region exists, replace it. If not, append it. Never delete user config outside the markers. Keep a timestamped backup of each file in `~/.local/share/irongall/backups/<iso>/` before first patch of a session. `irongall rollback` restores the last session.

**v1 targets** (skip silently if the app/config is absent; print what was done):

1. **fontconfig** — as above
2. **GTK 3/4** — `~/.config/gtk-3.0/gtk.css` and `gtk-4.0/gtk.css`: libadwaita `@define-color` named colors from the scheme (window_bg, view_bg, accent_bg, headerbar, popover, destructive, theme_selected_*). Also set `gtk-font-name` in `settings.ini`.
3. **gsettings** — font names + sizes; `color-scheme` prefer-dark/light from luminance
4. **qt6ct / qt5ct** — color scheme file under `~/.config/qt6ct/colors/irongall.conf` + `[Fonts]`
5. **kdeglobals** — ColorEffects/Colors:* from the palette + font keys
6. **Kitty** — `include` a generated `~/.config/kitty/themes/irongall.conf` (palette, bg, fg, cursor, selection) plus `font_family` / `font_size` / `disable_ligatures never`. Nerd Font `symbol_map` if a nerd font is installed. `pkill -USR1 kitty` after write.
7. **Ghostty** — font-family, font-size, 16 palette keys in `~/.config/ghostty/config` inside markers
8. **Alacritty** — `[font]` + `[colors]` inside markers or a generated file imported if they already use `import`
9. **Xresources** — `*.foreground/background/cursorColor/color0-15` if `~/.Xresources` exists; `xrdb -merge`
10. **Hyprland** — patch `general:col.active_border` / `inactive_border` from accent/surface **only if** hyprland config is present and already has those keys or a `IRONGALL-BEGIN` block

Print a report:

```
applied  theme=heartbox  font=Berkeley Mono  size=11
  gtk3        ok
  kitty       ok (reloaded)
  hyprland    skipped (no config)
```

After apply, `fc-match sans-serif` and `fc-match monospace` must resolve to the chosen family.

## Discovery + per-program tweaks

This is a v1 feature, not a later extra. Global apply without a list of *what is on this machine* is how other theme managers feel blind.

### Catalog

Ship a built-in catalog of adapters in `crates/irongall-core/src/catalog.rs` (or YAML under `catalog/`). Each entry:

```toml
id = "kitty"
name = "Kitty"
kind = "terminal"          # terminal | editor | system | compositor | cli | browser
theme = true               # understands 16-color / palette
font = true
size = true
binaries = ["kitty"]       # any on PATH => likely installed
config_globs = ["~/.config/kitty/kitty.conf"]
desktop_ids = ["kitty.desktop"]
reload = "signal-usr1"     # none | signal-usr1 | gsettings | command:…
```

**v1 catalog (implement adapters for these; discovery still lists them even before every writer is done):**

System: `fontconfig`, `gtk3`, `gtk4`, `gsettings`, `qt6ct`, `qt5ct`, `kdeglobals`, `xresources`, `xsettingsd`
Compositor: `hyprland`
Terminals: `kitty`, `ghostty`, `alacritty`, `foot`, `wezterm`
Editors: `neovim`, `zed`, `helix`, `micro`
CLI: `btop`, `cava`, `starship`, `bat`, `yazi`, `lazygit`, `zathura`
Browsers: `firefox` (userChrome/content only if a chrome dir exists — otherwise mark `unsupported` not `skipped`)

If an adapter is catalogued but its writer is not implemented yet, discovery still shows it as `installed / no-writer` rather than hiding it. Never pretend to theme an app you cannot write.

### How “installed” is decided

A program is **present** if **any** of these hit:

1. One of `binaries` is on `PATH`
2. One of `config_globs` exists
3. A matching `.desktop` is in `~/.local/share/applications` or `/usr/share/applications`
4. Optional: `pacman -Qq <pkg>` if the catalog lists `packages = ["kitty"]` and pacman exists

A program is **themable** if present AND `theme || font || size` is true AND a writer exists.

Do **not** scan the whole filesystem. Do **not** require the user to name programs. `irongall apps` is a scan of the catalog against the live system, cached in `~/.local/share/irongall/discovery.json` with a timestamp; rescanned on TUI open and on `irongall apps`.

Output of `irongall apps`:

```
id         name        kind       state          theme          font                 size
kitty      Kitty       terminal   tweak          heartbox       Berkeley Mono        13
ghostty    Ghostty     terminal   global         heartbox       Berkeley Mono        11
alacritty  Alacritty   terminal   missing        —              —                    —
zed        Zed         editor     global         heartbox       Berkeley Mono        11
neovim     Neovim      editor     skip           —              —                    —
gtk3       GTK 3       system     global         heartbox       Berkeley Mono        11
firefox    Firefox     browser    no-writer      —              —                    —
```

States:

| state | meaning |
|---|---|
| `global` | present, writer exists, no override — gets the global theme/font/size |
| `tweak` | present, at least one of theme/font/size overridden |
| `hold` | `follow = false` — frozen at last applied values, ignore later global changes |
| `skip` | user said leave it alone |
| `missing` | not installed |
| `no-writer` | installed but irongall cannot write it yet |

### Per-program tweaks

Each app may override **any subset** of `{theme, font, size}`. Unset keys inherit global.

Examples:

- Kitty 13pt, everything else 11pt: `[apps.kitty] size = 13.0`
- Zed keeps Berkeley Mono but GTK uses Inter: global `family = "Inter"`, `[apps.zed] font = "Berkeley Mono"`
- Neovim uses a different scheme than the desktop: `[apps.neovim] theme = "gruvbox-dark-hard"`
- Don’t touch Ghostty at all: `[apps.ghostty] skip = true`

`irongall app reset kitty` deletes `[apps.kitty]` so it follows global again.

`irongall apply` resolution order per program:

1. If `skip` → do not write
2. If `missing` → do not write
3. If `no-writer` → do not write, list it
4. Else effective_theme/font/size = app override ?? global
5. If `hold` and the app was applied before, reuse last written values instead of recomputing from a new global

Writers must use the **effective** values, not only the global ones. That is how Kitty can be 13pt while `gsettings` stays 11.

### TUI: Apps pane

Add **Apps** to the left nav (Themes / Fonts / Size / Apps / Market / Status).

- Default filter: **present** programs only (not `missing`). Toggle `m` to show missing, `w` to show no-writer.
- Columns: name, kind, state, effective theme, effective font, effective size
- Preview: which files would be patched, last apply result, inherit vs override chips
- Keys:
  - `t` / `f` / `s` — set theme / font / size override for the highlighted app (opens the same pickers as the global panes, but scoped)
  - `c` — clear that app’s overrides (`reset`)
  - `x` — toggle skip
  - `h` — toggle hold
  - `a` — apply **only this app**
  - Enter — detail view

Global Apply (`A` on Status) still means “apply everyone who is not skip/missing/no-writer”, using each app’s effective values.

### What not to do

- Do not auto-detect “any random electron app”. Catalog is allow-list.
- Do not let per-app fontconfig aliases fight the global fontconfig file. Per-app font/size is done **in that app’s own config** (kitty.conf, zed settings.json, alacritty.toml). fontconfig / gsettings / gtk settings.ini are the **system** adapters, not per-app, unless a later writer exists for an app-specific GTK CSS file.
- Do not store a second copy of the whole palette per app unless the theme override is set; inherit by reference.

## TUI

Launch with `irongall` or `irongall tui`. Keyboard-only. No mouse required. Must look good in a 80×24 terminal, better in 120×40.

**Layout**

- Left: navigation (Themes / Fonts / Size / Apps / Market / Status / Apply)
- Center: list (filterable)
- Right: preview pane

**Themes list**

- Name, author, dark/light, installed vs market
- Preview pane: 16 color chips (`base00`–`base0F`) drawn as ratatui cells; a fake window (titlebar, body text, selection, comment, string, keyword, error) using those colors; ANSI 0–15 strip
- `/` to filter
- Enter: set as current theme (does not apply until Apply)
- `a`: set + apply immediately

**Fonts list**

- Installed fonts from fontconfig (dedupe family names, skip last-resort / emoji / cjk in the main list, put them in a “other” group)
- Market fonts (not installed) greyed with license + source
- Preview pane: family, styles/weights found, license, pangram `The quick brown fox jumps over the lazy dog 0123456789 => != ===`, and a note that glyph rendering uses the **current terminal font** (cannot switch terminal font without apply). If `KITTY_WINDOW_ID` is set, optionally render a pangram PNG via `cosmic-text` + kitty graphics protocol; this is a stretch goal, not a blocker.
- Enter: set family
- `i`: install from market then set

**Size**

- Big number, stepper, preview of GTK-style string `Berkeley Mono 11`
- Shows both UI size and terminal size if overrides are set

**Market**

- Tabs: Schemes / Fonts
- Search
- Install (git clone or download zip to `~/.local/share/irongall/`)
- Show license. Refuse non-libre fonts.

**Apps** — see “Discovery + per-program tweaks”. This pane is how most people will live with the tool after the first global apply.

**Status / Apply**

- Current **global** theme, font, size
- Count of apps: `12 global · 2 tweaked · 1 skipped · 4 missing`
- Target list with last-apply result (include per-app effective size/font if tweaked)
- `A` apply all, `R` rollback, `q` quit
- Confirm apply if more than N files will change (show the list, including which are tweaks)

Keybindings in a footer. `?` help. Theme the TUI itself from the **selected** (not yet applied) scheme so browsing feels live.

## Marketplace (no money)

v1 is **not** a custom server. It is a JSON index + git repos.

In this repo include `market/index.json` with this shape:

```json
{
  "version": 1,
  "schemes": [
    {
      "name": "heartbox",
      "url": "https://…/heartbox.yaml",
      "license": "MIT",
      "system": "base16",
      "preview": ["#0A1528", "#E03818", "#1E8AE8"]
    }
  ],
  "fonts": [
    {
      "family": "JetBrains Mono",
      "license": "OFL-1.1",
      "source": "https://github.com/JetBrains/JetBrainsMono/releases/latest",
      "install": "github-release-zip",
      "notes": "installs into ~/.local/share/fonts/irongall/JetBrainsMono"
    }
  ]
}
```

`irongall market update` pulls the index from a configurable URL, defaulting to the in-repo file so offline works. Fonts install into `~/.local/share/fonts/irongall/<family>/` then `fc-cache`. Schemes install into `~/.local/share/irongall/schemes/`.

Ship at least these schemes in-tree so demos work with no network: default-dark, default-light, gravas, tokyo-night, catppuccin-mocha, dracula, nord, gruvbox-dark-hard, one-dark, solarized-dark, plus a `heartbox` scheme using:

```
base00 #0A1528
base01 #142238
base02 #4E1A22
base03 #2A3548
base04 #6E7A8A
base05 #EDE6DE
base06 #E8E2D8
base07 #B8C0C8
base08 #E03818
base09 #D4A838
base0A #E0BC55
base0B #3D9650
base0C #16A8B6
base0D #1E8AE8
base0E #D47A82
base0F #F05028
```

Ship at least metadata (not the files) for these fonts: JetBrains Mono, IBM Plex Mono, Fira Code, Iosevka (if OFL), Noto Sans, Noto Sans Mono, Inter, Source Serif 4. Actual download on install.

## CLI details

- `irongall status` prints global theme/font/size, `fc-match` results, and a one-line apps summary
- `irongall apps --json` / `irongall app list --json` for scripting
- `irongall preview theme <name>` prints a 16-color block using ANSI truecolor in the terminal (no TUI)
- `--dry-run` on apply and `app set` shows diffs
- Exit codes: 0 ok, 1 user error, 2 apply partial failure (still report which targets succeeded)
- Fish + bash completions

## Repo layout

```
Cargo.toml                  # workspace
crates/irongall/            # binary
crates/irongall-core/       # scheme parse, fontconfig, apply, market, catalog, discovery
crates/irongall-tui/
schemes/                    # vendored base16 yaml
market/index.json
templates/                  # handlebars/tera for each target
install.sh                  # curl | bash installer (GitHub releases)
packaging/aur/              # PKGBUILD + .SRCINFO (source) and PKGBUILD-bin
packaging/homebrew/         # Formula/irongall.rb for a tap
.github/workflows/release.yml
README.md                   # install methods first, then usage
LICENSE
```

Use `thiserror`, `serde`, `serde_yaml`, `toml`, `dirs`, `reqwest` (rustls), `hex_color` or similar. Call `fc-list` / `fc-match` / `fc-cache` / `gsettings` as subprocesses; do not link fontconfig unless it is painless.

## Implementation order (do this in order, keep it compiling)

1. Cargo workspace, `irongall --help`, `irongall status` stub, config load/save.
2. Base16 YAML load + vendored schemes + `irongall theme list/show` + ANSI preview.
3. Font discovery via `fc-list`, `irongall font list`, size in config.
4. Apply engine: fontconfig + GTK settings.ini + gsettings fonts/size. Prove `fc-match sans-serif` changes.
5. Catalog + discovery (`irongall apps`) against PATH / config globs / desktop files. No writers needed yet besides the system ones.
6. Kitty + Alacritty + Ghostty apply (colors, family, size) with IRONGALL markers + kitty USR1, using **effective** per-app values.
7. Per-app overrides in config: `irongall app set kitty --size 13` then `irongall apply` leaves GTK at 11 and Kitty at 13. `reset` / `skip` / `hold`.
8. GTK CSS colors, qt6ct, kdeglobals, xresources. Editors that are easy: zed `settings.json` (ui_font_family, buffer_font_family, sizes), neovim `guifont` plus a generated base16 colorscheme file if `~/.config/nvim` exists (do not wreck LazyVim — write `~/.config/nvim/lua/irongall.lua` and only add a one-line require inside IRONGALL markers in `options.lua`).
9. Backup + rollback (including per-app files).
10. TUI: theme browser, font list, size stepper, **Apps pane**, apply/status.
11. Market index + scheme install + libre font install into `~/.local/share/fonts/irongall`.
12. README, completions, tests.
13. Packaging: `install.sh`, AUR PKGBUILDs, Homebrew formula, crates.io metadata, GitHub release workflow. Do not actually publish to AUR/Homebrew/crates.io from this session (no credentials). Leave the files + a `packaging/PUBLISH.md` checklist the maintainer can run.

Do not start the TUI before step 4 works. A TUI that cannot apply is a toy.

## Tests

- Parse every vendored scheme.
- Marker replace is idempotent (applying twice does not duplicate blocks).
- Rollback restores a fixture file.
- `size` formats as GTK expects: `Berkeley Mono 11` and `Berkeley Mono 11.5`.
- Fontconfig XML is well-formed.
- Dark/light inference on black vs white `base00`.
- Discovery fixture: fake PATH + temp config dirs mark kitty present, foot missing.
- Effective value: global size 11 + `[apps.kitty] size = 13` → kitty 13, gtk 11.
- `skip` means the writer is not called.
- `reset` removes the override table.

## Packaging and installation

Users must be able to install without cloning and building by hand. **No Flatpak, Snap, AppImage, or other sandboxed/portable formats.** Native binary, native package managers.

README **Install** section order: Arch/CachyOS (AUR) → Homebrew → crates.io → curl/bash → from source. Each block is copy-pasteable.

Placeholder GitHub owner is `irongall-dev` and crate/bin name is `irongall` everywhere. Call that out in `packaging/PUBLISH.md` so the real owner can search-replace.

### 1) curl | bash (GitHub release binary)

`install.sh` at repo root. Intended use:

```sh
curl -fsSL https://raw.githubusercontent.com/irongall-dev/irongall/main/install.sh | bash
```

Requirements:

- Detects `linux-x86_64` / `linux-aarch64` (and `darwin-*` only if a release asset exists).
- Downloads the matching tarball + `SHA256SUMS` from the **latest GitHub release**, verifies the checksum, installs `irongall` to `${IRONGALL_BIN_DIR:-$HOME/.local/bin}`.
- **No `sudo` by default.** If the dest is not writable, error with the exact command to use `--prefix /usr/local` (that path may need sudo).
- Refuses to pipe-run as root.
- Installs fish/bash completions when those dirs exist (`~/.local/share/bash-completion/completions`, `~/.config/fish/completions`).
- Prints `irongall --version` and “add ~/.local/bin to PATH” if it is missing from PATH.
- `--version vX.Y.Z`, `--dry-run`, `--prefix DIR`.
- Uses `curl` or `wget`; fails loud if neither checksum tool (`sha256sum` / `shasum`) exists.

GitHub Actions `.github/workflows/release.yml`: on tag `v*.*.*`, `cargo build --release` for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`, pack `irongall-$ver-$target.tar.gz` (binary + LICENSE + completions), write `SHA256SUMS`, upload via `gh release create`. Cross from an x86_64 runner with `cross` or `cargo zigbuild` is fine; document the choice.

### 2) AUR (Arch / CachyOS)

Ship both a source package and a binary package. Files:

- `packaging/aur/irongall/PKGBUILD` — builds from the GitHub source tarball / crates.io crate with `cargo build --release --locked`. `depends` on things the binary actually needs at runtime (`fontconfig`, `gcc-libs`). `makedepends=('cargo')`.
- `packaging/aur/irongall-bin/PKGBUILD` — installs the GitHub release tarball, `sha256sums` filled from `SHA256SUMS`.
- Generate `.SRCINFO` with `makepkg --printsrcinfo`.
- `pkgname` is `irongall` / `irongall-bin`. `arch=('x86_64' 'aarch64')` for `-bin`; source pkg can be `arch=('x86_64' 'aarch64')` too if it is pure cargo.
- Install the binary to `/usr/bin/irongall`, completions to the distro paths, manpage if you wrote one.
- Do **not** `makepkg -si` against the user’s system as a side effect of the build session unless asked.

`packaging/PUBLISH.md` AUR section, exact steps:

```text
# one-time
git clone ssh://aur@aur.archlinux.org/irongall.git
# copy PKGBUILD + .SRCINFO, commit, git push
# repeat for irongall-bin
```

README user-facing:

```sh
paru -S irongall        # or irongall-bin
# yay -S irongall
```

### 3) Homebrew

Do **not** assume a homebrew-core merge for v1. Ship a **tap formula**.

- `packaging/homebrew/irongall.rb` — `url` is the GitHub release tarball (or source tarball + cargo), `sha256`, `depends_on` only what is needed. Linux bottle optional; macOS bottle optional. A source-build formula that uses the cargo crate is acceptable if prebuilt Darwin assets are not ready.
- Document the tap repo name `homebrew-irongall` and:

```sh
brew tap irongall-dev/irongall
brew install irongall
```

- `packaging/PUBLISH.md` Homebrew section: create `homebrew-irongall` on GitHub, put `Formula/irongall.rb` there, bump `url`/`sha256` on each release. Optional later: a release workflow job that opens a tap PR.

Linuxbrew/Homebrew-on-Linux must work (this is a Linux tool). macOS formula may exist for `cargo install`-equivalent convenience; apply remains Linux-only.

### 4) crates.io

- The publishable binary crate is `crates/irongall` with `name = "irongall"` (so `cargo install irongall` gets the CLI).
- `Cargo.toml`: `license`, `repository`, `description`, `readme`, `keywords` (`theme`, `font`, `base16`, `tui`), `categories` (`command-line-utilities`).
- `publish = true` only on the bin crate; core/tui can be `publish = false` if they are path deps, **or** publish them as `irongall-core` / `irongall-tui` if the bin crate cannot otherwise be packaged. Prefer **one** crates.io package (`irongall`) with the workspace inlined or with published deps — pick one and document it. Locked builds must work: commit `Cargo.lock` for the bin.
- README:

```sh
cargo install irongall --locked
```

- `packaging/PUBLISH.md` crates.io section:

```text
cargo login
cargo publish -p irongall-core   # if needed
cargo publish -p irongall-tui    # if needed
cargo publish -p irongall
```

Do not run `cargo publish` in the build session.

Also document the git form for people who do not want crates.io:

```sh
cargo install --git https://github.com/irongall-dev/irongall --locked
```

### 5) Other common methods (include in README, implement what is cheap)

**From source (git clone)** — required:

```sh
git clone https://github.com/irongall-dev/irongall
cd irongall
cargo build --release
install -Dm755 target/release/irongall ~/.local/bin/irongall
```

**GitHub Releases** — manual download of the same tarball `install.sh` uses. Link the releases page from README.

**Nix flake** — add `flake.nix` exposing `packages.default` = the rust package, and:

```sh
nix run github:irongall-dev/irongall
nix profile install github:irongall-dev/irongall
```

This is worth doing; Nix users expect it. Keep it a thin `crane` or `rustPlatform.buildRustPackage` flake, not a science project.

**Do not** add: Flatpak, Snap, AppImage, Distrobox-as-a-package, Ubuntu PPAs, Windows installers, Scoop/Chocolatey/WinGet (no Windows).

Optional one-liners in README only (no extra work unless trivial): Fedora Copr / Gentoo GURU / Alpine aport — list as “not yet”, not fake commands.

`packaging/PUBLISH.md` is the maintainer checklist: bump version in Cargo.toml → tag `vX.Y.Z` → Actions builds assets → update AUR checksums / Homebrew sha256 → `cargo publish`. Version is a single number shared by crate, AUR `pkgver`, and git tag.

## README must say

- Install methods (AUR, Homebrew, cargo, curl/bash, source, Nix) with working commands.
- This patches user config; use `irongall rollback`.
- Commercial fonts: `irongall font import /path/to/ttf-dir` then apply by family name.
- CJK and emoji are fallbacks, never replaced.
- Open apps may need a restart except Kitty (USR1).
- `irongall apps` is the inventory; `irongall app set` is the exception. Global remains the default.
- Apply is Linux-only.

When you are done, `cargo build --release` must produce `target/release/irongall`, `irongall --help` must work, `irongall tui` must open, and `irongall apply --theme heartbox --size 11` must be safe to run on this machine (markers only, backups first). `install.sh --dry-run` must run without erroring on missing releases (print the URL it would fetch). `packaging/aur/irongall/PKGBUILD` must be syntactically valid (`bash -n`). Homebrew formula must be valid Ruby (`ruby -c` if ruby exists).
