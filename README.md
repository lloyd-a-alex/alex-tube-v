# Alex's Tube V

**London Transport Network Visualiser & Spatial Analysis Engine**

[![CI](https://github.com/lloyd-a-alex/alex-tube-v/actions/workflows/ci.yml/badge.svg)](https://github.com/lloyd-a-alex/alex-tube-v/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-2021-orange?logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)]()

Interactive London Underground, DLR, Elizabeth Line & National Rail visualiser with A* pathfinding, Monte Carlo congestion simulation, demand modelling, AI urban planning, and a universal accessibility layer — all in a single-file Dioxus desktop application.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Dioxus Desktop App                     │
│                   (src/main.rs — ~15k lines)             │
├──────────┬──────────┬───────────┬───────────────────────┤
│  Axum    │  SQLite  │  Dioxus   │  Leaflet + Turf.js    │
│  Server  │  (WAL)   │  RSX UI   │  (WebView2 map)       │
├──────────┴──────────┴───────────┴───────────────────────┤
│  Engine Layer                                            │
│  ┌──────────┐ ┌──────────────┐ ┌────────────────────┐  │
│  │ A* +     │ │ Monte Carlo  │ │ Living Network     │  │
│  │ Kinematic│ │ Congestion   │ │ (demand + utility) │  │
│  │ Routing  │ │ Simulation   │ │                    │  │
│  └──────────┘ └──────────────┘ └────────────────────┘  │
├─────────────────────────────────────────────────────────┤
│  Accessibility Layer                                     │
│  ARIA · Focus Traps · Screen Reader · Keyboard Nav       │
│  High Contrast · Reduced Motion · Font Scaling           │
└─────────────────────────────────────────────────────────┘
```

## Features

| Category | Capabilities |
|----------|-------------|
| **Map** | Interactive Leaflet map with SVG roundels, offline basemap, CRT scanline overlay |
| **Routing** | A* pathfinding with congestion-aware costs, multi-leg journey planning |
| **Simulation** | Monte Carlo congestion, demand heatmaps, disruption scenarios |
| **Planning** | AI urban planner — facility-location station infill, MST network synthesis |
| **Analysis** | Transit scoring, coverage analysis, catchment isochrones, desert detection |
| **Data** | TfL live API integration, SQLite spatial cache (WAL mode), R-tree indexing |
| **A11y** | WCAG AA, screen reader announcer, focus traps, keyboard nav, high contrast, `prefers-reduced-motion` |

## Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (stable, edition 2021)
- Windows 10/11 (WebView2 runtime), macOS, or Linux (WebKitGTK)

### Build & Run

```bash
# Clone
git clone https://github.com/lloyd-a-alex/alex-tube-v.git
cd alex-tube-v

# Debug build
cargo run

# Release build (optimised)
cargo build --release
./target/release/alexs-tube-v
```

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+K` | Open omnibox search |
| `?` | Show keyboard help |
| `Esc` | Close active panel/modal |
| `Arrow keys` | Navigate menus |
| `Enter/Space` | Activate focused element |

## Project Structure

```
alex-tube-v/
├── .github/
│   ├── workflows/ci.yml      # CI: check → clippy → fmt → build
│   ├── ISSUE_TEMPLATE/        # Bug report + feature request templates
│   └── pull_request_template.md
├── data/                      # Vendor assets (Leaflet, GeoJSON, Turf.js)
├── src/
│   └── main.rs               # Entire application (~15k lines, single file)
├── Cargo.toml                 # Dependencies & metadata
├── Cargo.lock                 # Pinned dependency versions
├── .gitignore
├── CHANGELOG.md
└── README.md
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| UI Framework | [Dioxus 0.5](https://dioxuslabs.com/) (desktop, WebView) |
| Web Server | [Axum 0.7](https://github.com/tokio-rs/axum) |
| Async Runtime | [Tokio](https://tokio.rs/) |
| Database | [rusqlite](https://github.com/rusqlite/rusqlite) (bundled, WAL mode) |
| Spatial | [geo](https://docs.rs/geo), [rstar](https://docs.rs/rstar) (R-tree) |
| Map | [Leaflet.js](https://leafletjs.com/) + [Turf.js](https://turfjs.org/) |
| Performance | [rayon](https://docs.rs/rayon), [parking_lot](https://docs.rs/parking_lot), [moka](https://docs.rs/moka) cache |
| Serialization | [serde](https://serde.rs/) + [serde_json](https://docs.rs/serde_json) |

## CI/CD

GitHub Actions runs on every push/PR to `main`:

1. **cargo check** — compile verification
2. **cargo clippy** — lint warnings
3. **cargo fmt** — style check
4. **cargo build --release** — release binary + artifact upload

## Contributing

1. Fork the repo
2. Create a feature branch (`git checkout -b feat/my-feature`)
3. Commit with [conventional commits](https://www.conventionalcommits.org/)
4. Open a PR using the provided template
5. Ensure CI passes

## License

MIT © Alex
