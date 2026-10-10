#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Helper script to generate/update Homebrew Cask for JumpChamp releases
# Usage: ./scripts/generate_brew_cask.sh [vX.Y.Z]
# ==============================================================================

REPO="JunghunLeePhD/jumpchamp"
TARGET_FILE="packaging/homebrew/Casks/jumpchamp.rb"

# 1. Determine version
INPUT_TAG="${1:-}"
if [ -z "$INPUT_TAG" ]; then
    # Try git tag
    INPUT_TAG="$(git describe --tags --abbrev=0 2>/dev/null || echo "v1.0.0")"
fi

# Strip leading 'v' for Homebrew cask version
VERSION="${INPUT_TAG#v}"
TAG="v${VERSION}"

echo "=================================================="
echo "Generating JumpChamp Homebrew Cask for version ${VERSION} (tag: ${TAG})"
echo "=================================================="

ARM64_URL="https://github.com/${REPO}/releases/download/${TAG}/JumpChamp-macos-arm64.app.zip"
INTEL_URL="https://github.com/${REPO}/releases/download/${TAG}/JumpChamp-macos-intel-x86_64.app.zip"

TMP_DIR="$(mktemp -d /tmp/jumpchamp_cask_download.XXXXXX)"
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "Fetching Apple Silicon (arm64) SHA256 checksum..."
TMP_ARM="${TMP_DIR}/jumpchamp-arm64.zip"
if ! curl --fail -sL -o "${TMP_ARM}" "${ARM64_URL}"; then
    echo "Error: Failed to download arm64 asset at ${ARM64_URL}" >&2
    echo "Please ensure GitHub Actions has finished publishing release assets for ${TAG}." >&2
    exit 1
fi
ARM64_SHA="$(sha256sum "${TMP_ARM}" | awk '{print $1}')"

if [ -z "$ARM64_SHA" ] || [ ${#ARM64_SHA} -ne 64 ]; then
    echo "Error: Failed to compute SHA256 for arm64 binary at ${ARM64_URL}" >&2
    exit 1
fi
echo "  -> arm64: ${ARM64_SHA}"

echo "Fetching Intel (x86_64) SHA256 checksum..."
TMP_INTEL="${TMP_DIR}/jumpchamp-intel.zip"
if ! curl --fail -sL -o "${TMP_INTEL}" "${INTEL_URL}"; then
    echo "Error: Failed to download intel asset at ${INTEL_URL}" >&2
    echo "Please ensure GitHub Actions has finished publishing release assets for ${TAG}." >&2
    exit 1
fi
INTEL_SHA="$(sha256sum "${TMP_INTEL}" | awk '{print $1}')"

if [ -z "$INTEL_SHA" ] || [ ${#INTEL_SHA} -ne 64 ]; then
    echo "Error: Failed to compute SHA256 for intel binary at ${INTEL_URL}" >&2
    exit 1
fi
echo "  -> intel: ${INTEL_SHA}"

mkdir -p "$(dirname "${TARGET_FILE}")"

cat <<EOF > "${TARGET_FILE}"
cask "jumpchamp" do
  arch arm: "arm64", intel: "intel-x86_64"

  version "${VERSION}"
  sha256 arm:   "${ARM64_SHA}",
         intel: "${INTEL_SHA}"

  url "https://github.com/${REPO}/releases/download/v#{version}/JumpChamp-macos-#{arch}.app.zip"
  name "JumpChamp"
  desc "High-performance prime gap distribution explorer and analyzer"
  homepage "https://github.com/${REPO}"

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
EOF

echo "Successfully wrote Homebrew Cask to: ${TARGET_FILE}"
echo "=================================================="
