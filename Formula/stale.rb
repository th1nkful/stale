class Stale < Formula
  desc "CLI tool to run or skip commands based on file content hashes"
  homepage "https://github.com/th1nkful/stale"
  version "0.2.0"
  license "Apache-2.0"

  on_macos do
    on_intel do
      url "https://github.com/th1nkful/stale/releases/download/0.2.0/stale-x86_64-apple-darwin.tar.gz"
      sha256 "e571ecb91adfa3955a403c91cfbba4e3a19041cfc9697f3ed187b09190c4e798"
    end
    on_arm do
      url "https://github.com/th1nkful/stale/releases/download/0.2.0/stale-aarch64-apple-darwin.tar.gz"
      sha256 "e571ecb91adfa3955a403c91cfbba4e3a19041cfc9697f3ed187b09190c4e798"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/th1nkful/stale/releases/download/0.2.0/stale-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "e571ecb91adfa3955a403c91cfbba4e3a19041cfc9697f3ed187b09190c4e798"
    end
    on_arm do
      url "https://github.com/th1nkful/stale/releases/download/0.2.0/stale-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "e571ecb91adfa3955a403c91cfbba4e3a19041cfc9697f3ed187b09190c4e798"
    end
  end

  def install
    bin.install "stale"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/stale --version")
  end
end
