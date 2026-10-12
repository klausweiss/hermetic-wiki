# hermetic's frontend: a Bun workspace of web components under packages/, each a
# standalone project with its own Makefile. `app` bundles them into dist/app.js, the
# one script generated sites load; that file is committed (see banner.txt).

.PHONY: build check test verify clean

BUN ?= bun
# core first: the others depend on it. app last: it bundles the others.
PACKAGES := core theme-toggle sidebar timeline shortcuts live-reload search app

node_modules: package.json bun.lock $(wildcard packages/*/package.json)
	$(BUN) install --frozen-lockfile
	@touch node_modules

build check test: node_modules
	@for p in $(PACKAGES); do $(MAKE) --no-print-directory -C packages/$$p $@ || exit 1; done

verify: node_modules
	@$(MAKE) --no-print-directory -C packages/app verify

clean:
	@for p in $(PACKAGES); do $(MAKE) --no-print-directory -C packages/$$p clean; done
	rm -rf node_modules
