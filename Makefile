all: test

build:
	@cargo build --all-features

doc:
	@cargo doc --all-features

test: cargotest cargo-insta-tests

test-legacy: cargotest-legacy

cargo-insta-tests:
	@echo "CARGO-INSTA INTEGRATION TESTS"
	@cd cargo-insta/integration-tests; cargo run

cargotest:
	@echo "CARGO TESTS"
	@rustup component add rustfmt 2> /dev/null
	@cargo test
	@cargo test --all-features
	@cargo test --no-default-features
	@cargo test --features redactions,backtrace -- --test-threads 1
	@cd cargo-insta; cargo test

cargotest-legacy:
	# This can't run the tests against backtrace as this feature no longer works
	# on older rust
	@echo "CARGO TESTS LEGACY"
	@rustup component add rustfmt 2> /dev/null
	@cargo test
	@cargo test --features redactions,glob
	@cargo test --features redactions
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

.PHONY: all doc test cargotest format format-check lint update-readme
