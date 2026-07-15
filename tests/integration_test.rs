//! Integration tests for the core data-oriented design structures.
//! Since this is a binary crate, we replicate the core algorithms inline.

extern crate alloc;
use alloc::collections::BinaryHeap;
// Silence unused-crate-dependencies for workspace deps not used in test code.
use arc_swap as _;
use async_trait as _;
use axum as _;
use bincode as _;
use bytemuck as _;
use chrono as _;
use criterion as _;
use crossbeam_channel as _;
#[cfg_attr(not(feature = "desktop"), allow(unused_imports))]
use dioxus as _;
use dirs as _;
use fastrand as _;
use geo as _;
use memmap2 as _;
use mimalloc as _;
use open as _;
use phf as _;
use r2d2 as _;
use rand as _;
use rayon as _;
use reqwest as _;
use rkyv as _;
use rstar as _;
use rusqlite as _;
use serde as _;
use serde_json as _;
use sha2 as _;
#[cfg(feature = "shuttle")]
use shuttle_axum as _;
#[cfg(feature = "shuttle")]
use shuttle_runtime as _;
use thiserror as _;
use tokio as _;
use tokio_util as _;
use toml as _;
use tower as _;
use tower_http as _;
use tracing as _;

// ── AStarNode ───────────────────────────────────────────────────────────────

/// Priority-queue node for A* search.
/// `f_cost` is stored as bits in a `u32` so that `#[derive(Ord)]` gives
/// lexicographic ordering; the heap is a max-heap so we store the bitwise
/// complement to achieve min-heap by `f_cost`.
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
struct AStarNode {
    /// Bitwise-complement of `f_cost` bits — smaller `f_cost` → larger value → higher priority.
    f_cost_inv: u32,
    /// Node index.
    idx: usize,
}

impl AStarNode {
    /// Construct from a raw `f_cost` and node `idx`.
    const fn new(f_cost: f32, idx: usize) -> Self {
        return Self {
            f_cost_inv: !f_cost.to_bits(),
            idx,
        };
    }
}

// ── RouteScratchpad ─────────────────────────────────────────────────────────

/// Reusable A* scratchpad to avoid repeated allocations.
struct RouteScratchpad {
    /// Predecessor map for path reconstruction.
    came_from: Vec<usize>,
    /// Closed-set flags.
    closed: Vec<bool>,
    /// g-cost per node.
    g_cost: Vec<f32>,
    /// Priority queue.
    heap: BinaryHeap<AStarNode>,
}

impl RouteScratchpad {
    /// Run A* from `start` to `goal` on `grid`.
    fn astar(
        &mut self,
        grid: &TransitNetworkGrid,
        start: usize,
        goal: usize,
    ) -> Vec<usize> {
        let node_count = grid.node_count;
        if start >= node_count || goal >= node_count {
            return Vec::new();
        }
        self.reset(node_count);
        let heuristic = |node_index: usize| -> f32 {
            let gx = grid.coords_x.get(goal).copied().unwrap_or(0.0);
            let gy = grid.coords_y.get(goal).copied().unwrap_or(0.0);
            let nx = grid.coords_x.get(node_index).copied().unwrap_or(0.0);
            let ny = grid.coords_y.get(node_index).copied().unwrap_or(0.0);
            let dx = gx.mul_add(-1.0, nx);
            let dy = gy.mul_add(-1.0, ny);
            return dx.hypot(dy);
        };
        if let Some(val) = self.g_cost.get_mut(start) {
            *val = 0.0;
        }
        self.heap.push(AStarNode::new(heuristic(start), start));
        while let Some(current_node) = self.heap.pop() {
            let current_idx = current_node.idx;
            if current_idx == goal {
                let mut path = Vec::new();
                let mut cur = goal;
                while cur != usize::MAX {
                    path.push(cur);
                    cur = *self.came_from.get(cur).unwrap_or(&usize::MAX);
                }
                path.reverse();
                return path;
            }
            if *self.closed.get(current_idx).unwrap_or(&true) {
                continue;
            }
            if let Some(val) = self.closed.get_mut(current_idx) {
                *val = true;
            }
            let edges = grid.get_edges(u32::try_from(current_idx).unwrap_or(0));
            let weights = grid.get_edge_weights(u32::try_from(current_idx).unwrap_or(0));
            for (&next_node, &weight) in edges.iter().zip(weights.iter()) {
                let neighbour = usize::try_from(next_node).unwrap_or(0);
                if *self.closed.get(neighbour).unwrap_or(&true) {
                    continue;
                }
                let tentative_g = weight.mul_add(
                    1.0,
                    self.g_cost.get(current_idx).copied().unwrap_or(f32::INFINITY),
                );
                let current_g = self.g_cost.get(neighbour).copied().unwrap_or(f32::INFINITY);
                if tentative_g < current_g {
                    if let Some(val) = self.came_from.get_mut(neighbour) {
                        *val = current_idx;
                    }
                    if let Some(val) = self.g_cost.get_mut(neighbour) {
                        *val = tentative_g;
                    }
                    self.heap.push(AStarNode::new(
                        heuristic(neighbour).mul_add(1.0, tentative_g),
                        neighbour,
                    ));
                }
            }
        }
        return Vec::new();
    }

    /// Allocate a new scratchpad for `node_count` nodes.
    fn new(node_count: usize) -> Self {
        return Self {
            came_from: vec![usize::MAX; node_count],
            closed: vec![false; node_count],
            g_cost: vec![f32::INFINITY; node_count],
            heap: BinaryHeap::with_capacity(256),
        };
    }

    /// Reset all per-node state.
    fn reset(&mut self, node_count: usize) {
        self.heap.clear();
        for index in 0..node_count {
            if let Some(val) = self.g_cost.get_mut(index) {
                *val = f32::INFINITY;
            }
            if let Some(val) = self.came_from.get_mut(index) {
                *val = usize::MAX;
            }
            if let Some(val) = self.closed.get_mut(index) {
                *val = false;
            }
        }
    }
}

// ── TransitNetworkGrid ──────────────────────────────────────────────────────

/// Minimal replica of the main crate's transit network grid.
struct TransitNetworkGrid {
    /// X coordinates (longitude).
    coords_x: Vec<f32>,
    /// Y coordinates (latitude).
    coords_y: Vec<f32>,
    /// CSR edge offsets.
    edge_offsets: Vec<u32>,
    /// CSR edge targets.
    edge_targets: Vec<u32>,
    /// CSR edge weights.
    edge_weights: Vec<f32>,
    /// Number of nodes.
    node_count: usize,
}

impl TransitNetworkGrid {
    /// Return edge weights for `node`.
    fn get_edge_weights(&self, node: u32) -> &[f32] {
        let start_off = self
            .edge_offsets
            .get(usize::try_from(node).unwrap_or(0))
            .copied()
            .unwrap_or(0);
        let end_off = self
            .edge_offsets
            .get(usize::try_from(node).unwrap_or(0).wrapping_add(1))
            .copied()
            .unwrap_or(0);
        let start = usize::try_from(start_off).unwrap_or(0);
        let end = usize::try_from(end_off).unwrap_or(0);
        return self.edge_weights.get(start..end).unwrap_or(&[]);
    }

    /// Return edge targets for `node`.
    fn get_edges(&self, node: u32) -> &[u32] {
        let start_off = self
            .edge_offsets
            .get(usize::try_from(node).unwrap_or(0))
            .copied()
            .unwrap_or(0);
        let end_off = self
            .edge_offsets
            .get(usize::try_from(node).unwrap_or(0).wrapping_add(1))
            .copied()
            .unwrap_or(0);
        let start = usize::try_from(start_off).unwrap_or(0);
        let end = usize::try_from(end_off).unwrap_or(0);
        return self.edge_targets.get(start..end).unwrap_or(&[]);
    }
}

// ── free functions ──────────────────────────────────────────────────────────

/// Default synthetic edge weight (metres).
const GRID_EDGE_WEIGHT: f32 = 100.0;

/// Compute squared distances from (`query_x`, `query_y`) to each node.
fn batch_distance_squared(
    query_x: f32,
    query_y: f32,
    xs: &[f32],
    ys: &[f32],
) -> Vec<f32> {
    return xs
        .iter()
        .zip(ys.iter())
        .map(|(&x_val, &y_val)| {
            let dx = query_x.mul_add(-1.0, x_val);
            let dy = query_y.mul_add(-1.0, y_val);
            return dy.mul_add(dy, dx.mul_add(dx, 0.0));
        })
        .collect();
}

/// Build a synthetic line-topology grid with `node_count` nodes.
fn build_grid(node_count: usize) -> TransitNetworkGrid {
    let mut coords_x = Vec::with_capacity(node_count);
    let mut coords_y = Vec::with_capacity(node_count);
    let mut offsets = Vec::with_capacity(node_count.wrapping_add(1));
    let mut targets = Vec::new();
    let mut weights = Vec::new();
    for idx in 0..node_count {
        let fi = f32::from(u16::try_from(idx).unwrap_or(u16::MAX));
        coords_x.push(fi.mul_add(0.001, -0.1));
        coords_y.push(fi.mul_add(0.001, 51.5));
        offsets.push(u32::try_from(targets.len()).unwrap_or(0));
        if idx > 0 {
            targets.push(u32::try_from(idx.wrapping_sub(1)).unwrap_or(0));
            weights.push(GRID_EDGE_WEIGHT);
        }
        if idx.wrapping_add(1) < node_count {
            targets.push(u32::try_from(idx.wrapping_add(1)).unwrap_or(0));
            weights.push(GRID_EDGE_WEIGHT);
        }
    }
    offsets.push(u32::try_from(targets.len()).unwrap_or(0));
    return TransitNetworkGrid {
        coords_x,
        coords_y,
        edge_offsets: offsets,
        edge_targets: targets,
        edge_weights: weights,
        node_count,
    };
}

/// Find all node indices within `radius` of (`query_x`, `query_y`).
fn find_stations_within_radius(
    grid: &TransitNetworkGrid,
    query_x: f32,
    query_y: f32,
    radius: f32,
) -> Vec<u32> {
    const MERCATOR_STRETCH: f32 = 1.6094;
    let stretched = radius.mul_add(MERCATOR_STRETCH, 0.0);
    let radius_sq = stretched.mul_add(stretched, 0.0);
    let distances =
        batch_distance_squared(query_x, query_y, &grid.coords_x, &grid.coords_y);
    return distances
        .iter()
        .enumerate()
        .filter(|&(_, &dist)| return dist <= radius_sq)
        .map(|(index, _)| return u32::try_from(index).unwrap_or(0))
        .collect();
}

// ── bytemuck POD types ──────────────────────────────────────────────────────

/// Plain-old-data spatial coordinate.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct SpatialCoordPod {
    /// Longitude.
    x: f32,
    /// Latitude.
    y: f32,
}
// SAFETY: `SpatialCoordPod` is a plain-old-data type with no invalid bit
// patterns: two `f32` values are always valid for any bit pattern.
#[expect(
    clippy::undocumented_unsafe_blocks,
    reason = "safety documented in comment above"
)]
unsafe impl bytemuck::Zeroable for SpatialCoordPod {
    fn zeroed() -> Self {
        return Self { x: 0.0, y: 0.0 };
    }
}
// SAFETY: `SpatialCoordPod` is a POD type: `Copy` + `repr(C)` with no padding.
#[expect(
    clippy::undocumented_unsafe_blocks,
    reason = "safety documented in comment above"
)]
unsafe impl bytemuck::Pod for SpatialCoordPod {}

/// Plain-old-data station record.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct StationPod {
    /// Spatial coordinate.
    coord: SpatialCoordPod,
    /// Travelcard zone.
    zone: u8,
    /// Whether this is an interchange station.
    is_interchange: u8,
    /// Explicit padding for alignment.
    _padding: [u8; 2],
    /// FNV hash of the station name.
    name_hash: u64,
}
// SAFETY: `StationPod` has no invalid bit patterns (all fields are POD).
#[expect(
    clippy::undocumented_unsafe_blocks,
    reason = "safety documented in comment above"
)]
unsafe impl bytemuck::Zeroable for StationPod {
    fn zeroed() -> Self {
        return Self {
            coord: SpatialCoordPod::zeroed(),
            zone: 0,
            is_interchange: 0,
            _padding: [0; 2],
            name_hash: 0,
        };
    }
}
// SAFETY: `StationPod` is `Copy` + `repr(C)` with explicit padding.
#[expect(
    clippy::undocumented_unsafe_blocks,
    reason = "safety documented in comment above"
)]
unsafe impl bytemuck::Pod for StationPod {}

// ════════════════════════════════════════════════════════════════════════════
// TESTS
// ════════════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::vec;

    #[test]
    fn astar_batch_distance_squared_() {
        let xs = vec![0.0, 1.0, 2.0, 3.0];
        let ys = vec![0.0, 1.0, 2.0, 3.0];
        let dists = super::batch_distance_squared(0.0, 0.0, &xs, &ys);
        assert_eq!(dists.len(), 4);
        assert!(dists.first().copied().unwrap_or(1.0).abs() < 1e-6);
        assert!((dists.get(1).copied().unwrap_or(0.0).mul_add(1.0, -2.0)).abs() < 1e-6);
        assert!((dists.get(2).copied().unwrap_or(0.0).mul_add(1.0, -8.0)).abs() < 1e-6);
        assert!((dists.get(3).copied().unwrap_or(0.0).mul_add(1.0, -18.0)).abs() < 1e-6);
    }

    #[test]
    fn astar_route_scratchpad() {
        let grid = super::build_grid(50);
        let mut scratch = super::RouteScratchpad::new(50);
        let path = scratch.astar(&grid, 0, 49);
        assert!(
            !path.is_empty(),
            "A* should find a path on a connected line graph"
        );
        assert_eq!(*path.first().unwrap_or(&usize::MAX), 0);
        assert_eq!(*path.last().unwrap_or(&usize::MAX), 49);
        assert_eq!(path.len(), 50);
    }

    #[test]
    fn astar_route_scratchpad_no_path() {
        let grid = super::build_grid(1);
        let mut scratch = super::RouteScratchpad::new(1);
        let path = scratch.astar(&grid, 0, 0);
        assert_eq!(path, vec![0]);
    }

    #[test]
    fn bytemuck_pod_casting() {
        let pod = super::StationPod {
            coord: super::SpatialCoordPod { x: -0.1, y: 51.5 },
            zone: 3,
            is_interchange: 1,
            _padding: [0; 2],
            name_hash: 0xDEAD_BEEF_CAFE_BABE,
        };
        let bytes: &[u8] = bytemuck::bytes_of(&pod);
        assert_eq!(bytes.len(), 24);
        let restored: &super::StationPod = bytemuck::from_bytes(bytes);
        assert_eq!(restored.zone, 3);
        assert_eq!(restored.name_hash, 0xDEAD_BEEF_CAFE_BABE);
        assert!((restored.coord.x.mul_add(1.0, 0.1)).abs() < 1e-6);
    }

    #[test]
    fn find_stations_within_radius_test() {
        let grid = super::build_grid(1000);
        let found = super::find_stations_within_radius(
            &grid,
            grid.coords_x.first().copied().unwrap_or(0.0),
            grid.coords_y.first().copied().unwrap_or(0.0),
            500.0,
        );
        assert!(
            !found.is_empty(),
            "Should find at least the query station itself"
        );
        assert!(found.contains(&0), "Should contain the query station index 0");
        let all = super::find_stations_within_radius(&grid, 0.0, 51.5, 1_000_000.0);
        assert_eq!(all.len(), 1000, "Huge radius should capture all 1000 stations");
    }

    #[test]
    fn transit_network_grid_construction() {
        let grid = super::build_grid(100);
        assert_eq!(grid.node_count, 100);
        assert_eq!(grid.coords_x.len(), 100);
        assert_eq!(grid.coords_y.len(), 100);
        assert_eq!(grid.edge_offsets.len(), 101);
        assert_eq!(grid.get_edges(0).len(), 1);
        assert_eq!(grid.get_edges(50).len(), 2);
        assert_eq!(grid.get_edges(99).len(), 1);
    }
}
