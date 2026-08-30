# Homebrew formula for Fish.
# Local test:  brew install --build-from-source packaging/fish.rb
# Tap install (after creating homebrew-fish tap):
#   brew tap requla11/fish https://github.com/requla11/homebrew-fish
#   brew install fish
# Update `version`/`sha256` in the release pipeline.

class Fish < Formula
  desc "Fast, flexible, cache-first build orchestration system for Rust and beyond"
  homepage "https://github.com/requla11/fish"
  url "https://github.com/requla11/fish/archive/refs/tags/v0.5.0.tar.gz"
  sha256 "70f7274ac5262c6e0517842a1a6bfcb16ffad5d8c7a912ad1ed8522856920219"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/fish-cli")
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/fish --version")
  end
end
