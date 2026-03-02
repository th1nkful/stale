class Stale < Formula
  desc "CLI tool to run or skip commands based on file content hashes"
  homepage "https://github.com/th1nkful/stale"
  url "https://github.com/th1nkful/stale/archive/refs/tags/0.2.0.tar.gz"
  sha256 "e571ecb91adfa3955a403c91cfbba4e3a19041cfc9697f3ed187b09190c4e798"
  version "0.2.0"
  license "Apache-2.0"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/stale --version")
  end
end
