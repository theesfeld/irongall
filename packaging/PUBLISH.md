# Publishing checklist

Placeholder GitHub owner is **`theesfeld`**. Search-replace that with the
real owner before the first release. Vendored schemes live in
`crates/irongall-core/schemes/` (publishable with the crate); `schemes/` and
`market/` at the repo root are symlinks to those directories. Crate and binary name is `irongall`
everywhere. Version is a **single number** shared by crate, AUR `pkgver`,
Homebrew `version`, and the git tag `vX.Y.Z`.

Do **not** run `cargo publish`, `makepkg -si`, or push to AUR/Homebrew from a
build session unless you have credentials and intend to.

## Bump

1. Set `version` in the workspace `Cargo.toml` (`[workspace.package]`).
2. Set `pkgver` in both AUR PKGBUILDs.
3. Set `version` in `packaging/homebrew/irongall.rb`.
4. Commit `Cargo.lock`.
5. Tag: `git tag vX.Y.Z && git push origin vX.Y.Z`
6. GitHub Actions builds `irongall-$ver-$target.tar.gz` + `SHA256SUMS`.

## AUR

```text
# one-time
git clone ssh://aur@aur.archlinux.org/irongall.git
# copy packaging/aur/irongall/PKGBUILD + .SRCINFO, commit, git push
# repeat for irongall-bin
```

After a release:

- Fill `sha256sums` from the GitHub source tarball (`irongall`) and from
  `SHA256SUMS` (`irongall-bin`).
- `makepkg --printsrcinfo > .SRCINFO` in each AUR clone.
- Push.

Do not `makepkg -si` against a contributor machine as a side effect.

## Homebrew

Tap repo name: **`homebrew-irongall`**.

```sh
brew tap theesfeld/irongall
brew install irongall
```

1. Create `homebrew-irongall` on GitHub.
2. Put `Formula/irongall.rb` there (copy from `packaging/homebrew/irongall.rb`).
3. On each release, bump `url` / `sha256` (source tarball or bottle).
4. Optional later: a release-workflow job that opens a tap PR.

Linuxbrew must work. macOS formula may exist for `cargo install`-equivalent
convenience; apply remains Linux-only.

## crates.io

Prefer publishing workspace crates in order, then the bin:

```text
cargo login
cargo publish -p irongall-core
cargo publish -p irongall-tui
cargo publish -p irongall
```

`crates/irongall` is the `cargo install irongall` package.

Do not run `cargo publish` in a build session.

Git form:

```sh
cargo install --git https://github.com/theesfeld/irongall --locked
```

## Nix

`flake.nix` exposes `packages.default`. After a lockfile change,
`nix build` locally before tagging.
