# Test configuration.
GROUPS := bullseye stable nightly oldest
DOCKER_FILES := $(patsubst %,test/Dockerfile.%,$(GROUPS))
DOCKER_STAMPS := $(patsubst %,test/Dockerfile.%.stamp,$(GROUPS))
CI_TARGETS := $(patsubst %,ci-%,$(GROUPS))
INCLUDES := $(wildcard test/include/*.erb)

# Set this to a Docker target to build for a specific platform.
PLATFORM ?=
ifneq ($(PLATFORM),)
PLATFORM_ARG := --platform $(PLATFORM)
else
PLATFORM_ARG :=
endif

FEATURES ?=
ifneq ($(FEATURES),)
FEATURE_ARG := --features $(FEATURES)
else
FEATURE_ARG :=
endif

ASCIIDOCTOR ?= asciidoctor

CARGO_DEB_VERSION = 1.28.0

FREEBSD_VERSION ?= 13
NETBSD_VERSION ?= 9

all:
	cargo build --release $(FEATURE_ARG)

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
	cargo test $(FEATURE_ARG)

doc: doc/man/muter.1 doc/man/muter.1.gz

deb: all doc
	cargo deb

test-deb: deb
	lintian target/debian/muter_*.deb

%.1: %.adoc
	$(ASCIIDOCTOR) -b manpage -a compat-mode -o $@ $^

%.1.gz: %.1
	gzip -9fnk $^

%.md: %.adoc
	asciidoctor -o $@+ -b docbook5 $^
	pandoc -f docbook -t commonmark -o $@ $@+
	$(RM) $@+

package: README.md
	cargo package --locked --allow-dirty

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
		$(PLATFORM_ARG) \
		-v "$(PWD)/target/assets:/usr/src/muter/target/debian" \
		-e CARGO_NET_GIT_FETCH_WITH_CLI=true \
		$$(cat "$<") \
		sh -c 'cd /usr/src/muter && make test-full && ([ "$*" = oldest ] || expr "$$(uname -m)" : arm || (cargo install --version=$(CARGO_DEB_VERSION) cargo-deb && make package test-deb))'

ci-freebsd:
	vagrant init generic/freebsd$(FREEBSD_VERSION)
	vagrant up
	vagrant ssh -- sudo pkg install -y curl gettext git gmake rubygem-asciidoctor rust
	vagrant ssh -- git init /home/vagrant/muter
	GIT_SSH_COMMAND='f() { shift; vagrant ssh -- "$$@"; };f' git push vagrant@localhost:/home/vagrant/muter
	vagrant ssh -- "cd /home/vagrant/muter && git checkout $$(git rev-parse HEAD) && gmake test-full FEATURES=$(FEATURES)"

ci-netbsd:
	vagrant init generic/netbsd$(NETBSD_VERSION)
	vagrant up
	vagrant ssh -- sudo /usr/pkg/bin/pkgin update
	vagrant ssh -- sudo /usr/pkg/bin/pkgin -y install ca-certificates curl gettext gettext-lib git gmake ruby27-asciidoctor rust
	vagrant ssh -- git init /home/vagrant/muter
	GIT_SSH_COMMAND='f() { shift; vagrant ssh -- "$$@"; };f' git push vagrant@localhost:/home/vagrant/muter
	vagrant ssh -- "cd /home/vagrant/muter && git checkout $$(git rev-parse HEAD) && gmake test-full ASCIIDOCTOR=asciidoctor27  CARGO_HTTP_MULTIPLEXING=false FEATURES=$(FEATURES) GETTEXT_DIR=/usr/pkg LD_LIBRARY_PATH=/usr/pkg/lib"

test-full:
	$(MAKE) all
	$(MAKE) doc
	$(MAKE) test
	$(MAKE) lint

test/Dockerfile.%.stamp: test/Dockerfile.% $(SRC)
	docker build $(PLATFORM_ARG) --iidfile="$@" -f "$<" .

test/Dockerfile.%: test/Dockerfile.%.erb $(INCLUDES)
	test/template "$<" >"$@"

clippy:
	rm -rf target/debug target/release
	@# We exclude these lints here instead of in the file because Rust 1.24
	@# doesn't support excluding clippy warnings.  Similarly, it doesn't support
	@# the syntax these lints suggest.
	cargo clippy $(FEATURE_ARG) -- \
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
