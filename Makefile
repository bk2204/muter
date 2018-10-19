all:
	cargo build --release

fmt:
	find -name '*.rs' | xargs rustfmt

clean:
	$(RM) -r target
	find -name '*.bk' | xargs $(RM)

test:
	! find -name '*.rs' | xargs rustfmt --write-mode diff | grep '.'
	cargo test
