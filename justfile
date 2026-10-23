# Task runner for slimmap. Install just: https://just.systems

set shell := ["bash", "-cu"]
set windows-shell := ["C:/Program Files/Git/bin/bash.exe", "-cu"]

default:
    @just --list

# Type-check every crate, including tests and benches
check:
    cargo check --manifest-path slimmap/Cargo.toml --all-targets
    cargo check --manifest-path testing/Cargo.toml --all-targets
    cargo check --manifest-path benchmarks/Cargo.toml --all-targets

# Debug build of every crate
build:
    cargo build --manifest-path slimmap/Cargo.toml
    cargo build --manifest-path testing/Cargo.toml
    cargo build --manifest-path benchmarks/Cargo.toml

# Optimized build of every crate
build-release:
    cargo build --release --manifest-path slimmap/Cargo.toml
    cargo build --release --manifest-path testing/Cargo.toml
    cargo build --release --manifest-path benchmarks/Cargo.toml

# Run unit and integration tests for every crate
test:
    cargo test --manifest-path slimmap/Cargo.toml
    cargo test --manifest-path testing/Cargo.toml
    cargo test --manifest-path benchmarks/Cargo.toml

# Install the nightly toolchain with Miri (needed once, and by `ci`)
miri-setup:
    rustup toolchain install nightly --component miri

# Run tests under Miri to catch undefined behaviour (needs nightly; see miri-setup)
miri:
    cargo +nightly miri test --manifest-path slimmap/Cargo.toml
    cargo +nightly miri test --manifest-path testing/Cargo.toml

# Run clippy with warnings treated as errors
lint:
    cargo clippy --manifest-path slimmap/Cargo.toml --all-targets -- -D warnings
    cargo clippy --manifest-path testing/Cargo.toml --all-targets -- -D warnings
    cargo clippy --manifest-path benchmarks/Cargo.toml --all-targets -- -D warnings

# Format every crate in place
fmt:
    cargo fmt --manifest-path slimmap/Cargo.toml
    cargo fmt --manifest-path testing/Cargo.toml
    cargo fmt --manifest-path benchmarks/Cargo.toml

# Fail if any crate is not rustfmt-clean
fmt-check:
    cargo fmt --manifest-path slimmap/Cargo.toml --check
    cargo fmt --manifest-path testing/Cargo.toml --check
    cargo fmt --manifest-path benchmarks/Cargo.toml --check

# Build rustdoc for slimmap and open it
doc:
    cargo doc --manifest-path slimmap/Cargo.toml --no-deps --open

# Run the benchmarks, writing benchmarks/results/results.csv
bench:
    cargo run --release --manifest-path benchmarks/Cargo.toml

# Turn benchmark CSV into PNG charts
plot:
    python benchmarks/plot.py

# Benchmark, then plot the results
bench-plot: bench plot

# Everything CI would check: formatting, types, lints, tests, Miri
ci: fmt-check check lint test miri

# Remove build output from every crate
clean:
    cargo clean --manifest-path slimmap/Cargo.toml
    cargo clean --manifest-path testing/Cargo.toml
    cargo clean --manifest-path benchmarks/Cargo.toml
