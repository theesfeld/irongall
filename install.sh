#!/usr/bin/env bash
# irongall installer — downloads the latest GitHub release binary.
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/theesfeld/irongall/main/install.sh | bash
#   ./install.sh --dry-run
#   ./install.sh --version v0.1.0 --prefix "$HOME/.local"
set -euo pipefail

REPO="${IRONGALL_REPO:-theesfeld/irongall}"
VERSION=""
PREFIX=""
DRY_RUN=0
BIN_DIR="${IRONGALL_BIN_DIR:-}"

usage() {
  cat <<EOF
install.sh — install irongall from GitHub Releases

Options:
  --version vX.Y.Z   install this tag instead of latest
  --prefix DIR       install root (binary → DIR/bin/irongall)
  --dry-run          print URLs and destination, do not download
  --help             this help

Environment:
  IRONGALL_BIN_DIR   override binary directory (default: \$HOME/.local/bin)
  IRONGALL_REPO      GitHub owner/name (default: theesfeld/irongall)
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION="${2:-}"; shift 2 ;;
    --prefix)  PREFIX="${2:-}"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    --help|-h) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 1 ;;
  esac
done

if [ "$(id -u)" -eq 0 ]; then
  echo "refusing to run as root (do not pipe this script to sudo)" >&2
  exit 1
fi

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"
case "$os-$arch" in
  linux-x86_64|linux-amd64)   target="x86_64-unknown-linux-gnu" ;;
  linux-aarch64|linux-arm64)  target="aarch64-unknown-linux-gnu" ;;
  darwin-x86_64)              target="x86_64-apple-darwin" ;;
  darwin-arm64|darwin-aarch64) target="aarch64-apple-darwin" ;;
  *)
    echo "unsupported platform: $os $arch" >&2
    exit 1
    ;;
esac

if [ -n "$PREFIX" ]; then
  dest_dir="${PREFIX}/bin"
else
  dest_dir="${BIN_DIR:-$HOME/.local/bin}"
fi

if [ -z "$VERSION" ]; then
  tag="latest"
  asset_base="https://github.com/${REPO}/releases/latest/download"
else
  tag="$VERSION"
  asset_base="https://github.com/${REPO}/releases/download/${VERSION}"
fi

# Asset name includes the version when known; latest uses the same pattern
# after resolving the tag. For --dry-run without a tag we still print the
# latest/download URL so the script is useful offline.
if [ -z "$VERSION" ]; then
  tarball_url="${asset_base}/irongall-${target}.tar.gz"
  sums_url="${asset_base}/SHA256SUMS"
else
  ver="${VERSION#v}"
  tarball_url="${asset_base}/irongall-${ver}-${target}.tar.gz"
  sums_url="${asset_base}/SHA256SUMS"
fi

echo "repo      ${REPO}"
echo "tag       ${tag}"
echo "target    ${target}"
echo "tarball   ${tarball_url}"
echo "checksums ${sums_url}"
echo "dest      ${dest_dir}/irongall"

if [ "$DRY_RUN" -eq 1 ]; then
  echo "dry-run: not downloading"
  exit 0
fi

if [ ! -d "$dest_dir" ]; then
  if ! mkdir -p "$dest_dir" 2>/dev/null; then
    echo "cannot write to ${dest_dir}" >&2
    echo "re-run with:  $0 --prefix /usr/local" >&2
    echo "(that path may need sudo — do not pipe this script to sudo; download first)" >&2
    exit 1
  fi
fi
if [ ! -w "$dest_dir" ]; then
  echo "cannot write to ${dest_dir}" >&2
  echo "re-run with:  $0 --prefix /usr/local" >&2
  exit 1
fi

have_curl=0
have_wget=0
command -v curl >/dev/null 2>&1 && have_curl=1
command -v wget >/dev/null 2>&1 && have_wget=1
if [ "$have_curl" -eq 0 ] && [ "$have_wget" -eq 0 ]; then
  echo "need curl or wget" >&2
  exit 1
fi

sum_tool=""
if command -v sha256sum >/dev/null 2>&1; then
  sum_tool="sha256sum"
elif command -v shasum >/dev/null 2>&1; then
  sum_tool="shasum -a 256"
else
  echo "need sha256sum or shasum to verify the download" >&2
  exit 1
fi

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

fetch() {
  local url="$1" out="$2"
  if [ "$have_curl" -eq 1 ]; then
    curl -fsSL "$url" -o "$out"
  else
    wget -qO "$out" "$url"
  fi
}

echo "downloading…"
if ! fetch "$tarball_url" "$tmpdir/irongall.tar.gz"; then
  # Fallback: versioned name from latest tag if the short name 404s.
  if [ -z "$VERSION" ]; then
    echo "short asset name missing; trying GitHub API for the latest tag" >&2
    api="https://api.github.com/repos/${REPO}/releases/latest"
    if [ "$have_curl" -eq 1 ]; then
      tag_name="$(curl -fsSL "$api" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
    else
      tag_name="$(wget -qO- "$api" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)"
    fi
    if [ -z "$tag_name" ]; then
      echo "could not resolve latest release for ${REPO}" >&2
      exit 1
    fi
    ver="${tag_name#v}"
    tarball_url="https://github.com/${REPO}/releases/download/${tag_name}/irongall-${ver}-${target}.tar.gz"
    sums_url="https://github.com/${REPO}/releases/download/${tag_name}/SHA256SUMS"
    fetch "$tarball_url" "$tmpdir/irongall.tar.gz"
  else
    echo "failed to download ${tarball_url}" >&2
    exit 1
  fi
fi
fetch "$sums_url" "$tmpdir/SHA256SUMS" || {
  echo "failed to download SHA256SUMS" >&2
  exit 1
}

(
  cd "$tmpdir"
  fname="$(basename "$tarball_url")"
  # SHA256SUMS may list either the versioned or short name.
  if ! grep -q "$fname" SHA256SUMS && ! grep -q "irongall.tar.gz" SHA256SUMS; then
    # match any line containing the target triple
    if ! grep -q "$target" SHA256SUMS; then
      echo "SHA256SUMS does not mention ${target}" >&2
      cat SHA256SUMS >&2
      exit 1
    fi
  fi
  # Verify by hashing the file and looking up the digest.
  digest="$($sum_tool irongall.tar.gz | awk '{print $1}')"
  if ! grep -qi "$digest" SHA256SUMS; then
    echo "checksum mismatch for ${fname}" >&2
    echo "got ${digest}" >&2
    cat SHA256SUMS >&2
    exit 1
  fi
)

mkdir -p "$tmpdir/extract"
tar -xzf "$tmpdir/irongall.tar.gz" -C "$tmpdir/extract"
bin="$(find "$tmpdir/extract" -type f -name irongall | head -n1)"
if [ -z "$bin" ]; then
  echo "tarball did not contain an irongall binary" >&2
  exit 1
fi
install -m 0755 "$bin" "${dest_dir}/irongall"

# Completions, if present in the tarball and the dest dirs exist.
fish_comp="$(find "$tmpdir/extract" -name 'irongall.fish' | head -n1 || true)"
bash_comp="$(find "$tmpdir/extract" -name 'irongall.bash' -o -name 'irongall' | grep -E 'bash|completions' | head -n1 || true)"
if [ -n "$fish_comp" ] && [ -d "${HOME}/.config/fish/completions" ]; then
  install -m 0644 "$fish_comp" "${HOME}/.config/fish/completions/irongall.fish"
fi
if [ -n "$bash_comp" ] && [ -d "${HOME}/.local/share/bash-completion/completions" ]; then
  install -m 0644 "$bash_comp" "${HOME}/.local/share/bash-completion/completions/irongall"
fi

echo "installed ${dest_dir}/irongall"
if [ -x "${dest_dir}/irongall" ]; then
  "${dest_dir}/irongall" --version || true
fi

case ":${PATH}:" in
  *":${dest_dir}:"*) ;;
  *)
    echo "add ${dest_dir} to PATH, e.g.:"
    echo "  fish:  fish_add_path ${dest_dir}"
    echo "  bash:  export PATH=\"${dest_dir}:\$PATH\""
    ;;
esac
