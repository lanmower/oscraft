use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::fixed::ONE;
use crate::pathfinding::{Grid, COST_DIAGONAL, COST_STRAIGHT, NB_COST, NB_DX, NB_DY};

pub(crate) const ASTAR_NODE_BUDGET: usize = 4096;

pub(crate) const ASTAR_MAX_SPAN: i64 = 64;

#[derive(Default)]
struct Scratch {
    stamp: Vec<u32>,
    best: Vec<i64>,
    parent: Vec<u32>,
    epoch: u32,
}

thread_local! {
    static SCRATCH: RefCell<Scratch> = RefCell::new(Scratch::default());
}

pub(crate) fn span(a: (i32, i32), b: (i32, i32)) -> i64 {
    (a.0 - b.0).abs().max((a.1 - b.1).abs()) as i64
}

fn heuristic(a: (i32, i32), b: (i32, i32)) -> i64 {
    let dx = (a.0 - b.0).abs() as i64;
    let dy = (a.1 - b.1).abs() as i64;
    let (lo, hi) = if dx < dy { (dx, dy) } else { (dy, dx) };
    lo * (COST_DIAGONAL - COST_STRAIGHT) as i64 + hi * COST_STRAIGHT as i64
}

pub(crate) fn line_clear(g: &Grid, from: (i32, i32), to: (i32, i32)) -> bool {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let steps = dx.abs().max(dy.abs());
    if steps == 0 {
        return g.walkable(from.0, from.1);
    }
    let mut prev = from;
    (0..=steps).all(|t| {
        let x = from.0 + dx * t / steps;
        let y = from.1 + dy * t / steps;
        let corner_ok = (x == prev.0 || y == prev.1) || (g.walkable(x, prev.1) && g.walkable(prev.0, y));
        prev = (x, y);
        corner_ok && g.walkable(x, y)
    })
}

pub(crate) fn route(g: &Grid, from: (i32, i32), to: (i32, i32), budget: usize) -> Option<Vec<(i32, i32)>> {
    if !g.walkable(from.0, from.1) || !g.walkable(to.0, to.1) {
        return None;
    }
    if from == to {
        return Some(Vec::new());
    }
    SCRATCH.with(|s| search(&mut s.borrow_mut(), g, from, to, budget))
}

fn search(sc: &mut Scratch, g: &Grid, from: (i32, i32), to: (i32, i32), budget: usize) -> Option<Vec<(i32, i32)>> {
    let w = g.w as i32;
    let cell = |x: i32, y: i32| (y * w + x) as usize;
    let cells = g.w * g.h;
    if sc.stamp.len() != cells {
        sc.stamp = vec![0; cells];
        sc.best = vec![0; cells];
        sc.parent = vec![0; cells];
        sc.epoch = 0;
    }
    sc.epoch = sc.epoch.wrapping_add(1);
    if sc.epoch == 0 {
        sc.stamp.fill(0);
        sc.epoch = 1;
    }
    let epoch = sc.epoch;
    let start = cell(from.0, from.1);
    let goal = cell(to.0, to.1);
    sc.stamp[start] = epoch;
    sc.best[start] = 0;
    let mut open = BinaryHeap::new();
    open.push(Reverse((heuristic(from, to), 0i64, start)));
    let mut expanded = 0usize;
    while let Some(Reverse((_, cost, c))) = open.pop() {
        if cost > sc.best[c] {
            continue;
        }
        if c == goal {
            let mut out = Vec::new();
            let mut at = goal;
            while at != start {
                out.push(((at % g.w) as i32, (at / g.w) as i32));
                at = sc.parent[at] as usize;
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
            if sc.stamp[n] != epoch || next < sc.best[n] {
                sc.stamp[n] = epoch;
                sc.best[n] = next;
                sc.parent[n] = c as u32;
                open.push(Reverse((next + heuristic((nx, ny), to), next, n)));
            }
        }
    }
    None
}

pub(crate) const APPROACH_LOOKAHEAD: usize = 4;

pub(crate) fn cell_centre(c: (i32, i32)) -> (i32, i32) {
    (c.0 * ONE + ONE / 2, c.1 * ONE + ONE / 2)
}
