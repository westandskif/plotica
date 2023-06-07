.PHONY: build docs serve public release publish build_all

build:
	find dist -delete || true
	npm run build
	rsync -avh dist/ docs/dist/ --delete

build_all:
	BUILD_ALL=1 $(MAKE) build

release:
	test -f dist/index-esm.js
	test -f dist/index-iife.js
	npm publish --access public

serve:
	sfz -Cr -b 192.168.100.5 -p 7777 .

public:
	ngrok http 192.168.100.5:7777

docs:
	mkdocs serve -a 192.168.100.5:7777
