#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
(cd "$ROOT/rust" && cargo build --release -p vfk-ffi)
cmake -S "$HERE" -B "$HERE/build" \
  -DVIFASTKEY_LIB="$ROOT/rust/target/release/libvifastkey.a" \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_TESTING=ON
cmake --build "$HERE/build" -j"$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 2)"
ctest --test-dir "$HERE/build" --output-on-failure
