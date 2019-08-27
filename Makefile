all: test

build:
	@cargo build --all-features

doc:
	@cargo doc --all-features

test: cargotest

cargotest:
	@cargo test
	@cargo test --no-default-features
	@cd cargo-insta; cargo test

format:
	@rustup component add rustfmt 2> /dev/null
	@cargo fmt --all

format-check:
	@rustup component add rustfmt 2> /dev/null
	@cargo fmt --all -- --check

lint:
	@rustup component add clippy 2> /dev/null
	@cargo clippy

update-readme:
	@cargo readme > README.md
	@cd cargo-insta; cargo readme > README.md

.PHONY: all doc test cargotest format format-check lint update-readme
