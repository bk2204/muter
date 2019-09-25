all:
	cargo build --release

clean:
	$(RM) -r target
	find -name '*.bk' | xargs $(RM)

test: fmt
	cargo test

fmt:
	if rustfmt --help | grep -qse --check; \
	then \
			rustfmt --check $$(find . -name '*.rs' | grep -v '^./target'); \
	else \
			rustfmt --write-mode diff $$(find . -name '*.rs' | grep -v '^./target'); \
	fi
