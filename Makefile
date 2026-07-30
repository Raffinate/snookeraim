EMSCRIPTEN_PREFIX := $(shell brew --prefix emscripten 2>/dev/null)
WEB_DIR := target/wasm32-unknown-emscripten/release

.PHONY: run build web serve web-run clean

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
	EMCC_CFLAGS="-O3 -sUSE_GLFW=3 -sASSERTIONS=1 -sWASM=1 -sASYNCIFY -sGL_ENABLE_GET_PROC_ADDRESS=1 -sMAX_WEBGL_VERSION=2 -sMIN_WEBGL_VERSION=2 -sFULL_ES3=1" \
	BINDGEN_EXTRA_CLANG_ARGS="-I$(EMSCRIPTEN_PREFIX)/libexec/cache/sysroot/include" \
	cargo build --release --target wasm32-unknown-emscripten
	@cp web/index.html $(WEB_DIR)/index.html
	@echo "Built: $(WEB_DIR)/snookeraim.{js,wasm}"

## Build for web and serve it locally at http://localhost:8765
serve: web
	@echo "Serving $(WEB_DIR) at http://localhost:8765"
	cd $(WEB_DIR) && python3 -m http.server 8765

clean:
	cargo clean
