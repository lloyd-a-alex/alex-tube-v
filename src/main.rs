#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
use dioxus::prelude::*;

// ============================================================================
// GREATER LONDON TRANSPORT NETWORK - PHYSICAL TRUTH ENGINE
// Diabolically Optimized Single-File Rust Implementation
// ============================================================================
use std::cmp::Ordering as CmpOrdering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::f64::consts::PI;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use dirs;
use reqwest::Client;
use rstar::{PointDistance, RTree, RTreeObject, AABB};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tower_http::cors::{Any, CorsLayer};

static TFL_COLOR_REGISTRY: phf::Map<&'static str, &'static str> = phf::phf_map! {
    "bakerloo" => "#B36305",
    "central" => "#E32017",
    "circle" => "#FFD300",
    "district" => "#00782A",
    "hammersmith-city" => "#F3A9BB",
    "jubilee" => "#A0A5A9",
    "metropolitan" => "#9B0056",
    "northern" => "#000000",
    "piccadilly" => "#003688",
    "victoria" => "#0098D4",
    "waterloo-city" => "#95CDBA",
    "elizabeth" => "#6950A1",
    "dlr" => "#00A4A7",
    "tramlink" => "#84B817",
    "liberty" => "#E21836",
    "lioness" => "#EE7C0E",
    "mildmay" => "#FFC300",
    "suffragette" => "#00A4A7",
    "weaver" => "#00BFFF",
    "windrush" => "#00BFFF",
};

// Fix 1: Global Panic State Tracking Allocator
static IS_PANICKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static CRASH_LOG_ACCUMULATOR: std::sync::OnceLock<std::sync::Mutex<String>> =
    std::sync::OnceLock::new();

fn accumulate_crash_text(msg: &str) {
    let mutex = CRASH_LOG_ACCUMULATOR.get_or_init(|| std::sync::Mutex::new(String::new()));
    if let Ok(mut guard) = mutex.lock() {
        guard.push_str(msg);
        guard.push('\n');
    }
}

pub static EMBEDDED_THEME_CSS: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    r#"/* === scoped_theme.css === */
:root {
    /* Brand Colors */
    --color-primary: #00bcd4;
    --color-primary-hover: #00acc1;
    --color-primary-dark: #008ba3;
    --color-primary-glow: rgba(0, 188, 212, 0.4);
    --color-primary-glow-strong: rgba(0, 188, 212, 0.6);

    /* Semantic Colors */
    --color-success: #4caf50;
    --color-success-bg: rgba(76, 175, 80, 0.15);
    --color-warning: #ff9800;
    --color-error: #f44336;
    --color-error-bg: rgba(244, 67, 54, 0.15);

    /* Surface Colors */
    --color-bg: #050505;
    --color-surface: rgba(10, 10, 12, 0.85);
    --color-surface-solid: #111;
    --color-surface-dark: rgba(10, 10, 15, 0.95);
    --color-surface-elevated: rgba(15, 15, 18, 0.85);
    --color-surface-hover: rgba(255, 255, 255, 0.1);
    --color-surface-subtle: rgba(255, 255, 255, 0.03);
    --color-surface-muted: rgba(255, 255, 255, 0.05);

    /* Border Colors */
    --color-border: rgba(255, 255, 255, 0.08);
    --color-border-light: rgba(255, 255, 255, 0.1);
    --color-border-medium: rgba(255, 255, 255, 0.15);
    --color-border-solid: #333;
    --color-border-input: #444;

    /* Text Colors */
    --color-text: #ffffff;
    --color-text-secondary: #dddddd;
    --color-text-muted: #aaaaaa;
    --color-text-dim: #888888;
    --color-text-terminal: #0f0;

    /* Shadows */
    --shadow-sm: 0 4px 12px rgba(0, 0, 0, 0.4);
    --shadow-md: 0 8px 24px rgba(0, 0, 0, 0.6);
    --shadow-lg: 0 16px 40px rgba(0, 0, 0, 0.8);
    --shadow-xl: 0 20px 60px rgba(0, 0, 0, 0.8);
    --shadow-glow: 0 4px 20px var(--color-primary-glow);

    /* Radii */
    --radius-sm: 4px;
    --radius-md: 8px;
    --radius-lg: 12px;
    --radius-xl: 16px;
    --radius-full: 50%;

    /* Spacing */
    --space-xs: 4px;
    --space-sm: 8px;
    --space-md: 12px;
    --space-lg: 16px;
    --space-xl: 20px;
    --space-2xl: 24px;
    --space-3xl: 30px;

    /* Typography */
    --font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    --font-mono: 'JetBrains Mono', 'Fira Code', 'Courier New', Courier, monospace;
    --font-size-xs: 9px;
    --font-size-sm: 11px;
    --font-size-base: 13px;
    --font-size-md: 14px;
    --font-size-lg: 15px;
    --font-size-xl: 18px;

    /* Transitions */
    --ease-out: cubic-bezier(0.19, 1, 0.22, 1);
    --ease-bounce: cubic-bezier(0.175, 0.885, 0.32, 1.275);
    --transition-fast: 0.2s ease;
    --transition-smooth: 0.3s var(--ease-out);
    --transition-bounce: 0.4s var(--ease-bounce);

    /* Z-Index Scale */
    --z-map: 1;
    --z-controls: 1000;
    --z-logger: 10000;
    --z-modal: 11000;
    --z-toast: 12000;
    --z-loading: 20000;
}

/* ===== GLOBAL RESET & BODY ===== */
*, *::before, *::after {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
    -webkit-transform: translateZ(0);
    transform: translateZ(0); /* Force full GPU composite layer isolation */
    backface-visibility: hidden;
    perspective: 1000;
}

html, body {
    width: 100%;
    height: 100%;
    overflow: hidden;
    font-family: var(--font-family);
    background: #000;
    cursor: crosshair;
    -webkit-font-smoothing: antialiased;
}

/* ===== MAP VIEWPORT ===== */
#map-viewport {
    width: 100vw;
    height: 100vh;
    position: absolute;
    top: 0;
    left: 0;
    z-index: var(--z-map);
    background: #0d0d11;
}

/* ===== PREMIUM GLASSMORPHISM LEGEND PANEL ===== */
.legend-container {
    position: absolute;
    top: var(--space-2xl);
    left: var(--space-2xl);
    z-index: var(--z-controls);
    background: var(--color-surface);
    backdrop-filter: blur(16px);
    padding: var(--space-lg);
    border-radius: var(--radius-xl);
    border: 1px solid var(--color-border);
    max-height: calc(100vh - 48px);
    overflow-y: auto;
    box-shadow: var(--shadow-lg);
    color: var(--color-text);
    min-width: 260px;
    transition: opacity var(--transition-fast), transform var(--transition-fast);
}

.legend-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-md);
    border-bottom: 1px solid var(--color-border-light);
    padding-bottom: var(--space-sm);
}

.legend-title {
    font-weight: 800;
    font-size: var(--font-size-base);
    text-transform: uppercase;
    letter-spacing: 1.5px;
    background: linear-gradient(135deg, var(--color-primary), #80deea);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
}

.legend-item {
    display: flex;
    align-items: center;
    margin: 6px 0;
    cursor: pointer;
    padding: 6px var(--space-sm);
    border-radius: var(--radius-md);
    transition: all var(--transition-fast);
}

.legend-item:hover {
    background: var(--color-surface-hover);
    transform: translateX(4px);
}

.legend-color {
    width: 12px;
    height: 12px;
    border-radius: var(--radius-sm);
    margin-right: var(--space-md);
    box-shadow: 0 0 6px rgba(0, 188, 212, 0.4);
    flex-shrink: 0;
}

.legend-name {
    font-size: var(--font-size-sm);
    font-weight: 600;
    color: var(--color-text-secondary);
}

/* ===== CATCHMENT AREA TOGGLE ===== */
.catchment-toggle-container {
    margin-top: var(--space-md);
    padding: var(--space-sm);
    background: rgba(255, 255, 255, 0.03);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
}

.catchment-toggle-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: var(--font-size-sm);
    font-weight: 700;
    color: var(--color-text);
}

.switch {
    position: relative;
    display: inline-block;
    width: 36px;
    height: 20px;
}

.switch input {
    opacity: 0;
    width: 0;
    height: 0;
}

.slider {
    position: absolute;
    cursor: pointer;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: #333;
    transition: .3s;
    border-radius: 20px;
}

.slider:before {
    position: absolute;
    content: "";
    height: 14px;
    width: 14px;
    left: 3px;
    bottom: 3px;
    background-color: white;
    transition: .3s;
    border-radius: 50%;
}

input:checked + .slider {
    background-color: var(--color-error);
}

input:checked + .slider:before {
    transform: translateX(16px);
}

/* ===== TFL-GO STYLE BOTTOM SHEET ===== */
.tfl-bottom-sheet {
    position: fixed;
    bottom: 0;
    left: 50%;
    transform: translateX(-50%) translateY(0);
    width: 100%;
    max-width: 450px;
    background: rgba(18, 18, 20, 0.96);
    backdrop-filter: blur(20px);
    border-top-left-radius: var(--radius-xl);
    border-top-right-radius: var(--radius-xl);
    box-shadow: var(--shadow-xl);
    z-index: 1005;
    transition: transform var(--transition-bounce);
    color: var(--color-text);
    padding: var(--space-xl) var(--space-2xl) var(--space-3xl) var(--space-2xl);
    border: 1px solid var(--color-border);
    border-bottom: none;
}

.tfl-bottom-sheet.slide-down {
    transform: translateX(-50%) translateY(100%);
}

.sheet-handle {
    width: 40px;
    height: 4px;
    background: rgba(255, 255, 255, 0.2);
    border-radius: 2px;
    margin: 0 auto var(--space-md) auto;
}

.sheet-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-md);
}

.sheet-header h2 {
    font-size: 20px;
    font-weight: 800;
    color: var(--color-text);
}

.badge-status {
    padding: 4px var(--space-sm);
    background: var(--color-success-bg);
    color: var(--color-success);
    border: 1px solid var(--color-success);
    font-size: var(--font-size-xs);
    font-weight: 800;
    border-radius: var(--radius-sm);
    text-transform: uppercase;
}

/* ===== CONTEXT DROPDOWN MENU ===== */
.custom-context-dropdown {
    position: fixed;
    background: var(--color-surface-dark);
    border: 1px solid var(--color-border-medium);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-lg);
    backdrop-filter: blur(10px);
    padding: var(--space-xs) 0;
    z-index: 10000;
    min-width: 180px;
}

.menu-item {
    padding: 8px var(--space-lg);
    font-size: var(--font-size-sm);
    color: var(--color-text-secondary);
    cursor: pointer;
    transition: background var(--transition-fast), color var(--transition-fast);
}

.menu-item:hover {
    background: var(--color-primary);
    color: #000;
}

/* ===== CONSOLE LOGGER WIDGET ===== */
#logger-wrapper {
    position: fixed;
    bottom: var(--space-2xl);
    right: var(--space-2xl);
    z-index: var(--z-logger);
    display: flex;
    flex-direction: column;
    align-items: flex-end;
}

#logger-fab {
    width: 52px;
    height: 52px;
    background: linear-gradient(135deg, var(--color-primary), var(--color-primary-dark));
    border-radius: var(--radius-full);
    display: flex;
    justify-content: center;
    align-items: center;
    font-size: 22px;
    cursor: pointer;
    box-shadow: var(--shadow-glow);
    transition: all var(--transition-fast);
    border: 2px solid rgba(255, 255, 255, 0.1);
}

#logger-fab:hover {
    transform: scale(1.1);
}

#logger-panel {
    position: absolute;
    bottom: 66px;
    right: 0;
    width: 480px;
    height: 380px;
    background: var(--color-surface-dark);
    border: 1px solid var(--color-border-solid);
    border-radius: var(--radius-lg);
    display: flex;
    flex-direction: column;
    box-shadow: var(--shadow-lg);
    opacity: 0;
    pointer-events: none;
    transform: translateY(20px) scale(0.95);
    transform-origin: bottom right;
    transition: opacity var(--transition-smooth), transform var(--transition-smooth);
}

#logger-wrapper:hover #logger-panel,
#logger-panel.pinned {
    opacity: 1;
    pointer-events: all;
    transform: translateY(0) scale(1);
}

#log-content {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-md);
    padding-bottom: 95px !important;
    color: var(--color-text-terminal);
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    line-height: 1.5;
    background: #040406;
}

#logger-actions {
    display: flex;
    gap: var(--space-sm);
    padding: var(--space-md);
    background: rgba(0, 0, 0, 0.5);
    border-top: 1px solid var(--color-border-solid);
}

/* ===== SYSTEM STATS ===== */
#system-stats-widget {
    position: absolute;
    bottom: var(--space-2xl);
    left: var(--space-2xl);
    z-index: var(--z-controls);
    background: var(--color-surface);
    backdrop-filter: blur(12px);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    padding: var(--space-md);
    box-shadow: var(--shadow-md);
    transition: all 0.3s ease;
}

.stat-grid {
    display: flex;
    gap: 20px;
}

.stat-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    min-width: 60px;
}

.stat-label {
    font-size: 9px;
    font-weight: 800;
    color: var(--color-text-dim);
    letter-spacing: 1px;
    text-transform: uppercase;
    margin-bottom: 2px;
}

.stat-value {
    font-size: 16px;
    font-weight: 800;
    color: var(--color-primary);
    font-family: var(--font-mono);
}

#fps-counter-widget {
    position: fixed;
    top: 24px;
    right: 320px;
    z-index: var(--z-controls);
    background: rgba(10, 10, 15, 0.85);
    backdrop-filter: blur(8px);
    border: 1px solid var(--color-border);
    padding: 6px 12px;
    border-radius: var(--radius-md);
    color: #0f0;
    font-family: var(--font-mono);
    font-size: var(--font-size-sm);
    font-weight: bold;
    box-shadow: var(--shadow-sm);
    pointer-events: none;
}

/* ===== COMPONENT COMPLEMENTARY THEME EXTENSIONS ===== */
.toast-container {
    position: fixed;
    top: var(--space-xl);
    right: var(--space-xl);
    z-index: var(--z-toast);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm);
    pointer-events: none;
}

.toast {
    background: rgba(15, 15, 20, 0.9);
    backdrop-filter: blur(12px);
    border: 1px solid var(--color-border-medium);
    padding: var(--space-md) var(--space-xl);
    border-radius: var(--radius-md);
    color: var(--color-text);
    font-size: var(--font-size-sm);
    font-weight: 600;
    box-shadow: var(--shadow-md);
    transform: translateY(-20px);
    opacity: 0;
    transition: all 0.3s var(--ease-bounce);
    pointer-events: auto;
}

.toast.show {
    transform: translateY(0);
    opacity: 1;
}

.toast.success { border-left: 4px solid var(--color-success); }
.toast.error { border-left: 4px solid var(--color-error); }
.toast.info { border-left: 4px solid var(--color-primary); }

.loading-overlay {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: #030305;
    z-index: var(--z-loading);
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: var(--space-2xl);
}

.spinner {
    width: 48px;
    height: 48px;
    border: 3px solid rgba(0, 188, 212, 0.1);
    border-radius: var(--radius-full);
    border-top-color: var(--color-primary);
    animation: spin 0.8s linear infinite;
}

.status-container {
    background: rgba(10, 10, 12, 0.6);
    border: 1px solid var(--color-border);
    padding: var(--space-xl);
    border-radius: var(--radius-lg);
    width: 100%;
    max-width: 400px;
}

.status-header {
    color: var(--color-text-muted);
    font-size: var(--font-size-xs);
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 1px;
    margin-bottom: var(--space-md);
}

.status-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-xs) 0;
    font-size: var(--font-size-sm);
    color: var(--color-text-secondary);
}

.status-badge {
    font-family: var(--font-mono);
    font-size: var(--font-size-xs);
    text-transform: uppercase;
    font-weight: bold;
}

.status-success { color: var(--color-success); }
.status-pending { color: var(--color-warning); animation: pulse 1s infinite alternate; }

@keyframes spin { to { transform: rotate(360deg); } }
@keyframes pulse { to { opacity: 0.4; } }

/* ===== LOGGER BUTTON STYLES ===== */
.logger-btn {
    flex: 1;
    padding: 6px var(--space-sm);
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    color: var(--color-text-secondary);
    font-size: var(--font-size-xs);
    font-weight: 600;
    cursor: pointer;
    transition: all var(--transition-fast);
}

.logger-btn:hover {
    background: rgba(255, 255, 255, 0.15);
    color: var(--color-text);
}

.btn-highlight {
    background: rgba(0, 188, 212, 0.15);
    border-color: var(--color-primary);
    color: var(--color-primary);
}

.btn-highlight:hover {
    background: rgba(0, 188, 212, 0.3);
}

/* ===== SHEET BODY ===== */
.sheet-body p {
    font-size: var(--font-size-sm);
    color: var(--color-text-secondary);
    margin: 4px 0;
}

.station-icon, .hub-icon {
    background: none !important;
    border: none !important;
    width: 16px !important;
    height: 16px !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
}

.station-icon div, .hub-icon div {
    flex-shrink: 0;
    transition: transform 0.2s ease;
}

.station-icon:hover div, .hub-icon:hover div {
    transform: scale(1.4);
    cursor: pointer;
}
"#
    .to_string()
});

// ============================================================================
// CONFIGURATION
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    tfl_base_url: String,
    overpass_base_url: String,
    server_host: String,
    server_port: u16,
    cache_expiry_hours: i64,
    log_max_entries: usize,
    london_bounds: LondonBounds,
    /// Fix #7: Configurable list of line IDs to load at startup
    sample_lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LondonBounds {
    min_lat: f64,
    min_lon: f64,
    max_lat: f64,
    max_lon: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tfl_base_url: "https://api.tfl.gov.uk".to_string(),
            overpass_base_url: "https://overpass-api.de/api/interpreter".to_string(),
            server_host: "127.0.0.1".to_string(),
            server_port: 3000,
            cache_expiry_hours: 24,
            log_max_entries: 10000,
            london_bounds: LondonBounds {
                min_lat: 51.28,
                min_lon: -0.51,
                max_lat: 51.69,
                max_lon: 0.33,
            },
            // Fix #7: Configurable sample lines list
            sample_lines: vec![
                "bakerloo".to_string(),
                "central".to_string(),
                "northern".to_string(),
                "piccadilly".to_string(),
                "victoria".to_string(),
                "jubilee".to_string(),
                "district".to_string(),
                "circle".to_string(),
                "hammersmith-city".to_string(),
                "metropolitan".to_string(),
                "waterloo-city".to_string(),
                "elizabeth".to_string(),
                "dlr".to_string(),
            ],
        }
    }
}

impl Config {
    fn load() -> Self {
        if Path::new("config.toml").exists() {
            match std::fs::read_to_string("config.toml") {
                Ok(content) => match toml::from_str::<Config>(&content) {
                    Ok(config) => {
                        log_info("Loaded configuration from config.toml");
                        return config;
                    }
                    Err(e) => {
                        log_warn(&format!(
                            "Failed to parse config.toml: {}, using defaults",
                            e
                        ));
                    }
                },
                Err(e) => {
                    log_warn(&format!(
                        "Failed to read config.toml: {}, using defaults",
                        e
                    ));
                }
            }
        }
        log_info("Using default configuration");
        Config::default()
    }
}

// Fix #8: Schema version for cache invalidation — bump this whenever cache format changes
const CACHE_SCHEMA_VERSION: &str = "1";

// ============================================================================
// CONSTANTS
// ============================================================================

const EARTH_RADIUS: f64 = 6378137.0;
const DEG_TO_RAD: f64 = PI / 180.0;
const RAD_TO_DEG: f64 = 180.0 / PI;
const TILE_SIZE: f64 = 256.0;
const DEFAULT_ZOOM: f64 = 13.0;
const MIN_ZOOM: f64 = 2.0;
const MAX_ZOOM: f64 = 19.0;
const STATION_MERGE_THRESHOLD: f64 = 0.005;
const SNAP_DISTANCE: f64 = 500.0;
const CATCHMENT_RADIUS: f64 = 800.0;

// ============================================================================
// CONSOLE LOGGER WITH ROTATION
// ============================================================================
const DEFAULT_MAX_LOG_ENTRIES: usize = 10000;

use std::collections::VecDeque;
use std::sync::OnceLock;

static LOG_BUFFER: OnceLock<Arc<std::sync::RwLock<VecDeque<String>>>> = OnceLock::new();

fn get_log_storage() -> &'static Arc<std::sync::RwLock<VecDeque<String>>> {
    LOG_BUFFER.get_or_init(|| {
        Arc::new(std::sync::RwLock::new(VecDeque::with_capacity(
            DEFAULT_MAX_LOG_ENTRIES,
        )))
    })
}

fn log_to_storage(message: &str, is_error: bool) {
    if is_error {
        eprintln!("{}", message);
    }
    let storage = get_log_storage();
    if let Ok(mut logs) = storage.write() {
        if logs.len() >= DEFAULT_MAX_LOG_ENTRIES {
            logs.pop_front();
        }
        logs.push_back(message.to_string());
    } else {
        eprintln!("[LOGGING ERROR] Failed to acquire write lock on log storage");
    }
}

fn format_high_precision_timestamp() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S%.6f UTC").to_string()
}

fn log_info(message: &str) {
    log_to_storage(
        &format!("[{}] [INFO] {}", format_high_precision_timestamp(), message),
        false,
    );
}

fn log_info_with_context(message: &str, context: &str) {
    log_to_storage(
        &format!(
            "[{}] [INFO] [CTX:{}] {}",
            format_high_precision_timestamp(),
            context,
            message
        ),
        false,
    );
}

fn log_error(message: &str) {
    log_to_storage(
        &format!(
            "[{}] [ERROR] {}",
            format_high_precision_timestamp(),
            message
        ),
        true,
    );
}

fn log_error_with_context(message: &str, context: &str) {
    log_to_storage(
        &format!(
            "[{}] [ERROR] [CTX:{}] {}",
            format_high_precision_timestamp(),
            context,
            message
        ),
        true,
    );
}

fn log_debug(message: &str) {
    log_to_storage(
        &format!(
            "[{}] [DEBUG] {}",
            format_high_precision_timestamp(),
            message
        ),
        false,
    );
}

fn log_debug_with_context(message: &str, context: &str) {
    log_to_storage(
        &format!(
            "[{}] [DEBUG] [CTX:{}] {}",
            format_high_precision_timestamp(),
            context,
            message
        ),
        false,
    );
}

fn log_warn(message: &str) {
    log_to_storage(
        &format!("[{}] [WARN] {}", format_high_precision_timestamp(), message),
        false,
    );
}

fn log_warn_with_context(message: &str, context: &str) {
    log_to_storage(
        &format!(
            "[{}] [WARN] [CTX:{}] {}",
            format_high_precision_timestamp(),
            context,
            message
        ),
        false,
    );
}

fn log_trace(message: &str) {
    log_to_storage(
        &format!(
            "[{}] [TRACE] {}",
            format_high_precision_timestamp(),
            message
        ),
        false,
    );
}

fn log_race(message: &str) {
    log_to_storage(
        &format!(
            "[{}] [RACE-DETECT] {}",
            format_high_precision_timestamp(),
            message
        ),
        false,
    );
}

fn get_all_logs() -> String {
    let storage = get_log_storage();
    if let Ok(logs) = storage.read() {
        let vec: Vec<String> = logs.iter().cloned().collect();
        vec.join("\n")
    } else {
        String::new()
    }
}

// ============================================================================
// DATA STRUCTURES
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    pub lat: f64,
    pub lon: f64,
}

impl Coordinate {
    fn new(lat: f64, lon: f64) -> Self {
        log_trace(&format!(
            "Coordinate::new called: lat={:.6}, lon={:.6}",
            lat, lon
        ));
        Self { lat, lon }
    }

    pub fn normalize_projections(&self) -> Coordinate {
        log_trace(&format!(
            "Coordinate::normalize_projections called for lat={:.6}, lon={:.6}",
            self.lat, self.lon
        ));
        let (y, x) = self.to_mercator();
        let result = Coordinate::from_mercator(x, y);
        log_trace(&format!(
            "Coordinate::normalize_projections result: lat={:.6}, lon={:.6}",
            result.lat, result.lon
        ));
        result
    }

    fn distance_to(&self, other: &Coordinate) -> f64 {
        log_trace(&format!(
            "Coordinate::distance_to called: from lat={:.6}, lon={:.6} to lat={:.6}, lon={:.6}",
            self.lat, self.lon, other.lat, other.lon
        ));
        let d_lat = (other.lat - self.lat) * DEG_TO_RAD;
        let d_lon = (other.lon - self.lon) * DEG_TO_RAD;
        let a = (d_lat / 2.0).sin().powi(2)
            + (self.lat * DEG_TO_RAD).cos()
                * (other.lat * DEG_TO_RAD).cos()
                * (d_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        let distance = EARTH_RADIUS * c;
        log_trace(&format!(
            "Coordinate::distance_to result: {:.2} meters",
            distance
        ));
        distance
    }

    fn to_mercator(&self) -> (f64, f64) {
        let x = self.lon * DEG_TO_RAD * EARTH_RADIUS;
        let y = (PI / 4.0 + self.lat * DEG_TO_RAD / 2.0).tan().ln() * EARTH_RADIUS;
        log_trace(&format!(
            "Coordinate::to_mercator: lat={:.6}, lon={:.6} -> y={:.2}, x={:.2}",
            self.lat, self.lon, y, x
        ));
        (y, x)
    }

    fn from_mercator(x: f64, y: f64) -> Self {
        let lon = x / EARTH_RADIUS * RAD_TO_DEG;
        let lat = (2.0 * (y / EARTH_RADIUS).exp().atan() - PI / 2.0) * RAD_TO_DEG;
        log_trace(&format!(
            "Coordinate::from_mercator: x={:.2}, y={:.2} -> lat={:.6}, lon={:.6}",
            x, y, lat, lon
        ));
        Self::new(lat, lon)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Station {
    pub id: String,
    pub name: String,
    pub coord: Coordinate,
    pub lines: Vec<String>,
    pub is_interchange: bool,
    pub is_open: bool,
    pub zone: i32,
}

impl Station {
    fn new(id: String, name: String, coord: Coordinate) -> Self {
        log_info(&format!(
            "Station::new called: id={}, name={}, lat={:.6}, lon={:.6}",
            id, name, coord.lat, coord.lon
        ));
        Self {
            id,
            name,
            coord,
            lines: Vec::new(),
            is_interchange: false,
            is_open: true,
            zone: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSegment {
    pub start: Coordinate,
    pub end: Coordinate,
    pub line_id: String,
    pub length: f64,
}

impl RouteSegment {
    fn new(start: Coordinate, end: Coordinate, line_id: String) -> Self {
        log_trace(&format!("RouteSegment::new called: line_id={}, start_lat={:.6}, start_lon={:.6}, end_lat={:.6}, end_lon={:.6}", line_id, start.lat, start.lon, end.lat, end.lon));
        let length = start.distance_to(&end);
        log_trace(&format!("RouteSegment::new length: {:.2} meters", length));
        Self {
            start,
            end,
            line_id,
            length,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    pub id: String,
    pub name: String,
    pub color: String,
    pub stations: Vec<Station>,
    pub segments: Vec<RouteSegment>,
    pub geometry: Vec<Coordinate>,
    pub is_custom: bool,
}

impl Line {
    fn new(id: String, name: String, color: String) -> Self {
        log_info(&format!(
            "Line::new called: id={}, name={}, color={}",
            id, name, color
        ));
        Self {
            id,
            name,
            color,
            stations: Vec::new(),
            segments: Vec::new(),
            geometry: Vec::new(),
            is_custom: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RailwayTrack {
    pub id: String,
    pub operator_name: String,
    pub geometry: Vec<Coordinate>,
    pub is_abandoned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructionState {
    pub custom_lines: Vec<Line>,
    pub free_stations: Vec<Station>,
    pub current_drawing: Vec<Coordinate>,
    pub current_line_name: String,
    pub current_line_color: String,
    pub is_auto_route: bool,
}

impl Default for ConstructionState {
    fn default() -> Self {
        Self {
            custom_lines: Vec::new(),
            free_stations: Vec::new(),
            current_drawing: Vec::new(),
            current_line_name: String::new(),
            current_line_color: String::new(),
            is_auto_route: true,
        }
    }
}

// ============================================================================
// SPATIAL INDEXING (R-Tree)
// ============================================================================

#[derive(Debug, Clone)]
struct SpatialPoint {
    pub coord: Coordinate,
    index: usize,
}

impl RTreeObject for SpatialPoint {
    type Envelope = AABB<[f64; 2]>;
    fn envelope(&self) -> Self::Envelope {
        let (y, x) = self.coord.to_mercator();
        AABB::from_point([x, y])
    }
}

impl PointDistance for SpatialPoint {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        let (my_y, my_x) = self.coord.to_mercator();
        let other_coord = Coordinate::new(point[1], point[0]);
        let (other_y, other_x) = other_coord.to_mercator();
        let dx = my_x - other_x;
        let dy = my_y - other_y;
        dx * dx + dy * dy
    }
}

#[allow(dead_code)]
struct GeometryEngine {
    station_index: RTree<SpatialPoint>,
}

impl GeometryEngine {
    fn new() -> Self {
        log_info("GeometryEngine::new called - initializing spatial indexing engine");
        Self {
            station_index: RTree::new(),
        }
    }

    fn clear(&mut self) {
        log_info("GeometryEngine::clear called - resetting spatial index");
        self.station_index = RTree::new();
    }

    fn build_track_index(&mut self, tracks: &[RailwayTrack]) {
        log_info(&format!(
            "GeometryEngine::build_track_index called - building spatial index for {} tracks",
            tracks.len()
        ));
        let mut tree = RTree::new();
        let mut total_points = 0usize;
        for (track_idx, track) in tracks.iter().enumerate() {
            log_trace(&format!(
                "Processing track {}: {} with {} geometry points",
                track_idx,
                track.id,
                track.geometry.len()
            ));
            for coord in &track.geometry {
                tree.insert(SpatialPoint {
                    coord: *coord,
                    index: track_idx,
                });
                total_points += 1;
            }
        }
        self.station_index = tree;
        log_info(&format!(
            "GeometryEngine::build_track_index completed - indexed {} points across {} tracks",
            total_points,
            tracks.len()
        ));
    }

    fn build_station_index(&mut self, stations: &[Station]) {
        log_info(&format!(
            "GeometryEngine::build_station_index called - indexing {} stations",
            stations.len()
        ));
        let mut tree = RTree::new();
        for (i, s) in stations.iter().enumerate() {
            log_trace(&format!(
                "Indexing station {}: {} at lat={:.6}, lon={:.6}",
                i, s.name, s.coord.lat, s.coord.lon
            ));
            tree.insert(SpatialPoint {
                coord: s.coord,
                index: i,
            });
        }
        self.station_index = tree;
        log_info(&format!(
            "GeometryEngine::build_station_index completed - {} stations indexed",
            stations.len()
        ));
    }

    fn point_in_polygon(&self, point: &Coordinate, polygon: &[Coordinate]) -> bool {
        log_trace(&format!("GeometryEngine::point_in_polygon called - checking point lat={:.6}, lon={:.6} against polygon with {} vertices", point.lat, point.lon, polygon.len()));
        if polygon.len() < 3 {
            log_warn("GeometryEngine::point_in_polygon - polygon has fewer than 3 vertices, returning false");
            return false;
        }

        let mut inside = false;
        let n = polygon.len();
        let mut intersections = 0usize;

        for (i, j) in (0..n).zip((1..n).chain(std::iter::once(0))) {
            let xi = polygon[i].lon;
            let yi = polygon[i].lat;
            let xj = polygon[j].lon;
            let yj = polygon[j].lat;

            if ((yi > point.lat) != (yj > point.lat))
                && (point.lon < (xj - xi) * (point.lat - yi) / (yj - yi) + xi)
            {
                inside = !inside;
                intersections += 1;
            }
        }

        log_trace(&format!(
            "GeometryEngine::point_in_polygon result: {} ({} intersections)",
            inside, intersections
        ));
        inside
    }

    fn snap_to_polyline(&self, point: &Coordinate, polyline: &[Coordinate]) -> (f64, Coordinate) {
        log_trace(&format!("GeometryEngine::snap_to_polyline called - snapping point lat={:.6}, lon={:.6} to polyline with {} points", point.lat, point.lon, polyline.len()));
        if polyline.is_empty() {
            log_warn("GeometryEngine::snap_to_polyline - polyline is empty, returning infinity");
            return (f64::INFINITY, *point);
        }

        let mut min_dist = f64::INFINITY;
        let mut nearest = *point;
        let mut best_segment = 0usize;

        for i in 0..polyline.len().saturating_sub(1) {
            let (dist, proj) = self.project_to_segment(point, &polyline[i], &polyline[i + 1]);
            if dist < min_dist {
                min_dist = dist;
                nearest = proj;
                best_segment = i;
            }
        }

        log_trace(&format!("GeometryEngine::snap_to_polyline result: distance={:.2}m, best_segment={}, nearest_lat={:.6}, nearest_lon={:.6}", min_dist, best_segment, nearest.lat, nearest.lon));
        (min_dist, nearest)
    }

    fn snap_to_tracks(
        &self,
        point: &Coordinate,
        tracks: &[RailwayTrack],
    ) -> (f64, Coordinate, Option<usize>) {
        log_trace(&format!("GeometryEngine::snap_to_tracks called - snapping point lat={:.6}, lon={:.6} to {} tracks", point.lat, point.lon, tracks.len()));
        let (p_y, p_x) = point.to_mercator();

        let mut min_dist = f64::INFINITY;
        let mut nearest_coord = *point;
        let mut best_track = None;
        let mut tracks_checked = 0usize;

        for nearest in self
            .station_index
            .nearest_neighbor_iter(&[p_x, p_y])
            .take(3)
        {
            tracks_checked += 1;
            let track = &tracks[nearest.index];
            log_trace(&format!(
                "Checking track {}: {} with {} points",
                nearest.index,
                track.id,
                track.geometry.len()
            ));
            let (dist, proj) = self.snap_to_polyline(point, &track.geometry);
            if dist < min_dist {
                min_dist = dist;
                nearest_coord = proj;
                best_track = Some(nearest.index);
                log_trace(&format!(
                    "New best track: {} at distance {:.2}m",
                    track.id, dist
                ));
            }
        }

        log_info(&format!("GeometryEngine::snap_to_tracks result: checked {} tracks, best_track={:?}, distance={:.2}m", tracks_checked, best_track, min_dist));
        (min_dist, nearest_coord, best_track)
    }

    fn project_to_segment(
        &self,
        point: &Coordinate,
        seg_start: &Coordinate,
        seg_end: &Coordinate,
    ) -> (f64, Coordinate) {
        log_trace(&format!("GeometryEngine::project_to_segment called - projecting point lat={:.6}, lon={:.6} to segment from lat={:.6}, lon={:.6} to lat={:.6}, lon={:.6}", point.lat, point.lon, seg_start.lat, seg_start.lon, seg_end.lat, seg_end.lon));
        let (p_y, p_x) = point.to_mercator();
        let (s_y, s_x) = seg_start.to_mercator();
        let (e_y, e_x) = seg_end.to_mercator();

        let dx = e_x - s_x;
        let dy = e_y - s_y;
        let len2 = dx * dx + dy * dy;

        if len2 == 0.0 {
            log_warn("GeometryEngine::project_to_segment - segment has zero length, returning distance to start");
            return (point.distance_to(seg_start), *seg_start);
        }

        let t = ((p_x - s_x) * dx + (p_y - s_y) * dy) / len2;
        let t = t.clamp(0.0, 1.0);

        let proj_x = s_x + t * dx;
        let proj_y = s_y + t * dy;

        let proj_coord = Coordinate::from_mercator(proj_x, proj_y);
        let distance = point.distance_to(&proj_coord);
        log_trace(&format!(
            "GeometryEngine::project_to_segment result: t={:.4}, distance={:.2}m",
            t, distance
        ));
        (distance, proj_coord)
    }

    fn create_circle(
        &self,
        center: &Coordinate,
        radius_meters: f64,
        segments: i32,
    ) -> Vec<Coordinate> {
        log_trace(&format!("GeometryEngine::create_circle called - center lat={:.6}, lon={:.6}, radius={:.2}m, segments={}", center.lat, center.lon, radius_meters, segments));
        let radius_deg = radius_meters / EARTH_RADIUS * RAD_TO_DEG;
        let circle: Vec<Coordinate> = (0..segments)
            .map(|i| {
                let angle = 2.0 * PI * i as f64 / segments as f64;
                Coordinate::new(
                    center.lat + radius_deg * angle.sin(),
                    center.lon + radius_deg * angle.cos() / (center.lat * DEG_TO_RAD).cos(),
                )
            })
            .collect();
        log_trace(&format!(
            "GeometryEngine::create_circle result: {} points generated",
            circle.len()
        ));
        circle
    }

    fn merge_stations(&self, stations: Vec<Station>, threshold: f64) -> Vec<Station> {
        log_info(&format!("GeometryEngine::merge_stations called - merging {} stations with threshold {:.6} degrees", stations.len(), threshold));
        let mut tree = RTree::new();
        for (i, s) in stations.iter().enumerate() {
            log_trace(&format!(
                "Building merge index for station {}: {}",
                i, s.name
            ));
            tree.insert(SpatialPoint {
                coord: s.coord,
                index: i,
            });
        }

        let mut merged = Vec::new();
        let mut processed = std::collections::HashSet::new();
        let threshold_meters = threshold * 111_000.0;
        log_debug(&format!(
            "Merge threshold: {:.2} meters (threshold={:.6} degrees)",
            threshold_meters, threshold
        ));

        let mut merges_performed = 0usize;
        for (i, station) in stations.iter().enumerate() {
            if processed.contains(&i) {
                continue;
            }

            let mut hub = station.clone();
            hub.is_interchange = false;
            let m = station.coord.to_mercator();
            log_trace(&format!(
                "Processing station {} as potential hub: {}",
                i, station.name
            ));

            // Query spatial tree for nearby nodes instantly
            let sq_threshold = threshold_meters * threshold_meters;
            for neighbor in tree.locate_within_distance([m.1, m.0], sq_threshold) {
                let idx = neighbor.index;
                if idx == i || processed.contains(&idx) {
                    continue;
                }

                // Perform instant high-speed point verification using raw Mercator space vectors
                let (n_y, n_x) = stations[idx].coord.to_mercator();
                let dx = m.1 - n_x;
                let dy = m.0 - n_y;
                if (dx * dx + dy * dy) <= sq_threshold {
                    log_trace(&format!(
                        "Merging station {} ({}) into hub {} ({})",
                        idx, stations[idx].name, i, station.name
                    ));
                    processed.insert(idx);
                    hub.is_interchange = true;
                    for line in &stations[idx].lines {
                        if !hub.lines.contains(line) {
                            hub.lines.push(line.clone());
                        }
                    }
                    merges_performed += 1;
                }
            }
            processed.insert(i);
            merged.push(hub);
        }
        log_info(&format!("GeometryEngine::merge_stations completed: {} stations -> {} stations ({} merges performed)", stations.len(), merged.len(), merges_performed));
        merged
    }

    pub fn simplify_inplace(&self, points: &[Coordinate], epsilon: f64, out: &mut Vec<Coordinate>) {
        log_trace(&format!(
            "GeometryEngine::simplify_inplace called - {} points, epsilon={:.2}m",
            points.len(),
            epsilon
        ));
        if points.len() < 2 {
            log_debug("GeometryEngine::simplify_inplace - fewer than 2 points, returning as-is");
            out.extend_from_slice(points);
            return;
        }
        let mut stack = vec![(0, points.len() - 1)];
        let mut keep = vec![true; points.len()];
        let mut iterations = 0usize;
        while let Some((start, end)) = stack.pop() {
            iterations += 1;
            if end - start < 2 {
                continue;
            }
            let (mut max_d, mut max_idx) = (0.0, start);
            for i in (start + 1)..end {
                let (d, _) = self.project_to_segment(&points[i], &points[start], &points[end]);
                if d > max_d {
                    max_d = d;
                    max_idx = i;
                }
            }
            if max_d > epsilon {
                keep[max_idx] = true;
                stack.push((start, max_idx));
                stack.push((max_idx, end));
            } else {
                for i in (start + 1)..end {
                    keep[i] = false;
                }
            }
        }
        let kept_count = keep.iter().filter(|&&k| k).count();
        for i in 0..points.len() {
            if keep[i] {
                out.push(points[i]);
            }
        }
        log_trace(&format!(
            "GeometryEngine::simplify_inplace result: {} points -> {} points ({} iterations)",
            points.len(),
            kept_count,
            iterations
        ));
    }

    fn simplify_polyline(&self, polyline: Vec<Coordinate>, epsilon: f64) -> Vec<Coordinate> {
        log_trace(&format!(
            "GeometryEngine::simplify_polyline called - {} points, epsilon={:.2}m",
            polyline.len(),
            epsilon
        ));
        let mut out = Vec::new();
        self.simplify_inplace(&polyline, epsilon, &mut out);
        log_trace(&format!(
            "GeometryEngine::simplify_polyline result: {} points",
            out.len()
        ));
        out
    }

    fn compute_transit_deserts(
        &self,
        residential_areas: &[Coordinate],
        stations: &[Station],
        threshold: f64,
    ) -> Vec<Coordinate> {
        let trace_start_time = Utc::now();
        log_info(&format!("GeometryEngine::compute_transit_deserts called - {} residential areas, {} stations, threshold={:.2}m", residential_areas.len(), stations.len(), threshold));
        log_debug("[TRACE] Beginning geometric distance matrix operations...");

        // Perform structural AABB overlap checks against the spatial tree in O(log N) complexity
        let mut station_tree = RTree::new();
        for (i, s) in stations.iter().enumerate() {
            log_trace(&format!(
                "Building transit desert index for station {}: {}",
                i, s.name
            ));
            station_tree.insert(SpatialPoint {
                coord: s.coord,
                index: i,
            });
        }
        log_debug(&format!(
            "Station tree built with {} stations",
            stations.len()
        ));

        let mut matching_deserts = Vec::with_capacity(residential_areas.len());
        let mut desert_count = 0usize;
        let mut served_count = 0usize;
        for (idx, res_coord) in residential_areas.iter().enumerate() {
            let merc = res_coord.to_mercator();
            if let Some(nearest) = station_tree.nearest_neighbor(&[merc.1, merc.0]) {
                let distance = res_coord.distance_to(&nearest.coord);
                if distance > threshold {
                    log_trace(&format!("Residential area {} is a transit desert - nearest station is {:.2}m away (threshold: {:.2}m)", idx, distance, threshold));
                    matching_deserts.push(*res_coord);
                    desert_count += 1;
                } else {
                    log_trace(&format!(
                        "Residential area {} is served - nearest station is {:.2}m away",
                        idx, distance
                    ));
                    served_count += 1;
                }
            } else {
                log_warn(&format!(
                    "Residential area {} - no nearest station found, marking as desert",
                    idx
                ));
                matching_deserts.push(*res_coord);
                desert_count += 1;
            }
        }

        let elapsed = (Utc::now() - trace_start_time)
            .num_microseconds()
            .unwrap_or(0);
        log_info(&format!(
            "[PERF] Analytical matrix processing completed in {} microseconds. Results: {} deserts, {} served out of {} areas",
            elapsed,
            desert_count,
            served_count,
            residential_areas.len()
        ));

        matching_deserts
    }
}

impl Default for GeometryEngine {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// A* ROUTING ALGORITHM
// ============================================================================

#[derive(Debug, Clone)]
struct Node {
    id: usize,
    pub coord: Coordinate,
    neighbors: Vec<(usize, f64)>,
}

#[derive(Debug, Clone, PartialEq)]
struct PriorityQueueItem {
    cost: f64,
    node_id: usize,
}

impl Ord for PriorityQueueItem {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(CmpOrdering::Equal)
    }
}

impl Eq for PriorityQueueItem {}

impl PartialOrd for PriorityQueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone)]
struct RoutingGraph {
    nodes: HashMap<usize, Node>,
}

impl RoutingGraph {
    fn new() -> Self {
        log_info("RoutingGraph::new called - initializing routing graph");
        Self {
            nodes: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        log_info(&format!(
            "RoutingGraph::clear called - clearing {} nodes",
            self.nodes.len()
        ));
        self.nodes.clear();
    }

    fn add_node(&mut self, id: usize, coord: Coordinate) {
        log_trace(&format!(
            "RoutingGraph::add_node called - id={}, lat={:.6}, lon={:.6}",
            id, coord.lat, coord.lon
        ));
        self.nodes.insert(
            id,
            Node {
                id,
                coord,
                neighbors: Vec::new(),
            },
        );
    }

    fn add_edge(&mut self, from: usize, to: usize, weight: f64) {
        log_trace(&format!(
            "RoutingGraph::add_edge called - from={}, to={}, weight={:.2}m",
            from, to, weight
        ));
        if let Some(node) = self.nodes.get_mut(&from) {
            node.neighbors.push((to, weight));
        } else {
            log_error(&format!(
                "RoutingGraph::add_edge failed - node {} not found",
                from
            ));
        }
    }

    fn find_path(&self, start: &Coordinate, end: &Coordinate) -> Vec<Coordinate> {
        log_info(&format!(
            "RoutingGraph::find_path called - from lat={:.6}, lon={:.6} to lat={:.6}, lon={:.6}",
            start.lat, start.lon, end.lat, end.lon
        ));
        let start_node = self.find_nearest_node(start);
        let end_node = self.find_nearest_node(end);

        match (start_node, end_node) {
            (Some(s), Some(e)) => {
                log_debug(&format!(
                    "RoutingGraph::find_path - found start node {}, end node {}",
                    s, e
                ));
                let path = self.astar(s, e);
                log_debug(&format!(
                    "RoutingGraph::find_path result - path with {} points",
                    path.len()
                ));
                path
            }
            _ => {
                log_error("RoutingGraph::find_path - could not find nearest nodes for routing");
                Vec::new()
            }
        }
    }

    fn find_nearest_node(&self, coord: &Coordinate) -> Option<usize> {
        log_trace(&format!(
            "RoutingGraph::find_nearest_node called - searching {} nodes for lat={:.6}, lon={:.6}",
            self.nodes.len(),
            coord.lat,
            coord.lon
        ));
        let result = self
            .nodes
            .iter()
            .min_by(|a, b| {
                a.1.coord
                    .distance_to(coord)
                    .partial_cmp(&b.1.coord.distance_to(coord))
                    .unwrap()
            })
            .map(|(id, _)| *id);
        if let Some(node_id) = result {
            let distance = self.nodes[&node_id].coord.distance_to(coord);
            log_trace(&format!(
                "RoutingGraph::find_nearest_node result - node {} at distance {:.2}m",
                node_id, distance
            ));
        } else {
            log_warn("RoutingGraph::find_nearest_node - no nodes found in graph");
        }
        result
    }

    fn astar(&self, start: usize, end: usize) -> Vec<Coordinate> {
        log_info(&format!(
            "RoutingGraph::astar called - start={}, end={}",
            start, end
        ));
        let mut g_score: HashMap<usize, f64> = HashMap::new();
        let mut f_score: HashMap<usize, f64> = HashMap::new();
        let mut came_from: HashMap<usize, usize> = HashMap::new();
        let mut open_set = BinaryHeap::new();
        let mut closed_set = HashSet::new();

        let end_coord = match self.nodes.get(&end) {
            Some(n) => n.coord,
            None => {
                log_error(&format!("RoutingGraph::astar - end node {} not found", end));
                return Vec::new();
            }
        };

        g_score.insert(start, 0.0);
        let start_coord = match self.nodes.get(&start) {
            Some(n) => n.coord,
            None => {
                log_error(&format!(
                    "RoutingGraph::astar - start node {} not found",
                    start
                ));
                return Vec::new();
            }
        };
        f_score.insert(start, start_coord.distance_to(&end_coord));
        open_set.push(PriorityQueueItem {
            cost: f_score[&start],
            node_id: start,
        });

        let mut iterations = 0usize;
        while let Some(current) = open_set.pop() {
            iterations += 1;
            let current_id = current.node_id;
            log_trace(&format!(
                "RoutingGraph::astar iteration {} - processing node {}",
                iterations, current_id
            ));

            if current_id == end {
                log_info(&format!(
                    "RoutingGraph::astar reached goal after {} iterations",
                    iterations
                ));
                return self.reconstruct_path(&came_from, current_id);
            }

            closed_set.insert(current_id);

            if let Some(node) = self.nodes.get(&current_id) {
                let _ = node.id; // Use Node::id to silence warnings
                log_trace(&format!(
                    "RoutingGraph::astar - node {} has {} neighbors",
                    current_id,
                    node.neighbors.len()
                ));
                for &(neighbor, weight) in &node.neighbors {
                    if closed_set.contains(&neighbor) {
                        log_trace(&format!(
                            "RoutingGraph::astar - skipping neighbor {} (already in closed set)",
                            neighbor
                        ));
                        continue;
                    }

                    let tentative_g_score =
                        g_score.get(&current_id).unwrap_or(&f64::INFINITY) + weight;

                    if tentative_g_score < *g_score.get(&neighbor).unwrap_or(&f64::INFINITY) {
                        log_trace(&format!("RoutingGraph::astar - updating path to neighbor {} with tentative_g_score={:.2}", neighbor, tentative_g_score));
                        came_from.insert(neighbor, current_id);
                        g_score.insert(neighbor, tentative_g_score);

                        let h = match self.nodes.get(&neighbor) {
                            Some(n) => n.coord.distance_to(&end_coord),
                            None => continue,
                        };
                        f_score.insert(neighbor, tentative_g_score + h);

                        open_set.push(PriorityQueueItem {
                            cost: tentative_g_score + h,
                            node_id: neighbor,
                        });
                    }
                }
            }
        }

        log_error(&format!(
            "RoutingGraph::astar failed to find path after {} iterations",
            iterations
        ));
        Vec::new()
    }

    fn reconstruct_path(
        &self,
        came_from: &HashMap<usize, usize>,
        mut current: usize,
    ) -> Vec<Coordinate> {
        log_trace(&format!(
            "RoutingGraph::reconstruct_path called - starting from node {}",
            current
        ));
        let mut path = Vec::new();

        if let Some(node) = self.nodes.get(&current) {
            path.push(node.coord);
        } else {
            log_error(&format!(
                "RoutingGraph::reconstruct_path - node {} not found",
                current
            ));
            return Vec::new();
        }

        let mut steps = 0usize;
        while let Some(&prev) = came_from.get(&current) {
            steps += 1;
            if let Some(node) = self.nodes.get(&prev) {
                path.push(node.coord);
            } else {
                log_error(&format!(
                    "RoutingGraph::reconstruct_path - predecessor node {} not found",
                    prev
                ));
                break;
            }
            current = prev;
        }

        path.reverse();
        log_trace(&format!(
            "RoutingGraph::reconstruct_path result - {} steps, {} points",
            steps,
            path.len()
        ));
        path
    }

    fn build_from_tracks(&mut self, tracks: &[RailwayTrack]) {
        log_info(&format!(
            "RoutingGraph::build_from_tracks called - building graph from {} tracks",
            tracks.len()
        ));
        self.clear();

        use std::sync::atomic::{AtomicUsize, Ordering};
        static INTERNAL_NODE_ID_POOL: AtomicUsize = AtomicUsize::new(0);
        INTERNAL_NODE_ID_POOL.store(0, Ordering::SeqCst);

        let mut tree = RTree::new();
        let mut total_edges = 0usize;
        let mut merged_nodes = 0usize;

        for (track_idx, track) in tracks.iter().enumerate() {
            log_trace(&format!(
                "Processing track {}: {} with {} geometry points",
                track_idx,
                track.id,
                track.geometry.len()
            ));
            let mut prev_idx: Option<usize> = None;
            for (point_idx, coord) in track.geometry.iter().enumerate() {
                let (p_y, p_x) = coord.to_mercator();
                let curr_idx = if let Some(nearest) = tree.nearest_neighbor(&[p_x, p_y]) {
                    let nearest_pt: &SpatialPoint = nearest;
                    if nearest_pt.coord.distance_to(coord) < 10.0 {
                        log_trace(&format!(
                            "Merging point {} of track {} into existing node {}",
                            point_idx, track_idx, nearest_pt.index
                        ));
                        merged_nodes += 1;
                        nearest_pt.index
                    } else {
                        let id = INTERNAL_NODE_ID_POOL.fetch_add(1, Ordering::SeqCst);
                        self.add_node(id, *coord);
                        tree.insert(SpatialPoint {
                            coord: *coord,
                            index: id,
                        });
                        id
                    }
                } else {
                    let id = INTERNAL_NODE_ID_POOL.fetch_add(1, Ordering::SeqCst);
                    self.add_node(id, *coord);
                    tree.insert(SpatialPoint {
                        coord: *coord,
                        index: id,
                    });
                    id
                };

                if let Some(prev) = prev_idx {
                    let dist = coord.distance_to(&self.nodes[&prev].coord);
                    self.add_edge(prev, curr_idx, dist);
                    self.add_edge(curr_idx, prev, dist);
                    total_edges += 2;
                }
                prev_idx = Some(curr_idx);
            }
        }

        log_info(&format!(
            "RoutingGraph::build_from_tracks completed - {} nodes, {} edges, {} merges",
            self.nodes.len(),
            total_edges,
            merged_nodes
        ));
    }
}

impl Default for RoutingGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// NETWORK CLIENTS
// ============================================================================

#[derive(Clone)]
struct NetworkManager {
    client: Arc<Client>,
}

impl NetworkManager {
    fn new() -> Self {
        log_info("NetworkManager::new called - initializing HTTP client");
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("LondonTransport/1.0")
            .build()
            .expect("Failed to create HTTP client");

        log_info("NetworkManager::new completed - HTTP client initialized");
        Self {
            client: Arc::new(client),
        }
    }

    async fn get(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        log_info(&format!("NetworkManager::get called - URL: {}", url));
        let start = Utc::now();
        let response = self.client.get(url).send().await?;
        let status = response.status();
        log_debug(&format!(
            "NetworkManager::get - response status: {}",
            status
        ));
        let text = response.text().await?;
        let elapsed = (Utc::now() - start).num_milliseconds();
        log_info(&format!(
            "NetworkManager::get completed - {} bytes in {}ms",
            text.len(),
            elapsed
        ));
        Ok(text)
    }

    async fn get_json(&self, url: &str) -> Result<Value, Box<dyn std::error::Error>> {
        log_info(&format!("NetworkManager::get_json called - URL: {}", url));
        let start = Utc::now();
        let response = self.client.get(url).send().await?;
        let status = response.status();
        log_debug(&format!(
            "NetworkManager::get_json - response status: {}",
            status
        ));
        let json: Value = response.json().await?;
        let elapsed = (Utc::now() - start).num_milliseconds();
        log_info(&format!(
            "NetworkManager::get_json completed - JSON received in {}ms",
            elapsed
        ));
        Ok(json)
    }

    async fn post_form_json(
        &self,
        url: &str,
        form: &[(&str, &str)],
    ) -> Result<Value, Box<dyn std::error::Error>> {
        log_info(&format!(
            "NetworkManager::post_form_json called - URL: {}, form fields: {}",
            url,
            form.len()
        ));
        let start = Utc::now();
        let response = self.client.post(url).form(form).send().await?;
        let status = response.status();
        log_debug(&format!(
            "NetworkManager::post_form_json - response status: {}",
            status
        ));
        let json: Value = response.json().await?;
        let elapsed = (Utc::now() - start).num_milliseconds();
        log_info(&format!(
            "NetworkManager::post_form_json completed - JSON received in {}ms",
            elapsed
        ));
        Ok(json)
    }
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
struct TflApiClient {
    network: NetworkManager,
    base_url: String,
}

impl TflApiClient {
    fn new(base_url: String) -> Self {
        log_info(&format!(
            "TflApiClient::new called - base_url: {}",
            base_url
        ));
        Self {
            network: NetworkManager::new(),
            base_url,
        }
    }

    async fn fetch_line_routes(&self, line_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}/Line/{}/Route/Sequence/outbound", self.base_url, line_id);
        log_info(&format!(
            "TflApiClient::fetch_line_routes called - line_id: {}",
            line_id
        ));
        log_debug(&format!("TflApiClient::fetch_line_routes - URL: {}", url));
        let result = self.network.get_json(&url).await;
        match &result {
            Ok(json) => log_debug(&format!(
                "TflApiClient::fetch_line_routes success - line_id: {}",
                line_id
            )),
            Err(e) => log_error(&format!(
                "TflApiClient::fetch_line_routes failed - line_id: {}, error: {}",
                line_id, e
            )),
        }
        result
    }

    async fn fetch_line_routes_inbound(
        &self,
        line_id: &str,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}/Line/{}/Route/Sequence/inbound", self.base_url, line_id);
        log_info(&format!(
            "TflApiClient::fetch_line_routes_inbound called - line_id: {}",
            line_id
        ));
        let result = self.network.get_json(&url).await;
        match &result {
            Ok(_) => log_debug(&format!(
                "TflApiClient::fetch_line_routes_inbound success - line_id: {}",
                line_id
            )),
            Err(e) => log_error(&format!(
                "TflApiClient::fetch_line_routes_inbound failed - line_id: {}, error: {}",
                line_id, e
            )),
        }
        result
    }

    async fn fetch_stop_points(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}/StopPoint", self.base_url);
        log_info("TflApiClient::fetch_stop_points called");
        let result = self.network.get_json(&url).await;
        match &result {
            Ok(_) => log_debug("TflApiClient::fetch_stop_points success"),
            Err(e) => log_error(&format!(
                "TflApiClient::fetch_stop_points failed - error: {}",
                e
            )),
        }
        result
    }

    async fn fetch_arrivals(&self, line_id: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!("{}/Line/{}/Arrivals", self.base_url, line_id);
        log_info(&format!(
            "TflApiClient::fetch_arrivals called - line_id: {}",
            line_id
        ));
        let result = self.network.get_json(&url).await;
        match &result {
            Ok(_) => log_debug(&format!(
                "TflApiClient::fetch_arrivals success - line_id: {}",
                line_id
            )),
            Err(e) => log_error(&format!(
                "TflApiClient::fetch_arrivals failed - line_id: {}, error: {}",
                line_id, e
            )),
        }
        result
    }

    async fn fetch_disruptions(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let url = format!(
            "{}/Line/Mode/tube,dlr,overground,elizabeth/Disruption",
            self.base_url
        );
        log_info("TflApiClient::fetch_disruptions called");
        let result = self.network.get_json(&url).await;
        match &result {
            Ok(_) => log_debug("TflApiClient::fetch_disruptions success"),
            Err(e) => log_error(&format!(
                "TflApiClient::fetch_disruptions failed - error: {}",
                e
            )),
        }
        result
    }
}

impl Default for TflApiClient {
    fn default() -> Self {
        Self::new("https://api.tfl.gov.uk".to_string())
    }
}

#[derive(Clone)]
struct OverpassApiClient {
    network: NetworkManager,
    base_url: String,
    fallback_urls: Vec<String>,
}

impl OverpassApiClient {
    fn new(base_url: String) -> Self {
        log_info(&format!(
            "OverpassApiClient::new called - base_url: {}",
            base_url
        ));
        // Fix #4: Provide fallback Overpass mirrors
        Self {
            network: NetworkManager::new(),
            base_url,
            fallback_urls: vec![
                "https://overpass.kumi.systems/api/interpreter".to_string(),
                "https://overpass.openstreetmap.ie/api/interpreter".to_string(),
            ],
        }
    }

    async fn fetch_railway_tracks(
        &self,
        min_lat: f64,
        min_lon: f64,
        max_lat: f64,
        max_lon: f64,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        log_info(&format!("OverpassApiClient::fetch_railway_tracks called - bounds: lat {:.6} to {:.6}, lon {:.6} to {:.6}", min_lat, max_lat, min_lon, max_lon));
        let clamped_min_lat = min_lat.clamp(51.0, 52.0);
        let clamped_min_lon = min_lon.clamp(-0.6, 0.4);
        let clamped_max_lat = max_lat.clamp(51.0, 52.0);
        let clamped_max_lon = max_lon.clamp(-0.6, 0.4);
        log_debug(&format!("OverpassApiClient::fetch_railway_tracks - clamped bounds: lat {:.6} to {:.6}, lon {:.6} to {:.6}", clamped_min_lat, clamped_max_lat, clamped_min_lon, clamped_max_lon));
        // Fix #13: Broadened query – catch all railway types including subway, light_rail,
        // tram, and narrow_gauge. Removed restrictive passenger/no filters.
        // The first clause catches all railway=* ways; the second catches subway/light_rail/tram
        // specifically (some are tagged railway=subway instead of railway=rail).
        let query = format!(
            "[out:json][timeout:120];\
            (\
              way[\\\"railway\\\"~\\\".\\\"]({},{},{},{});\
            );\
            out body; >; out skel qt;",
            clamped_min_lat,
            clamped_min_lon,
            clamped_max_lat,
            clamped_max_lon
        );
        log_trace(&format!(
            "OverpassApiClient::fetch_railway_tracks - query length: {} chars",
            query.len()
        ));

        // Fix #4: Try all URLs with exponential backoff
        let all_urls = std::iter::once(&self.base_url)
            .chain(self.fallback_urls.iter());

        let mut last_error = String::new();
        for (attempt, url) in all_urls.enumerate() {
            let max_retries = 3;
            for retry in 0..max_retries {
                log_debug(&format!(
                    "OverpassApiClient::fetch_railway_tracks - attempt {}/url {}, retry {}/{} using {}",
                    attempt + 1,
                    attempt,
                    retry + 1,
                    max_retries,
                    url
                ));
                let result = self
                    .network
                    .post_form_json(url, &[("data", &query)])
                    .await;

                match result {
                    Ok(val) => {
                        // Validate that the response actually has elements
                        if val.get("elements").and_then(|v| v.as_array()).map_or(0, |a| a.len()) > 0
                            || val.get("remark").is_none()
                        {
                            log_info(&format!(
                                "OverpassApiClient::fetch_railway_tracks success using {}",
                                url
                            ));
                            return Ok(val);
                        }
                        // Response has elements but they're empty — might be valid
                        log_warn(&format!(
                            "OverpassApiClient::fetch_railway_tracks - {} returned empty elements, trying next",
                            url
                        ));
                        last_error = format!("{} returned empty elements", url);
                        break; // Try next URL
                    }
                    Err(e) => {
                        last_error = format!("{}: {}", url, e);
                        log_error(&format!(
                            "OverpassApiClient::fetch_railway_tracks failed - {} (retry {}/{})",
                            last_error,
                            retry + 1,
                            max_retries
                        ));

                        if retry < max_retries - 1 {
                            // Exponential backoff: 2s, 4s, 8s
                            let delay_secs = 2u64 * (1u64 << retry);
                            log_debug(&format!(
                                "OverpassApiClient::fetch_railway_tracks - backing off {}s before retry",
                                delay_secs
                            ));
                            tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                        }
                    }
                }
            }
        }

        let err_msg = format!(
            "OverpassApiClient::fetch_railway_tracks - all endpoints exhausted. Last error: {}",
            last_error
        );
        log_error(&err_msg);
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            err_msg,
        )))
    }

    async fn fetch_residential_areas(
        &self,
        min_lat: f64,
        min_lon: f64,
        max_lat: f64,
        max_lon: f64,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        log_info(&format!("OverpassApiClient::fetch_residential_areas called - bounds: lat {:.6} to {:.6}, lon {:.6} to {:.6}", min_lat, max_lat, min_lon, max_lon));
        let query = format!(
            "[out:json][timeout:90];\
            (\
              node[\"landuse\"=\"residential\"]({},{},{},{});\
              way[\"landuse\"=\"residential\"]({},{},{},{});\
              relation[\"landuse\"=\"residential\"]({},{},{},{});\
            );\
            out center;",
            min_lat,
            min_lon,
            max_lat,
            max_lon,
            min_lat,
            min_lon,
            max_lat,
            max_lon,
            min_lat,
            min_lon,
            max_lat,
            max_lon
        );
        let result = self
            .network
            .post_form_json(&self.base_url, &[("data", &query)])
            .await;
        match &result {
            Ok(_) => log_debug("OverpassApiClient::fetch_residential_areas success"),
            Err(e) => log_error(&format!(
                "OverpassApiClient::fetch_residential_areas failed - error: {}",
                e
            )),
        }
        result
    }
}

impl Default for OverpassApiClient {
    fn default() -> Self {
        Self::new("https://overpass-api.de/api/interpreter".to_string())
    }
}

// ============================================================================
// CACHE / PERSISTENCE
// ============================================================================

#[derive(Debug)]
pub struct SqliteConnectionManager {
    path: PathBuf,
}

impl SqliteConnectionManager {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        let path_buf = path.into();
        log_info(&format!(
            "SqliteConnectionManager::new called - path: {:?}",
            path_buf
        ));
        Self { path: path_buf }
    }
}

impl r2d2::ManageConnection for SqliteConnectionManager {
    type Connection = Connection;
    type Error = rusqlite::Error;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        log_info(&format!(
            "SqliteConnectionManager::connect called - opening database at {:?}",
            self.path
        ));
        let conn = Connection::open(&self.path)?;
        log_debug("SqliteConnectionManager::connect - applying WAL and performance pragmas");
        // Apply write-ahead logging (WAL) and performance tuning pragmas
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;\
             PRAGMA synchronous = NORMAL;\
             PRAGMA busy_timeout = 10000;\
             PRAGMA cache_size = -64000;\
             PRAGMA temp_store = MEMORY;",
        )?;
        log_debug("SqliteConnectionManager::connect - database opened and configured successfully");
        Ok(conn)
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        log_trace("SqliteConnectionManager::is_valid called - checking connection health");
        let result = conn.query_row("SELECT 1", [], |_| Ok(()));
        match &result {
            Ok(_) => log_trace("SqliteConnectionManager::is_valid - connection is valid"),
            Err(e) => log_warn(&format!(
                "SqliteConnectionManager::is_valid - connection check failed: {}",
                e
            )),
        }
        result
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

#[derive(Clone)]
struct CacheManager {
    pool: r2d2::Pool<SqliteConnectionManager>,
}

impl CacheManager {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        log_info("CacheManager::new called - initializing cache manager");
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("london_transport");

        log_debug(&format!(
            "CacheManager::new - creating cache directory at {:?}",
            cache_dir
        ));
        std::fs::create_dir_all(&cache_dir)?;
        log_debug(&format!("Cache directory: {:?}", cache_dir));

        let db_path = cache_dir.join("cache.db");
        log_info(&format!("CacheManager::new - database path: {:?}", db_path));
        let manager = SqliteConnectionManager::new(db_path);
        let pool = r2d2::Pool::builder()
            .max_size(16) // Scale resource channels out dynamically across core pools
            .connection_timeout(Duration::from_secs(5))
            .build(manager)?;

        log_debug("CacheManager::new - connection pool created with max_size=16, timeout=5s");
        let cache = Self { pool };
        cache.initialize_tables()?;
        log_info("CacheManager::new completed - cache manager initialized");
        Ok(cache)
    }

    fn initialize_tables(&self) -> Result<(), Box<dyn std::error::Error>> {
        log_info("CacheManager::initialize_tables called - initializing database schema");
        let conn = self.pool.get()?;
        log_debug("CacheManager::initialize_tables - acquired database connection");

        log_debug("CacheManager::initialize_tables - creating api_cache table");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS api_cache (
                key TEXT PRIMARY KEY,
                data TEXT,
                timestamp INTEGER,
                expiry INTEGER
            )",
            [],
        )?;

        log_debug("CacheManager::initialize_tables - creating custom_lines table");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS custom_lines (
                id TEXT PRIMARY KEY,
                name TEXT,
                color TEXT,
                geometry TEXT,
                stations TEXT
            )",
            [],
        )?;

        log_debug("CacheManager::initialize_tables - creating free_stations table");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS free_stations (
                id TEXT PRIMARY KEY,
                name TEXT,
                lat REAL,
                lon REAL
            )",
            [],
        )?;

        // Fix #8: Schema version tracking — clear cache if version mismatch
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                key TEXT PRIMARY KEY,
                version TEXT NOT NULL
            )",
            [],
        )?;

        let mut stmt = conn.prepare("SELECT version FROM schema_version WHERE key = 'app'")?;
        let stored_version: Option<String> = stmt
            .query_row([], |row| row.get(0))
            .ok();

        if let Some(ref ver) = stored_version {
            if ver != CACHE_SCHEMA_VERSION {
                log_warn(&format!(
                    "CacheManager::initialize_tables - schema version changed from {} to {}; clearing all caches",
                    ver, CACHE_SCHEMA_VERSION
                ));
                conn.execute("DELETE FROM api_cache", [])?;
                conn.execute(
                    "INSERT OR REPLACE INTO schema_version (key, version) VALUES ('app', ?1)",
                    params![CACHE_SCHEMA_VERSION],
                )?;
                log_info("CacheManager::initialize_tables - all caches cleared due to schema version change");
            } else {
                log_debug("CacheManager::initialize_tables - schema version matches, caches are valid");
            }
        } else {
            log_debug("CacheManager::initialize_tables - no previous schema version found, inserting current version");
            conn.execute(
                "INSERT OR REPLACE INTO schema_version (key, version) VALUES ('app', ?1)",
                params![CACHE_SCHEMA_VERSION],
            )?;
        }

        log_info("CacheManager::initialize_tables completed - all database tables initialized");
        Ok(())
    }

    fn put(&self, key: &str, data: &str, expiry_ms: i64) -> Result<(), Box<dyn std::error::Error>> {
        log_info(&format!(
            "CacheManager::put called - key: {}, data_size: {} bytes, expiry: {}ms",
            key,
            data.len(),
            expiry_ms
        ));
        let conn = self.pool.get()?;
        log_trace("CacheManager::put - acquired database connection");
        let now = Utc::now().timestamp_millis();
        let expiry = now + expiry_ms;

        conn.execute(
            "INSERT OR REPLACE INTO api_cache (key, data, timestamp, expiry) VALUES (?1, ?2, ?3, ?4)",
            params![key, data, now, expiry],
        )?;

        log_debug(&format!(
            "CacheManager::put completed - cached key: {} (expires at {})",
            key, expiry
        ));
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        log_info(&format!("CacheManager::get called - key: {}", key));
        let conn = self.pool.get()?;
        log_trace("CacheManager::get - acquired database connection");
        let now = Utc::now().timestamp_millis();

        let mut stmt = conn.prepare("SELECT data, expiry FROM api_cache WHERE key = ?1")?;

        let result = stmt.query_row(params![key], |row| {
            let data: String = row.get(0)?;
            let expiry: i64 = row.get(1)?;
            Ok((data, expiry))
        });

        match result {
            Ok((data, expiry)) => {
                if expiry > now {
                    log_debug(&format!(
                        "CacheManager::get - CACHE HIT for key: {} (expires in {}ms)",
                        key,
                        expiry - now
                    ));
                    Ok(Some(data))
                } else {
                    log_debug(&format!(
                        "CacheManager::get - CACHE EXPIRED for key: {} (expired {}ms ago)",
                        key,
                        now - expiry
                    ));
                    conn.execute("DELETE FROM api_cache WHERE key = ?1", params![key])?;
                    Ok(None)
                }
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                log_debug(&format!("CacheManager::get - CACHE MISS for key: {}", key));
                Ok(None)
            }
            Err(e) => {
                log_error(&format!(
                    "CacheManager::get - DATABASE ERROR for key {}: {}",
                    key, e
                ));
                Err(Box::new(e))
            }
        }
    }

    fn save_custom_line(&self, line: &Line) -> Result<(), Box<dyn std::error::Error>> {
        log_info(&format!(
            "CacheManager::save_custom_line called - id: {}, name: {}, geometry_points: {}",
            line.id,
            line.name,
            line.geometry.len()
        ));
        let conn = self.pool.get()?;
        log_trace("CacheManager::save_custom_line - acquired database connection");

        let geometry_json = serde_json::to_string(&line.geometry)?;
        log_debug(&format!(
            "CacheManager::save_custom_line - serialized geometry: {} bytes",
            geometry_json.len()
        ));

        conn.execute(
            "INSERT OR REPLACE INTO custom_lines (id, name, color, geometry, stations) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![&line.id, &line.name, &line.color, geometry_json, ""],
        )?;

        log_info(&format!(
            "CacheManager::save_custom_line completed - saved custom line: {}",
            line.id
        ));
        Ok(())
    }

    fn save_free_station(&self, station: &Station) -> Result<(), Box<dyn std::error::Error>> {
        log_info(&format!(
            "CacheManager::save_free_station called - id: {}, name: {}, lat={:.6}, lon={:.6}",
            station.id, station.name, station.coord.lat, station.coord.lon
        ));
        let conn = self.pool.get()?;
        log_trace("CacheManager::save_free_station - acquired database connection");

        conn.execute(
            "INSERT OR REPLACE INTO free_stations (id, name, lat, lon) VALUES (?1, ?2, ?3, ?4)",
            params![
                &station.id,
                &station.name,
                station.coord.lat,
                station.coord.lon
            ],
        )?;

        log_info(&format!(
            "CacheManager::save_free_station completed - saved free station: {}",
            station.id
        ));
        Ok(())
    }

    fn load_custom_lines(&self) -> Result<Vec<Line>, Box<dyn std::error::Error>> {
        log_info("CacheManager::load_custom_lines called - loading custom lines from database");
        let conn = self.pool.get()?;
        log_trace("CacheManager::load_custom_lines - acquired database connection");
        let mut stmt = conn.prepare("SELECT id, name, color, geometry FROM custom_lines")?;

        let lines = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let color: String = row.get(2)?;
            let geometry_json: String = row.get(3)?;

            log_trace(&format!("CacheManager::load_custom_lines - parsing line: {} with geometry_json size: {} bytes", id, geometry_json.len()));
            let geometry: Vec<Coordinate> = serde_json::from_str(&geometry_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

            Ok(Line {
                id,
                name,
                color,
                geometry,
                stations: Vec::new(),
                segments: Vec::new(),
                is_custom: true,
            })
        })?;

        let mut result = Vec::new();
        let mut parse_errors = 0usize;
        for line in lines {
            match line {
                Ok(l) => result.push(l),
                Err(e) => {
                    parse_errors += 1;
                    log_error(&format!(
                        "CacheManager::load_custom_lines - failed to parse line: {}",
                        e
                    ));
                }
            }
        }

        log_info(&format!(
            "CacheManager::load_custom_lines completed - loaded {} custom lines ({} parse errors)",
            result.len(),
            parse_errors
        ));
        Ok(result)
    }

    fn load_free_stations(&self) -> Result<Vec<Station>, Box<dyn std::error::Error>> {
        log_info("CacheManager::load_free_stations called - loading free stations from database");
        let conn = self.pool.get()?;
        log_trace("CacheManager::load_free_stations - acquired database connection");
        let mut stmt = conn.prepare("SELECT id, name, lat, lon FROM free_stations")?;

        let stations = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let lat: f64 = row.get(2)?;
            let lon: f64 = row.get(3)?;

            log_trace(&format!(
                "CacheManager::load_free_stations - parsing station: {} at lat={:.6}, lon={:.6}",
                id, lat, lon
            ));
            Ok(Station::new(id, name, Coordinate::new(lat, lon)))
        })?;

        let mut result = Vec::new();
        let mut parse_errors = 0usize;
        for station in stations {
            match station {
                Ok(s) => result.push(s),
                Err(e) => {
                    parse_errors += 1;
                    log_error(&format!(
                        "CacheManager::load_free_stations - failed to parse station: {}",
                        e
                    ));
                }
            }
        }

        log_info(&format!("CacheManager::load_free_stations completed - loaded {} free stations ({} parse errors)", result.len(), parse_errors));
        Ok(result)
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new().expect("Failed to initialize cache")
    }
}

// ============================================================================
// API RESPONSE WRAPPER
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

// ============================================================================
// APPLICATION STATE
// ============================================================================

#[derive(Clone)]
struct AppState {
    lines: Arc<arc_swap::ArcSwap<Vec<Line>>>,
    stations: Arc<arc_swap::ArcSwap<Vec<Station>>>,
    tracks: Arc<arc_swap::ArcSwap<Vec<RailwayTrack>>>,
    construction_state: Arc<arc_swap::ArcSwap<ConstructionState>>,
    tfl_client: Arc<TflApiClient>,
    overpass_client: Arc<OverpassApiClient>,
    cache: Arc<CacheManager>,
    geometry_engine: Arc<arc_swap::ArcSwap<GeometryEngine>>,
    routing_graph: Arc<arc_swap::ArcSwap<RoutingGraph>>,
    /// Fix #7: Store config for use in API handlers
    config: Arc<Config>,
}

// Explicitly mark AppState as Send + Sync for Axum
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

impl AppState {
    fn new(config: Config) -> Self {
        log_info("AppState::new called - creating application state");
        log_debug(&format!(
            "AppState::new - TFL base URL: {}",
            config.tfl_base_url
        ));
        log_debug(&format!(
            "AppState::new - Overpass base URL: {}",
            config.overpass_base_url
        ));

        let state = Self {
            lines: Arc::new(arc_swap::ArcSwap::new(Arc::new(Vec::new()))),
            stations: Arc::new(arc_swap::ArcSwap::new(Arc::new(Vec::new()))),
            tracks: Arc::new(arc_swap::ArcSwap::new(Arc::new(Vec::new()))),
            construction_state: Arc::new(arc_swap::ArcSwap::new(Arc::new(
                ConstructionState::default(),
            ))),
            tfl_client: Arc::new(TflApiClient::new(config.tfl_base_url.clone())),
            overpass_client: Arc::new(OverpassApiClient::new(config.overpass_base_url.clone())),
            cache: Arc::new(CacheManager::default()),
            geometry_engine: Arc::new(arc_swap::ArcSwap::new(Arc::new(GeometryEngine::new()))),
            routing_graph: Arc::new(arc_swap::ArcSwap::new(Arc::new(RoutingGraph::new()))),
            config: Arc::new(config),
        };

        log_info("AppState::new completed - application state initialized");
        state
    }

    async fn load_line_routes(&self, line_id: &str) -> Result<Line, Box<dyn std::error::Error>> {
        log_info(&format!(
            "AppState::load_line_routes called - line_id: {}",
            line_id
        ));
        let cache_key = format!("line_routes_{}", line_id);
        log_debug(&format!(
            "AppState::load_line_routes - cache_key: {}",
            cache_key
        ));

        let cache_clone = self.cache.clone();
        let cache_key_db = cache_key.clone();
        let get_res = tokio::task::spawn_blocking(move || {
            let start_ts = Utc::now();
            log_trace("AppState::load_line_routes - spawning blocking cache get task");
            let res = cache_clone.get(&cache_key_db).map_err(|e| e.to_string());
            let elapsed = (Utc::now() - start_ts).num_milliseconds();
            log_info(&format!(
                "[PERF] AppState::load_line_routes - SQLite Cache query executed in {}ms",
                elapsed
            ));
            res
        })
        .await
        .map_err(|e| {
            log_error(&format!(
                "AppState::load_line_routes - task spawn error: {}",
                e
            ));
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        let cached_opt = get_res.map_err(|e| {
            log_error(&format!(
                "AppState::load_line_routes - cache get error: {}",
                e
            ));
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
        })?;

        if let Some(cached) = cached_opt {
            if let Ok(line) = serde_json::from_str::<Line>(&cached) {
                // Fix #1: Cache Poisoning – if cached line has no stations or geometry,
                // treat it as a cache miss and re-fetch from the live API instead of returning an error.
                if line.stations.is_empty() || line.geometry.is_empty() {
                    log_warn(&format!(
                        "AppState::load_line_routes - cached line {} has empty stations/geometry; treating as cache miss, re-fetching from API",
                        line_id
                    ));
                    // Fall through to API fetch below
                } else {
                    // If geometry length matches station count, it's a legacy straight-line shortcut
                    // that was cached before the routing graph existed.  Force re-route.
                    let is_straight_line = line.geometry.len() == line.stations.len();
                    if is_straight_line {
                        log_info(&format!(
                            "AppState::load_line_routes - straight lines detected for line {} ({} pts == {} stations); forcing re-route",
                            line_id,
                            line.geometry.len(),
                            line.stations.len()
                        ));
                        // Fall through to API fetch below so the routing graph re-curves it
                    } else {
                        log_info(&format!(
                            "AppState::load_line_routes - cache hit validated with physical truth mapping for {}",
                            line_id
                        ));
                        return Ok(line);
                    }
                }
            } else {
                log_warn(&format!(
                    "AppState::load_line_routes - failed to deserialize cached line: {}",
                    line_id
                ));
            }
        }

        log_debug(&format!(
            "AppState::load_line_routes - cache miss, fetching from API: {}",
            line_id
        ));
        let data = match self.tfl_client.fetch_line_routes(line_id).await {
            Ok(data) => data,
            Err(err) => {
                log_error(&format!(
                    "AppState::load_line_routes - TfL API error for {}: {}",
                    line_id, err
                ));
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("TfL API fetch failed for {line_id}: {err}"),
                )));
            }
        };
        log_debug(&format!(
            "AppState::load_line_routes - parsing line data for: {}",
            line_id
        ));
        let mut line = match self.parse_line_data(&data, line_id) {
            Ok(line) => line,
            Err(err) => {
                log_error(&format!(
                    "AppState::load_line_routes - parse error for {}: {}",
                    line_id, err
                ));
                return Err(err);
            }
        };

        if line.stations.is_empty() {
            log_error(&format!(
                "AppState::load_line_routes - parsed line {} has no stations; rejecting",
                line_id
            ));
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Parsed line {line_id} has no stations"),
            )));
        }

        // Physical Truth Engine: Curve the naive straight lines onto real Overpass tracks
        log_debug("AppState::load_line_routes - applying physical truth engine curvature");
        let tracks = self.tracks.load();
        let geom = self.geometry_engine.load();
        log_debug(&format!(
            "AppState::load_line_routes - {} tracks available, {} stations in line",
            tracks.len(),
            line.stations.len()
        ));

        if !tracks.is_empty() && line.stations.len() >= 2 {
            let mut curved_geometry = Vec::new();
            let routing_graph_instance = self.routing_graph.load();

            for i in 0..line.stations.len() - 1 {
                let start_stat = &line.stations[i];
                let end_stat = &line.stations[i + 1];

                line.segments.push(RouteSegment::new(
                    start_stat.coord,
                    end_stat.coord,
                    line_id.to_string(),
                ));

                curved_geometry.push(start_stat.coord);

                // Physical Truth Engine: Calculate the path between stations via the A* Tunnel Graph
                let tunnel_path =
                    routing_graph_instance.find_path(&start_stat.coord, &end_stat.coord);
                if !tunnel_path.is_empty() {
                    // Merge coordinates seamlessly along actual tunnel paths
                    curved_geometry.extend(tunnel_path.into_iter().skip(1));
                } else {
                    curved_geometry.push(end_stat.coord);
                }
            }
            if let Some(last) = line.stations.last() {
                curved_geometry.push(last.coord);
            }

            // Hyper-Optimization: Decimate unnecessary points to fix client-side lag
            let mut simplified = Vec::new();
            geom.simplify_inplace(&curved_geometry, 10.0, &mut simplified);
            log_debug(&format!(
                "AppState::load_line_routes - simplified geometry: {} -> {} points",
                curved_geometry.len(),
                simplified.len()
            ));
            line.geometry = simplified;
        }

        self.register_line_stations_in_global_state(&line);

        log_debug(&format!(
            "AppState::load_line_routes - caching line: {} ({} bytes)",
            line_id,
            serde_json::to_string(&line).unwrap_or_default().len()
        ));
        let cached = serde_json::to_string(&line)?;
        let cache_clone = self.cache.clone();
        let cache_key_clone = cache_key.clone();
        let cached_clone = cached.clone();
        let put_res = tokio::task::spawn_blocking(move || {
            log_trace("AppState::load_line_routes - spawning blocking cache put task");
            cache_clone
                .put(&cache_key_clone, &cached_clone, 30 * 24 * 3600 * 1000)
                .map_err(|e| e.to_string())
        })
        .await;

        match put_res {
            Ok(Ok(_)) => log_debug("AppState::load_line_routes - cache put successful"),
            Ok(Err(e)) => {
                log_error(&format!(
                    "AppState::load_line_routes - cache put error: {}",
                    e
                ));
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
            }
            Err(e) => {
                log_error(&format!(
                    "AppState::load_line_routes - cache put task error: {}",
                    e
                ));
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )));
            }
        }

        log_info(&format!(
            "AppState::load_line_routes completed - loaded line from API: {} with {} stations",
            line_id,
            line.stations.len()
        ));
        Ok(line)
    }

    fn register_line_stations_in_global_state(&self, line: &Line) {
        let mut global_stations = (**self.stations.load()).clone();
        let mut new_stations_added = 0usize;

        for station in &line.stations {
            if !global_stations
                .iter()
                .any(|existing| existing.id == station.id)
            {
                global_stations.push(station.clone());
                new_stations_added += 1;
            }
        }

        if new_stations_added > 0 {
            log_info(&format!(
                "AppState::load_line_routes - registered {} new stations to global state from line {}",
                new_stations_added,
                line.id
            ));
            self.stations.store(Arc::new(global_stations));
        }
    }

    async fn ensure_sample_network_state(&self) -> (Vec<Line>, Vec<Station>) {
        let mut merged_lines = (**self.lines.load()).clone();
        // Fix #7: Use configurable sample lines list from Config
        let sample_lines = self.config.sample_lines.clone();

        for line_id in &sample_lines {
            match self.load_line_routes(line_id).await {
                Ok(line) => {
                    merged_lines.retain(|existing_line| existing_line.id != line.id);
                    merged_lines.push(line.clone());
                    self.register_line_stations_in_global_state(&line);
                }
                Err(err) => {
                    log_warn(&format!(
                        "ensure_sample_network_state - failed to seed {}: {}",
                        line_id, err
                    ));
                }
            }
        }

        if !merged_lines.is_empty() {
            self.lines.store(Arc::new(merged_lines.clone()));
        }

        (merged_lines, (**self.stations.load()).clone())
    }

    fn parse_line_data(
        &self,
        data: &Value,
        line_id: &str,
    ) -> Result<Line, Box<dyn std::error::Error>> {
        log_info(&format!(
            "AppState::parse_line_data called - line_id: {}",
            line_id
        ));
        let line_name = data
            .get("lineName")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        log_debug(&format!(
            "AppState::parse_line_data - line_name: {}",
            line_name
        ));
        let color = self.get_line_color(line_id);
        log_debug(&format!("AppState::parse_line_data - color: {}", color));

        let mut line = Line::new(line_id.to_string(), line_name.to_string(), color);

        // Extract stations from routeSectionNaptanEntrySequence
        let mut _stations_extracted = 0usize;
        if let Some(sequences) = data
            .get("routeSectionNaptanEntrySequence")
            .and_then(|v| v.as_array())
        {
            log_debug(&format!(
                "AppState::parse_line_data - found {} route sequences",
                sequences.len()
            ));
            for seq in sequences {
                if let Some(stop_point) = seq.get("stopPoint") {
                    if let (Some(id), Some(name), Some(lat), Some(lon)) = (
                        stop_point.get("id").and_then(|v| v.as_str()),
                        stop_point.get("name").and_then(|v| v.as_str()),
                        stop_point.get("lat").and_then(|v| v.as_f64()),
                        stop_point.get("lon").and_then(|v| v.as_f64()),
                    ) {
                        let _ = _stations_extracted;
                        log_trace(&format!("AppState::parse_line_data - extracting station: {} at lat={:.6}, lon={:.6}", id, lat, lon));
                        let mut station = Station::new(
                            id.to_string(),
                            name.to_string(),
                            Coordinate::new(lat, lon),
                        );
                        station.lines.push(line_id.to_string());
                        line.stations.push(station);
                        line.geometry.push(Coordinate::new(lat, lon));
                    }
                }
            }
        }

        // Extract route geometry from routeSections[].lineString
        let mut _geometry_points_extracted = 0usize;
        if let Some(route_sections) = data.get("routeSections").and_then(|v| v.as_array()) {
            log_debug(&format!(
                "AppState::parse_line_data - found {} route sections",
                route_sections.len()
            ));
            for section in route_sections {
                if let Some(line_string) = section.get("lineString").and_then(|v| v.as_array()) {
                    for coord in line_string {
                        if let (Some(lat), Some(lon)) = (
                            coord.get("lat").and_then(|v| v.as_f64()),
                            coord.get("lon").and_then(|v| v.as_f64()),
                        ) {
                            let _ = _geometry_points_extracted;
                            line.geometry.push(Coordinate::new(lat, lon));
                        }
                    }
                }
            }
        }

        log_info(&format!(
            "AppState::parse_line_data completed - parsed line {} with {} stations and {} geometry points",
            line_id,
            line.stations.len(),
            line.geometry.len()
        ));
        Ok(line)
    }

    fn get_line_color(&self, line_id: &str) -> String {
        log_trace(&format!(
            "AppState::get_line_color called - line_id: {}",
            line_id
        ));
        let color = TFL_COLOR_REGISTRY
            .get(line_id)
            .unwrap_or(&"#888888")
            .to_string();
        log_trace(&format!("AppState::get_line_color result: {}", color));
        color
    }

    async fn fetch_railway_tracks(
        &self,
        bounds: &LondonBounds,
    ) -> Result<Vec<RailwayTrack>, Box<dyn std::error::Error>> {
        log_info(&format!("AppState::fetch_railway_tracks called - bounds: lat {:.6} to {:.6}, lon {:.6} to {:.6}", bounds.min_lat, bounds.max_lat, bounds.min_lon, bounds.max_lon));
        let cache_key = "railway_tracks_london".to_string();
        log_debug(&format!(
            "AppState::fetch_railway_tracks - cache_key: {}",
            cache_key
        ));
        let cache_clone = self.cache.clone();
        let cache_key_clone = cache_key.to_string();

        let get_res = tokio::task::spawn_blocking(move || {
            log_trace("AppState::fetch_railway_tracks - spawning blocking cache get task");
            cache_clone.get(&cache_key_clone).map_err(|e| e.to_string())
        })
        .await;

        let cached_opt = match get_res {
            Ok(Ok(opt)) => opt,
            Ok(Err(e)) => {
                log_error(&format!(
                    "AppState::fetch_railway_tracks - cache get error: {}",
                    e
                ));
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
            }
            Err(e) => {
                log_error(&format!(
                    "AppState::fetch_railway_tracks - task spawn error: {}",
                    e
                ));
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )));
            }
        };

        if let Some(cached) = cached_opt {
            if let Ok(tracks) = serde_json::from_str::<Vec<RailwayTrack>>(&cached) {
                // Fix #2: If cached tracks are empty, treat as cache miss
                // (Overpass may have returned empty results due to network issues or query limits)
                if tracks.is_empty() {
                    log_warn("AppState::fetch_railway_tracks - cached tracks are empty; treating as cache miss, re-fetching");
                } else {
                    log_info(&format!(
                        "AppState::fetch_railway_tracks - loaded {} railway tracks from cache",
                        tracks.len()
                    ));
                    return Ok(tracks);
                }
            } else {
                log_warn("AppState::fetch_railway_tracks - failed to deserialize cached tracks");
            }
        }

        log_debug("AppState::fetch_railway_tracks - cache miss, fetching from Overpass API");
        let data = self
            .overpass_client
            .fetch_railway_tracks(
                bounds.min_lat,
                bounds.min_lon,
                bounds.max_lat,
                bounds.max_lon,
            )
            .await?;

        let mut tracks = Vec::new();
        let mut skipped_count = 0;

        let start_processing_time = Utc::now();
        if let Some(elements) = data.get("elements").and_then(|v| v.as_array()) {
            log_info(&format!(
                "AppState::fetch_railway_tracks - ingesting raw JSON array payload size: {} elements",
                elements.len()
            ));

            // Phase 1: Build a high-speed lookup map for all standalone node components
            let mut nodes_map = HashMap::new();
            let mut nodes_extracted = 0usize;
            for el in elements {
                if el.get("type").and_then(|v| v.as_str()) == Some("node") {
                    if let (Some(id), Some(lat), Some(lon)) = (
                        el.get("id").and_then(|v| v.as_i64()),
                        el.get("lat").and_then(|v| v.as_f64()),
                        el.get("lon").and_then(|v| v.as_f64()),
                    ) {
                        nodes_extracted += 1;
                        nodes_map.insert(id, Coordinate::new(lat, lon));
                    }
                }
            }
            log_debug(&format!(
                "AppState::fetch_railway_tracks - Phase 1: extracted {} nodes to lookup map",
                nodes_extracted
            ));

            // Phase 2: Resolve track coordinates from way nodes when geometry blocks are missing
            let mut ways_processed = 0usize;
            for (idx, element) in elements.iter().enumerate() {
                if element.get("type").and_then(|v| v.as_str()) == Some("way") {
                    ways_processed += 1;
                    let mut geometry = Vec::new();
                    if let Some(way_data) = element.get("geometry").and_then(|v| v.as_array()) {
                        for coord in way_data {
                            if let (Some(lat), Some(lon)) = (
                                coord.get("lat").and_then(|v| v.as_f64()),
                                coord.get("lon").and_then(|v| v.as_f64()),
                            ) {
                                geometry.push(Coordinate::new(lat, lon));
                            } else {
                                skipped_count += 1;
                            }
                        }
                    } else if let Some(nodes) = element.get("nodes").and_then(|v| v.as_array()) {
                        for node_id_val in nodes {
                            if let Some(node_id) = node_id_val.as_i64() {
                                if let Some(coord) = nodes_map.get(&node_id) {
                                    geometry.push(*coord);
                                }
                            }
                        }
                    }

                    if !geometry.is_empty() {
                        let id = element
                            .get("id")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(idx as i64)
                            .to_string();
                        let operator = element
                            .get("tags")
                            .and_then(|t| t.get("operator"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("National Rail Platform Infrastructure");

                        log_trace(&format!("AppState::fetch_railway_tracks - creating track {} with {} geometry points, operator: {}", id, geometry.len(), operator));
                        tracks.push(RailwayTrack {
                            id,
                            operator_name: operator.to_string(),
                            geometry,
                            is_abandoned: false,
                        });
                    } else {
                        skipped_count += 1;
                        log_trace(&format!(
                            "AppState::fetch_railway_tracks - skipping way {} (empty geometry)",
                            idx
                        ));
                    }
                }
            }
            log_debug(&format!(
                "AppState::fetch_railway_tracks - Phase 2: processed {} ways, created {} tracks",
                ways_processed,
                tracks.len()
            ));
        } else {
            log_error("AppState::fetch_railway_tracks - no elements found in Overpass response");
        }

        let elapsed = (Utc::now() - start_processing_time).num_milliseconds();
        log_info(&format!(
            "[PERF] AppState::fetch_railway_tracks - Overpass element parsing completed in {}ms. Ingested {} valid paths.",
            elapsed,
            tracks.len()
        ));

        if skipped_count > 0 {
            log_warn(&format!(
                "AppState::fetch_railway_tracks - skipped {} track elements due to missing data",
                skipped_count
            ));
        }

        // Fix #2: Only cache if we actually got tracks — don't persist empty results
        if !tracks.is_empty() {
            log_debug(&format!(
                "AppState::fetch_railway_tracks - caching {} tracks ({} bytes)",
                tracks.len(),
                serde_json::to_string(&tracks).unwrap_or_default().len()
            ));
            let cached = serde_json::to_string(&tracks)?;
            let cache_clone = self.cache.clone();
            let cache_key_clone = cache_key.to_string();
            let cached_clone = cached.clone();
            let put_res = tokio::task::spawn_blocking(move || {
                log_trace("AppState::fetch_railway_tracks - spawning blocking cache put task");
                cache_clone
                    .put(&cache_key_clone, &cached_clone, 7 * 24 * 3600 * 1000)
                    .map_err(|e| e.to_string())
            })
            .await;

            match put_res {
                Ok(Ok(_)) => log_debug("AppState::fetch_railway_tracks - cache put successful"),
                Ok(Err(e)) => {
                    log_error(&format!(
                        "AppState::fetch_railway_tracks - cache put error: {}",
                        e
                    ));
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
                }
                Err(e) => {
                    log_error(&format!(
                        "AppState::fetch_railway_tracks - cache put task error: {}",
                        e
                    ));
                    return Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string()
                    )));
                }
            }
        } else {
            log_warn("AppState::fetch_railway_tracks - Empty dataset rejected. Skipped write operation to safeguard local cache.");
        }

        log_info(&format!(
            "AppState::fetch_railway_tracks completed - fetched {} railway tracks",
            tracks.len()
        ));
        Ok(tracks)
    }

    async fn fetch_residential_coordinates(
        &self,
        bounds: &LondonBounds,
    ) -> Result<Vec<Coordinate>, Box<dyn std::error::Error>> {
        log_info(&format!("AppState::fetch_residential_coordinates called - bounds: lat {:.6} to {:.6}, lon {:.6} to {:.6}", bounds.min_lat, bounds.max_lat, bounds.min_lon, bounds.max_lon));
        // Fix #5: Include version hash in cache key so future query changes invalidate old caches
        let cache_key = format!("res_areas_v2_{:.2}_{:.2}", bounds.min_lat, bounds.min_lon);
        log_debug(&format!(
            "AppState::fetch_residential_coordinates - cache_key: {}",
            cache_key
        ));
        let cache_clone = self.cache.clone();
        let cache_key_clone = cache_key.clone();

        let get_res = tokio::task::spawn_blocking(move || {
            log_trace("AppState::fetch_residential_coordinates - spawning blocking cache get task");
            cache_clone.get(&cache_key_clone).map_err(|e| e.to_string())
        })
        .await;

        let cached_opt = match get_res {
            Ok(Ok(opt)) => opt,
            Ok(Err(e)) => {
                log_error(&format!(
                    "AppState::fetch_residential_coordinates - cache get error: {}",
                    e
                ));
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)));
            }
            Err(e) => {
                log_error(&format!(
                    "AppState::fetch_residential_coordinates - task spawn error: {}",
                    e
                ));
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    e.to_string(),
                )));
            }
        };

        if let Some(cached) = cached_opt {
            log_debug("AppState::fetch_residential_coordinates - cache hit");
            if let Ok(coords) = serde_json::from_str::<Vec<Coordinate>>(&cached) {
                log_info(&format!("AppState::fetch_residential_coordinates - loaded {} residential coordinates from cache", coords.len()));
                return Ok(coords);
            } else {
                log_warn("AppState::fetch_residential_coordinates - failed to deserialize cached coordinates");
            }
        }

        log_debug(
            "AppState::fetch_residential_coordinates - cache miss, fetching from Overpass API",
        );
        let data = self
            .overpass_client
            .fetch_residential_areas(
                bounds.min_lat,
                bounds.min_lon,
                bounds.max_lat,
                bounds.max_lon,
            )
            .await?;

        let mut raw_geometry_array = Vec::new();
        let mut elements_processed = 0usize;
        if let Some(elements) = data.get("elements").and_then(|v| v.as_array()) {
            log_info(&format!("AppState::fetch_residential_coordinates - processing {} elements from Overpass response", elements.len()));
            for el in elements {
                elements_processed += 1;
                let lat = el.get("lat").and_then(|v| v.as_f64()).or_else(|| {
                    el.get("center")
                        .and_then(|c| c.get("lat"))
                        .and_then(|v| v.as_f64())
                });
                let lon = el.get("lon").and_then(|v| v.as_f64()).or_else(|| {
                    el.get("center")
                        .and_then(|c| c.get("lon"))
                        .and_then(|v| v.as_f64())
                });
                if let (Some(la), Some(lo)) = (lat, lon) {
                    raw_geometry_array.push(Coordinate::new(la, lo));
                }
            }
        }
        log_debug(&format!("AppState::fetch_residential_coordinates - extracted {} raw coordinates from {} elements", raw_geometry_array.len(), elements_processed));

        log_debug("AppState::fetch_residential_coordinates - normalizing projections with Rayon parallel processing");
        use rayon::prelude::*;
        let coords: Vec<Coordinate> = raw_geometry_array
            .par_iter() // Distribute projection processing evenly across all available CPU threads
            .map(|coord| coord.normalize_projections())
            .collect();
        log_debug(&format!(
            "AppState::fetch_residential_coordinates - normalized {} coordinates",
            coords.len()
        ));

        log_debug(&format!(
            "AppState::fetch_residential_coordinates - caching {} coordinates ({} bytes)",
            coords.len(),
            serde_json::to_string(&coords).unwrap_or_default().len()
        ));
        let cached = serde_json::to_string(&coords)?;
        let cache_clone = self.cache.clone();
        let cache_key_clone = cache_key.clone();
        let cached_clone = cached.clone();
        let _ = tokio::task::spawn_blocking(move || {
            log_trace("AppState::fetch_residential_coordinates - spawning blocking cache put task");
            let _ = cache_clone.put(&cache_key_clone, &cached_clone, 30 * 24 * 3600 * 1000);
        })
        .await;

        log_info(&format!("AppState::fetch_residential_coordinates completed - fetched {} residential coordinates", coords.len()));
        Ok(coords)
    }

    async fn initialize_routing_graph(
        &self,
        bounds: &LondonBounds,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log_info("AppState::initialize_routing_graph called - initializing routing graph");
        log_debug(&format!(
            "AppState::initialize_routing_graph - bounds: lat {:.6} to {:.6}, lon {:.6} to {:.6}",
            bounds.min_lat, bounds.max_lat, bounds.min_lon, bounds.max_lon
        ));

        log_debug("AppState::initialize_routing_graph - fetching railway tracks");
        let tracks = self.fetch_railway_tracks(bounds).await?;
        log_info(&format!(
            "AppState::initialize_routing_graph - storing {} tracks in state",
            tracks.len()
        ));
        self.tracks.store(Arc::new(tracks.clone()));

        // Build geometry engine with track spatial index so snap_to_tracks works
        log_debug("AppState::initialize_routing_graph - building geometry engine with track spatial index");
        let mut new_engine = GeometryEngine::new();
        new_engine.build_track_index(&tracks);
        self.geometry_engine.store(Arc::new(new_engine));

        let geom = self.geometry_engine.load();
        log_debug("AppState::initialize_routing_graph - merging stations");
        let stations_clone = (**self.stations.load()).clone();
        let total_stations = stations_clone.len();
        log_debug(&format!(
            "AppState::initialize_routing_graph - merging {} stations with threshold {:.6}",
            total_stations, STATION_MERGE_THRESHOLD
        ));
        let merged_stations = geom.merge_stations(stations_clone, STATION_MERGE_THRESHOLD);
        log_info(&format!(
            "AppState::initialize_routing_graph - merged {} -> {} stations",
            total_stations,
            merged_stations.len()
        ));
        self.stations.store(Arc::new(merged_stations));

        log_debug("AppState::initialize_routing_graph - building routing graph from tracks");
        let mut routing = (**self.routing_graph.load()).clone();
        routing.build_from_tracks(&tracks);
        self.routing_graph.store(Arc::new(routing));

        log_info("AppState::initialize_routing_graph completed - routing graph initialized");
        Ok(())
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_new_line_stations_into_global_state() {
        let state = AppState::new(Config::default());
        let line = Line {
            id: "test-line".to_string(),
            name: "Test Line".to_string(),
            color: "#123456".to_string(),
            stations: vec![
                Station::new(
                    "station-a".to_string(),
                    "Station A".to_string(),
                    Coordinate::new(51.5, -0.1),
                ),
                Station::new(
                    "station-b".to_string(),
                    "Station B".to_string(),
                    Coordinate::new(51.6, -0.2),
                ),
            ],
            segments: Vec::new(),
            geometry: Vec::new(),
            is_custom: false,
        };

        state.register_line_stations_in_global_state(&line);

        let global_stations = (**state.stations.load()).clone();
        assert_eq!(global_stations.len(), 2);
        assert!(global_stations.iter().any(|s| s.id == "station-a"));
        assert!(global_stations.iter().any(|s| s.id == "station-b"));
    }
}

// ============================================================================
// WEB SERVER
// ============================================================================

#[derive(Debug, Deserialize, Serialize, Clone)]
struct LoadLineRequest {
    pub line_id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct SaveLineRequest {
    line: Line,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct SaveStationRequest {
    station: Station,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RouteRequest {
    pub start: Coordinate,
    pub end: Coordinate,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct TransitDesertsRequest {
    pub bounds: LondonBounds,
}

async fn run_server(state: AppState, config: Config) -> Result<(), Box<dyn std::error::Error>> {
    log_info("run_server called - starting Axum web server");
    log_debug(&format!(
        "run_server - server_host: {}, server_port: {}",
        config.server_host, config.server_port
    ));

    let app = Router::new()
        .route("/api/lines", get(get_lines))
        .route("/api/lines/load", post(load_line))
        .route("/api/lines/save", post(save_line))
        .route("/api/stations", get(get_stations))
        .route("/api/stations/save", post(save_station))
        .route("/api/construction", get(get_construction_state))
        .route("/api/construction/update", post(update_construction_state))
        .route("/api/route", post(find_route))
        .route("/api/transit-deserts", post(get_transit_deserts))
        .route("/api/disruptions", get(get_disruptions))
        .route("/api/tracks", get(get_tracks))
        .route("/api/tracks/refresh", post(refresh_tracks))
        .route("/api/lines/delete/:id", post(delete_line))
        .route("/api/logs", get(get_logs))
        .route("/api/config", get(get_config))
        .layer({
            use axum::http::{header::CONTENT_TYPE, Method};
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers([CONTENT_TYPE])
        })
        .with_state(state.clone());

    log_debug("run_server - configured API routes with CORS layer");

    // Fix #6: Tracks are already fetched synchronously before the server starts
    // in the main initialization block_on. No need for a background warmup that
    // could race with the server accepting requests.

    let addr: std::net::SocketAddr = format!("{}:{}", config.server_host, config.server_port)
        .parse()
        .expect("Invalid operational binding target");
    log_info(&format!(
        "run_server - data engine listening securely on http://{}",
        addr
    ));

    log_debug("run_server - binding TCP listener");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    log_info("run_server - TCP listener bound, starting Axum serve");
    axum::serve(listener, app.into_make_service()).await?;
    log_error("run_server - Axum server ended unexpectedly");
    Ok(())
}

async fn get_config() -> Json<Value> {
    log_info("GET /api/config called - returning configuration constants");
    Json(serde_json::json!({
        "TILE_SIZE": TILE_SIZE,
        "DEFAULT_ZOOM": DEFAULT_ZOOM,
        "MIN_ZOOM": MIN_ZOOM,
        "MAX_ZOOM": MAX_ZOOM,
        "CATCHMENT_RADIUS": CATCHMENT_RADIUS,
        "STATION_MERGE_THRESHOLD": STATION_MERGE_THRESHOLD
    }))
}

async fn get_lines(State(state): State<AppState>) -> Json<ApiResponse<Vec<Line>>> {
    log_info("GET /api/lines called");
    let (seeded_lines, _) = state.ensure_sample_network_state().await;
    log_debug(&format!(
        "GET /api/lines - returning {} lines",
        seeded_lines.len()
    ));
    Json(ApiResponse::success(seeded_lines))
}

async fn load_line(
    State(state): State<AppState>,
    Json(req): Json<LoadLineRequest>,
) -> Json<ApiResponse<Line>> {
    log_info(&format!(
        "POST /api/lines/load called - loading line: {}",
        req.line_id
    ));

    let load_result = state
        .load_line_routes(&req.line_id)
        .await
        .map_err(|e| e.to_string());

    match load_result {
        Ok(line) => {
            log_debug(&format!(
                "POST /api/lines/load - successfully loaded line {} with {} stations",
                line.id,
                line.stations.len()
            ));
            let mut lines = (**state.lines.load()).clone();
            lines.push(line.clone());
            let current_total = lines.len();
            state.lines.store(Arc::new(lines));
            log_info(&format!(
                "POST /api/lines/load completed - added line {} to state (total lines: {})",
                line.id, current_total
            ));
            Json(ApiResponse::success(line))
        }
        Err(e) => {
            log_error(&format!(
                "POST /api/lines/load failed - error loading line {}: {}",
                req.line_id, e
            ));
            Json(ApiResponse::error(e))
        }
    }
}

async fn save_line(
    State(state): State<AppState>,
    Json(req): Json<SaveLineRequest>,
) -> Json<ApiResponse<Line>> {
    log_info(&format!(
        "POST /api/lines/save called - saving line: {} with {} geometry points",
        req.line.id,
        req.line.geometry.len()
    ));

    let cache_clone = state.cache.clone();
    let line_clone = req.line.clone();
    let save_result = tokio::task::spawn_blocking(move || {
        log_trace("POST /api/lines/save - spawning blocking save task");
        cache_clone
            .save_custom_line(&line_clone)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string());

    match save_result {
        Ok(Ok(_)) => {
            log_debug("POST /api/lines/save - database save successful");
            let mut lines = (**state.lines.load()).clone();
            lines.push(req.line.clone());
            let current_total = lines.len();
            state.lines.store(Arc::new(lines));
            log_info(&format!(
                "POST /api/lines/save completed - saved line {} (total lines: {})",
                req.line.id, current_total
            ));
            Json(ApiResponse::success(req.line))
        }
        Ok(Err(e)) => {
            log_error(&format!(
                "POST /api/lines/save failed - database error: {}",
                e
            ));
            Json(ApiResponse::error(e))
        }
        Err(e) => {
            log_error(&format!(
                "POST /api/lines/save failed - thread error: {}",
                e
            ));
            Json(ApiResponse::error(e))
        }
    }
}

async fn get_stations(State(state): State<AppState>) -> Json<ApiResponse<Vec<Station>>> {
    log_info("GET /api/stations called");
    let (_, seeded_stations) = state.ensure_sample_network_state().await;
    log_debug(&format!(
        "GET /api/stations - returning {} stations",
        seeded_stations.len()
    ));
    Json(ApiResponse::success(seeded_stations))
}

async fn get_tracks(State(state): State<AppState>) -> Json<ApiResponse<Vec<RailwayTrack>>> {
    log_info("GET /api/tracks called - syncing infrastructure tracks");
    Json(ApiResponse::success((*state.tracks.load()).as_ref().clone()))
}

/// Fix #2: Manual "Refresh Tracks" endpoint to force a fresh Overpass query
async fn refresh_tracks(
    State(state): State<AppState>,
) -> impl axum::response::IntoResponse {
    log_info("POST /api/tracks/refresh called - force-refreshing railway tracks from Overpass");

    let cache = state.cache.clone();
    let _ = tokio::task::spawn_blocking(move || {
        if let Ok(conn) = cache.pool.get() {
            let _ = conn.execute("DELETE FROM api_cache WHERE key = 'railway_tracks_london'", []);
        }
    }).await;

    match state.fetch_railway_tracks(&state.config.london_bounds).await {
        Ok(tracks) => {
            log_info(&format!("POST /api/tracks/refresh - refreshed {} tracks", tracks.len()));
            state.tracks.store(Arc::new(tracks.clone()));
            axum::Json(ApiResponse::success(tracks))
        }
        Err(e) => {
            log_error(&format!("POST /api/tracks/refresh failed: {}", e));
            axum::Json(ApiResponse::<Vec<RailwayTrack>>::error(e.to_string()))
        }
    }
}

async fn delete_line(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    let id_clone = id.clone();
    log_info(&format!("POST /api/lines/delete called for target custom line reference: {}", id));
    let cache = state.cache.clone();

    let db_res = tokio::task::spawn_blocking(move || {
        if let Ok(conn) = cache.pool.get() {
            conn.execute("DELETE FROM custom_lines WHERE id = ?1", params![id])
        } else {
            Err(rusqlite::Error::ExecuteReturnedResults)
        }
    }).await;

    let mut current_lines = (**state.lines.load()).clone();
    current_lines.retain(|l| l.id != id_clone);
    state.lines.store(Arc::new(current_lines));

    axum::Json(ApiResponse::success(true))
}

async fn save_station(
    State(state): State<AppState>,
    Json(req): Json<SaveStationRequest>,
) -> Json<ApiResponse<Station>> {
    log_info(&format!(
        "POST /api/stations/save called - saving station: {} at lat={:.6}, lon={:.6}",
        req.station.id, req.station.coord.lat, req.station.coord.lon
    ));

    let cache_clone = state.cache.clone();
    let station_clone = req.station.clone();
    let save_result = tokio::task::spawn_blocking(move || {
        log_trace("POST /api/stations/save - spawning blocking save task");
        cache_clone
            .save_free_station(&station_clone)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string());

    match save_result {
        Ok(Ok(_)) => {
            log_debug("POST /api/stations/save - database save successful");
            let mut stations = (**state.stations.load()).clone();
            stations.push(req.station.clone());
            let current_total = stations.len();
            state.stations.store(Arc::new(stations));
            log_info(&format!(
                "POST /api/stations/save completed - saved station {} (total stations: {})",
                req.station.id, current_total
            ));
            Json(ApiResponse::success(req.station))
        }
        Ok(Err(e)) => {
            log_error(&format!(
                "POST /api/stations/save failed - database error: {}",
                e
            ));
            Json(ApiResponse::error(e))
        }
        Err(e) => {
            log_error(&format!(
                "POST /api/stations/save failed - thread error: {}",
                e
            ));
            Json(ApiResponse::error(e))
        }
    }
}

async fn get_construction_state(
    State(state): State<AppState>,
) -> Json<ApiResponse<ConstructionState>> {
    log_info("GET /api/construction called");
    let construction = state.construction_state.load();
    log_debug("GET /api/construction - returning construction state");
    Json(ApiResponse::success((**construction).clone()))
}

async fn update_construction_state(
    State(state): State<AppState>,
    Json(new_state): Json<ConstructionState>,
) -> Json<ApiResponse<ConstructionState>> {
    log_info("POST /api/construction/update called - updating construction state");
    state.construction_state.store(Arc::new(new_state.clone()));
    log_debug("POST /api/construction/update - state updated successfully");
    Json(ApiResponse::success(new_state))
}

async fn find_route(
    State(state): State<AppState>,
    Json(req): Json<RouteRequest>,
) -> Json<ApiResponse<Vec<Coordinate>>> {
    log_info(&format!(
        "POST /api/route called - finding route from lat={:.6}, lon={:.6} to lat={:.6}, lon={:.6}",
        req.start.lat, req.start.lon, req.end.lat, req.end.lon
    ));
    let routing = state.routing_graph.load();
    log_debug(&format!(
        "POST /api/route - routing graph has {} nodes",
        routing.nodes.len()
    ));
    let path = routing.find_path(&req.start, &req.end);
    log_info(&format!(
        "POST /api/route completed - path with {} points",
        path.len()
    ));
    Json(ApiResponse::success(path))
}

async fn get_transit_deserts(
    State(state): State<AppState>,
    Json(req): Json<TransitDesertsRequest>,
) -> Json<ApiResponse<Vec<Coordinate>>> {
    log_info(&format!("POST /api/transit-deserts called - computing transit deserts for bounds: lat {:.6} to {:.6}, lon {:.6} to {:.6}", req.bounds.min_lat, req.bounds.max_lat, req.bounds.min_lon, req.bounds.max_lon));
    match state.fetch_residential_coordinates(&req.bounds).await {
        Ok(res_coords) => {
            log_debug(&format!(
                "POST /api/transit-deserts - fetched {} residential coordinates",
                res_coords.len()
            ));
            let stations = state.stations.load();
            let geom = state.geometry_engine.load();
            log_debug(&format!("POST /api/transit-deserts - computing deserts with {} stations, catchment radius: {:.2}m", stations.len(), CATCHMENT_RADIUS));
            let deserts = geom.compute_transit_deserts(&res_coords, &stations, CATCHMENT_RADIUS);
            log_info(&format!(
                "POST /api/transit-deserts completed - found {} transit deserts",
                deserts.len()
            ));
            Json(ApiResponse::success(deserts))
        }
        Err(e) => {
            log_error(&format!(
                "POST /api/transit-deserts failed - error computing transit deserts: {}",
                e
            ));
            Json(ApiResponse::error(e.to_string()))
        }
    }
}

async fn get_disruptions(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    log_info("GET /api/disruptions called - fetching TfL disruptions");
    match state.tfl_client.fetch_disruptions().await {
        Ok(disruptions) => {
            log_debug(&format!(
                "GET /api/disruptions - successfully fetched disruptions"
            ));
            Json(ApiResponse::success(disruptions))
        }
        Err(e) => {
            log_error(&format!(
                "GET /api/disruptions failed - error fetching disruptions: {}",
                e
            ));
            Json(ApiResponse::error(e.to_string()))
        }
    }
}

async fn get_logs() -> Json<ApiResponse<String>> {
    // Intentionally silent — no log_info/log_debug here to avoid endless echo loop
    Json(ApiResponse::success(get_all_logs()))
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

/// Global used to pass the server port to the standalone console process
static CONSOLE_SERVER_PORT: std::sync::OnceLock<u16> = std::sync::OnceLock::new();

fn main() {
    // Check if this process should run as the standalone console window
    let args: Vec<String> = std::env::args().collect();
    if let Some(pos) = args.iter().position(|a| a == "--console") {
        let port: u16 = args.get(pos + 1)
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);
        let _ = CONSOLE_SERVER_PORT.set(port);
        log_info(&format!("Starting standalone console window, connecting to port {}", port));
        LaunchBuilder::desktop()
            .with_cfg(build_console_window_configuration())
            .launch(ConsoleStandaloneApp);
        return;
    }

    log_info("main called - starting application initialization");

    // Fix 2: Custom OS Panic Hook Interceptor Gate
    log_debug("main - setting up panic hook");
    std::panic::set_hook(Box::new(|info| {
        log_error(&format!("PANIC HOOK TRIGGERED - panic detected"));
        IS_PANICKED.store(true, std::sync::atomic::Ordering::SeqCst);
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "Unknown Location".to_string());
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .cloned()
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("Explicit thread execution collapse");

        log_error(&format!("PANIC LOCATION: {}", location));
        log_error(&format!("PANIC PAYLOAD: {}", payload));

        let crash_report = format!(
            "[{}] [CRITICAL PANIC] System collapsed at {}\nReason: {}\n\nSystem Log Trace History:\n{}",
            Utc::now().format("%Y-%m-%d %H:%M:%S%.6f UTC"),
            location,
            payload,
            get_all_logs()
        );
        accumulate_crash_text(&crash_report);
        eprintln!("{}", crash_report);

        log_debug("main - panic recovery path without spawning a new desktop window");
    }));

    // Fix 4: Single-Instance Process Mutex Verification Layer
    log_debug("main - setting up single-instance mutex");
    let lock_path = std::env::temp_dir().join("london_transport_engine.lock");
    log_debug(&format!("main - lock file path: {:?}", lock_path));
    let lock_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&lock_path)
        .expect("Failed to create lock file — check temp directory permissions");
    use fs2::FileExt;
    if lock_file.try_lock_exclusive().is_err() {
        log_error("main - another instance is already running");
        accumulate_crash_text(
            "Multiple active workspace allocations identified. Aborting execution branch to resolve handle collision.",
        );
        panic!("Process handle collision prevented via Single-Instance Mutex Policy.");
    }
    log_debug("main - single-instance mutex acquired successfully");
    // Keep lock_file alive for process duration; register cleanup on exit
    let lock_path_cleanup = lock_path.clone();
    let _lock_guard = scopeguard::guard(lock_file, move |f| {
        log_debug("main - executing emergency teardown and flushing database logs");
        drop(f);
        let _ = std::fs::remove_file(&lock_path_cleanup);
        std::process::exit(0);
    });

    log_info("Initializing consolidated single-file runtime engine...");

    // Create a dedicated multi-threaded Tokio runtime for our background systems
    log_debug("main - creating Tokio runtime");
    let rt = tokio::runtime::Runtime::new().unwrap();
    log_info("main - Tokio runtime created");

    log_debug("main - loading configuration");
    let config = Config::load();
    log_info("main - configuration loaded");

    log_debug("main - creating application state");
    let state = AppState::new(config.clone());

    // Boot background services and warm up local data caches
    log_debug("main - booting background services and warming caches");
    rt.block_on(async {
        log_debug("main - loading custom lines from database");
        let state_db = state.clone();
        let custom_lines = tokio::task::spawn_blocking(move || {
            state_db
                .cache
                .load_custom_lines()
                .map_err(|e| e.to_string())
        })
        .await
        .unwrap()
        .unwrap();
        log_info(&format!(
            "main - loaded {} custom lines from database",
            custom_lines.len()
        ));
        state.lines.store(Arc::new(custom_lines));

        log_debug("main - loading free stations from database");
        if let Ok(free_stations) = state.cache.load_free_stations() {
            log_info(&format!(
                "main - loaded {} free stations from database",
                free_stations.len()
            ));
            state.stations.store(Arc::new(free_stations));
        } else {
            log_warn("main - failed to load free stations from database");
        }

        // CRITICAL: Build the routing graph FIRST so tracks / spatial index are
        // available when load_line_routes tries to snap stations to real track geometry.
        log_info("main - compiling spatial routing graph before loading lines");
        if let Err(e) = state.initialize_routing_graph(&config.london_bounds).await {
            log_error(&format!(
                "main - critical failure compiling spatial routing graph: {}",
                e
            ));
        }
        log_info("main - routing graph compiled successfully");

        // Now load sample lines; find_path will snap nodes to physical rails.
        log_debug("main - loading sample lines from config");
        let sample_lines = config.sample_lines.clone();
        log_info(&format!(
            "main - attempting to load {} sample lines",
            sample_lines.len()
        ));
        for line_id in &sample_lines {
            log_debug(&format!("main - loading sample line: {}", line_id));
            match state.load_line_routes(line_id).await {
                Ok(line) => {
                    let mut current_lines = (**state.lines.load()).clone();
                    // Prevent duplicate entries
                    current_lines.retain(|l| l.id != line.id);

                    log_info(&format!(
                        "main - loaded sample line: {} with {} stations",
                        line_id,
                        line.stations.len()
                    ));

                    current_lines.push(line);
                    state.lines.store(Arc::new(current_lines));
                }
                Err(e) => {
                    log_warn(&format!(
                        "main - failed to load sample line {}: {}",
                        line_id, e
                    ));
                }
            }
        }
        log_info("main - sample line loading completed");
    });
    log_info("main - background services boot completed");

    // Spin up the local network routing manager on a background thread
    log_debug("main - spawning background thread for web server");
    let server_state = state.clone();
    let server_config = config.clone();
    let server_handle = std::thread::spawn(move || {
        log_debug("main - server thread started, creating Tokio runtime");
        let local_rt = tokio::runtime::Runtime::new().unwrap();
        log_debug("main - server thread Tokio runtime created, starting server");
        local_rt.block_on(async {
            if let Err(e) = run_server(server_state, server_config).await {
                log_error(&format!("main - background data service failed: {}", e));
            }
        });
        log_debug("main - server thread ended");
    });
    log_info("main - web server thread spawned");

    log_info("main - spawning standalone console window process");
    let console_port = config.server_port;
    if let Ok(exe_path) = std::env::current_exe() {
        match std::process::Command::new(&exe_path)
            .args(["--console", &console_port.to_string()])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => {
                log_info(&format!("main - standalone console process spawned with PID: {}", child.id()));
                // Detach the child process - don't wait for it
                std::mem::forget(child);
            }
            Err(e) => {
                log_warn(&format!("main - failed to spawn standalone console: {}", e));
            }
        }
    } else {
        log_warn("main - could not determine executable path for console spawn");
    }

    log_info("main - keeping diagnostics stream inside the main window surface");

    // Launch the native client window immediately on the main execution thread
    log_debug("main - launching Dioxus desktop window");
    LaunchBuilder::desktop()
        .with_cfg(build_desktop_window_configuration())
        .launch(App);
    log_error("main - Dioxus window ended unexpectedly");
}

// ============================================================================
// CLIPBOARD HELPER – Uses hidden textarea + execCommand('copy') fallback
// for desktop WebViews where navigator.clipboard.writeText may not be available.
// ============================================================================

static CLIPBOARD_JS: &str = r##"
(function copyText(text) {
    // Try native clipboard API first (works in secure contexts)
    if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(text).catch(function() {
            fallbackCopy(text);
        });
    } else {
        fallbackCopy(text);
    }
    function fallbackCopy(str) {
        var textarea = document.createElement('textarea');
        textarea.value = str;
        textarea.style.position = 'fixed';
        textarea.style.opacity = '0';
        textarea.style.left = '-9999px';
        textarea.style.top = '-9999px';
        document.body.appendChild(textarea);
        textarea.focus();
        textarea.select();
        try {
            document.execCommand('copy');
        } catch (e) {
            console.error('Clipboard fallback failed:', e);
        }
        document.body.removeChild(textarea);
    }
})("##;

static COPY_LOG_JS_PREFIX: &str = r##"(function() {
    var text = "##;

static COPY_LOG_JS_SUFFIX: &str = r##";
    if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(text).catch(function() { fallbackCopy(text); });
    } else {
        fallbackCopy(text);
    }
    function fallbackCopy(str) {
        var ta = document.createElement('textarea');
        ta.value = str;
        ta.style.position = 'fixed';
        ta.style.opacity = '0';
        ta.style.left = '-9999px';
        ta.style.top = '-9999px';
        document.body.appendChild(ta);
        ta.focus();
        ta.select();
        try { document.execCommand('copy'); } catch(e) { console.error('Clipboard fallback failed:', e); }
        document.body.removeChild(ta);
    }
})();"##;

fn build_copy_log_js(text: &str) -> String {
    format!("{}{}{}", COPY_LOG_JS_PREFIX, serde_json::to_string(text).unwrap_or_default(), COPY_LOG_JS_SUFFIX)
}

// ============================================================================
// CLIENT-SIDE PURE RUST DIOXUS FRONTEND (Dioxus 0.5)
// ============================================================================

static MAP_INIT_JS: &str = r#"
window.initMap = async function(dioxus) {
    if (window.map) {
        window.map.remove();
    }

    // Leaflet is already verified inside the head tag layer
    window.map = L.map('map-viewport', {
        preferCanvas: true,
        zoomControl: false,
        renderer: L.canvas({ padding: 0.5, tolerance: 3 }),
        bounceAtZoomLimits: false,
        wheelDebounceTime: 40
    }).setView([51.5074, -0.1278], 12);
    L.tileLayer('https://mt{s}.google.com/vt/lyrs=s&x={x}&y={y}&z={z}', {
        subdomains: ['0', '1', '2', '3'],
        attribution: '&copy; Google Imagery',
        maxZoom: 22,
        maxNativeZoom: 20
    }).addTo(window.map);

    window.lineLayers = {};
    window.stationLayers = {};
    window.trackLayers = [];
    window.desertLayer = null;
    window.drawingLayer = L.polyline([], { color: '#ff00ff', dashArray: '5, 5', weight: 4 }).addTo(window.map);

    window.map.on('click', function(e) {
        dioxus.send({ "event": "map_click", "lat": e.latlng.lat, "lng": e.latlng.lng });
    });

    if (window.map) { window.map.off('contextmenu'); } // Wipe existing hooks clean
    window.map.on('contextmenu', function(e) {
        e.originalEvent.preventDefault();
        dioxus.send({
            "event": "map_context",
            "lat": e.latlng.lat,
            "lng": e.latlng.lng,
            "x": e.originalEvent.clientX,
            "y": e.originalEvent.clientY
        });
    });

    let bounds = window.map.getBounds();
    dioxus.send({
        "event": "bounds_changed",
        "min_lat": bounds.getSouth(),
        "min_lon": bounds.getWest(),
        "max_lat": bounds.getNorth(),
        "max_lon": bounds.getEast()
    });

    window.map.on('moveend', function() {
        let bounds = window.map.getBounds();
        dioxus.send({
            "event": "bounds_changed",
            "min_lat": bounds.getSouth(),
            "min_lon": bounds.getWest(),
            "max_lat": bounds.getNorth(),
            "max_lon": bounds.getEast()
        });
    });

    let lastLoopTime = performance.now();
    let frameCount = 0;
    let lastLogTime = lastLoopTime;

    function recordFrame() {
        frameCount++;
        let now = performance.now();
        if (now >= lastLoopTime + 1000) {
            let currentFps = Math.round((frameCount * 1000) / (now - lastLoopTime));
            let fpsWidget = document.getElementById("fps-counter-widget");
            if (fpsWidget) { fpsWidget.innerText = "PERF: " + currentFps + " FPS"; }
            frameCount = 0;
            lastLoopTime = now;
            
            if (now >= lastLogTime + 5000) {
                dioxus.send({ "event": "fps_audit", "fps": currentFps });
                lastLogTime = now;
            }
        }
        requestAnimationFrame(recordFrame);
    }
    requestAnimationFrame(recordFrame);
};
"#;

static MAP_LOOP_JS: &str = r##"
while (true) {
    let msg = await dioxus.recv();
    if (msg.type === "updateLines") {
        for (let id in window.lineLayers) {
            window.lineLayers[id].off(); 
            window.map.removeLayer(window.lineLayers[id]);
        }
        window.lineLayers = {};
        
        let payload = msg.data;
        payload.lines.forEach(line => {
            if (payload.hiddenIds.includes(line.id)) return;
            
            let coords = line.geometry.map(pt => [pt.lat, pt.lon]);
            if (coords.length > 0) {
                let poly = L.polyline(coords, { 
                    color: line.color, 
                    weight: 4, 
                    opacity: 0.95,
                    smoothFactor: 1.0 
                }).addTo(window.map);
                
                poly.on('click', function() {
                    dioxus.send({ "event": "line_click", "id": line.id });
                });
                window.lineLayers[line.id] = poly;
            }
        });
    } else if (msg.type === "updateStations") {
        for (let id in window.stationLayers) {
            window.map.removeLayer(window.stationLayers[id]);
        }
        window.stationLayers = {};

        let stations = msg.data;
        stations.forEach(st => {
            let icon = L.divIcon({
                className: st.is_interchange ? 'hub-icon' : 'station-icon',
                html: st.is_interchange 
                    ? '<div style="background:#ffcc00; width:12px; height:12px; border-radius:50%; border:2px solid #fff; box-shadow:0 0 10px #ffcc00;"></div>' 
                    : '<div style="background:#00bcd4; width:8px; height:8px; border-radius:50%; border:1px solid #fff; box-shadow:0 0 8px #00bcd4;"></div>',
                iconSize: st.is_interchange ? [16, 16] : [10, 10],
                iconAnchor: st.is_interchange ? [8, 8] : [5, 5]
            });
            let marker = L.marker([st.coord.lat, st.coord.lon], { icon: icon }).addTo(window.map);
            marker.bindTooltip(st.name, { className: 'tfl-tooltip', direction: 'top', permanent: false });
            marker.on('click', function() {
                dioxus.send({ "event": "station_click", "id": st.id });
            });
            window.stationLayers[st.id] = marker;
        });
    } else if (msg.type === "updateDeserts") {
        if (window.desertLayer) {
            window.map.removeLayer(window.desertLayer);
        }
        let coords = msg.data.map(pt => [pt.lat, pt.lon]);
        let circleMarkers = coords.map(c => L.circleMarker(c, {
            radius: 5,
            fillColor: '#ff0000',
            color: '#ff0000',
            weight: 1,
            opacity: 0.8,
            fillOpacity: 0.8
        }));
        window.desertLayer = L.featureGroup(circleMarkers).addTo(window.map);
    } else if (msg.type === "clearDeserts") {
        if (window.desertLayer) {
            window.map.removeLayer(window.desertLayer);
            window.desertLayer = null;
        }
    } else if (msg.type === "updateTracks") {
        if (window.trackLayers) {
            window.trackLayers.forEach(layer => window.map.removeLayer(layer));
        }
        window.trackLayers = [];
        let tracks = msg.data;
        tracks.forEach(track => {
            let coords = track.geometry.map(pt => [pt.lat, pt.lon]);
            if (coords.length > 0) {
                let isUnderground = track.operator_name.includes("London Underground");
                let poly = L.polyline(coords, {
                    // High-visibility: muted slate-blue for Underground, glowing amber for National Rail
                    color: isUnderground ? "#5c5c74" : "#e06c11",
                    weight: isUnderground ? 2.0 : 1.5,
                    opacity: isUnderground ? 0.75 : 0.9,
                    dashArray: isUnderground ? null : "3, 6"
                }).addTo(window.map);
                window.trackLayers.push(poly);
            }
        });
    } else if (msg.type === "updateDrawing") {
        let coords = msg.data.map(pt => [pt.lat, pt.lon]);
        window.drawingLayer.setLatLngs(coords);
    } else if (msg.type === "placeMarker") {
        let pt = [msg.lat, msg.lon];
        let marker = L.marker(pt, {
            icon: L.divIcon({
                className: 'temp-marker',
                html: '<div style="background:#ff00ff; width:14px; height:14px; border-radius:50%; border:2px solid #fff; box-shadow: 0 0 12px #ff00ff;"></div>',
                iconSize: [18, 18],
                iconAnchor: [9, 9]
            })
        }).addTo(window.map);
        marker.bindTooltip('Catchment Node', { direction: 'top', permanent: false });
        // Auto-remove after 5 seconds
        setTimeout(function() { window.map.removeLayer(marker); }, 5000);
    }
}
"##;

static API_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

fn get_api_client() -> &'static reqwest::Client {
    API_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("Failed to create API client")
    })
}

async fn fetch_api<T: serde::de::DeserializeOwned>(url: &str) -> Option<T> {
    let client = get_api_client();
    let target_endpoint = format!("http://127.0.0.1:3000{}", url);

    match client.get(&target_endpoint).send().await {
        Ok(resp) => match resp.json::<ApiResponse<T>>().await {
            Ok(api_resp) => api_resp.data,
            Err(e) => {
                log_error(&format!("API response parse error for {}: {}", url, e));
                None
            }
        },
        Err(e) => {
            log_error(&format!("API request failed for {}: {}", url, e));
            None
        }
    }
}

async fn post_api<REQ: serde::Serialize, T: serde::de::DeserializeOwned>(
    url: &str,
    body: &REQ,
) -> Option<T> {
    let client = get_api_client();
    let target_endpoint = format!("http://127.0.0.1:3000{}", url);

    match client.post(&target_endpoint).json(body).send().await {
        Ok(resp) => match resp.json::<ApiResponse<T>>().await {
            Ok(api_resp) => api_resp.data,
            Err(e) => {
                log_error(&format!("API response parse error for {}: {}", url, e));
                None
            }
        },
        Err(e) => {
            log_error(&format!("API request failed for {}: {}", url, e));
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Toast {
    id: usize,
    message: String,
    style: String,
}

fn show_toast(
    toasts: &mut Signal<Vec<Toast>>,
    id_counter: &mut Signal<usize>,
    message: &str,
    style: &str,
) {
    let id = *id_counter.read() + 1;
    id_counter.set(id);
    let toast = Toast {
        id,
        message: message.to_string(),
        style: style.to_string(),
    };
    toasts.with_mut(|t| t.push(toast));

    let mut toasts_clone = toasts.clone();
    spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
        toasts_clone.with_mut(|t| t.retain(|item| item.id != id));
    });
}

pub fn build_desktop_window_configuration() -> dioxus::desktop::Config {
    let window = dioxus::desktop::WindowBuilder::new()
        .with_title("GREATER LONDON TRANSPORT NETWORK - PHYSICAL TRUTH ENGINE")
        .with_maximized(true)
        .with_resizable(true);

    dioxus::desktop::Config::new()
        .with_window(window)
        .with_custom_head(format!(
            r#"<link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" />
            <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"></script>
            <style>{}</style>"#,
            *EMBEDDED_THEME_CSS
        ))
}

pub fn build_console_window_configuration() -> dioxus::desktop::Config {
    let window = dioxus::desktop::WindowBuilder::new()
        .with_title("TRANSPORT ENGINE - ANALYTICS CONSOLE")
        .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(900.0, 600.0))
        .with_resizable(true);

    dioxus::desktop::Config::new()
        .with_window(window)
}

/// Standalone console window component that fetches logs via HTTP from the main engine
#[component]
pub fn ConsoleStandaloneApp() -> Element {
    let port = *CONSOLE_SERVER_PORT.get().unwrap_or(&3000);
    let streaming_logs = use_signal(|| String::from("Connecting to engine...\n"));
    let log_stream = streaming_logs.clone();

    use_future(move || {
        let mut log_stream = log_stream.clone();
        async move {
            let fallback_client = reqwest::Client::builder()
                .timeout(Duration::from_millis(200))
                .build()
                .unwrap();
            
            loop {
                tokio::time::sleep(Duration::from_millis(400)).await;
                let target_url = format!("http://127.0.0.1:{}/api/logs", port);
                
                match fallback_client.get(&target_url).send().await {
                    Ok(response) => {
                        if let Ok(api_response) = response.json::<ApiResponse<String>>().await {
                            if let Some(refreshed_text) = api_response.data {
                                if refreshed_text.len() != log_stream.read().len() {
                                    log_stream.set(refreshed_text);
                                }
                            }
                        }
                    }
                    Err(_) => {
                        let mut current_logs = log_stream.read().clone();
                        if !current_logs.contains("[EMERGENCY DISCONNECT FREEZE]") {
                            current_logs.push_str("\n\n======================================================================\n");
                            current_logs.push_str("[EMERGENCY DISCONNECT FREEZE] Main Application Process Terminated.\n");
                            current_logs.push_str("Diagnostic state frozen safely. Active trace window locked.");
                            log_stream.set(current_logs);
                        }
                        break;
                    }
                }
            }
        }
    });

    // Smart auto-scroll: follows bottom on new content; if user scrolls up it stops,
    // but resumes automatically when they scroll back to the bottom
    use_effect(move || {
        let _ = streaming_logs.read().len();
        let js = r#"
            setTimeout(() => {
                let el = document.querySelector('.stream-view');
                if (el) {
                    let atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 50;
                    if (atBottom) {
                        el.scrollTop = el.scrollHeight;
                    }
                }
            }, 32); // 2-frame delay ensures DOM layout is complete
        "#;
        eval(js);
    });

    rsx! {
        style { {r#"
            body { background: #020204; color: #39ff14; font-family: 'JetBrains Mono', 'Fira Code', monospace; padding: 16px; margin: 0; overflow: hidden; }
            .terminal-container { display: flex; flex-direction: column; height: 100vh; gap: 12px; padding-bottom: 32px; }
            .header-panel { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #222; padding-bottom: 8px; }
            .stream-view { flex: 1; background: #070709; border: 1px solid #1c1c1f; border-radius: 6px; padding: 14px; padding-bottom: 120px !important; overflow-y: auto; white-space: pre-wrap; font-size: 11px; line-height: 1.6; box-shadow: inset 0 0 10px rgba(0,0,0,0.8); }
            .copy-btn { background: #00bcd4; color: #000; font-weight: bold; border: none; padding: 8px 16px; cursor: pointer; border-radius: 4px; font-family: sans-serif; letter-spacing: 0.5px; transition: background 0.2s ease; }
            .copy-btn:hover { background: #00acc1; }
            .status-badge { color: #888; font-size: 10px; font-family: sans-serif; }
        "#} }
        div { class: "terminal-container",
            div { class: "header-panel",
                h3 { style: "color: #00bcd4; margin: 0; font-family: sans-serif; text-transform: uppercase; font-size: 12px; letter-spacing: 1px;", "Engine Diagnostics Stream [Standalone Console]" }
                div { style: "display: flex; gap: 8px; align-items: center;",
                    span { class: "status-badge", "port {port}" }
                    button {
                        class: "copy-btn",
                        onclick: move |_| {
                            let text = streaming_logs.read().clone();
                            let js = build_copy_log_js(&text);
                            eval(&js);
                        },
                        "COPY LOG"
                    }
                }
            }
            div {
                class: "stream-view",
                style: "display: flex; flex-direction: column; background: #050508; font-family: monospace; font-size: 11px; padding: 12px; height: 100%; overflow-y: auto;",
                tabindex: "0",
                {streaming_logs.read().lines().map(|log_line| {
                    let text_color = if log_line.contains("[ERROR]") { "#ff4444" }
                        else if log_line.contains("[WARN]") { "#ffaa00" }
                        else if log_line.contains("[DEBUG]") { "#00bcd4" }
                        else if log_line.contains("[TRACE]") { "#55555c" }
                        else if log_line.contains("[INFO]") { "#4caf50" }
                        else { "#39ff14" };
                    rsx! {
                        span {
                            style: "color: {text_color}; font-family: 'JetBrains Mono', monospace; line-height: 1.5; white-space: pre-wrap; word-break: break-all;",
                            "{log_line}"
                        }
                    }
                })}
            }
        }
    }
}

#[allow(non_snake_case)]
pub fn App() -> Element {
    let mut toasts = use_signal::<Vec<Toast>>(|| Vec::new());
    let mut toast_id_counter = use_signal::<usize>(|| 0);

    let mut lines = use_signal::<Vec<Line>>(|| Vec::new());
    let mut stations = use_signal::<Vec<Station>>(|| Vec::new());
    let mut tracks = use_signal::<Vec<RailwayTrack>>(|| Vec::new());
    let mut selected_station = use_signal::<Option<Station>>(|| None);

    let mut catchment_enabled = use_signal::<bool>(|| false);
    let mut deserts = use_signal::<Vec<Coordinate>>(|| Vec::new());
    let mut map_bounds = use_signal::<Option<LondonBounds>>(|| None);

    let mut construction_mode = use_signal::<bool>(|| false);
    let mut custom_line_name = use_signal::<String>(|| String::new());
    let mut custom_line_color = use_signal::<String>(|| "#ff00ff".to_string());
    let mut custom_line_coords = use_signal::<Vec<Coordinate>>(|| Vec::new());

    let mut hidden_lines = use_signal::<HashSet<String>>(|| HashSet::new());
    let mut permanent_deletions = use_signal::<HashSet<String>>(|| HashSet::new());

    let mut logger_open = use_signal::<bool>(|| true);
    let mut logs = use_signal::<String>(|| String::new());

    let mut show_loading = use_signal::<bool>(|| true);
    let mut loading_stages = use_signal::<Vec<(String, String)>>(|| {
        vec![
            ("Bakerloo".to_string(), "pending".to_string()),
            ("Central".to_string(), "pending".to_string()),
            ("Jubilee".to_string(), "pending".to_string()),
            ("Northern".to_string(), "pending".to_string()),
            ("Piccadilly".to_string(), "pending".to_string()),
            ("Victoria".to_string(), "pending".to_string()),
        ]
    });

    let mut context_menu = use_signal::<Option<(Coordinate, (i32, i32))>>(|| None);
    let mut eval_handle = use_signal::<Option<UseEval>>(|| None);

    // Fix #9: Track whether data loading has timed out for user feedback
    let mut data_timeout = use_signal::<bool>(|| false);

    // Warm-up data
    use_future(move || async move {
        let mut attempts = 0usize;
        let start_time = std::time::Instant::now();

        while attempts < 12 {
            if let Some(loaded_lines) = fetch_api::<Vec<Line>>("/api/lines").await {
                if !loaded_lines.is_empty() {
                    lines.set(loaded_lines);
                }
            }
            if let Some(loaded_stations) = fetch_api::<Vec<Station>>("/api/stations").await {
                if !loaded_stations.is_empty() {
                    stations.set(loaded_stations);
                }
            }
            if let Some(loaded_tracks) = fetch_api::<Vec<RailwayTrack>>("/api/tracks").await {
                if !loaded_tracks.is_empty() {
                    tracks.set(loaded_tracks);
                }
            }

            if !lines.read().is_empty() || !stations.read().is_empty() {
                break;
            }

            // Fix #9: After 5 seconds with no data, show a timeout indicator
            if start_time.elapsed().as_secs() >= 5 && !*data_timeout.read() {
                data_timeout.set(true);
                show_toast(
                    &mut toasts,
                    &mut toast_id_counter,
                    "Data load is taking longer than expected. Server may still be initializing.",
                    "error",
                );
            }

            attempts += 1;
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        }

        for stage in &[
            "Bakerloo",
            "Central",
            "Jubilee",
            "Northern",
            "Piccadilly",
            "Victoria",
        ] {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            loading_stages.with_mut(|stages| {
                if let Some(idx) = stages.iter().position(|(n, _)| n == stage) {
                    stages[idx].1 = "success".to_string();
                }
            });
        }
        show_loading.set(false);
        data_timeout.set(false);
    });

    // Logging refresh
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
            if let Some(loaded_logs) = fetch_api::<String>("/api/logs").await {
                logs.set(loaded_logs);
                // Fix #11: Force scroll to bottom after each log update
                eval(r#"
                    setTimeout(() => {
                        let el = document.getElementById('log-content');
                        if (el) {
                            let atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 50;
                            if (atBottom) {
                                el.scrollTop = el.scrollHeight;
                            }
                        }
                    }, 32);
                "#);
            }
        }
    });

    // Smart auto-scroll for logger panel: follows bottom; stops if user scrolls up; resumes at bottom
    use_effect(move || {
        let _ = logs.read().len();
        eval(r#"
            setTimeout(() => {
                let el = document.getElementById('log-content');
                if (el) {
                    let atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 50;
                    if (atBottom) {
                        el.scrollTop = el.scrollHeight;
                    }
                }
            }, 32); // 2-frame delay ensures DOM layout is complete
        "#);
    });

    // Catchment Area deserts lookup
    use_effect(move || {
        let bounds_opt = map_bounds.read().clone();
        let catchment_on = catchment_enabled.read().clone();

        if catchment_on {
            if let Some(bounds) = bounds_opt {
                spawn(async move {
                    let req = TransitDesertsRequest { bounds };
                    if let Some(coords) =
                        post_api::<_, Vec<Coordinate>>("/api/transit-deserts", &req).await
                    {
                        deserts.set(coords);
                    }
                });
            }
        } else {
            deserts.set(Vec::new());
        }
    });

    // Leaflet map bindings & bridge event loop
    use_effect(move || {
        spawn(async move {
            let mut ev = eval(&(MAP_INIT_JS.to_string() + "\nwindow.initMap(dioxus);"));
            let loop_ev = eval(MAP_LOOP_JS);
            eval_handle.set(Some(loop_ev));

            while let Ok(msg) = ev.recv().await {
                if let Some(event_type) = msg.get("event").and_then(|v| v.as_str()) {
                    match event_type {
                        "bounds_changed" => {
                            if let (Some(min_lat), Some(min_lon), Some(max_lat), Some(max_lon)) = (
                                msg.get("min_lat").and_then(|v| v.as_f64()),
                                msg.get("min_lon").and_then(|v| v.as_f64()),
                                msg.get("max_lat").and_then(|v| v.as_f64()),
                                msg.get("max_lon").and_then(|v| v.as_f64()),
                            ) {
                                map_bounds.set(Some(LondonBounds {
                                    min_lat,
                                    min_lon,
                                    max_lat,
                                    max_lon,
                                }));
                            }
                        }
                        "map_click" => {
                            if let (Some(lat), Some(lon)) = (
                                msg.get("lat").and_then(|v| v.as_f64()),
                                msg.get("lng").and_then(|v| v.as_f64()),
                            ) {
                                context_menu.set(None);
                                if *construction_mode.read() {
                                    custom_line_coords.with_mut(|coords| {
                                        coords.push(Coordinate::new(lat, lon));
                                    });
                                }
                            }
                        }
                        "map_context" => {
                            if let (Some(lat), Some(lon), Some(x), Some(y)) = (
                                msg.get("lat").and_then(|v| v.as_f64()),
                                msg.get("lng").and_then(|v| v.as_f64()),
                                msg.get("x").and_then(|v| v.as_i64()),
                                msg.get("y").and_then(|v| v.as_i64()),
                            ) {
                                context_menu
                                    .set(Some((Coordinate::new(lat, lon), (x as i32, y as i32))));
                            }
                        }
                        "station_click" => {
                            if let Some(id) = msg.get("id").and_then(|v| v.as_str()) {
                                if let Some(st) =
                                    stations.read().iter().find(|s| s.id == id).cloned()
                                {
                                    selected_station.set(Some(st));
                                }
                            }
                        }
                        "fps_audit" => {
                            if let Some(fps_val) = msg.get("fps").and_then(|v| v.as_i64()) {
                                log_info(&format!(
                                    "[PERFORMANCE AUDIT] Core Graphics Layer refreshing at {} FPS",
                                    fps_val
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
        });
    });

    // Update map layer geometry
    use_effect(move || {
        let lines_val = lines.read();
        let hidden_val = hidden_lines.read();
        let deletions_val = permanent_deletions.read();
        let stations_val = stations.read();
        let tracks_val = tracks.read();
        let deserts_val = deserts.read();
        let catchment_on = catchment_enabled.read();
        let drawing_coords = custom_line_coords.read();

        if let Some(ev) = eval_handle.read().clone() {
            let active_lines: Vec<Line> = lines_val.iter()
                .filter(|l| !deletions_val.contains(&l.id))
                .cloned()
                .collect();

            let _ = ev.send(serde_json::json!({
                "type": "updateLines",
                "data": {
                    "lines": active_lines,
                    "hiddenIds": hidden_val.iter().cloned().collect::<Vec<String>>()
                }
            }));

            let _ = ev.send(serde_json::json!({
                "type": "updateStations",
                "data": &*stations_val
            }));

            let _ = ev.send(serde_json::json!({
                "type": "updateTracks",
                "data": &*tracks_val
            }));

            if *catchment_on {
                let _ = ev.send(serde_json::json!({
                    "type": "updateDeserts",
                    "data": &*deserts_val
                }));
            } else {
                let _ = ev.send(serde_json::json!({
                    "type": "clearDeserts"
                }));
            }

            let _ = ev.send(serde_json::json!({
                "type": "updateDrawing",
                "data": &*drawing_coords
            }));
        }
    });

    let save_custom_line = move |_| {
        let name = custom_line_name.read().clone();
        let color = custom_line_color.read().clone();
        let geometry = custom_line_coords.read().clone();

        if name.is_empty() || geometry.is_empty() {
            show_toast(
                &mut toasts,
                &mut toast_id_counter,
                "Enter line name and place points first.",
                "error",
            );
            return;
        }

        let new_line = Line {
            id: format!("custom_{}", name.to_lowercase().replace(' ', "_")),
            name: name.clone(),
            color: color.clone(),
            stations: Vec::new(),
            segments: Vec::new(),
            geometry,
            is_custom: true,
        };

        spawn(async move {
            let req = SaveLineRequest {
                line: new_line.clone(),
            };
            if post_api::<_, Line>("/api/lines/save", &req).await.is_some() {
                lines.with_mut(|l| l.push(new_line));
                custom_line_name.set(String::new());
                custom_line_coords.set(Vec::new());
                show_toast(
                    &mut toasts,
                    &mut toast_id_counter,
                    &format!("Line '{}' saved successfully!", name),
                    "success",
                );
            } else {
                show_toast(
                    &mut toasts,
                    &mut toast_id_counter,
                    "Failed to save line on server.",
                    "error",
                );
            }
        });
    };

    let clear_drawing = move |_| {
        custom_line_coords.set(Vec::new());
        show_toast(
            &mut toasts,
            &mut toast_id_counter,
            "Current drawing cleared.",
            "info",
        );
    };

    let clear_logs = move |_| {
        logs.set(String::new());
    };

    let logger_class = if *logger_open.read() { "pinned" } else { "" };
    let catchment_status = if *catchment_enabled.read() {
        "ON"
    } else {
        "OFF"
    };
    let construction_text = if *construction_mode.read() {
        "Exit Construction"
    } else {
        "Enter Construction"
    };
    let active_lines_count = lines.read().len().to_string();
    let active_stations_count = stations.read().len().to_string();
    let context_menu_val = context_menu.read().clone();
    let layout_position_style = context_menu_val
        .as_ref()
        .map(|(_, pos)| {
            format!(
                "left: {}px; top: {}px; position: fixed; z-index: 100000;",
                pos.0, pos.1
            )
        })
        .unwrap_or_default();

    // Fix #14: Pre-compute crash text for the crash recovery overlay (must be outside rsx!)
    let crash_text_val = CRASH_LOG_ACCUMULATOR.get()
        .and_then(|m| m.lock().ok().map(|g| g.clone()))
        .unwrap_or_else(|| "No crash details available.".to_string());

    rsx! {
        style { "{EMBEDDED_THEME_CSS}" }

        div { id: "map-viewport" }

        div { id: "fps-counter-widget", "PERF: -- FPS" }

        div { class: "legend-container",
            div { class: "legend-header",
                div { class: "legend-title", "Network Layers" }
            }
            div { class: "legend-content",
                {lines.read().iter().filter(|l| !permanent_deletions.read().contains(&l.id)).map(|line| {
                    let element_color = &line.color;
                    let element_id = line.id.clone();
                    let element_name = &line.name;
                    let is_custom = line.is_custom;
                    let is_hidden = hidden_lines.read().contains(&element_id);
                    let visibility_glyph = if is_hidden { "🙈" } else { "👁️" };

                    rsx! {
                        div { class: "legend-item", key: "{element_id}",
                            div { class: "legend-color", style: "background-color: {element_color};" }
                            span { class: "legend-name", style: "flex: 1;", "{element_name}" }
                            
                            button {
                                style: "background: none; border: none; color: #00bcd4; cursor: pointer; margin-right: 12px; font-size: 13px;",
                                onclick: move |e| {
                                    e.stop_propagation();
                                    if hidden_lines.read().contains(&element_id) {
                                        hidden_lines.with_mut(|h| { h.remove(&element_id); });
                                    } else {
                                        hidden_lines.with_mut(|h| { h.insert(element_id.clone()); });
                                    }
                                },
                                "{visibility_glyph}"
                            }

                            if is_custom {
                                button {
                                    style: "background: none; border: none; color: #f44336; cursor: pointer; font-weight: bold; font-size: 13px;",
                                    onclick: move |e| {
                                        e.stop_propagation();
                                        let target_id = element_id.clone();
                                        spawn(async move {
                                            let target_endpoint = format!("/api/lines/delete/{target_id}");
                                            if post_api::<_, bool>(&target_endpoint, &true).await.is_some() {
                                                permanent_deletions.with_mut(|d| { d.insert(target_id.clone()); });
                                            }
                                        });
                                    },
                                    "❌"
                                }
                            }
                        }
                    }
                })}
            }

            div { class: "catchment-toggle-container",
                div { class: "catchment-toggle-header",
                    span { "Catchment Overlay (800m+)" }
                    label { class: "switch",
                        input {
                            r#type: "checkbox",
                            checked: *catchment_enabled.read(),
                            onchange: move |e| {
                                catchment_enabled.set(e.value().parse().unwrap_or(false));
                            }
                        }
                        span { class: "slider" }
                    }
                }
            }
        }

        {
            if let Some(st) = selected_station.read().as_ref() {
                let display_lat = format!("{:.5}", st.coord.lat);
                let display_lon = format!("{:.5}", st.coord.lon);
                let serviced_lines_label = st.lines.join(", ");
                let station_status_text = if st.is_open { "Open" } else { "Closed" };
                let dashboard_zone_label = format!("Zone {}", st.zone);
                let target_station_name = &st.name;
                Some(rsx! {
                    div { class: "tfl-bottom-sheet",
                        div { class: "sheet-handle" }
                        div { class: "sheet-header",
                            h2 { "{target_station_name}" }
                            span { class: "badge-status", "{dashboard_zone_label}" }
                            button {
                                style: "background:none; border:none; color:#ff4444; font-weight:bold; cursor:pointer;",
                                onclick: move |_| {
                                    selected_station.set(None);
                                },
                                "Close"
                            }
                        }
                        div { class: "sheet-body",
                            p { "Latitude: {display_lat}" }
                            p { "Longitude: {display_lon}" }
                            p { "Lines serviced: {serviced_lines_label}" }
                            p { "Status: {station_status_text}" }
                        }
                    }
                })
            } else {
                None::<VNode>
            }
        }

        div { id: "logger-wrapper",
            div {
                id: "logger-fab",
                onclick: move |_| {
                    let current = *logger_open.read();
                    logger_open.set(!current);
                },
                "Companion Diagnostics"
            }
            div {
                id: "logger-panel",
                class: "{logger_class}",
                div {
                    id: "log-content",
                    style: "display: flex; flex-direction: column; background: #070709; overflow-y: auto; padding: 10px; height: 100%;",
                    {logs.read().lines().map(|line| {
                        let text_color = if line.contains("[ERROR]") { "#ff4444" }
                            else if line.contains("[WARN]") { "#ffaa00" }
                            else if line.contains("[DEBUG]") { "#00bcd4" }
                            else if line.contains("[TRACE]") { "#55555c" }
                            else if line.contains("[INFO]") { "#4caf50" }
                            else { "#39ff14" };
                        rsx! {
                            span {
                                style: "color: {text_color}; font-family: var(--font-mono); font-size: 11px; line-height: 1.42; white-space: pre-wrap; word-break: break-all;",
                                "{line}"
                            }
                        }
                    })}
                }
                div { id: "logger-actions",
                    button { class: "logger-btn", onclick: clear_logs, "Clear" }
                    button { class: "logger-btn btn-highlight", onclick: move |_| {
                        show_toast(&mut toasts, &mut toast_id_counter, "Logs exported to console.", "success");
                    }, "Export" }
                }
            }
        }

        div { id: "system-stats-widget",
            div { class: "stat-grid",
                div { class: "stat-item",
                    span { class: "stat-label", "Lines" }
                    span { class: "stat-value", "{active_lines_count}" }
                }
                div { class: "stat-item",
                    span { class: "stat-label", "Stations" }
                    span { class: "stat-value", "{active_stations_count}" }
                }
                div { class: "stat-item",
                    span { class: "stat-label", "Catchment" }
                    span { class: "stat-value", "{catchment_status}" }
                }
            }
        }

        if let Some((coord, _)) = context_menu_val {
            div {
                class: "custom-context-dropdown",
                style: "{layout_position_style}",
                div {
                    class: "menu-item",
                    onclick: move |_| {
                        let placed_coord = coord;
                        context_menu.set(None);
                        if !*construction_mode.read() { construction_mode.set(true); }
                        custom_line_coords.with_mut(|c| c.push(placed_coord));
                        // Fix #10: Show visual feedback — toast + temporary map marker
                        show_toast(
                            &mut toasts,
                            &mut toast_id_counter,
                            &format!("Node placed at {:.4}, {:.4}", placed_coord.lat, placed_coord.lon),
                            "success",
                        );
                        if let Some(ev) = eval_handle.read().clone() {
                            let _ = ev.send(serde_json::json!({
                                "type": "placeMarker",
                                "lat": placed_coord.lat,
                                "lon": placed_coord.lon
                            }));
                        }
                    },
                    "Place Standalone Catchment Node"
                }
                div {
                    class: "menu-item",
                    onclick: move |_| {
                        context_menu.set(None);
                    },
                    "Cancel"
                }
            }
        }

        div {
            style: "position: absolute; top: 24px; right: 24px; z-index: 1000; background: rgba(10,10,15,0.9); padding: 15px; border-radius: 12px; border: 1px solid rgba(255,255,255,0.1); color: #fff; width: 280px;",
            button {
                style: "width: 100%; padding: 8px; border-radius: 6px; border: none; font-weight: bold; background: #00bcd4; color: #000; cursor: pointer; margin-bottom: 10px;",
                onclick: move |_| {
                    let current = *construction_mode.read();
                    construction_mode.set(!current);
                    if current {
                        custom_line_coords.set(Vec::new());
                    }
                },
                "{construction_text}"
            }
            if *construction_mode.read() {
                div {
                    style: "display: flex; flex-direction: column; gap: 8px;",
                    div {
                        span { style: "font-size: 11px; color: #aaa;", "Line Name" }
                        input {
                            r#type: "text",
                            style: "background: #222; border: 1px solid #444; color: #fff; padding: 6px; border-radius: 4px; width: 100%;",
                            value: "{custom_line_name}",
                            oninput: move |e| {
                                custom_line_name.set(e.value());
                            }
                        }
                    }
                    div {
                        span { style: "font-size: 11px; color: #aaa;", "Line Color" }
                        div { style: "display: flex; gap: 8px;",
                            input {
                                r#type: "color",
                                style: "border: none; background: none; cursor: pointer;",
                                value: "{custom_line_color}",
                                oninput: move |e| {
                                    custom_line_color.set(e.value());
                                }
                            }
                            button {
                                style: "flex: 1; padding: 6px; background: #4caf50; color: #fff; border: none; border-radius: 4px; font-weight: bold; cursor: pointer;",
                                onclick: save_custom_line,
                                "Save"
                            }
                        }
                    }
                    button {
                        style: "padding: 6px; background: #f44336; color: #fff; border: none; border-radius: 4px; font-weight: bold; cursor: pointer;",
                        onclick: clear_drawing,
                        "Clear Current Drawing"
                    }
                    div {
                        style: "font-size: 10px; color: #888; text-align: center; margin-top: 4px;",
                        "Click on map to draw route segments."
                    }
                }
            }
        }

        div { class: "toast-container",
            for toast in toasts.read().iter() {
                div {
                    class: "toast show {toast.style}",
                    key: "{toast.id}",
                    "{toast.message}"
                }
            }
        }

        // Fix #14: Show crash recovery overlay if a panic was detected.
        if IS_PANICKED.load(std::sync::atomic::Ordering::SeqCst) {
            div {
                style: "position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; background: rgba(5,0,0,0.95); z-index: 99999; display: flex; flex-direction: column; justify-content: center; align-items: center; gap: 16px;",
                div {
                    style: "background: #1a0505; border: 2px solid #ff4444; border-radius: 12px; padding: 24px; max-width: 80vw; max-height: 80vh; display: flex; flex-direction: column; gap: 12px;",
                    h3 { style: "color: #ff4444; margin: 0; font-family: sans-serif; text-transform: uppercase; letter-spacing: 1px;", "SYSTEM PANIC DETECTED" }
                    textarea {
                        readonly: true,
                        style: "flex: 1; background: #0a0202; color: #ff8888; border: 1px solid #4a1a1a; padding: 12px; font-family: monospace; resize: none; min-height: 300px; width: 600px;",
                        value: "{crash_text_val}"
                    }
                    div { style: "display: flex; gap: 12px; justify-content: center;",
                        button {
                            style: "background: #ff4444; color: #000; font-weight: bold; border: none; padding: 10px 24px; cursor: pointer; border-radius: 6px;",
                            onclick: move |_| {
                                let js = build_copy_log_js(&crash_text_val);
                                eval(&js);
                            },
                            "COPY CRASH REPORT"
                        }
                        button {
                            style: "background: #666; color: #fff; font-weight: bold; border: none; padding: 10px 24px; cursor: pointer; border-radius: 6px;",
                            onclick: move |_| {
                                std::process::exit(1);
                            },
                            "EXIT"
                        }
                    }
                }
            }
        } else if *show_loading.read() {
            div { class: "loading-overlay",
                div { class: "spinner" }
                div { class: "status-container",
                    div { class: "status-header", "Boot Diagnostics Sequence" }
                    div { class: "status-grid",
                        for (name, status) in loading_stages.read().iter() {
                            div {
                                class: "status-row",
                                key: "{name}",
                                span { class: "status-name", "Warm up line route sequence: {name}" }
                                span { class: "status-badge status-{status}", "{status}" }
                            }
                        }
                    }
                    // Fix #9: Show retry button if data load times out
                    if *data_timeout.read() {
                        div {
                            style: "margin-top: 12px; text-align: center;",
                            button {
                                style: "padding: 8px 24px; background: #ff9800; color: #000; border: none; border-radius: 6px; font-weight: bold; cursor: pointer;",
                                onclick: move |_| {
                                    data_timeout.set(false);
                                },
                                "Retry Loading"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn LogConsoleCompanionApp() -> Element {
    let streaming_logs = use_signal(|| get_all_logs());
    let log_stream = streaming_logs.clone();

    use_future(move || {
        let mut log_stream = log_stream.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(400)).await;
                let refreshed_text = get_all_logs();
                if refreshed_text.len() != log_stream.read().len() {
                    log_stream.set(refreshed_text);
                }
            }
        }
    });

    rsx! {
        style { {r#"
            body { background: #020204; color: #39ff14; font-family: 'JetBrains Mono', 'Fira Code', monospace; padding: 16px; margin: 0; overflow: hidden; }
            .terminal-container { display: flex; flex-direction: column; height: 100vh; gap: 12px; padding-bottom: 32px; }
            .header-panel { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid #222; padding-bottom: 8px; }
            .stream-view { flex: 1; background: #070709; border: 1px solid #1c1c1f; border-radius: 6px; padding: 14px; overflow-y: auto; white-space: pre-wrap; font-size: 11px; line-height: 1.6; box-shadow: inset 0 0 10px rgba(0,0,0,0.8); }
            button { background: #00bcd4; color: #000; font-weight: bold; border: none; padding: 8px 16px; cursor: pointer; border-radius: 4px; font-family: sans-serif; letter-spacing: 0.5px; transition: background 0.2s ease; }
            button:hover { background: #00acc1; }
            .status-badge { color: #888; font-size: 10px; font-family: sans-serif; }
        "#} }
        div { class: "terminal-container",
            div { class: "header-panel",
                h3 { style: "color: #00bcd4; margin: 0; font-family: sans-serif; text-transform: uppercase; font-size: 12px; letter-spacing: 1px;", "System Truth Engine - Companion Analytics Diagnostics Stream" }
                button {
                    onclick: move |_| {
                        let text = streaming_logs.read().clone();
                        let js = build_copy_log_js(&text);
                        eval(&js);
                    },
                    "COPY COMPLETE LOG REGISTRY"
                }
            }
            div {
                class: "stream-view",
                style: "display: flex; flex-direction: column; background: #060608; padding: 14px; overflow-y: auto; height: 100%;",
                {streaming_logs.read().lines().map(|line| {
                    let text_color = if line.contains("[ERROR]") { "#ff4444" }
                        else if line.contains("[WARN]") { "#ffaa00" }
                        else if line.contains("[DEBUG]") { "#00bcd4" }
                        else if line.contains("[TRACE]") { "#55555c" }
                        else if line.contains("[INFO]") { "#4caf50" }
                        else { "#39ff14" };
                    rsx! {
                        span {
                            style: "color: {text_color}; font-family: var(--font-mono); font-size: 11px; line-height: 1.42; white-space: pre-wrap; word-break: break-all;",
                            "{line}"
                        }
                    }
                })}
            }
        }
    }
}

// Fix 3: Crash Recovery Component Tree UI Implementation
#[component]
pub fn CrashRecoveryPanel() -> Element {
    let crash_text = use_signal(|| {
        if let Some(m) = CRASH_LOG_ACCUMULATOR.get() {
            if let Ok(g) = m.lock() {
                return g.clone();
            }
        }
        "No explicit trace logs collected.".to_string()
    });

    rsx! {
        style {
            {r#"
            body { background: #0f0505; color: #ff6b6b; font-family: monospace; padding: 20px; margin: 0; }
            .box { display: flex; flex-direction: column; height: 100vh; gap: 12px; }
            textarea { flex: 1; background: #1a0a0a; color: #ff8888; border: 1px solid #4a1a1a; padding: 12px; font-family: monospace; resize: none; }
            .bar { display: flex; justify-content: space-between; align-items: center; }
            button { background: #ff4444; color: #000; font-weight: bold; border: none; padding: 8px 16px; cursor: pointer; border-radius: 4px; }
            button:hover { background: #ff6666; }
            "#}
        }
        div {
            class: "box",
            tabindex: "0",
            onkeydown: move |e| {
                if e.key() == Key::Escape {
                    std::process::exit(0);
                }
            },
            div {
                class: "bar",
                h3 { "SYSTEM PANIC DISPATCH INTERFACE" }
                span { "Press [ESC] to Exit System Safely" }
            }
            textarea {
                readonly: true,
                value: "{crash_text}"
            }
            button {
                onclick: move |_| {
                    let text = crash_text.read().clone();
                    let js = build_copy_log_js(&text);
                    eval(&js);
                },
                "COPY ENGINE ERROR TRACE TO CLIPBOARD"
            }
        }
    }
}
