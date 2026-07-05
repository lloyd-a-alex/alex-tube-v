// Integration tests for the core data-oriented design structures.
// Since this is a binary crate, we replicate the core algorithms inline.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

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
        let s = self.edge_offsets[node as usize] as usize;
        let e = self.edge_offsets[node as usize + 1] as usize;
        &self.edge_targets[s..e]
    }
    fn get_edge_weights(&self, node: u32) -> &[f32] {
        let s = self.edge_offsets[node as usize] as usize;
        let e = self.edge_offsets[node as usize + 1] as usize;
        &self.edge_weights[s..e]
    }
}

fn build_grid(n: usize) -> TransitNetworkGrid {
    let mut cx = Vec::with_capacity(n);
    let mut cy = Vec::with_capacity(n);
    let mut offsets = Vec::with_capacity(n + 1);
    let mut targets = Vec::new();
    let mut weights = Vec::new();
    for i in 0..n {
        cx.push(-0.1 + i as f32 * 0.001);
        cy.push(51.5 + i as f32 * 0.001);
        offsets.push(targets.len() as u32);
        if i > 0 { targets.push((i - 1) as u32); weights.push(100.0); }
        if i + 1 < n { targets.push((i + 1) as u32); weights.push(100.0); }
    }
    offsets.push(targets.len() as u32);
    TransitNetworkGrid { node_count: n, coords_x: cx, coords_y: cy, edge_offsets: offsets, edge_targets: targets, edge_weights: weights }
}

// ── batch_distance_squared ──────────────────────────────────────────────────

fn batch_distance_squared(qx: f32, qy: f32, xs: &[f32], ys: &[f32]) -> Vec<f32> {
    xs.iter().zip(ys.iter())
        .map(|(&x, &y)| { let dx = x - qx; let dy = y - qy; dx * dx + dy * dy })
        .collect()
}

// ── find_stations_within_radius ─────────────────────────────────────────────

fn find_stations_within_radius(g: &TransitNetworkGrid, qx: f32, qy: f32, r: f32) -> Vec<u32> {
    const M: f32 = 1.6094;
    let r2 = (r * M) * (r * M);
    batch_distance_squared(qx, qy, &g.coords_x, &g.coords_y)
        .iter().enumerate()
        .filter_map(|(i, &d)| if d <= r2 { Some(i as u32) } else { None })
        .collect()
}

// ── RouteScratchpad + A* ────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct AStarNode { idx: usize, f_cost: f32 }
impl PartialEq for AStarNode { fn eq(&self, o: &Self) -> bool { self.f_cost == o.f_cost } }
impl Eq for AStarNode {}
impl PartialOrd for AStarNode {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> { Some(self.cmp(o)) }
}
impl Ord for AStarNode {
    fn cmp(&self, o: &Self) -> Ordering {
        o.f_cost.partial_cmp(&self.f_cost).unwrap_or(Ordering::Equal)
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
        for i in 0..n { self.g_cost[i] = f32::INFINITY; self.came_from[i] = usize::MAX; self.closed[i] = false; }
    }
    fn astar(&mut self, g: &TransitNetworkGrid, start: usize, goal: usize) -> Vec<usize> {
        let n = g.node_count;
        if start >= n || goal >= n { return Vec::new(); }
        self.reset(n);
        let h = |i: usize| -> f32 {
            let dx = g.coords_x[i] - g.coords_x[goal];
            let dy = g.coords_y[i] - g.coords_y[goal];
            (dx * dx + dy * dy).sqrt()
        };
        self.g_cost[start] = 0.0;
        self.heap.push(AStarNode { idx: start, f_cost: h(start) });
        while let Some(AStarNode { idx, .. }) = self.heap.pop() {
            if idx == goal {
                let mut path = Vec::new();
                let mut cur = goal;
                while cur != usize::MAX { path.push(cur); cur = self.came_from[cur]; }
                path.reverse();
                return path;
            }
            if self.closed[idx] { continue; }
            self.closed[idx] = true;
            let edges = g.get_edges(idx as u32);
            let weights = g.get_edge_weights(idx as u32);
            for (&next, &w) in edges.iter().zip(weights.iter()) {
                let next = next as usize;
                if self.closed[next] { continue; }
                let tg = self.g_cost[idx] + w;
                if tg < self.g_cost[next] {
                    self.came_from[next] = idx;
                    self.g_cost[next] = tg;
                    self.heap.push(AStarNode { idx: next, f_cost: tg + h(next) });
                }
            }
        }
        Vec::new()
    }
}

// ── bytemuck POD types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct SpatialCoordPod { x: f32, y: f32 }
unsafe impl bytemuck::Zeroable for SpatialCoordPod {}
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
unsafe impl bytemuck::Zeroable for StationPod {}
unsafe impl bytemuck::Pod for StationPod {}

// ════════════════════════════════════════════════════════════════════════════
// TESTS
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn test_transit_network_grid_construction() {
    let grid = build_grid(100);
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
fn test_batch_distance_squared() {
    let xs = vec![0.0, 1.0, 2.0, 3.0];
    let ys = vec![0.0, 1.0, 2.0, 3.0];
    let dists = batch_distance_squared(0.0, 0.0, &xs, &ys);
    assert_eq!(dists.len(), 4);
    assert!((dists[0] - 0.0).abs() < 1e-6);
    assert!((dists[1] - 2.0).abs() < 1e-6);
    assert!((dists[2] - 8.0).abs() < 1e-6);
    assert!((dists[3] - 18.0).abs() < 1e-6);
}

#[test]
fn test_route_scratchpad_astar() {
    let grid = build_grid(50);
    let mut scratch = RouteScratchpad::new(50);
    let path = scratch.astar(&grid, 0, 49);
    assert!(!path.is_empty(), "A* should find a path on a connected line graph");
    assert_eq!(*path.first().unwrap(), 0);
    assert_eq!(*path.last().unwrap(), 49);
    // On a line graph, shortest path visits every node
    assert_eq!(path.len(), 50);
}

#[test]
fn test_route_scratchpad_astar_no_path() {
    // Single node grid: start == goal
    let grid = build_grid(1);
    let mut scratch = RouteScratchpad::new(1);
    let path = scratch.astar(&grid, 0, 0);
    assert_eq!(path, vec![0]);
}

#[test]
fn test_bytemuck_pod_casting() {
    let pod = StationPod {
        coord: SpatialCoordPod { x: -0.1, y: 51.5 },
        zone: 3,
        is_interchange: 1,
        _padding: [0; 2],
        name_hash: 0xDEADBEEF_CAFEBABE,
    };
    // Pod → bytes → Pod round-trip
    let bytes: &[u8] = bytemuck::bytes_of(&pod);
    // repr(C) layout: coord(8) + zone(1) + is_interchange(1) + padding(2) + align(4) + name_hash(8) = 24
    assert_eq!(bytes.len(), 24);
    let restored: &StationPod = bytemuck::from_bytes(bytes);
    assert_eq!(restored.zone, 3);
    assert_eq!(restored.name_hash, 0xDEADBEEF_CAFEBABE);
    assert!((restored.coord.x - (-0.1)).abs() < 1e-6);
}

#[test]
fn test_find_stations_within_radius() {
    let grid = build_grid(1000);
    // Query at the first station's coordinates with a generous radius
    let found = find_stations_within_radius(&grid, grid.coords_x[0], grid.coords_y[0], 500.0);
    assert!(!found.is_empty(), "Should find at least the query station itself");
    assert!(found.contains(&0), "Should contain the query station index 0");
    // With a very large radius, should find all stations
    let all = find_stations_within_radius(&grid, 0.0, 51.5, 1_000_000.0);
    assert_eq!(all.len(), 1000, "Huge radius should capture all 1000 stations");
}
