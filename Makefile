.PHONY: build test test-perf site serve-test lint fmt frontend bench-search icons clean

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

# Builds the test fixture notes into ./site, for looking at the output in a browser.
site:
	cargo run -- build tests/fixtures/notes site

# Serves the test fixture notes with live reload (Ctrl+C to stop).
serve-test:
	cargo run -- serve tests/fixtures/notes

# Serves the project's wiki with live reload (Ctrl+C to stop).
wiki-serve:
	cargo run -- serve wiki --title "hermetic.wiki"

lint:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
	$(MAKE) -C static check verify

fmt:
	cargo fmt

# Relevance and performance benchmark of search against Pagefind (development only;
# downloads Pagefind and a headless browser).
bench-search:
	$(MAKE) -C sandbox/search-bench bench

# Redraws the favicon and touch icon in static/ from sandbox/favicon/ (needs python3
# and rsvg-convert).
icons:
	$(MAKE) -C sandbox/favicon icons

# Rebuilds the browser script (static/dist/app.js) from static/ts/.
frontend:
	$(MAKE) -C static build

clean:
	cargo clean
