# ── justfile for alexs-tube-v ──────────────────────────────────────────────────
# Install `just`: cargo install just
# List recipes: just --list

# Build & Run
run:
    cargo run --release

# Dev run (debug)
dev:
    cargo run

# Quick check (no build)
check:
    cargo check

# Lint with clippy (all targets, deny warnings)
clippy:
    cargo clippy --all-targets -- -D warnings

# Run all tests with nextest
test:
    cargo nextest run

# Run default test runner
test-legacy:
    cargo test

# Run benchmarks
bench:
    cargo bench

# Watch for changes and auto-check
watch:
    cargo watch -x check

# Watch and auto-run
watch-run:
    cargo watch -x "run --release"

# Security audit
audit:
    cargo audit

# Update dependencies in Cargo.toml
upgrade:
    cargo upgrade

# Update Cargo.lock only
update:
    cargo update

# Sort Cargo.toml dependencies
sort-deps:
    cargo sort

# Expand all macros
expand:
    cargo expand

# Binary size analysis
bloat:
    cargo bloat --release

# Generate flamegraph
flamegraph:
    cargo flamegraph --bin alexs-tube-v

# Build docs
docs:
    cargo doc --no-deps

# Clean build artifacts
clean:
    cargo clean

# Full CI pipeline (clippy + test + audit)
ci: clippy test audit
