.PHONY: build

build:
	wasm-pack build --target web

serve:
	sfz .
