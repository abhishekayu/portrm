class Portrm < Formula
  desc "portrm - Fast, cross-platform CLI to inspect ports, understand processes, and recover broken dev environments"
  homepage "https://portrm.dev"
  version "2.2.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/abhishekayu/portrm/releases/download/v2.1.0/portrm-darwin-arm64.tar.gz"
      sha256 ""
    else
      url "https://github.com/abhishekayu/portrm/releases/download/v2.1.0/portrm-darwin-amd64.tar.gz"
      sha256 ""
    end
  end

  on_linux do
    url "https://github.com/abhishekayu/portrm/releases/download/v2.1.0/portrm-linux-amd64.tar.gz"
    sha256 ""
  end

  def install
    bin.install "ptrm"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/ptrm --version")
  end
end
