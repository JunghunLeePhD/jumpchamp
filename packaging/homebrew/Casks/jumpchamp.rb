cask "jumpchamp" do
  arch arm: "arm64", intel: "intel-x86_64"

  version "1.0.0"
  sha256 arm:   "5294e4dd82ed8288883d072654f9de5216871317f4023759597cfc8b38b6fd7c",
         intel: "013f42f76e0c5d4d07190816c02d02654f5d0a19f177a18cd8015707a4b8a211"

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
