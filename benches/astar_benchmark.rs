use core::hint::black_box;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

// We need to import from the main crate binary. Since this is a bench for a
// binary (not a library), we replicate the core data structures inline for
// benchmarking purposes. This avoids needing to refactor the binary into a lib.

/// Minimal `TransitNetworkGrid` replica for benchmarking.
struct BenchGrid {
    node_count: usize,
    coords_x: Vec<f32>,
    coords_y: Vec<f32>,
    edge_offsets: Vec<u32>,
    edge_targets: Vec<u32>,
    edge_weights: Vec<f32>,
}

impl BenchGrid {
    fn get_edges(&self, node: u32) -> &[u32] {
        let s = self.edge_offsets[node as usize] as usize;
        let e = self.edge_offsets[node as usize + 1] as usize;
        return &self.edge_targets[s..e]
    }
    fn get_edge_weights(&self, node: u32) -> &[f32] {
        let s = self.edge_offsets[node as usize] as usize;
        let e = self.edge_offsets[node as usize + 1] as usize;
        return &self.edge_weights[s..e]
    }
}

/// Build a synthetic grid with `n` nodes in a line topology.
fn build_synthetic_grid(n: usize) -> BenchGrid {
    let mut coords_x = Vec::with_capacity(n);
    let mut coords_y = Vec::with_capacity(n);
    let mut edge_offsets = Vec::with_capacity(n + 1);
    let mut edge_targets = Vec::new();
    let mut edge_weights = Vec::new();

    for i in 0..n {
        coords_x.push((i as f32).mul_add(0.001, -0.1));
        coords_y.push((i as f32).mul_add(0.001, 51.5));
        edge_offsets.push(edge_targets.len() as u32);
        if i > 0 {
            edge_targets.push((i - 1) as u32);
            edge_weights.push(100.0);
        }
        if i + 1 < n {
            edge_targets.push((i + 1) as u32);
            edge_weights.push(100.0);
        }
    }
    edge_offsets.push(edge_targets.len() as u32);

    return BenchGrid {
        node_count: n,
        coords_x,
        coords_y,
        edge_offsets,
        edge_targets,
        edge_weights,
    }
}

fn batch_distance_squared(qx: f32, qy: f32, xs: &[f32], ys: &[f32]) -> Vec<f32> {
    xs.iter()
        .zip(ys.iter())
        .map(|(&x, &y)| {
            let dx = x - qx;
            let dy = y - qy;
            dy.mul_add(dy, dx * dx)
        })
        .collect()
}

fn find_stations_within_radius(grid: &BenchGrid, qx: f32, qy: f32, radius: f32) -> Vec<u32> {
    const MERCATOR_STRETCH: f32 = 1.6094;
    let r2 = (radius * MERCATOR_STRETCH) * (radius * MERCATOR_STRETCH);
    let dists = batch_distance_squared(qx, qy, &grid.coords_x, &grid.coords_y);
    dists
        .iter()
        .enumerate()
        .filter_map(|(i, &d)| (d <= r2).then_some(i as u32))
        .collect()
}

use core::cmp::Ordering;
/// Minimal A* scratchpad for benchmarking.
use std::collections::BinaryHeap;

#[derive(Clone, Copy)]
struct AStarNode {
    idx: usize,
    f_cost: f32,
}
impl PartialEq for AStarNode {
    fn eq(&self, o: &Self) -> bool {
        return self.f_cost == o.f_cost
    }
}
impl Eq for AStarNode {}
impl PartialOrd for AStarNode {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        return Some(self.cmp(o))
    }
}
impl Ord for AStarNode {
    fn cmp(&self, o: &Self) -> Ordering {
        return o.f_cost
            .partial_cmp(&self.f_cost)
            .unwrap_or(Ordering::Equal)
    }
}

struct Scratchpad {
    heap: BinaryHeap<AStarNode>,
    g_cost: Vec<f32>,
    came_from: Vec<usize>,
    closed: Vec<bool>,
}

impl Scratchpad {
    fn new(n: usize) -> Self {
        return Self {
            heap: BinaryHeap::with_capacity(256),
            g_cost: vec![f32::INFINITY; n],
            came_from: vec![usize::MAX; n],
            closed: vec![false; n],
        }
    }

    fn reset(&mut self, n: usize) {
        self.heap.clear();
        for i in 0..n {
            self.g_cost[i] = f32::INFINITY;
            self.came_from[i] = usize::MAX;
            self.closed[i] = false;
        }
    }

    fn astar(&mut self, grid: &BenchGrid, start: usize, goal: usize) -> Vec<usize> {
        let n = grid.node_count;
        if start >= n || goal >= n {
            return Vec::new();
        }
        self.reset(n);
        let h = |i: usize| -> f32 {
            let dx = grid.coords_x[i] - grid.coords_x[goal];
            let dy = grid.coords_y[i] - grid.coords_y[goal];
            dx.hypot(dy)
        };
        self.g_cost[start] = 0.0;
        self.heap.push(AStarNode {
            idx: start,
            f_cost: h(start),
        });
        while let Some(AStarNode { idx, .. }) = self.heap.pop() {
            if idx == goal {
                let mut path = Vec::new();
                let mut cur = goal;
                while cur != usize::MAX {
                    path.push(cur);
                    cur = self.came_from[cur];
                }
                path.reverse();
                return path;
            }
            if self.closed[idx] {
                continue;
            }
            self.closed[idx] = true;
            let edges = grid.get_edges(idx as u32);
            let weights = grid.get_edge_weights(idx as u32);
            for (&next, &w) in edges.iter().zip(weights.iter()) {
                let next = next as usize;
                if self.closed[next] {
                    continue;
                }
                let tg = self.g_cost[idx] + w;
                if tg < self.g_cost[next] {
                    self.came_from[next] = idx;
                    self.g_cost[next] = tg;
                    self.heap.push(AStarNode {
                        idx: next,
                        f_cost: tg + h(next),
                    });
                }
            }
        }
        return Vec::new()
    }
}

fn bench_batch_distance(c: &mut Criterion) {
    let grid = build_synthetic_grid(10_000);
    c.bench_function("batch_distance_squared_10k", |b| {
        b.iter(|| {
            return black_box(batch_distance_squared(
                0.0,
                51.5,
                &grid.coords_x,
                &grid.coords_y,
            ))
        })
    });
}

fn bench_find_within_radius(c: &mut Criterion) {
    let grid = build_synthetic_grid(10_000);
    c.bench_function("find_stations_within_radius_10k", |b| {
        b.iter(|| return black_box(find_stations_within_radius(&grid, 0.0, 51.5, 500.0)))
    });
}

fn bench_astar(c: &mut Criterion) {
    let mut group = c.benchmark_group("astar_line_topology");
    for size in [100, 1_000, 10_000] {
        let grid = build_synthetic_grid(size);
        let mut scratch = Scratchpad::new(size);
        group.bench_with_input(BenchmarkId::new("pathfinding", size), &size, |b, _| {
            b.iter(|| return black_box(scratch.astar(&grid, 0, size - 1)))
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_batch_distance,
    bench_find_within_radius,
    bench_astar,
);
criterion_main!(benches);
