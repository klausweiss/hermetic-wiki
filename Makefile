.PHONY: build test test-perf run site serve lint fmt frontend bench-search clean

# Arguments passed to the binary, e.g. `make run ARGS="build notes/"`.
ARGS ?=

build:
	cargo build --release

test:
	cargo test
	$(MAKE) -C static test

# Performance checks, ignored by `make test`; they need release mode.
test-perf:
	cargo test --release -- --ignored --nocapture

run:
	cargo run -- $(ARGS)

# Builds the test fixture notes into ./site, for looking at the output in a browser.
site:
	cargo run -- build tests/fixtures/notes site

# Serves the test fixture notes with live reload (Ctrl+C to stop).
serve:
	cargo run -- serve tests/fixtures/notes

lint:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	$(MAKE) -C static check verify

fmt:
	cargo fmt

# Relevance and performance benchmark of search against Pagefind (development only;
# downloads Pagefind and a headless browser).
bench-search:
	$(MAKE) -C bench/search bench

# Rebuilds the browser script (static/dist/app.js) from static/ts/.
frontend:
	$(MAKE) -C static build

clean:
	cargo clean
