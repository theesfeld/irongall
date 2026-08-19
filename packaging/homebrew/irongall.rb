# frozen_string_literal: true

class Irongall < Formula
  desc "One 16-color theme, one typeface, one font size — applied across Linux"
  homepage "https://github.com/theesfeld/irongall"
  url "https://github.com/theesfeld/irongall/archive/refs/tags/v0.1.5.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  license "MIT"
  head "https://github.com/theesfeld/irongall.git", branch: "main"

  depends_on "rust" => :build
  depends_on "fontconfig"

  def install
    system "cargo", "install", "--locked", "--root", prefix, "--path", "crates/irongall"
    generate_completions_from_executable(bin/"irongall", "completions")
  end

  test do
    assert_match "irongall", shell_output("#{bin}/irongall --help")
  end

  def caveats
    <<~EOS
      irongall apply is Linux-only (fontconfig, GTK, Qt, Hyprland).
      On macOS this formula ships the binary; apply will error until a later port.
    EOS
  end
end
