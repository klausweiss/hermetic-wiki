.PHONY: build test test-perf run site lint fmt clean

# Arguments passed to the binary, e.g. `make run ARGS="build notes/"`.
ARGS ?=

build:
	cargo build --release

test:
	cargo test

# Performance checks, ignored by `make test`; they need release mode.
test-perf:
	cargo test --release -- --ignored --nocapture

run:
	cargo run -- $(ARGS)

# Builds the test fixture notes into ./site, for looking at the output in a browser.
site:
	cargo run -- build tests/fixtures/notes site

lint:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt

clean:
	cargo clean
