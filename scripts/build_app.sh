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

# 4. App ikon (.icns) a Finder / app switcher számára.
# A tray ikonoktól függetlenül a .app bundle-nek külön CFBundleIconFile kell,
# különben macOS általános/üres alkalmazásikont mutathat.
APP_ICON_NAME="${APP_NAME}.icns"
ICON_SOURCE="assets/cup_4@2x.png"
ICONSET_DIR="${APP_DIR}/Contents/Resources/${APP_NAME}.iconset"

if [[ -f "${ICON_SOURCE}" ]] && command -v sips >/dev/null 2>&1 && command -v iconutil >/dev/null 2>&1; then
  mkdir -p "${ICONSET_DIR}"
  sips -z 16 16     "${ICON_SOURCE}" --out "${ICONSET_DIR}/icon_16x16.png" >/dev/null
  sips -z 32 32     "${ICON_SOURCE}" --out "${ICONSET_DIR}/icon_16x16@2x.png" >/dev/null
  sips -z 32 32     "${ICON_SOURCE}" --out "${ICONSET_DIR}/icon_32x32.png" >/dev/null
  sips -z 64 64     "${ICON_SOURCE}" --out "${ICONSET_DIR}/icon_32x32@2x.png" >/dev/null
  sips -z 128 128   "${ICON_SOURCE}" --out "${ICONSET_DIR}/icon_128x128.png" >/dev/null
  sips -z 256 256   "${ICON_SOURCE}" --out "${ICONSET_DIR}/icon_128x128@2x.png" >/dev/null
  sips -z 256 256   "${ICON_SOURCE}" --out "${ICONSET_DIR}/icon_256x256.png" >/dev/null
  sips -z 512 512   "${ICON_SOURCE}" --out "${ICONSET_DIR}/icon_256x256@2x.png" >/dev/null
  sips -z 512 512   "${ICON_SOURCE}" --out "${ICONSET_DIR}/icon_512x512.png" >/dev/null
  sips -z 1024 1024 "${ICON_SOURCE}" --out "${ICONSET_DIR}/icon_512x512@2x.png" >/dev/null
  iconutil -c icns "${ICONSET_DIR}" -o "${APP_DIR}/Contents/Resources/${APP_ICON_NAME}"
  rm -rf "${ICONSET_DIR}"
else
  echo ">> figyelem: app ikon nem generálható (hiányzó ${ICON_SOURCE}, sips vagy iconutil)"
fi

# 5. Info.plist — agent app, nincs dock/terminál
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
  <key>CFBundleIconFile</key>
  <string>${APP_ICON_NAME}</string>
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

# 6. PkgInfo (APPL magic) a Finder gyors felismeréséhez.
printf "APPL????" > "${APP_DIR}/Contents/PkgInfo"
touch "${APP_DIR}"

echo ">> kész: ${APP_DIR}"
echo "   indítás: open ${APP_DIR}"
