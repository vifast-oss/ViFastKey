#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
if [ "$(uname)" = "Darwin" ] && /usr/libexec/java_home -v 21 >/dev/null 2>&1; then
  export JAVA_HOME="$(/usr/libexec/java_home -v 21)"
fi
: "${ANDROID_HOME:=/opt/homebrew/share/android-commandlinetools}"
: "${ANDROID_NDK_HOME:=$(find "$ANDROID_HOME/ndk" -maxdepth 1 -mindepth 1 -type d | sort | tail -1)}"
export ANDROID_HOME ANDROID_NDK_HOME
mkdir -p "$HERE/app/src/main/jniLibs"
(cd "$ROOT/rust" && cargo ndk -t arm64-v8a -t x86_64 \
  -o "$HERE/app/src/main/jniLibs" build --release -p vfk-jni)
printf 'sdk.dir=%s\n' "$ANDROID_HOME" > "$HERE/local.properties"
gradle -p "$HERE" assembleDebug --no-daemon
echo "APK: $HERE/app/build/outputs/apk/debug/app-debug.apk"
