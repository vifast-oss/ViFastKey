# ViFastKey: lệnh phát triển thường dùng.
#   make test        chạy test Python + Rust
#   make check       thêm lint/fmt/conformance
#   make conformance sinh lại oracle và đồng bộ sang Rust
#   make web         dựng WASM + asset cho web demo
PYTHON ?= python3

.PHONY: test test-python test-rust check conformance web clean

test: test-python test-rust

test-python:
	cd python && $(PYTHON) -m pytest -q

test-rust:
	cd rust && cargo test --workspace

check: test
	cd python && $(PYTHON) -m ruff check .
	cd rust && cargo fmt --all -- --check
	cd rust && cargo clippy --workspace --all-targets -- -D warnings
	$(MAKE) conformance
	git diff --exit-code -- python/conformance rust/conformance python/tests/golden python/tests/data rust/testdata python/vfk_core/data rust/crates/vfk-core/data

conformance:
	cd python && $(PYTHON) -m tools.export_conformance
	cd rust && PYTHON=$(PYTHON) tools/sync_conformance.sh ../python

web:
	./web/build.sh

clean:
	cd python && rm -rf .pytest_cache __pycache__ */__pycache__ *.egg-info build dist
	cd rust && cargo clean
