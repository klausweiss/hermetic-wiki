# Development-only benchmark of hermetic's search against Pagefind (see the search
# design, "Benchmark"). Downloads Pagefind and uses a headless browser;
# nothing here is part of the product or of `make test`.

.PHONY: bench corpus clean

BUN ?= bun

node_modules: package.json bun.lock
	$(BUN) install --frozen-lockfile
	@touch node_modules

bench: node_modules
	cargo build --release --manifest-path ../../Cargo.toml
	$(BUN) x playwright-core install chromium-headless-shell
	$(BUN) run src/run.ts

# Regenerates the committed corpus from the current OpenSpec documents. Review the
# judgments in queries.json afterwards: the corpus text changes.
corpus:
	$(BUN) -e 'import { relevanceCorpus } from "./src/corpus"; relevanceCorpus("corpus")'

clean:
	rm -rf out node_modules
