SHELL := /bin/bash

.PHONY: clean rust-build rust-test rust-lint

RUST_CRATES := echo

rust-build:
	@for crate in $(RUST_CRATES); do \
		cd $$crate && cargo build --release --locked && cd - >/dev/null || exit 1; \
	done

rust-test:
	@for crate in $(RUST_CRATES); do \
		cd $$crate && cargo test --locked && cd - >/dev/null || exit 1; \
	done

rust-lint:
	@for crate in $(RUST_CRATES); do \
		cd $$crate && cargo fmt -- --check && cargo clippy --all-targets --locked -- -D warnings && cd - >/dev/null || exit 1; \
	done

clean:
	rm -rf dist build echo/target containers/dist
