EMSCRIPTEN_PREFIX := $(shell brew --prefix emscripten 2>/dev/null)
WEB_DIR := target/wasm32-unknown-emscripten/release
DOCS_DIR := docs

.PHONY: run build web serve pages clean

## Native desktop app (debug, run directly)
run:
	cargo run

## Native desktop app (debug build only)
build:
	cargo build

## Web (WebAssembly via Emscripten) release build.
## Requires: `brew install emscripten` and `rustup target add wasm32-unknown-emscripten`.
web:
	@if [ -z "$(EMSCRIPTEN_PREFIX)" ]; then \
		echo "Emscripten not found. Install it with: brew install emscripten"; \
		exit 1; \
	fi
	PATH="$(EMSCRIPTEN_PREFIX)/bin:$$PATH" \
	EMCC_CFLAGS="-O3 -sUSE_GLFW=3 -sASSERTIONS=1 -sWASM=1 -sASYNCIFY -sGL_ENABLE_GET_PROC_ADDRESS=1 -sMAX_WEBGL_VERSION=2 -sMIN_WEBGL_VERSION=2 -sFULL_ES3=1 -sALLOW_MEMORY_GROWTH=1 -sINITIAL_MEMORY=268435456 --preload-file $(CURDIR)/assets@assets" \
	BINDGEN_EXTRA_CLANG_ARGS="-I$(EMSCRIPTEN_PREFIX)/libexec/cache/sysroot/include" \
	cargo build --release --target wasm32-unknown-emscripten
	@# --preload-file's sidecar .data file is a side effect of the link
	@# step that cargo doesn't know to copy out of deps/ like it does the
	@# .wasm/.js it recognizes as the crate's actual build artifacts.
	@cp $(WEB_DIR)/deps/snookeraim.data $(WEB_DIR)/snookeraim.data
	@cp web/index.html $(WEB_DIR)/index.html
	@echo "Built: $(WEB_DIR)/snookeraim.{js,wasm,data}"

## Build for web and serve it locally at http://localhost:8765
serve: web
	@echo "Serving $(WEB_DIR) at http://localhost:8765"
	cd $(WEB_DIR) && python3 -m http.server 8765

## Build for web and stage the output in docs/ for GitHub Pages.
## After running this, commit docs/ and enable Pages in the repo's
## Settings > Pages > Deploy from a branch > main / docs.
pages: web
	rm -rf $(DOCS_DIR)
	mkdir -p $(DOCS_DIR)
	cp $(WEB_DIR)/snookeraim.js $(WEB_DIR)/snookeraim.wasm $(WEB_DIR)/snookeraim.data $(DOCS_DIR)/
	cp web/index.html $(DOCS_DIR)/index.html
	touch $(DOCS_DIR)/.nojekyll
	@echo "Staged in $(DOCS_DIR)/ -- commit it, then enable Pages (Settings > Pages > Deploy from a branch > main / docs)"

clean:
	cargo clean
