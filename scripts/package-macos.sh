#!/bin/bash
# Local build only. Distribution identity is optional; the default is ad-hoc signing.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(uname -s)" != Darwin ]]; then
  echo "macOS packaging requires macOS and Xcode Command Line Tools" >&2
  exit 1
fi
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)"
arch="$(uname -m)"
out="$(pwd)/dist"
app="$out/UML Studio.app"
cargo build --release --locked
python3 scripts/collect-licenses.py "$out/ThirdPartyLicenses"
cargo run --release --locked --example app_icon -- "$out/UmlStudio.iconset"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
iconutil -c icns "$out/UmlStudio.iconset" -o "$app/Contents/Resources/UmlStudio.icns"
cp target/release/uml-studio "$app/Contents/MacOS/uml-studio"
cp -R "$out/ThirdPartyLicenses" "$app/Contents/Resources/"
cp assets/NanumGothic-OFL.txt "$app/Contents/Resources/"
cp docs/dependency-licenses.csv docs/third-party-notices.md "$app/Contents/Resources/"
cp -R examples/review.umlproj "$app/Contents/Resources/"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>UML Studio</string>
<key>CFBundleDisplayName</key><string>UML Studio</string>
<key>CFBundleIdentifier</key><string>io.github.kyunghyunHan.uml-studio</string>
<key>CFBundleExecutable</key><string>uml-studio</string>
<key>CFBundleIconFile</key><string>UmlStudio</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>CFBundleVersion</key><string>$version</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>NSHumanReadableCopyright</key><string>UML Studio contributors</string>
</dict></plist>
PLIST
plutil -lint "$app/Contents/Info.plist"
identity="${MACOS_SIGN_IDENTITY:--}"
if [[ "$identity" == "-" ]]; then
  codesign --force --sign - --timestamp=none "$app"
else
  codesign --force --options runtime --timestamp --sign "$identity" "$app"
fi
codesign --verify --deep --strict --verbose=2 "$app"
ditto -c -k --keepParent "$app" "$out/UML-Studio-$version-macos-$arch.zip"
(
  cd "$out"
  shasum -a 256 "UML-Studio-$version-macos-$arch.zip" > "UML-Studio-$version-macos-$arch.zip.sha256"
)
printf 'Created %s\n' "$app" "$out/UML-Studio-$version-macos-$arch.zip"
