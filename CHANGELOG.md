# Changelog

All notable changes to Alex's Tube V are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] — 2025-07-04

### Added
- **Universal accessibility overhaul** — WCAG AA compliance across all UI surfaces
  - CSS accessibility foundation: focus-visible rings, `prefers-reduced-motion`, responsive breakpoints, `forced-colors` support
  - ARIA semantics: roles, labels, live regions on all interactive elements
  - Keyboard navigation: extended Escape cascade, arrow-key menu traversal, Enter/Space activation for custom controls
  - Focus traps on all modals (omnibox, keyboard help, station sheet)
  - Screen reader announcer (`aria-live="assertive"`) wired to toasts, journey results, cost estimates, transit scores, coverage stats, map movements
  - CRT scanline overlay toggle (auto-disabled for `prefers-reduced-motion`)
  - Font scaling via CSS `rem` units
  - Color-independent legend indicators (pattern overlays via `data-type` attributes)
  - High-contrast cursor and interactive element styling
- **Living Network Engine** — kinematic A*, route utility scoring, cohesive UX bridge
- **Monte Carlo congestion simulation** — stochastic demand modelling with parallel computation
- **AI urban planning engine** — facility-location station infill, MST network synthesis
- **Offline-first basemap** — baked London lines + stations into binary so map always renders
- **TfL live API integration** — real-time arrival data
- **SQLite spatial cache** — WAL mode, R-tree indexing, quantised coordinate binary IPC
- **Interactive Leaflet map** — SVG roundel icons, catchment isochrones, desert detection
- **Demand heatmaps** — spatial analysis with Turf.js integration
- **Journey planner** — multi-leg A* routing with interchanges
- **Cost estimation engine** — infrastructure cost modelling
- **Transit scoring system** — grade-based accessibility scoring
- **Coverage analysis** — desert detection, catchment statistics

### Fixed
- Eval handle delay and brace escaping in WebView IPC
- SVG roundel rendering corruption (pre-encode as base64)
- Catchment desert rendering edge cases
- rstar catchment panic on empty results
- Premature "Engine not ready" boot message
- Blank map: stations via direct fetch fallback
- Compilation errors: Handler trait bounds, type mismatches, Send safety

### Changed
- Rebranded package to `alexs-tube-v` with full crate metadata
- Reorganised project files — removed stale config, tracked vendor assets
- Comprehensive `.gitignore` — Rust targets, IDE, OS junk, agent worktrees, secrets
- Tracked `Cargo.lock` for reproducible binary builds

### Infrastructure
- GitHub Actions CI: check → clippy → fmt → release build + artifact upload
- Issue templates (bug report, feature request)
- Pull request template with checklist
- Enterprise README with architecture diagram, badges, quickstart guide
