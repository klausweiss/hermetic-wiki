# @hermetic/app: the bundle every generated site loads, built into ../../dist/app.js.
# That file is committed (see ../../banner.txt for why); `verify` checks it is current.

.PHONY: build check test verify clean

BUN ?= bun
OUT := ../../dist/app.js
# The banner is read by the shell at build time, with the Bun version filled in.
BANNER = "$$(sed "s/@BUN_VERSION@/$$($(BUN) --version)/" ../../banner.txt)"

# Compiles src/main.ts into the file named by $(1), starting with the banner.
define compile
	$(BUN) build src/main.ts --target browser --minify --banner $(BANNER) --outfile $(1)
endef

build:
	$(call compile,$(OUT))

check:
	$(BUN) x tsc -p tsconfig.json

test:
	@echo "app: no tests (it only wires the elements together)"

# Fails if the committed $(OUT) is not what the current sources produce.
verify:
	@tmp=$$(mktemp -d) && \
	$(call compile,$$tmp/app.js) >/dev/null && \
	head -c 2 $$tmp/app.js | grep -q '/\*' && \
	cmp -s $$tmp/app.js $(OUT) && rm -rf $$tmp || \
	{ rm -rf $$tmp; echo "static/dist/app.js is out of date or lacks its banner: run 'make -C static build' and commit it" >&2; exit 1; }

clean:
	@true
