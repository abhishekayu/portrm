class Portrm < Formula
  desc "portrm - Fast, cross-platform CLI to inspect ports, understand processes, and recover broken dev environments"
  homepage "https://portrm.dev"
  version "1.0.6"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/abhishekayu/portrm/releases/download/v1.0.6/portrm-darwin-arm64.tar.gz"
      sha256 "15de0084e26066cb037d76edff191930992cb8fc487ac7d780ce9aabcd4aa6a5"
    else
      url "https://github.com/abhishekayu/portrm/releases/download/v1.0.6/portrm-darwin-amd64.tar.gz"
      sha256 "23f58b742175c8a6458d807943c794cc3cbf9e93474167790ad748366a78bbe3"
    end
  end

  on_linux do
    url "https://github.com/abhishekayu/portrm/releases/download/v1.0.6/portrm-linux-amd64.tar.gz"
    sha256 "3e9ab419a616d6a05424c1b6a8cf9a09069c50afcac288307a5bd4064f00d40a"
  end

  def install
    bin.install "ptrm"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/ptrm --version")
  end
end
