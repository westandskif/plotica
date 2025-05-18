.PHONY: build docs serve public release publish build_all

build:
	find pkg -delete || true
	wasm-pack build --target web --out-name index
	rsync -avh pkg/ docs/dist/ --delete
	rm pkg/.gitignore

build_all:
	BUILD_ALL=1 $(MAKE) build

release:
	test -f dist/index-esm.js
	test -f dist/index-iife.js
	npm publish --access public

public:
	ngrok http 192.168.100.5:7777

docs:
	mkdocs serve -a 192.168.100.56:7777
