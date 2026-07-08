# Dev Tools for alexs-tube-v

A curated set of Cargo subcommands and utilities to supercharge your
edit-compile-run loop, debug macros, profile performance, and ship with
confidence.

---

## 🏃 Dev Velocity (Speed up your edit-compile-run loop)

| Tool | Install | Why it's insane for *this* project |
|------|---------|------------------------------------|
| **`cargo-watch`** | `cargo install cargo-watch` | Automatically runs `cargo check` on file save. ~16k lines — catches syntax/type errors instantly without waiting for a full build. Pair with `cargo watch -x "run --release"` for auto-restart. |
| **`cargo-nextest`** | `cargo install cargo-nextest` | Runs tests in parallel with much less overhead than the default test runner. Great for geometry, routing, and AI algorithm tests. |

## 🔍 Debugging & Macros (Untangle the complexity)

| Tool | Install | Why it's insane for *this* project |
|------|---------|------------------------------------|
| **`cargo-expand`** | `cargo install cargo-expand` | Dioxus uses lots of macros (`rsx!`, `#[component]`). Shows exactly what Rust code the macros generate. Essential for tracking down weird compile errors or lifetime issues in your UI. |
| **`cargo-bloat`** | `cargo install cargo-bloat` | Binary is huge (Tokio + Axum + Dioxus + leaflet + rstar + mimalloc). Bloat shows which crates contribute the most to binary size. |

## 📈 Performance Profiling

| Tool | Install | Why it's insane for *this* project |
|------|---------|------------------------------------|
| **`cargo-flamegraph`** | `cargo install flamegraph` | Generates interactive flame graphs. Run on A* pathfinding or Monte Carlo simulation to see exactly where CPU time is spent. Zero code changes required — just `cargo flamegraph --bin alexs-tube-v`. |

## 🛡️ Production & Security

| Tool | Install | Why it's insane for *this* project |
|------|---------|------------------------------------|
| **`cargo-audit`** | `cargo install cargo-audit` | Checks for known vulnerabilities in dependencies. Run `cargo audit` regularly. |
| **`cargo-deny`** | `cargo install cargo-deny` | Superset of `audit`. Checks for vulnerabilities, license compliance, and bans duplicate or outdated dependencies. |
| **`cargo-upgrade`** | `cargo install cargo-edit` | Bumps all deps to latest semver-compatible versions in `Cargo.toml`. Unlike `cargo update` (which only updates `Cargo.lock`), this updates `Cargo.toml`. |

## 🧹 Project Hygiene

| Tool | Install | Why it's insane for *this* project |
|------|---------|------------------------------------|
| **`cargo-sort`** | `cargo install cargo-sort` | Sorts `Cargo.toml` dependencies alphabetically. Prevents merge conflicts and keeps it readable with 40+ deps. |
| **`just`** | `cargo install just` | Command runner. See `justfile` for available recipes. |

---

## Quick Start

```bash
# Watch for changes and auto-check
cargo watch -x check

# Run tests with nextest
cargo nextest run

# Profile with flamegraph
cargo flamegraph --bin alexs-tube-v

# Check for vulnerabilities
cargo audit

# Update dependencies
cargo upgrade

# Sort Cargo.toml dependencies
cargo sort

# Expand macros
cargo expand

# Binary size analysis
cargo bloat --release

# List available just recipes
just --list
```
