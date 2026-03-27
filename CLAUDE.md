# insta — Snapshot Testing Library for Rust

## Project Overview

insta is a snapshot testing (approval testing) library for Rust, created by Armin Ronacher. It lets you assert values against reference snapshots stored as `.snap` files or inline in source code. The companion CLI tool `cargo-insta` provides an interactive review workflow for accepting or rejecting snapshot changes.

- **Primary language:** Rust
- **MSRV:** 1.66.0 (insta), check `cargo-insta/Cargo.toml` for cargo-insta
- **License:** Apache-2.0

## Workspace Structure

This is a Cargo workspace with two main crates and a VS Code extension:

```
insta/                  # Core library crate (published as `insta`)
  src/
    lib.rs              # Public API, macro re-exports, feature gates
    macros.rs           # assert_*_snapshot! macro definitions
    runtime.rs          # Test runtime: snapshot comparison, update behavior
    snapshot.rs         # Snapshot and PendingInlineSnapshot types, .snap file I/O
    settings.rs         # Thread-local Settings (redactions, snapshot path, etc.)
    redaction.rs        # Value redaction with selectors (requires `redactions` feature)
    filters.rs          # Regex-based snapshot filters (requires `filters` feature)
    glob.rs             # Glob-based test generation (requires `glob` feature)
    serialization.rs    # Serialization dispatch for yaml/json/ron/csv/toml
    content/            # Content abstraction layer (intermediate repr for structured data)
    env.rs              # Environment variable handling (INSTA_UPDATE, INSTA_FORCE_PASS, etc.)
    output.rs           # SnapshotPrinter — terminal diff output
    comparator.rs       # Comparison logic between old and new snapshots
    select_grammar.pest # PEG grammar for redaction selectors
  tests/                # Integration tests for the core library
cargo-insta/            # CLI companion tool (published as `cargo-insta`)
  src/
    main.rs             # Entry point
    cli.rs              # Clap-based CLI: test, review, accept, reject, pending-snapshots
    container.rs        # SnapshotContainer — manages snapshot file operations
    walk.rs             # Filesystem walker for finding .snap and .pending-snap files
    inline.rs           # Inline snapshot update logic (rewrites Rust source files)
    cargo.rs            # Cargo integration (workspace detection, package discovery)
vscode-insta/           # VS Code extension for .snap file support (TypeScript)
```

## Key Commands

```sh
# Build everything
cargo build --all-features

# Run all tests (recommended way)
make test

# Run tests for individual crates
cargo test -p insta
cargo test -p insta --all-features
cargo test -p cargo-insta

# Format code
make format
# or: cargo fmt --all

# Lint
make lint
# or: cargo clippy --all-targets --all-features -- --deny warnings

# Run the local cargo-insta (not the globally installed one)
cargo run -p cargo-insta -- test
cargo run -p cargo-insta -- review

# Review pending snapshots interactively
cargo insta review

# Accept all pending snapshots
cargo insta accept

# Check MSRV compliance
make check-msrv

# Check minimum dependency versions
make check-minver
```

## Feature Flags (insta crate)

- `colors` (default) — Colored terminal output via `console`
- `redactions` — Value redaction in serialized snapshots (enables `pest`, `serde`)
- `filters` — Regex-based content filters (enables `regex`)
- `glob` — Glob-based test generation (enables `walkdir`, `globset`)
- `json` — JSON serialization format (enables `serde`)
- `yaml` — YAML serialization format (enables `serde`)
- `ron` — RON serialization format
- `csv` — CSV serialization format
- `toml` — TOML serialization format
- `_cargo_insta_internal` — Internal feature for cargo-insta integration (do not use directly)

## Architecture Notes

### Snapshot Lifecycle
1. Test macros (`assert_debug_snapshot!`, `assert_yaml_snapshot!`, etc.) capture a value and serialize it
2. The runtime (`runtime.rs`) compares the serialized value against the stored snapshot
3. On mismatch, behavior depends on `INSTA_UPDATE` env var:
   - `no` (default in CI): test fails with a diff
   - `new`: writes a `.snap.new` pending file for review
   - `always`: auto-accepts the new snapshot
4. `cargo insta review` walks pending snapshots and presents an interactive accept/reject UI
5. For inline snapshots, pending changes are stored as `.pending-snap` JSON files, and `cargo-insta` rewrites the Rust source to update the inline string

### Key Design Patterns
- **Macro-driven API:** All assertion entry points are macros (in `macros.rs`) that capture `file!()`, `line!()`, `module_path!()` for snapshot naming
- **Thread-local settings:** `Settings` in `settings.rs` uses a thread-local stack so tests can configure snapshot behavior (path, redactions, filters) without global state conflicts
- **Content abstraction:** The `content` module provides an intermediate `Content` enum that normalizes different serialization formats for comparison
- **PEG selectors:** Redaction paths use a PEG grammar (`select_grammar.pest`) parsed by `pest` for dot-notation field selection

## Code Style

- Follow standard Rust conventions: `snake_case` for functions/variables, `CamelCase` for types
- Use `clippy` lints — the project enables `clippy::doc_markdown` and `clippy::needless_raw_strings`
- Run `cargo fmt --all` before committing (enforced by `make format-check`)
- Prefer `Result`/`Option` over `unwrap()` in library code
- Public API items must have doc comments
- Keep backward compatibility in mind — this is a widely-used library
- Feature-gated code uses `#[cfg(feature = "...")]` attributes
- Tests go in `tests/` directories within each crate, not inline `#[cfg(test)]` modules (with some exceptions)

## Environment Variables

These control insta's runtime behavior during tests:
- `INSTA_UPDATE` — Snapshot update behavior: `always`, `new`, `no`, `unseen`
- `INSTA_FORCE_PASS` — Force tests to pass even on mismatch (used by cargo-insta)
- `INSTA_WORKSPACE_ROOT` — Override workspace root detection
- `INSTA_GLOB_FILTER` — Filter which glob patterns to run
- `INSTA_OUTPUT` — Output format: `diff`, `summary`, `minimal`, `none`
- `INSTA_REQUIRE_FULL_MATCH` — Require all snapshot assertions to match

## Contributing

- Read `CONTRIBUTING.md` before submitting PRs
- Non-trivial `cargo-insta` changes need integration tests in `cargo-insta/tests/main.rs`
- Website/docs changes go to the separate [insta-website](https://github.com/mitsuhiko/insta-website) repo

---
*Generated by [ai-ready](https://github.com/lunacompsia-oss/ai-ready)*
