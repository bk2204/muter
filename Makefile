# Test configuration.
GROUPS := stretch buster stable nightly oldest
DOCKER_FILES := $(patsubst %,test/Dockerfile.%,$(GROUPS))
DOCKER_STAMPS := $(patsubst %,test/Dockerfile.%.stamp,$(GROUPS))
CI_TARGETS := $(patsubst %,ci-%,$(GROUPS))
INCLUDES := $(wildcard test/include/*.erb)

all:
	cargo build --release

clean:
	cargo clean
	$(RM) -fr target tmp
	for i in "$(DOCKER_STAMPS)"; \
	do \
		[ ! -f "$$i" ] || docker image rm -f "$$i"; \
	done
	$(RM) -f $(DOCKER_FILES) $(DOCKER_STAMPS)
	$(RM) -f *.md *.md+
	$(RM) -fr tmp
	$(RM) -fr doc/man/*.1

test:
	cargo test

doc:
	for i in doc/man/*.adoc; do \
		asciidoctor -b manpage -a compat-mode $$i; \
	done

deb: all doc
	cargo deb

test-deb: deb
	lintian target/debian/muter_*.deb

%.md: %.adoc
	asciidoctor -o $@+ -b docbook5 $^
	pandoc -f docbook -t commonmark -o $@ $@+
	$(RM) $@+

# We do not require both of these commands here since nightly Rust may be
# missing one or more of these. When run under CI, they should be present for
# stable Rust and catch any issues.
#
# Note if we're using rustup, cargo-clippy may exist in the PATH even if clippy
# isn't installed, but it may be a wrapper that just fails when invoked. Check
# that it can successfully print help output to check if we really have clippy.
# The same goes for rustfmt.
lint:
	if command -v cargo-clippy && cargo-clippy --help >/dev/null 2>&1; \
	then \
	        $(MAKE) clippy; \
	fi
	if command -v rustfmt && rustfmt --help >/dev/null 2>&1; \
	then \
	        $(MAKE) fmt; \
	fi

ci: $(CI_TARGETS)

ci-%: test/Dockerfile.%.stamp
	mkdir -p target/assets
	docker run --rm \
		-v "$(PWD)/target/assets:/usr/src/muter/target/debian" \
		$$(cat "$<") \
		sh -c 'cd /usr/src/muter && make test-full && ([ "$*" = oldest ] || (cargo install cargo-deb && make test-deb))'

test-full:
	make all
	make doc
	make test
	make lint

test/Dockerfile.%.stamp: test/Dockerfile.% $(SRC)
	docker build --iidfile="$@" -f "$<" .

test/Dockerfile.%: test/Dockerfile.%.erb $(INCLUDES)
	test/template "$<" >"$@"

clippy:
	rm -rf target/debug target/release
	@# We exclude these lints here instead of in the file because Rust 1.24
	@# doesn't support excluding clippy warnings.  Similarly, it doesn't support
	@# the syntax these lints suggest.
	cargo clippy -- \
		-A clippy::range-plus-one -A clippy::needless-lifetimes -A clippy::unknown-clippy-lints \
		-D warnings

fmt:
	if rustfmt --help | grep -qse --check; \
	then \
			rustfmt --check $$(find . -name '*.rs' | grep -v '^./target'); \
	else \
			rustfmt --write-mode diff $$(find . -name '*.rs' | grep -v '^./target'); \
	fi

.PHONY: all lint ci clean doc clippy fmt test
