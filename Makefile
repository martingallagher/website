.ONESHELL:
.PHONY: test

fmt:
	@rustfmt src/main.rs

clean:
	@cargo clean

test:
	@cargo test -- --nocapture

bench:
	@cargo bench

debug:
	@cargo build

release:
	@cargo build --release

SASS_FILES := $(shell find assets/sass -type f -printf "%f\n" | grep -Po '.*(?=\.)')

css:
	@rm -f assets/static/*.css

	for name in $(SASS_FILES); do
		sassc --style=compressed assets/sass/$$name.scss assets/static/$$name.css &
	done

	wait

	node optimizeCSS.js

TS_FILES := $(shell find assets/ts -type f -printf "%f\n" | grep -Po '.*(?=\.)')

js:
	@rm -f assets/static/*.js

	for name in $(TS_FILES); do
		tsc --outFile assets/static/$$name.tmp assets/ts/$$name.ts && \
		google-closure-compiler \
			--warning_level=QUIET \
			--compilation_level=ADVANCED \
			--language_in=ECMASCRIPT5_STRICT \
			--language_out=ECMASCRIPT5_STRICT \
			assets/static/$$name.tmp > assets/static/$$name.js && \
		rm -f assets/static/$$name.tmp &
	done

	wait

run-debug:
	@ADDRESS=0.0.0.0:8484 \
		target/debug/website

run-release:
	@ADDRESS=0.0.0.0:80 \
	ENABLE_INLINE_CSS=true \
		target/release/website
