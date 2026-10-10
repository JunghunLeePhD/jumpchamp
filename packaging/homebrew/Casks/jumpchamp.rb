cask "jumpchamp" do
  arch arm: "arm64", intel: "intel-x86_64"

  version "1.0.1"
  sha256 arm:   "a08d99a9a1e315906b2f679080d1f59cfc8db1348ad26aeeca8ad95bfd654757",
         intel: "6c06304374178613f12740f9db9a565db8f048e738796ab13564e6e80ccb5faf"

  url "https://github.com/JunghunLeePhD/jumpchamp/releases/download/v#{version}/JumpChamp-macos-#{arch}.app.zip"
  name "JumpChamp"
  desc "High-performance prime gap distribution explorer and analyzer"
  homepage "https://github.com/JunghunLeePhD/jumpchamp"

  livecheck do
    url :url
    strategy :github_latest
  end

  app "JumpChamp.app"

  postflight do
    system_command "/usr/bin/xattr",
                   args: ["-cr", "#{appdir}/JumpChamp.app"]
  end

  zap trash: [
    "~/.jumpchamp",
    "~/Library/Application Support/jumpchamp",
    "~/Library/Preferences/com.jumpchamp.gui.plist",
  ]
end
