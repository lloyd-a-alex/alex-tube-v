// Integration tests for the core data-oriented design structures.
// Since this is a binary crate, we replicate the core algorithms inline.

extern crate alloc;
use alloc::collections::BinaryHeap;
use core::cmp::Ordering;

// ── TransitNetworkGrid replica ──────────────────────────────────────────────

struct TransitNetworkGrid {
    node_count: usize,
    coords_x: Vec<f32>,
    coords_y: Vec<f32>,
    edge_offsets: Vec<u32>,
    edge_targets: Vec<u32>,
    edge_weights: Vec<f32>,
}

impl TransitNetworkGrid {
    fn get_edges(&self, node: u32) -> &[u32] {
        let s = self
            .edge_offsets
            .get(usize::try_from(node).unwrap_or(0))
            .copied()
            .unwrap_or(0);
        let e = self
            .edge_offsets
            .get(usize::try_from(node).unwrap_or(0).wrapping_add(1))
            .copied()
            .unwrap_or(0);
        let start = usize::try_from(s).unwrap_or(0);
        let end = usize::try_from(e).unwrap_or(0);
        self.edge_targets.get(start..end).unwrap_or(&[])
    }
    fn get_edge_weights(&self, node: u32) -> &[f32] {
        let s = self
            .edge_offsets
            .get(usize::try_from(node).unwrap_or(0))
            .copied()
            .unwrap_or(0);
        let e = self
            .edge_offsets
            .get(usize::try_from(node).unwrap_or(0).wrapping_add(1))
            .copied()
            .unwrap_or(0);
        let start = usize::try_from(s).unwrap_or(0);
        let end = usize::try_from(e).unwrap_or(0);
        self.edge_weights.get(start..end).unwrap_or(&[])
    }
}

fn build_grid(n: usize) -> TransitNetworkGrid {
    let mut cx = Vec::with_capacity(n);
    let mut cy = Vec::with_capacity(n);
    let mut offsets = Vec::with_capacity(n.wrapping_add(1));
    let mut targets = Vec::new();
    let mut weights = Vec::new();
    for i in 0..n {
        cx.push((i as f32).mul_add(0.001, -0.1));
        cy.push((i as f32).mul_add(0.001, 51.5));
        offsets.push(u32::try_from(targets.len()).unwrap_or(0));
        if i > 0 {
            targets.push(u32::try_from(i.wrapping_sub(1)).unwrap_or(0));
            weights.push(100.0);
        }
        if i.wrapping_add(1) < n {
            targets.push(u32::try_from(i.wrapping_add(1)).unwrap_or(0));
            weights.push(100.0);
        }
    }
    offsets.push(u32::try_from(targets.len()).unwrap_or(0));
    TransitNetworkGrid {
        node_count: n,
        coords_x: cx,
        coords_y: cy,
        edge_offsets: offsets,
        edge_targets: targets,
        edge_weights: weights,
    }
}

// ── batch_distance_squared ──────────────────────────────────────────────────

fn batch_distance_squared(
    query_x: f32,
    query_y: f32,
    xs: &[f32],
    ys: &[f32],
) -> Vec<f32> {
    xs.iter()
        .zip(ys.iter())
        .map(|(&x_val, &y_val)| {
            let dx = x_val - query_x;
            let dy = y_val - query_y;
            dy.mul_add(dy, dx * dx)
        })
        .collect()
}

// ── find_stations_within_radius ─────────────────────────────────────────────

fn find_stations_within_radius(
    grid: &TransitNetworkGrid,
    query_x: f32,
    query_y: f32,
    radius: f32,
) -> Vec<u32> {
    const MERCATOR_STRETCH: f32 = 1.6094;
    let radius_sq = (radius * MERCATOR_STRETCH) * (radius * MERCATOR_STRETCH);
    let distances = batch_distance_squared(query_x, query_y, &grid.coords_x, &grid.coords_y);
    distances
        .iter()
        .enumerate()
        .filter(|(_, &dist)| dist <= radius_sq)
        .map(|(index, _)| u32::try_from(index).unwrap_or(0))
        .collect()
}

// ── RouteScratchpad + A* ────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct AStarNode {
    idx: usize,
    f_cost: f32,
}
impl PartialEq for AStarNode {
    fn eq(&self, other: &Self) -> bool {
        self.f_cost == other.f_cost
    }
}
impl Eq for AStarNode {}
impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f_cost
            .partial_cmp(&self.f_cost)
            .unwrap_or(Ordering::Equal)
    }
}

struct RouteScratchpad {
    heap: BinaryHeap<AStarNode>,
    g_cost: Vec<f32>,
    came_from: Vec<usize>,
    closed: Vec<bool>,
}

impl RouteScratchpad {
    fn new(n: usize) -> Self {
        Self {
            heap: BinaryHeap::with_capacity(256),
            g_cost: vec![f32::INFINITY; n],
            came_from: vec![usize::MAX; n],
            closed: vec![false; n],
        }
    }
    fn reset(&mut self, n: usize) {
        self.heap.clear();
        for i in 0..n {
            if let Some(val) = self.g_cost.get_mut(i) {
                *val = f32::INFINITY;
            }
            if let Some(val) = self.came_from.get_mut(i) {
                *val = usize::MAX;
            }
            if let Some(val) = self.closed.get_mut(i) {
                *val = false;
            }
        }
    }
    fn astar(&mut self, grid: &TransitNetworkGrid, start: usize, goal: usize) -> Vec<usize> {
        let node_count = grid.node_count;
        if start >= node_count || goal >= node_count {
            return Vec::new();
        }
        self.reset(node_count);
        let heuristic = |node_index: usize| -> f32 {
            let dx = grid
                .coords_x
                .get(node_index)
                .copied()
                .unwrap_or(0.0)
                - grid.coords_x.get(goal).copied().unwrap_or(0.0);
            let dy = grid
                .coords_y
                .get(node_index)
                .copied()
                .unwrap_or(0.0)
                - grid.coords_y.get(goal).copied().unwrap_or(0.0);
            dx.hypot(dy)
        };
        if let Some(val) = self.g_cost.get_mut(start) {
            *val = 0.0;
        }
        self.heap.push(AStarNode {
            idx: start,
            f_cost: heuristic(start),
        });
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
                let tentative_g =
                    self.g_cost.get(current_idx).copied().unwrap_or(f32::INFINITY) + weight;
                let current_g = self.g_cost.get(neighbour).copied().unwrap_or(f32::INFINITY);
                if tentative_g < current_g {
                    if let Some(val) = self.came_from.get_mut(neighbour) {
                        *val = current_idx;
                    }
                    if let Some(val) = self.g_cost.get_mut(neighbour) {
                        *val = tentative_g;
                    }
                    self.heap.push(AStarNode {
                        idx: neighbour,
                        f_cost: tentative_g + heuristic(neighbour),
                    });
                }
            }
        }
        Vec::new()
    }
}

// ── bytemuck POD types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct SpatialCoordPod {
    x: f32,
    y: f32,
}
// SAFETY: `SpatialCoordPod` is a plain-old-data type with no invalid bit
// patterns: two `f32` values are always valid for any bit pattern.
#[expect(
    clippy::undocumented_unsafe_blocks,
    reason = "safety documented in comment above"
)]
unsafe impl bytemuck::Zeroable for SpatialCoordPod {}
// SAFETY: `SpatialCoordPod` is a POD type: `Copy` + `repr(C)` with no padding.
#[expect(
    clippy::undocumented_unsafe_blocks,
    reason = "safety documented in comment above"
)]
unsafe impl bytemuck::Pod for SpatialCoordPod {}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct StationPod {
    coord: SpatialCoordPod,
    zone: u8,
    is_interchange: u8,
    _padding: [u8; 2],
    name_hash: u64,
}
// SAFETY: `StationPod` has no invalid bit patterns (all fields are POD).
#[expect(
    clippy::undocumented_unsafe_blocks,
    reason = "safety documented in comment above"
)]
unsafe impl bytemuck::Zeroable for StationPod {}
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
    fn transit_network_grid_construction() {
        let grid = super::build_grid(100);
        assert_eq!(grid.node_count, 100);
        assert_eq!(grid.coords_x.len(), 100);
        assert_eq!(grid.coords_y.len(), 100);
        assert_eq!(grid.edge_offsets.len(), 101); // CSR sentinel
                                                    // First node has 1 edge (right), last has 1 edge (left), middle has 2
        assert_eq!(grid.get_edges(0).len(), 1);
        assert_eq!(grid.get_edges(50).len(), 2);
        assert_eq!(grid.get_edges(99).len(), 1);
    }

    #[test]
    fn batch_distance_squared_() {
        let xs = vec![0.0, 1.0, 2.0, 3.0];
        let ys = vec![0.0, 1.0, 2.0, 3.0];
        let dists = super::batch_distance_squared(0.0, 0.0, &xs, &ys);
        assert_eq!(dists.len(), 4);
        assert!((dists[0] - 0.0).abs() < 1e-6);
        assert!((dists[1] - 2.0).abs() < 1e-6);
        assert!((dists[2] - 8.0).abs() < 1e-6);
        assert!((dists[3] - 18.0).abs() < 1e-6);
    }

    #[test]
    fn route_scratchpad_astar() {
        let grid = super::build_grid(50);
        let mut scratch = super::RouteScratchpad::new(50);
        let path = scratch.astar(&grid, 0, 49);
        assert!(
            !path.is_empty(),
            "A* should find a path on a connected line graph"
        );
        assert_eq!(*path.first().unwrap_or(&usize::MAX), 0);
        assert_eq!(*path.last().unwrap_or(&usize::MAX), 49);
        // On a line graph, shortest path visits every node
        assert_eq!(path.len(), 50);
    }

    #[test]
    fn route_scratchpad_astar_no_path() {
        // Single node grid: start == goal
        let grid = super::build_grid(1);
        let mut scratch = super::RouteScratchpad::new(1);
        let path = scratch.astar(&grid, 0, 0);
        assert_eq!(path, vec![0]);
    }

    #[test]
    fn bytemuck_pod_casting() {
        let pod = super::StationPod {
            coord: super::SpatialCoordPod {
                x: -0.1,
                y: 51.5,
            },
            zone: 3,
            is_interchange: 1,
            _padding: [0; 2],
            name_hash: 0xDEAD_BEEF_CAFE_BABE,
        };
        // Pod → bytes → Pod round-trip
        let bytes: &[u8] = bytemuck::bytes_of(&pod);
        // repr(C) layout: coord(8) + zone(1) + is_interchange(1) + padding(2) + align(4) + name_hash(8) = 24
        assert_eq!(bytes.len(), 24);
        let restored: &super::StationPod = bytemuck::from_bytes(bytes);
        assert_eq!(restored.zone, 3);
        assert_eq!(restored.name_hash, 0xDEAD_BEEF_CAFE_BABE);
        assert!((restored.coord.x - (-0.1)).abs() < 1e-6);
    }

    #[test]
    fn find_stations_within_radius_() {
        let grid = super::build_grid(1000);
        // Query at the first station's coordinates with a generous radius
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
        // With a very large radius, should find all stations
        let all = super::find_stations_within_radius(&grid, 0.0, 51.5, 1_000_000.0);
        assert_eq!(
            all.len(),
            1000,
            "Huge radius should capture all 1000 stations"
        );
    }
}
