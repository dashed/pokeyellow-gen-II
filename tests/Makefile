.PHONY: check test clippy fmt fmt-check build clean

check: clippy fmt-check test

test:
	cargo test

clippy:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

build:
	cargo build

clean:
	cargo clean
