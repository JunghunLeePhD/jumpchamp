#!/usr/bin/env bash
set -e

# ============================================================================
# JumpChamp Linux Desktop Shortcut & Application Icon Installer
# ============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="${HOME}/.local/share/applications"
ICON_DIR="${HOME}/.local/share/icons/hicolor/512x512/apps"
DESKTOP_DIR="${HOME}/Desktop"

echo " Installing JumpChamp Desktop Shortcut..."

# 1. Ensure target directories exist
mkdir -p "${APP_DIR}"
mkdir -p "${ICON_DIR}"

# 2. Copy application icon
if [ -f "${SCRIPT_DIR}/assets/512x512.png" ]; then
    cp "${SCRIPT_DIR}/assets/512x512.png" "${ICON_DIR}/jumpchamp.png"
    echo " Icon installed to ${ICON_DIR}/jumpchamp.png"
fi

# 3. Determine binary path
BIN_PATH="${SCRIPT_DIR}/target/release/jumpchamp_gui"
if [ ! -f "${BIN_PATH}" ]; then
    BIN_PATH="$(which jumpchamp_gui 2>/dev/null || echo "${SCRIPT_DIR}/target/release/jumpchamp_gui")"
fi

# 4. Generate system .desktop file
cat << EOF > "${APP_DIR}/jumpchamp.desktop"
[Desktop Entry]
Type=Application
Name=JumpChamp
Comment=Interactive desktop application for exploring prime gap distributions
Exec=${BIN_PATH}
Icon=jumpchamp
Categories=Science;Math;Education;
Terminal=false
StartupWMClass=jumpchamp_gui
EOF

chmod +x "${APP_DIR}/jumpchamp.desktop"
echo " Desktop launcher installed to ${APP_DIR}/jumpchamp.desktop"

# 5. Copy to Desktop if ~/Desktop exists
if [ -d "${DESKTOP_DIR}" ]; then
    cp "${APP_DIR}/jumpchamp.desktop" "${DESKTOP_DIR}/jumpchamp.desktop"
    chmod +x "${DESKTOP_DIR}/jumpchamp.desktop"
    # Mark as trusted if gio is present (GNOME Desktop)
    if command -v gio &> /dev/null; then
        gio set "${DESKTOP_DIR}/jumpchamp.desktop" metadata::trusted true 2>/dev/null || true
    fi
    echo " Desktop shortcut created at ${DESKTOP_DIR}/jumpchamp.desktop"
fi

echo " Setup complete! JumpChamp is now visible in your Application Menu and Desktop."
