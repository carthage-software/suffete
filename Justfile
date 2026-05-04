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

# Builds the book and serves it locally with live reload on port 4321.
book:
    @just _book-install-mermaid
    cd book && mdbook serve --open --port 4321

# Builds the book once into book/book/ without serving.
book-build:
    @just _book-install-mermaid
    cd book && mdbook build

# Removes the rendered book.
book-clean:
    cd book && mdbook clean

# One-time install of mdbook + mdbook-mermaid via cargo, plus the
# mermaid theme assets the book.toml references.
_book-install-mermaid:
    @command -v mdbook >/dev/null         || cargo install mdbook
    @command -v mdbook-mermaid >/dev/null || cargo install mdbook-mermaid
    @test -f book/mermaid.min.js          || (cd book && mdbook-mermaid install .)

# Publishes the crate to crates.io.
publish:
    cargo publish
