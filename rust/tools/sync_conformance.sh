#!/usr/bin/env bash
# Sinh lại conformance từ bản Python (oracle) rồi chép sang workspace Rust.
#   tools/sync_conformance.sh ../python
#   PYTHON=python3.12 tools/sync_conformance.sh ../python
set -euo pipefail
PY=${1:-../python}
PYTHON=${PYTHON:-python3}
HERE=$(cd "$(dirname "$0")/.." && pwd)
(cd "$PY" && "$PYTHON" -m tools.export_conformance)
cp "$PY"/conformance/*.jsonl "$HERE/conformance/"
cp "$PY"/tests/golden/*.tsv "$HERE/testdata/golden/"
cp "$PY"/tests/data/corpus_vi.txt "$HERE/testdata/"
cp "$PY"/adapters/data/*.txt "$HERE/crates/vfk-engine/data/"
cp "$PY"/vfk_core/data/charsets.json "$HERE/crates/vfk-core/data/"
echo "Đã đồng bộ. Chạy: cargo test --workspace"
