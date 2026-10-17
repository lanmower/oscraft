//! Grid A* over the walkability bits, for routes a straight line cannot take. Integer
//! costs and octile heuristic from the flow-field cost table, so the same grid always gives
//! the same route. Ties break on the smaller cell index, which keeps replays exact.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::fixed::ONE;
use crate::pathfinding::{Grid, COST_DIAGONAL, COST_STRAIGHT, NB_COST, NB_DX, NB_DY};

/// Cells expanded before a search gives up and reports no route.
pub(crate) const ASTAR_NODE_BUDGET: usize = 4096;

/// Octile distance in path cost: the cheapest possible cost from `a` to `b`.
fn heuristic(a: (i32, i32), b: (i32, i32)) -> i64 {
    let dx = (a.0 - b.0).abs() as i64;
    let dy = (a.1 - b.1).abs() as i64;
    let (lo, hi) = if dx < dy { (dx, dy) } else { (dy, dx) };
    lo * (COST_DIAGONAL - COST_STRAIGHT) as i64 + hi * COST_STRAIGHT as i64
}

/// True when the straight segment between two cells crosses no blocked cell. Samples one point
/// per cell along the longer axis, so it never skips a cell it passes through.
pub(crate) fn line_clear(g: &Grid, from: (i32, i32), to: (i32, i32)) -> bool {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let steps = dx.abs().max(dy.abs());
    if steps == 0 {
        return g.walkable(from.0, from.1);
    }
    (0..=steps).all(|t| {
        let x = from.0 + dx * t / steps;
        let y = from.1 + dy * t / steps;
        g.walkable(x, y)
    })
}

/// A walkable route of cells from `from` to `to`, start excluded and goal included. An empty
/// route means the start is the goal. None when either end is blocked, no route exists, or the
/// search exceeds `budget` expansions. Diagonal steps never cut a blocked corner.
pub(crate) fn route(g: &Grid, from: (i32, i32), to: (i32, i32), budget: usize) -> Option<Vec<(i32, i32)>> {
    if !g.walkable(from.0, from.1) || !g.walkable(to.0, to.1) {
        return None;
    }
    if from == to {
        return Some(Vec::new());
    }
    let w = g.w as i32;
    let cell = |x: i32, y: i32| (y * w + x) as usize;
    let cells = g.w * g.h;
    let mut best = vec![i64::MAX; cells];
    let mut parent = vec![usize::MAX; cells];
    let start = cell(from.0, from.1);
    let goal = cell(to.0, to.1);
    best[start] = 0;
    let mut open = BinaryHeap::new();
    open.push(Reverse((heuristic(from, to), 0i64, start)));
    let mut expanded = 0usize;
    while let Some(Reverse((_, cost, c))) = open.pop() {
        if cost > best[c] {
            continue;
        }
        if c == goal {
            let mut out = Vec::new();
            let mut at = goal;
            while at != start {
                out.push(((at % g.w) as i32, (at / g.w) as i32));
                at = parent[at];
            }
            out.reverse();
            return Some(out);
        }
        expanded += 1;
        if expanded > budget {
            return None;
        }
        let (cx, cy) = ((c % g.w) as i32, (c / g.w) as i32);
        for k in 0..8 {
            let (nx, ny) = (cx + NB_DX[k], cy + NB_DY[k]);
            if !g.walkable(nx, ny) {
                continue;
            }
            if NB_COST[k] == COST_DIAGONAL && (!g.walkable(nx, cy) || !g.walkable(cx, ny)) {
                continue;
            }
            let n = cell(nx, ny);
            let next = cost + NB_COST[k] as i64;
            if next < best[n] {
                best[n] = next;
                parent[n] = c;
                open.push(Reverse((next + heuristic((nx, ny), to), next, n)));
            }
        }
    }
    None
}

/// Cells of route lookahead: an approach steers at most this far along its route per tick.
pub(crate) const APPROACH_LOOKAHEAD: usize = 4;

/// World point at the centre of cell `c`, in Q16.16.
pub(crate) fn cell_centre(c: (i32, i32)) -> (i32, i32) {
    (c.0 * ONE + ONE / 2, c.1 * ONE + ONE / 2)
}
