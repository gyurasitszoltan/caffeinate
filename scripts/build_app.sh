#!/usr/bin/env bash
# Összerak egy minimális macOS .app bundle-t a release binárisból.
# LSUIElement=1 => "background-only agent app": nem nyit Terminalt,
# nem jelenik meg a Dockban, csak a tray ikon fut a menüsorban.
#
# Használat:  scripts/build_app.sh
# Eredmény:   target/release/KeepAwake.app

set -euo pipefail

cd "$(dirname "$0")/.."

APP_NAME="KeepAwake"
BIN_NAME="keepawake"
RELEASE_DIR="target/release"
APP_DIR="${RELEASE_DIR}/${APP_NAME}.app"

# 1. Bináris létezése
if [[ ! -f "${RELEASE_DIR}/${BIN_NAME}" ]]; then
  echo ">> release bináris hiányzik, először: cargo build --release"
  exit 1
fi

# 2. Bundle struktúra
echo ">> bundle: ${APP_DIR}"
rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/Contents/MacOS"
mkdir -p "${APP_DIR}/Contents/Resources"

# 3. Bináris másolása MacOS/ alá
cp "${RELEASE_DIR}/${BIN_NAME}" "${APP_DIR}/Contents/MacOS/${BIN_NAME}"
chmod +x "${APP_DIR}/Contents/MacOS/${BIN_NAME}"

# 4. Info.plist — agent app, nincs dock/terminál
cat > "${APP_DIR}/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>${APP_NAME}</string>
  <key>CFBundleDisplayName</key>
  <string>${APP_NAME}</string>
  <key>CFBundleIdentifier</key>
  <string>com.earendil.keepawake</string>
  <key>CFBundleVersion</key>
  <string>1</string>
  <key>CFBundleShortVersionString</key>
  <string>0.1.0</string>
  <key>CFBundleExecutable</key>
  <string>${BIN_NAME}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>LSUIElement</key>
  <true/>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>NSSupportsAutomaticTermination</key>
  <false/>
  <key>NSSupportsSuddenTermination</key>
  <false/>
</dict>
</plist>
PLIST

# 5. Ikon (opcionális): a cup_4@2x a leglátványosabb app ikon.
# PkgInfo (APPL magic) a Finder gyors felismeréséhez.
printf "APPL????" > "${APP_DIR}/Contents/PkgInfo"

echo ">> kész: ${APP_DIR}"
echo "   indítás: open ${APP_DIR}"
