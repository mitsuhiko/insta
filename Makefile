all: test

build:
	@cargo build --all-features

doc:
	@RUSTC_BOOTSTRAP=1 RUSTDOCFLAGS="--cfg=docsrs" cargo doc --no-deps --all-features

test: cargotest

cargotest:
	@echo "CARGO TESTS"
	@rustup component add rustfmt 2> /dev/null
	@cargo test -p insta
	@cargo test -p insta --all-features
	@cargo test -p insta --no-default-features
	@cargo test -p insta --features redactions -- --test-threads 1
	@echo "CARGO-INSTA TESTS"
	# Turn off CI flag so that cargo insta test behaves as we expect
	# under normal operation
	@CI=0 cargo test -p cargo-insta

check-minver:
	@echo "MINVER CHECK"
	@cargo minimal-versions check -p insta
	@cargo minimal-versions check -p insta --all-features
	@cargo minimal-versions check -p insta --no-default-features
	@cargo minimal-versions check -p insta --features redactions

check-msrv:
	@echo "MSRV CHECK"
	@cd insta && cargo msrv verify
	@cd cargo-insta && cargo msrv verify

format:
	@rustup component add rustfmt 2> /dev/null
	@cargo fmt --all

format-check:
	@rustup component add rustfmt 2> /dev/null
	@cargo fmt --all -- --check

lint:
	@rustup component add clippy 2> /dev/null
	@cargo clippy --all-targets --workspace -- --deny warnings

.PHONY: all doc test cargotest format format-check lint update-readme
