//! Integration tests for the core data-oriented design structures used in the
//! London transport visualiser.
//!
//! Because this is a binary crate the production types are not importable, so
//! the key algorithms and data structures are replicated inline here.  Each
//! replica is kept intentionally minimal — just enough to exercise the logic
//! under test.
//!
//! # Test modules
//!
//! * [`tests`] — A\* routing, distance queries, radius search, and
//!   [`bytemuck`] POD casting.

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

// ── Constants ────────────────────────────────────────────────────────────────

/// Default edge weight assigned to every synthetic grid edge, in metres.
///
/// All edges in the test grid produced by [`build_grid`] are bidirectional and
/// carry this uniform cost, which simplifies path-length assertions.
const GRID_EDGE_WEIGHT: f32 = 100.0;

// ── Structs (alphabetical) ───────────────────────────────────────────────────

/// A single node in the A\* open-set priority queue.
///
/// The f-cost is stored as the **bitwise complement** of its IEEE-754 bit
/// pattern so that the standard [`BinaryHeap`] max-heap pops the node with the
/// *smallest* f-cost first, giving correct A\* behaviour without a custom
/// comparator.
///
/// # Ordering
///
/// `f_cost_inv` is the most-significant field, so the derived [`Ord`]
/// implementation compares f-costs before node indices, which is exactly the
/// priority ordering A\* requires.
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
struct AStarNode {
    /// Bitwise complement of the f-cost bits.
    ///
    /// A smaller f-cost produces a larger `f_cost_inv`, giving higher priority
    /// in the max-heap.
    f_cost_inv: u32,
    /// Index of this node in the [`TransitNetworkGrid`] node arrays.
    idx: usize,
}

/// Reusable scratch space for a single A\* search.
///
/// Allocating these vectors once and resetting them between searches avoids
/// repeated heap allocation in tight benchmark loops.  Call [`RouteScratchpad::new`]
/// to create an instance sized for a given network, then call
/// [`RouteScratchpad::astar`] as many times as needed.
struct RouteScratchpad {
    /// Predecessor map used to reconstruct the shortest path.
    ///
    /// `came_from[v]` is the node index from which `v` was first reached, or
    /// [`usize::MAX`] if `v` has not yet been visited.
    came_from: Vec<usize>,
    /// Closed-set membership flags.
    ///
    /// `closed[v]` is `true` once node `v` has been finalised and should not
    /// be relaxed again.
    closed: Vec<bool>,
    /// Best known g-cost (distance from the start) for each node.
    ///
    /// Initialised to [`f32::INFINITY`] and updated whenever a shorter path is
    /// found.
    g_cost: Vec<f32>,
    /// Open-set priority queue ordered by ascending f-cost.
    heap: BinaryHeap<AStarNode>,
}

/// Plain-old-data (POD) representation of a geographic coordinate.
///
/// Both fields are `f32` so the struct is 8 bytes with no padding, making it
/// safe to cast to and from `&[u8]` via [`bytemuck`].
///
/// # Memory layout (`repr(C)`)
///
/// | Field | Offset | Size |
/// |-------|--------|------|
/// | `x`   | 0      | 4    |
/// | `y`   | 4      | 4    |
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
#[repr(C)]
struct SpatialCoordPod {
    /// Longitude in decimal degrees (WGS-84).
    x: f32,
    /// Latitude in decimal degrees (WGS-84).
    y: f32,
}

/// Plain-old-data (POD) record for a single transit station.
///
/// The layout is carefully padded so that there are **no implicit gaps**
/// between fields, which is required by [`bytemuck::Pod`].
///
/// # Memory layout (`repr(C)`)
///
/// | Field            | Offset | Size | Notes                        |
/// |------------------|--------|------|------------------------------|
/// | `coord`          | 0      | 8    | `SpatialCoordPod`            |
/// | `zone`           | 8      | 1    | Travelcard zone 1–9          |
/// | `is_interchange` | 9      | 1    | `0` = no, `1` = yes          |
/// | `_padding`       | 10     | 6    | Explicit; fills to offset 16 |
/// | `name_hash`      | 16     | 8    | FNV-1a hash of station name  |
///
/// Total size: 24 bytes, no implicit padding.
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
#[repr(C)]
struct StationPod {
    /// Geographic position of the station entrance.
    coord: SpatialCoordPod,
    /// Travelcard zone (1–9).  Zone 1 is central London.
    zone: u8,
    /// `1` if this station is a multi-line interchange, `0` otherwise.
    is_interchange: u8,
    /// Explicit padding that fills the 6 bytes between `is_interchange` and
    /// `name_hash`, preventing any implicit compiler-inserted gaps.
    _padding: [u8; 6],
    /// FNV-1a hash of the station name, used as a compact identifier.
    name_hash: u64,
}

/// Minimal compressed-sparse-row (CSR) transit network used in tests.
///
/// Nodes are identified by contiguous `usize` indices `0..node_count`.
/// Edges are stored in CSR format: for node `v`, its outgoing edges are
/// `edge_targets[edge_offsets[v]..edge_offsets[v+1]]` with corresponding
/// weights in `edge_weights`.
///
/// # Construction
///
/// Use [`build_grid`] to create a synthetic line-topology instance.
struct TransitNetworkGrid {
    /// Longitude of each node, indexed by node index.
    coords_x: Vec<f32>,
    /// Latitude of each node, indexed by node index.
    coords_y: Vec<f32>,
    /// CSR row-pointer array.  Length is `node_count + 1`.
    edge_offsets: Vec<u32>,
    /// CSR column-index array of edge target node indices.
    edge_targets: Vec<u32>,
    /// Edge weights in metres, parallel to `edge_targets`.
    edge_weights: Vec<f32>,
    /// Total number of nodes in the network.
    node_count: usize,
}

// ── Inherent impls (alphabetical by type) ────────────────────────────────────

impl AStarNode {
    /// Construct an [`AStarNode`] from a raw f-cost and node index.
    ///
    /// # Parameters
    ///
    /// * `f_cost` — The A\* f-cost `g + h` for this node.
    /// * `idx`    — Index of the node in the network.
    ///
    /// # How the inversion works
    ///
    /// `f_cost.to_bits()` gives the IEEE-754 bit pattern.  For non-negative
    /// finite floats, bit-pattern order matches numeric order, so
    /// `!f_cost.to_bits()` reverses the order: a *smaller* f-cost produces a
    /// *larger* `f_cost_inv`, which sorts higher in the max-heap.
    const fn new(f_cost: f32, idx: usize) -> Self {
        return Self {
            f_cost_inv: !f_cost.to_bits(),
            idx,
        };
    }
}

impl RouteScratchpad {
    /// Run A\* from `start` to `goal` on `grid` and return the node-index path.
    ///
    /// Returns an empty `Vec` if either index is out of bounds or no path
    /// exists.  The returned path includes both the start and goal nodes.
    ///
    /// # Parameters
    ///
    /// * `grid`  — The network to search.
    /// * `start` — Source node index.
    /// * `goal`  — Destination node index.
    ///
    /// # Returns
    ///
    /// A `Vec<usize>` of node indices from `start` to `goal` (inclusive), or
    /// an empty `Vec` if no path was found.
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

    /// Allocate a new [`RouteScratchpad`] sized for a network with `node_count` nodes.
    ///
    /// All g-costs are initialised to [`f32::INFINITY`], all predecessor
    /// entries to [`usize::MAX`], and all closed flags to `false`.
    ///
    /// # Parameters
    ///
    /// * `node_count` — Number of nodes in the network this scratchpad will
    ///   be used with.
    fn new(node_count: usize) -> Self {
        return Self {
            came_from: vec![usize::MAX; node_count],
            closed: vec![false; node_count],
            g_cost: vec![f32::INFINITY; node_count],
            heap: BinaryHeap::with_capacity(256),
        };
    }

    /// Reset all per-node state so the scratchpad can be reused for a new search.
    ///
    /// This is cheaper than dropping and reallocating because the backing
    /// allocations are retained.
    ///
    /// # Parameters
    ///
    /// * `node_count` — Number of nodes to reset; must match the value passed
    ///   to [`RouteScratchpad::new`].
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

impl TransitNetworkGrid {
    /// Return the slice of edge weights for outgoing edges of `node`.
    ///
    /// Uses the CSR `edge_offsets` array to locate the correct sub-slice of
    /// `edge_weights`.  Returns an empty slice if `node` is out of range.
    ///
    /// # Parameters
    ///
    /// * `node` — Node index whose outgoing edge weights are requested.
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

    /// Return the slice of target node indices for outgoing edges of `node`.
    ///
    /// Uses the CSR `edge_offsets` array to locate the correct sub-slice of
    /// `edge_targets`.  Returns an empty slice if `node` is out of range.
    ///
    /// # Parameters
    ///
    /// * `node` — Node index whose outgoing edge targets are requested.
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

// ── Free functions (alphabetical) ─────────────────────────────────────────────

/// Compute the squared Euclidean distance from `(query_x, query_y)` to every
/// node in parallel coordinate arrays.
///
/// Squaring avoids a `sqrt` call; callers that only need relative ordering or
/// threshold comparisons should compare against `radius²` rather than `radius`.
///
/// # Parameters
///
/// * `query_x` — X coordinate of the query point.
/// * `query_y` — Y coordinate of the query point.
/// * `xs`      — X coordinates of the candidate nodes.
/// * `ys`      — Y coordinates of the candidate nodes; must be the same length
///   as `xs`.
///
/// # Returns
///
/// A `Vec<f32>` of length `xs.len()` where element `i` is
/// `(xs[i] - query_x)² + (ys[i] - query_y)²`.
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

/// Build a synthetic line-topology [`TransitNetworkGrid`] with `node_count` nodes.
///
/// Nodes are laid out along a straight line with coordinates
/// `x = i × 0.001 − 0.1`, `y = i × 0.001 + 51.5` for node index `i`.
/// Each interior node has two bidirectional edges (to its predecessor and
/// successor); the two terminal nodes each have one edge.  Every edge carries
/// weight [`GRID_EDGE_WEIGHT`].
///
/// # Parameters
///
/// * `node_count` — Number of nodes in the resulting grid.
///
/// # Returns
///
/// A fully initialised [`TransitNetworkGrid`] in CSR format.
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

/// Return the indices of all nodes within `radius` of `(query_x, query_y)`.
///
/// Distances are computed in the same coordinate space as the grid (decimal
/// degrees), scaled by an approximate Mercator stretch factor before squaring,
/// so the threshold is not a true metric radius but is consistent across calls.
///
/// # Parameters
///
/// * `grid`    — The network whose nodes are searched.
/// * `query_x` — Longitude of the query point.
/// * `query_y` — Latitude of the query point.
/// * `radius`  — Search radius in the same units as the coordinate arrays.
///
/// # Returns
///
/// A `Vec<u32>` of node indices whose distance from the query point is at most
/// `radius` (after Mercator scaling).
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
            _padding: [0; 6],
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
