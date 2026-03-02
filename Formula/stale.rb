class Stale < Formula
  desc "CLI tool to run or skip commands based on file content hashes"
  homepage "https://github.com/th1nkful/stale"
  url "https://github.com/th1nkful/stale/archive/refs/tags/0.1.0.tar.gz"
  sha256 "a0894b54d9c30ca148a6cb9798ee9d78baf8e90127cd3c33aacd442f235db4f0"
  version "0.1.0"
  license "Apache-2.0"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/stale --version")
  end
end
