//! Waypoint routes and destination requests. A route is the ordered list of points one player's units
//! walk for one route id (data, the ROUT section). A unit that finishes its route, or that a spawn puts on
//! the map with a route id, asks for a destination: the sim queues a destination request (unit, player)
//! and the host answers it with a move order. Choosing that destination is mod code, not engine rule.
//! Route id and step are per-unit sim state, hashed and saved. Requests are transient: drained each step.

use crate::arrays::{dist2, Sim};
use crate::fixed;
use crate::loader::{LoadError, CELL_MILLI};
use crate::*;

/// Waypoint record size in the ROUT section: player, route id, count, reserved, then 64 x/y pairs.
const ROUT_RECORD: usize = 8 + MAX_ROUTE_POINTS * 8;
pub const MAX_ROUTE_POINTS: usize = 64;
pub const MAX_ROUTE_ID: u16 = 3;
/// Distance to a waypoint, in milli-units, at which a unit moves on to the next one.
pub const ROUTE_REACH_MILLI: i32 = 3000;
/// Destination requests the log holds between drains. A host that drains every step never reaches it.
const REQUEST_CAP: usize = 1024;

/// The waypoints of one route id for one player, in milli-units.
#[derive(Clone, Debug, Default)]
pub struct Route {
    pub player: u16,
    pub id: u16,
    pub points: Vec<(i32, i32)>,
}

/// Reads the ROUT section: `(off, count)` locates it in `bytes`; `w` x `h` is the grid and `flags` its cells.
pub(crate) fn parse_routes(bytes: &[u8], (off, count): (usize, usize), w: usize, h: usize, flags: &[u8]) -> Result<Vec<Route>, LoadError> {
    let mut out: Vec<Route> = Vec::with_capacity(count);
    for n in 0..count {
        let at = off + n * ROUT_RECORD;
        let rec = bytes.get(at..at + ROUT_RECORD).ok_or(LoadError::Truncated("ROUT record"))?;
        let u16_at = |k: usize| u16::from_le_bytes([rec[k], rec[k + 1]]);
        let u32_at = |k: usize| u32::from_le_bytes([rec[k], rec[k + 1], rec[k + 2], rec[k + 3]]);
        let (player, id, points) = (u16_at(0), u16_at(2), u16_at(4) as usize);
        if player as usize >= MAX_PLAYERS || id == 0 || id > MAX_ROUTE_ID || points == 0 || points > MAX_ROUTE_POINTS {
            return Err(LoadError::Range("route player, id or point count"));
        }
        if out.iter().any(|r| r.player == player && r.id == id) {
            return Err(LoadError::Range("route repeats a player and id"));
        }
        let mut list = Vec::with_capacity(points);
        for k in 0..MAX_ROUTE_POINTS {
            let (x, y) = (u32_at(8 + k * 8) as usize, u32_at(12 + k * 8) as usize);
            if k >= points {
                if x != 0 || y != 0 {
                    return Err(LoadError::Range("route unused point is not zero"));
                }
                continue;
            }
            let (cx, cy) = (x / CELL_MILLI as usize, y / CELL_MILLI as usize);
            if cx >= w || cy >= h || flags[cy * w + cx] & 1 != 0 {
                return Err(LoadError::Range("route point off the grid or on a blocked cell"));
            }
            list.push((x as i32, y as i32));
        }
        out.push(Route { player, id, points: list });
    }
    Ok(out)
}

/// Per-unit route progress (the route id being followed, 0 for none, and the next waypoint index), and the
/// transient log of destination requests.
#[derive(Clone, Debug)]
pub(crate) struct DestState {
    pub(crate) id: Vec<u8>,
    pub(crate) step: Vec<u8>,
    /// (unit, player) pairs whose unit needs a destination since the host last drained the log.
    pub(crate) requests: Vec<(u32, u8)>,
}

impl DestState {
    pub(crate) fn new(cap: usize) -> DestState {
        DestState { id: vec![0; cap], step: vec![0; cap], requests: Vec::new() }
    }
}

impl Sim {
    pub(crate) fn route_clear(&mut self, i: usize) {
        self.dests.id[i] = 0;
        self.dests.step[i] = 0;
    }

    fn route_point(&self, player: u8, id: u8, step: usize) -> Option<(i32, i32, bool)> {
        let route = self.cat.routes.iter().find(|r| r.player == player as u16 && r.id == id as u16)?;
        let (x, y) = *route.points.get(step)?;
        Some((fixed::from_milli(x), fixed::from_milli(y), step + 1 == route.points.len()))
    }

    /// Asks the host for a destination for unit `j` of `player`. The request is queued, not answered here.
    pub(crate) fn request_destination(&mut self, j: usize, player: u8) {
        if self.dests.requests.len() < REQUEST_CAP {
            self.dests.requests.push((j as u32, player));
        }
    }

    /// Takes the destination requests since the last call, oldest first, as (unit, player).
    pub fn take_destination_requests(&mut self) -> Vec<(usize, u8)> {
        std::mem::take(&mut self.dests.requests).into_iter().map(|(u, p)| (u as usize, p)).collect()
    }

    /// Generic waypoint walker: advances unit `i` along its route: on reaching the current waypoint, the next
    /// one is ordered, or after the last the unit asks for a destination. A changed order ends the route. The
    /// walker carries no lane or content rule; which destination a request gets is the host's on_march.
    pub(crate) fn walk_waypoint(&mut self, i: usize) {
        let id = self.dests.id[i];
        if id == 0 {
            return;
        }
        let step = self.dests.step[i] as usize;
        let player = self.player[i];
        let Some((x, y, last)) = self.route_point(player, id, step) else {
            self.route_clear(i);
            return;
        };
        if self.order_kind[i] != ORDER_ATTACK_MOVE || self.tgt_x[i] != x || self.tgt_y[i] != y {
            self.route_clear(i);
            return;
        }
        let reach = fixed::from_milli(ROUTE_REACH_MILLI) as i64;
        if dist2(self.pos_x[i], self.pos_y[i], x, y) > reach * reach {
            return;
        }
        if last {
            self.route_clear(i);
            self.request_destination(i, player);
            return;
        }
        if let Some((nx, ny, _)) = self.route_point(player, id, step + 1) {
            if self.issue_order_kind(i, ORDER_ATTACK_MOVE, nx, ny).is_ok() {
                self.dests.step[i] = (step + 1) as u8;
            }
        }
    }
}
