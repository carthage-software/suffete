# Lists all available commands.
list:
    @just --list

# Builds the library in release mode.
build:
    cargo build --release

# Detects problems using rustfmt, clippy, and cargo check.
check:
    cargo +nightly fmt --all -- --check --unstable-features
    cargo +nightly clippy --all-targets --all-features -- -D warnings
    cargo +nightly check --locked

# Fixes linting problems automatically using clippy, cargo fix, and rustfmt.
fix:
    cargo +nightly clippy --all-targets --all-features --fix --allow-dirty --allow-staged
    cargo +nightly fix --allow-dirty --allow-staged
    cargo +nightly fmt --all -- --unstable-features

# Runs all tests.
test:
    cargo test --locked --all-targets

# Runs the benchmark suite.
bench:
    cargo bench

# Cleans all build artifacts.
clean:
    cargo clean

# Publishes the crate to crates.io.
publish:
    cargo publish
