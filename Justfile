set positional-arguments
alias t := test
alias f := fix
alias b := build
alias c := clean

# Default to display help menu
default:
    @just --list

# Runs all CI checks
ci: fix check lychee check-udeps

# Performs lychee checks
lychee:
    @command -v lychee >/dev/null 2>&1 || cargo install lychee
    lychee --config ./lychee.toml .

# Checks formatting, clippy, and tests
check: check-format check-clippy test

# Fixes formatting and clippy issues
fix: format-fix clippy-fix

# Runs tests across workspace
test:
    @command -v cargo-nextest >/dev/null 2>&1 || cargo install cargo-nextest
    RUSTFLAGS="-D warnings" cargo nextest run --workspace --all-features

# Checks formatting
check-format:
    cargo +nightly fmt --all -- --check

# Fixes formatting issues
format-fix:
    cargo fix --allow-dirty --allow-staged
    cargo +nightly fmt --all

# Checks clippy
check-clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Fixes clippy issues
clippy-fix:
    cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# Builds the workspace with release
build:
    cargo build --workspace --release

# Cleans the workspace
clean:
    cargo clean

# Checks for unused dependencies
check-udeps:
    @command -v cargo-udeps >/dev/null 2>&1 || cargo install cargo-udeps
    cargo +nightly udeps --workspace --all-features --all-targets

# Runs benchmarks
bench:
    cargo bench --workspace

# Generates documentation
doc:
    cargo doc --workspace --no-deps --open

# Watches for changes and runs tests
watch-test:
    cargo watch -x "nextest run --workspace --all-features"

# Watches for changes and checks
watch-check:
    cargo watch -x "clippy --workspace --all-targets"

# ============================================================================
# Examples
# ============================================================================

# Run the Yahoo Finance example (fetches real market data)
example-yahoo:
    cargo run --package toraniko --example yahoo_finance

# Run the style factors example
example-style:
    cargo run --package toraniko --example style_factors

# Run the factor returns example
example-factor:
    cargo run --package toraniko --example factor_returns

# Run the data utilities example
example-utils:
    cargo run --package toraniko --example data_utilities

# Run the full pipeline example
example-pipeline:
    cargo run --package toraniko --example full_pipeline

# Run all examples
examples: example-yahoo example-style example-factor example-utils example-pipeline

# ============================================================================
# Factor Attribution Analysis
# ============================================================================

# Analyze factor attribution for a stock (default: 5 years)
# Usage: just analyze SYMBOL [YEARS]
# Example: just analyze UNH
# Example: just analyze AAPL 3
analyze symbol years="5":
    cargo run --package toraniko --bin analyze --features cli -- {{symbol}} --years {{years}}
