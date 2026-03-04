class Stale < Formula
  desc "CLI tool to run or skip commands based on file content hashes"
  homepage "https://github.com/th1nkful/stale"
  version "0.2.1"
  license "Apache-2.0"

  on_macos do
    on_intel do
      url "https://github.com/th1nkful/stale/releases/download/0.2.1/stale-x86_64-apple-darwin.tar.gz"
      sha256 "24182d2ed37667f04b8e58e2b6e57442d1787931221dd10dd6a5821dfdb85f4e"
    end
    on_arm do
      url "https://github.com/th1nkful/stale/releases/download/0.2.1/stale-aarch64-apple-darwin.tar.gz"
      sha256 "6e4728457b41583f58ce91c5dcba1755dcf0e8d28c9e78f9d5b336573ae82add"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/th1nkful/stale/releases/download/0.2.1/stale-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "6686744144edc2e6b0e1704d88fb1aa8e5b2efa21d098bb011709d8f62d5f178"
    end
    on_arm do
      url "https://github.com/th1nkful/stale/releases/download/0.2.1/stale-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "03bcb852f5123c830526c452262d49e2be7115b056a84d60df224ab9d0bac67f"
    end
  end

  def install
    bin.install "stale"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/stale --version")
  end
end
