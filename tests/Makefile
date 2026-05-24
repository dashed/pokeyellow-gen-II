.PHONY: check test clippy fmt fmt-check

RUST_TEST_THREADS ?= 4

check: clippy fmt-check test

test:
	RUST_TEST_THREADS=$(RUST_TEST_THREADS) cargo test

clippy:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check
