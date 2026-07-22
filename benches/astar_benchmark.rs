// SPDX-License-Identifier: MIT
//! Benchmarks for the core data-oriented transit algorithms.

#![allow(
    clippy::float_arithmetic,
    clippy::single_call_fn,
    clippy::missing_trait_methods,
    reason = "benchmark-only file: float math is intentional, 
    single-use bench fns are required by criterion, 
    and default trait methods are not needed here"
)]

extern crate alloc;
use alloc::collections::BinaryHeap;
use core::cmp::Ordering;
use core::hint::black_box;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

// Silence "unused crate dependency" for workspace crates not used in bench code.
use arc_swap as _;
use async_trait as _;
use axum as _;
use bincode as _;
use bytemuck as _;
use chrono as _;
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

/// Priority-queue node for A*.
#[derive(Clone, Copy)]
struct AStarNode {
    /// f-cost (g + h) for the priority queue ordering.
    f_cost: f32,
    /// Node index in the grid.
    idx: usize,
}

impl Eq for AStarNode {}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        return other
            .f_cost
            .partial_cmp(&self.f_cost)
            .unwrap_or(Ordering::Equal);
    }
}

impl PartialEq for AStarNode {
    fn eq(&self, other: &Self) -> bool {
        return self.f_cost == other.f_cost;
    }
}

impl PartialOrd for AStarNode {
    fn ge(&self, other: &Self) -> bool {
        return self.cmp(other) != Ordering::Less;
    }

    fn gt(&self, other: &Self) -> bool {
        return self.cmp(other) == Ordering::Greater;
    }

    fn le(&self, other: &Self) -> bool {
        return self.cmp(other) != Ordering::Greater;
    }

    fn lt(&self, other: &Self) -> bool {
        return self.cmp(other) == Ordering::Less;
    }

    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        return Some(self.cmp(other));
    }
}

/// Minimal `TransitNetworkGrid` replica for benchmarking.
struct BenchGrid {
    /// X coordinates (longitude) for each node.
    coords_x: Vec<f32>,
    /// Y coordinates (latitude) for each node.
    coords_y: Vec<f32>,
    /// Edge offset for each node (CSR format).
    edge_offsets: Vec<u32>,
    /// Target node indices for edges.
    edge_targets: Vec<u32>,
    /// Edge weights (distance in metres).
    edge_weights: Vec<f32>,
    /// Number of nodes.
    node_count: usize,
}

impl BenchGrid {
    /// Return the edge targets for the given node (slice into `edge_targets`).
    fn get_edge_targets(&self, node: u32) -> &[u32] {
        let start_idx = self
            .edge_offsets
            .get(usize::try_from(node).unwrap_or(0))
            .copied()
            .unwrap_or(0);
        let end_idx = self
            .edge_offsets
            .get(usize::try_from(node).unwrap_or(0).wrapping_add(1))
            .copied()
            .unwrap_or(0);
        let start = usize::try_from(start_idx).unwrap_or(0);
        let end = usize::try_from(end_idx).unwrap_or(0);
        return self.edge_targets.get(start..end).unwrap_or(&[]);
    }

    /// Return the edge weights for the given node (slice into `edge_weights`).
    fn get_edge_weights(&self, node: u32) -> &[f32] {
        let start_idx = self
            .edge_offsets
            .get(usize::try_from(node).unwrap_or(0))
            .copied()
            .unwrap_or(0);
        let end_idx = self
            .edge_offsets
            .get(usize::try_from(node).unwrap_or(0).wrapping_add(1))
            .copied()
            .unwrap_or(0);
        let start = usize::try_from(start_idx).unwrap_or(0);
        let end = usize::try_from(end_idx).unwrap_or(0);
        return self.edge_weights.get(start..end).unwrap_or(&[]);
    }
}

/// Scratchpad memory for A* search (avoids repeated allocations).
struct Scratchpad {
    /// Predecessor trace for path reconstruction.
    came_from: Vec<usize>,
    /// Closed-set marker per node.
    closed: Vec<bool>,
    /// g-cost (shortest distance from start) per node.
    g_cost: Vec<f32>,
    /// Binary-heap priority queue of frontier nodes.
    heap: BinaryHeap<AStarNode>,
}

impl Scratchpad {
    /// Run A* from `start` to `goal` on `grid`, returning the path.
    fn astar(&mut self, grid: &BenchGrid, start: usize, goal: usize) -> Vec<usize> {
        let node_count = grid.node_count;
        if start >= node_count || goal >= node_count {
            return Vec::new();
        }
        self.reset(node_count);
        if let Some(val) = self.g_cost.get_mut(start) {
            *val = 0.0;
        }
        let dx = grid.coords_x.get(start).copied().unwrap_or(0.0)
            - grid.coords_x.get(goal).copied().unwrap_or(0.0);
        let dy = grid.coords_y.get(start).copied().unwrap_or(0.0)
            - grid.coords_y.get(goal).copied().unwrap_or(0.0);
        self.heap.push(AStarNode {
            f_cost: dx.hypot(dy),
            idx: start,
        });
        while let Some(current_node) = self.heap.pop() {
            let current_idx = current_node.idx;
            if current_idx == goal {
                let mut path = Vec::new();
                let mut current = goal;
                while current != usize::MAX {
                    path.push(current);
                    current = *self.came_from.get(current).unwrap_or(&usize::MAX);
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
            let edges = grid.get_edge_targets(u32::try_from(current_idx).unwrap_or(0));
            let weights = grid.get_edge_weights(u32::try_from(current_idx).unwrap_or(0));
            for (&edge_target, &weight) in edges.iter().zip(weights.iter()) {
                let neighbour = usize::try_from(edge_target).unwrap_or(0);
                if *self.closed.get(neighbour).unwrap_or(&true) {
                    continue;
                }
                let tentative_g = self
                    .g_cost
                    .get(current_idx)
                    .copied()
                    .unwrap_or(f32::INFINITY)
                    + weight;
                let current_g = self.g_cost.get(neighbour).copied().unwrap_or(f32::INFINITY);
                if tentative_g < current_g {
                    if let Some(val) = self.came_from.get_mut(neighbour) {
                        *val = current_idx;
                    }
                    if let Some(val) = self.g_cost.get_mut(neighbour) {
                        *val = tentative_g;
                    }
                    let h_dx = grid.coords_x.get(neighbour).copied().unwrap_or(0.0)
                        - grid.coords_x.get(goal).copied().unwrap_or(0.0);
                    let h_dy_val = grid.coords_y.get(neighbour).copied().unwrap_or(0.0)
                        - grid.coords_y.get(goal).copied().unwrap_or(0.0);
                    self.heap.push(AStarNode {
                        f_cost: tentative_g + h_dx.hypot(h_dy_val),
                        idx: neighbour,
                    });
                }
            }
        }
        return Vec::new();
    }

    /// Allocate a new scratchpad for a graph with `node_count` nodes.
    fn new(node_count: usize) -> Self {
        return Self {
            came_from: vec![usize::MAX; node_count],
            closed: vec![false; node_count],
            g_cost: vec![f32::INFINITY; node_count],
            heap: BinaryHeap::with_capacity(256),
        };
    }

    /// Reset all per-node arrays back to their initial state.
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

/// Build a synthetic grid with `node_count` nodes in a line topology.
fn build_synthetic_grid(node_count: usize) -> BenchGrid {
    let mut coords_x = Vec::with_capacity(node_count);
    let mut coords_y = Vec::with_capacity(node_count);
    let mut edge_offsets = Vec::with_capacity(node_count.wrapping_add(1));
    let mut edge_targets = Vec::new();
    let mut edge_weights = Vec::new();

    for index in 0..node_count {
        let index_u32: u32 = index.try_into().unwrap_or(0);
        let f_index: f32 = f32::from(u16::try_from(index_u32).unwrap_or(u16::MAX));
        coords_x.push(f_index.mul_add(0.001, -0.1));
        coords_y.push(f_index.mul_add(0.001, 51.5));
        edge_offsets.push(edge_targets.len().try_into().unwrap_or(0));
        if index > 0 {
            edge_targets.push(index.wrapping_sub(1).try_into().unwrap_or(0));
            edge_weights.push(100.0);
        }
        if index.wrapping_add(1) < node_count {
            edge_targets.push(index.wrapping_add(1).try_into().unwrap_or(0));
            edge_weights.push(100.0);
        }
    }
    edge_offsets.push(edge_targets.len().try_into().unwrap_or(0));

    return BenchGrid {
        coords_x,
        coords_y,
        edge_offsets,
        edge_targets,
        edge_weights,
        node_count,
    };
}

/// Benchmark: batch distance squared on 10k nodes.
fn bench_batch_distance(criterion: &mut Criterion) {
    let grid = build_synthetic_grid(10_000);
    criterion.bench_function("batch_distance_squared_10k", |bencher| {
        bencher.iter(|| {
            return black_box({
                grid.coords_x
                    .iter()
                    .zip(grid.coords_y.iter())
                    .map(|(x_val, y_val)| {
                        let dx = x_val - 0.0;
                        let dy = y_val - 51.5;
                        return dy.mul_add(dy, dx * dx);
                    })
                    .collect::<Vec<f32>>()
            });
        });
    });
}

/// Benchmark: find stations within radius on 10k nodes.
fn bench_find_within_radius(criterion: &mut Criterion) {
    let grid = build_synthetic_grid(10_000);
    criterion.bench_function("find_stations_within_radius_10k", |bencher| {
        bencher.iter(|| {
            return black_box({
                const MERCATOR_STRETCH: f32 = 1.6094;
                let radius: f32 = 500.0;
                let radius_sq = radius
                    .mul_add(MERCATOR_STRETCH, 0.0)
                    .mul_add(MERCATOR_STRETCH, 0.0);
                let distances = grid
                    .coords_x
                    .iter()
                    .zip(grid.coords_y.iter())
                    .map(|(x_val, y_val)| {
                        let dx = x_val - 0.0;
                        let dy = y_val - 51.5;
                        return dy.mul_add(dy, dx.mul_add(dx, 0.0));
                    })
                    .collect::<Vec<f32>>();
                distances
                    .iter()
                    .enumerate()
                    .filter(|&(_, dist)| return *dist <= radius_sq)
                    .map(|(index, _)| return index.try_into().unwrap_or(0))
                    .collect::<Vec<u32>>()
            });
        });
    });
}

/// Benchmark: A* pathfinding on a line topology at various scales.
fn bench_astar(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("astar_line_topology");
    for &grid_size in &[100, 1_000, 10_000] {
        let grid = build_synthetic_grid(grid_size);
        let mut scratch = Scratchpad::new(grid_size);
        group.bench_with_input(
            BenchmarkId::new("pathfinding", grid_size),
            &grid_size,
            |bencher, _size| {
                bencher.iter(|| {
                    return black_box(scratch.astar(&grid, 0, grid_size.wrapping_sub(1)));
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_astar,
    bench_batch_distance,
    bench_find_within_radius,
);
criterion_main!(benches);
