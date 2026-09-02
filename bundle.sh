#!/bin/sh
set -e
cd "$(dirname "$0")"
cargo build --release
APP="target/Bluesky Gif Unfucker.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources"
cp macos/Info.plist "$APP/Contents/"
cp macos/unfucker.icns "$APP/Contents/Resources/"
cp target/release/bsky-gif-unfucker "$APP/Contents/MacOS/"
codesign --force --sign - "$APP"
ditto -c -k --sequesterRsrc --keepParent "$APP" "$APP.zip"
echo "Built $APP and $APP.zip"
