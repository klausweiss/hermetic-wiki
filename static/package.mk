# Shared targets for every package under packages/. A package's Makefile sets NAME
# (its element name, or empty for a library without a bundle) and includes this file.

.PHONY: build check test clean

BUN ?= bun

build:
ifneq ($(NAME),)
	$(BUN) build src/register.ts --target browser --minify --outfile dist/$(NAME).js
else
	@echo "$(notdir $(CURDIR)): library, nothing to bundle"
endif

check:
	$(BUN) x tsc -p tsconfig.json

test:
	@if ls src/*.test.ts >/dev/null 2>&1; then $(BUN) test src/; else echo "$(notdir $(CURDIR)): no tests"; fi

clean:
	rm -rf dist
