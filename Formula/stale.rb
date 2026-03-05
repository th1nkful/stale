class Stale < Formula
  desc "CLI tool to run or skip commands based on file content hashes"
  homepage "https://github.com/th1nkful/stale"
  version "0.2.2"
  license "Apache-2.0"

  on_macos do
    on_intel do
      url "https://github.com/th1nkful/stale/releases/download/0.2.2/stale-x86_64-apple-darwin.tar.gz"
      sha256 "577a3389c5c088e72228c7110ca5566976c6f40c2de9ba0b9dc9916704c981f3"
    end
    on_arm do
      url "https://github.com/th1nkful/stale/releases/download/0.2.2/stale-aarch64-apple-darwin.tar.gz"
      sha256 "3418dbfb35d9a7d0ff93f535e2d40a85ce8afcc201ad405adf8b68dd0ea13af1"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/th1nkful/stale/releases/download/0.2.2/stale-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "edaf139a1be97682ccb6b95671e508e1b755bdbdb261c363ec9d54921b52f91c"
    end
    on_arm do
      url "https://github.com/th1nkful/stale/releases/download/0.2.2/stale-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "9c01691597c67ff0ac912d35d2a923fcd6f6bc15f10fc0fc154bd51452f2d071"
    end
  end

  def install
    bin.install "stale"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/stale --version")
  end
end
