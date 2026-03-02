class Stale < Formula
  desc "CLI tool to run or skip commands based on file content hashes"
  homepage "https://github.com/th1nkful/stale"
  url "https://github.com/th1nkful/stale/archive/refs/tags/v0.1.0.tar.gz"
  sha256 ""
  license "Apache-2.0"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/stale --version")
  end
end
